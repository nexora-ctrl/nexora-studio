//! WASAPI capture-stream opening for the recording mixer.
//!
//! This module turns a [`crate::audio::bus::SourceBusSpec`] (or one of the two global mixer
//! buses) into a live WASAPI capture stream:
//!
//! * **Full-desktop loopback** — `AUDCLNT_STREAMFLAGS_LOOPBACK` on a render endpoint.
//! * **Capture input** — a direct capture endpoint stream.
//! * **Per-process loopback** — `ActivateAudioInterfaceAsync` with
//!   `VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK`, available from Windows build 20348 onward, with a
//!   graceful fallback to full-desktop loopback.
//!
//! Device-name resolution and PID lookup live in [`crate::audio::endpoints`] so the live meters
//! and the recorder always agree on which endpoint a friendly name refers to.

#![cfg(target_os = "windows")]

use std::collections::VecDeque;
use std::mem::size_of;

use windows::core::{Interface, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::Media::Audio::*;
use windows::Win32::Media::KernelStreaming::WAVE_FORMAT_EXTENSIBLE;
use windows::Win32::Media::Multimedia::KSDATAFORMAT_SUBTYPE_IEEE_FLOAT;
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows::Win32::System::Com::*;
use windows::Win32::System::Threading::{CreateEventW, SetEvent, WaitForSingleObject};
use windows::Win32::System::Variant::VT_BLOB;

use super::writer::MIX_RATE;
use crate::audio::bus::{SourceBusKind, SourceBusSpec};
use crate::audio::endpoints::{resolve_endpoint, take_pwstr};

/// How long to wait for `ActivateAudioInterfaceAsync` to complete, in milliseconds.
const PROCESS_LOOPBACK_TIMEOUT_MS: u32 = 4_000;

// ---------------------------------------------------------------------------------------------
// COM completion handler for async activation
// ---------------------------------------------------------------------------------------------

/// Minimal `IActivateAudioInterfaceCompletionHandler` that signals a Win32 event once
/// `ActivateAudioInterfaceAsync` has finished.
#[windows::core::implement(IActivateAudioInterfaceCompletionHandler)]
struct ActivationCompletionHandler {
    done: HANDLE,
}

impl IActivateAudioInterfaceCompletionHandler_Impl for ActivationCompletionHandler_Impl {
    fn ActivateCompleted(
        &self,
        _operation: windows::core::Ref<IActivateAudioInterfaceAsyncOperation>,
    ) -> windows::core::Result<()> {
        unsafe {
            let _ = SetEvent(self.done);
        }
        Ok(())
    }
}

/// Describes a 48 kHz stereo 32-bit float stream, which the per-process loopback engine
/// requires (`AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM` will resample the real mix into it).
fn stereo_float_48k() -> WAVEFORMATEXTENSIBLE {
    let mut fmt = WAVEFORMATEXTENSIBLE::default();
    fmt.Format.wFormatTag = WAVE_FORMAT_EXTENSIBLE as u16;
    fmt.Format.nChannels = 2;
    fmt.Format.nSamplesPerSec = MIX_RATE;
    fmt.Format.wBitsPerSample = 32;
    fmt.Format.nBlockAlign = 8;
    fmt.Format.nAvgBytesPerSec = MIX_RATE * 8;
    fmt.Format.cbSize = (size_of::<WAVEFORMATEXTENSIBLE>() - size_of::<WAVEFORMATEX>()) as u16;
    fmt.Samples.wValidBitsPerSample = 32;
    fmt.dwChannelMask = 3; // SPEAKER_FRONT_LEFT | SPEAKER_FRONT_RIGHT
    fmt.SubFormat = KSDATAFORMAT_SUBTYPE_IEEE_FLOAT;
    fmt
}

// ---------------------------------------------------------------------------------------------
// Capture stream
// ---------------------------------------------------------------------------------------------

/// A live WASAPI capture stream feeding the mixer at its native rate.
pub struct CaptureEndpoint {
    _client: Option<IAudioClient>,
    capture: Option<IAudioCaptureClient>,
    channels: usize,
    bits_per_sample: u16,
    sample_rate: u32,
    /// Stable WASAPI endpoint id, used to detect a source bus and a global bus capturing the
    /// very same endpoint (which would otherwise double the audio).
    endpoint_id: String,
}

impl CaptureEndpoint {
    /// Opens a shared-mode stream on an already resolved endpoint.
    ///
    /// `is_loopback` selects full-desktop system loopback (render endpoints only).
    pub unsafe fn open_device(dev: &IMMDevice, is_loopback: bool) -> Self {
        let mut client_opt = None;
        let mut capture_opt = None;
        let mut channels = 2usize;
        let mut bits_per_sample = 32u16;
        let mut sample_rate = MIX_RATE;
        let endpoint_id = dev
            .GetId()
            .ok()
            .and_then(|p| take_pwstr(p))
            .unwrap_or_default();

        if let Ok(client) = dev.Activate::<IAudioClient>(CLSCTX_ALL, None) {
            if let Ok(mix_fmt_ptr) = client.GetMixFormat() {
                if !mix_fmt_ptr.is_null() {
                    let fmt = *mix_fmt_ptr;
                    channels = (fmt.nChannels as usize).max(1);
                    bits_per_sample = fmt.wBitsPerSample;
                    sample_rate = fmt.nSamplesPerSec.max(1);

                    let flags = if is_loopback {
                        AUDCLNT_STREAMFLAGS_LOOPBACK
                    } else {
                        0
                    };

                    if client
                        .Initialize(
                            AUDCLNT_SHAREMODE_SHARED,
                            flags,
                            1_000_000, // 100 ms buffer
                            0,
                            mix_fmt_ptr,
                            None,
                        )
                        .is_ok()
                    {
                        if let Ok(cap) = client.GetService::<IAudioCaptureClient>() {
                            if client.Start().is_ok() {
                                capture_opt = Some(cap);
                                client_opt = Some(client);
                            }
                        }
                    }
                    CoTaskMemFree(Some(mix_fmt_ptr as *const _));
                }
            }
        }

        Self {
            _client: client_opt,
            capture: capture_opt,
            channels,
            bits_per_sample,
            sample_rate,
            endpoint_id,
        }
    }

    /// Opens a **per-process** loopback stream (Windows 10 build 20348+).
    ///
    /// Returns `None` when the platform refuses the activation (older build, protected
    /// process, …) so the caller can fall back to full-desktop loopback.
    pub unsafe fn open_process_loopback(pid: u32) -> Option<Self> {
        if pid == 0 {
            return None;
        }

        let event = CreateEventW(None, true, false, PCWSTR::null()).ok()?;
        let client = activate_process_loopback_client(pid, event);
        let _ = CloseHandle(event);
        let client = client?;

        let fmt = stereo_float_48k();
        let flags = AUDCLNT_STREAMFLAGS_LOOPBACK
            | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM
            | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY;

        if client
            .Initialize(AUDCLNT_SHAREMODE_SHARED, flags, 1_000_000, 0, &fmt.Format, None)
            .is_err()
        {
            return None;
        }

        let capture = client.GetService::<IAudioCaptureClient>().ok()?;
        if client.Start().is_err() {
            return None;
        }

        Some(Self {
            _client: Some(client),
            capture: Some(capture),
            channels: 2,
            bits_per_sample: 32,
            sample_rate: MIX_RATE,
            // Per-process loopback has no single endpoint; tag it so it never collides with
            // (and is therefore never suppressed by) the global Desktop Audio bus.
            endpoint_id: format!("process-loopback:{}", pid),
        })
    }

    /// Native sample rate of the captured stream, in Hz.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Stable WASAPI endpoint id of the captured device (empty when unavailable).
    pub fn endpoint_id(&self) -> &str {
        &self.endpoint_id
    }

    /// `true` once a usable capture client exists (i.e. the stream really opened).
    pub fn is_open(&self) -> bool {
        self.capture.is_some()
    }

    /// Drains every pending WASAPI packet into a continuous native-rate stereo FIFO.
    /// The FIFO is never truncated mid-waveform, so no sample discontinuity is introduced.
    pub unsafe fn drain_into(&self, queue: &mut VecDeque<(f32, f32)>) {
        let Some(ref cap) = self.capture else { return };
        let mut packet_len = cap.GetNextPacketSize().unwrap_or(0);
        let mut safety = 0;

        while packet_len > 0 && safety < 128 {
            safety += 1;
            let mut data_ptr: *mut u8 = std::ptr::null_mut();
            let mut num_frames = 0u32;
            let mut flags = 0u32;

            if cap
                .GetBuffer(&mut data_ptr, &mut num_frames, &mut flags, None, None)
                .is_ok()
            {
                let silent = (flags & (AUDCLNT_BUFFERFLAGS_SILENT.0 as u32)) != 0;
                let ch = self.channels;

                if silent || data_ptr.is_null() || num_frames == 0 {
                    for _ in 0..num_frames {
                        queue.push_back((0.0, 0.0));
                    }
                } else if self.bits_per_sample == 32 {
                    let total_samples = (num_frames as usize) * ch;
                    let floats = std::slice::from_raw_parts(data_ptr as *const f32, total_samples);

                    for frame_idx in 0..(num_frames as usize) {
                        let base = frame_idx * ch;
                        let l = floats[base];
                        let r = if ch > 1 { floats[base + 1] } else { l };
                        let l_val = if l.is_finite() { l.clamp(-1.0, 1.0) } else { 0.0 };
                        let r_val = if r.is_finite() { r.clamp(-1.0, 1.0) } else { 0.0 };
                        queue.push_back((l_val, r_val));
                    }
                } else if self.bits_per_sample == 16 {
                    let total_samples = (num_frames as usize) * ch;
                    let ints = std::slice::from_raw_parts(data_ptr as *const i16, total_samples);

                    for frame_idx in 0..(num_frames as usize) {
                        let base = frame_idx * ch;
                        let l = (ints[base] as f32) / 32768.0;
                        let r = if ch > 1 { (ints[base + 1] as f32) / 32768.0 } else { l };
                        queue.push_back((l.clamp(-1.0, 1.0), r.clamp(-1.0, 1.0)));
                    }
                }

                let _ = cap.ReleaseBuffer(num_frames);
            } else {
                break;
            }
            packet_len = cap.GetNextPacketSize().unwrap_or(0);
        }
    }
}

/// Performs the blocking `ActivateAudioInterfaceAsync` handshake and returns an `IAudioClient`
/// for the target process tree, or `None` on any failure.
unsafe fn activate_process_loopback_client(pid: u32, event: HANDLE) -> Option<IAudioClient> {
    let mut params = AUDIOCLIENT_ACTIVATION_PARAMS::default();
    params.ActivationType = AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK;
    params.Anonymous.ProcessLoopbackParams = AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS {
        TargetProcessId: pid,
        ProcessLoopbackMode: PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE,
    };

    // The activation params travel inside a VT_BLOB PROPVARIANT, exactly as documented. The
    // blob only *borrows* `params`, so the PROPVARIANT must not be cleared (that would free a
    // stack address); it simply goes out of scope.
    let mut prop: PROPVARIANT = PROPVARIANT::default();
    {
        let inner = &mut prop.Anonymous.Anonymous;
        inner.vt = VT_BLOB;
        inner.Anonymous.blob = windows::Win32::System::Com::BLOB {
            cbSize: size_of::<AUDIOCLIENT_ACTIVATION_PARAMS>() as u32,
            pBlobData: &mut params as *mut _ as *mut u8,
        };
    }

    let handler: IActivateAudioInterfaceCompletionHandler =
        ActivationCompletionHandler { done: event }.into();

    let op = ActivateAudioInterfaceAsync(
        VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK,
        &IAudioClient::IID,
        Some(&prop),
        &handler,
    )
    .ok()?;

    if WaitForSingleObject(event, PROCESS_LOOPBACK_TIMEOUT_MS) != WAIT_OBJECT_0 {
        return None;
    }

    let mut activation_hresult = windows::core::HRESULT(0);
    let mut activated: Option<windows::core::IUnknown> = None;
    op.GetActivateResult(&mut activation_hresult, &mut activated)
        .ok()?;
    if activation_hresult.is_err() {
        return None;
    }
    activated?.cast::<IAudioClient>().ok()
}

/// Opens a capture stream for one per-source bus, applying the documented fallbacks.
///
/// * `CaptureInput`    → named capture endpoint (Windows default when unresolved).
/// * `RenderLoopback`  → named render endpoint, full-desktop loopback (default when unresolved).
/// * `ProcessLoopback` → per-process loopback, falling back to full-desktop loopback when the
///   PID cannot be resolved or the platform rejects the activation.
pub unsafe fn open_for_spec(spec: &SourceBusSpec) -> Option<CaptureEndpoint> {
    let enumerator: IMMDeviceEnumerator =
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;

    match spec.kind {
        SourceBusKind::CaptureInput => {
            let dev = resolve_endpoint(&enumerator, eCapture, &spec.device_name)?;
            let endpoint = CaptureEndpoint::open_device(&dev, false);
            endpoint.is_open().then_some(endpoint)
        }
        SourceBusKind::RenderLoopback => {
            let dev = resolve_endpoint(&enumerator, eRender, &spec.device_name)?;
            let endpoint = CaptureEndpoint::open_device(&dev, true);
            endpoint.is_open().then_some(endpoint)
        }
        SourceBusKind::ProcessLoopback { pid } => {
            if let Some(endpoint) = CaptureEndpoint::open_process_loopback(pid) {
                return Some(endpoint);
            }
            // Fallback: whole-desktop loopback so the user still hears *something*.
            let dev = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
            let endpoint = CaptureEndpoint::open_device(&dev, true);
            endpoint.is_open().then_some(endpoint)
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Loopback silence pump
// ---------------------------------------------------------------------------------------------

/// Keeper of a silent playback stream so WASAPI loopback capture never starves when no
/// application is currently playing audio on the endpoint.
pub struct LoopbackSilencePump {
    client: IAudioClient,
    render: IAudioRenderClient,
    block_align: u32,
    period_frames: u32,
}

impl LoopbackSilencePump {
    pub unsafe fn start(dev: &IMMDevice) -> Option<Self> {
        let client: IAudioClient = dev.Activate(CLSCTX_ALL, None).ok()?;
        let mix_fmt_ptr = client.GetMixFormat().ok()?;
        let block_align = (*mix_fmt_ptr).nBlockAlign as u32;
        let rate = (*mix_fmt_ptr).nSamplesPerSec;

        if client
            .Initialize(AUDCLNT_SHAREMODE_SHARED, 0, 1_000_000, 0, mix_fmt_ptr, None)
            .is_err()
        {
            CoTaskMemFree(Some(mix_fmt_ptr as *const _));
            return None;
        }
        CoTaskMemFree(Some(mix_fmt_ptr as *const _));

        let render: IAudioRenderClient = client.GetService().ok()?;
        if client.Start().is_err() {
            return None;
        }

        Some(Self {
            client,
            render,
            block_align,
            // ~20 ms period keeps the endpoint clock alive with minimal writes
            period_frames: (rate / 50).max(1),
        })
    }

    pub unsafe fn pump(&self) {
        let padding = self.client.GetCurrentPadding().unwrap_or(0);
        let needed = self.period_frames.saturating_sub(padding);
        if needed > 0 {
            if let Ok(buf_ptr) = self.render.GetBuffer(needed) {
                if !buf_ptr.is_null() {
                    let total_bytes = (needed as usize) * (self.block_align as usize);
                    std::ptr::write_bytes(buf_ptr, 0, total_bytes);
                    let _ = self.render.ReleaseBuffer(needed, 0);
                }
            }
        }
    }
}
