use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

#[cfg(target_os = "windows")]
use windows::core::*;
#[cfg(target_os = "windows")]
use windows::Win32::Media::Audio::*;
#[cfg(target_os = "windows")]
use windows::Win32::Media::MediaFoundation::*;
#[cfg(target_os = "windows")]
use windows::Win32::System::Com::*;

/// Internal mixing sample rate (AAC stream rate)
const MIX_RATE: u32 = 48_000;

#[inline]
fn pack_u32_pair(high: u32, low: u32) -> u64 {
    ((high as u64) << 32) | (low as u64)
}

/// Soft-knee limiter: transparent below 0.7, asymptotically approaches 1.0 above it.
/// Prevents hard clipping distortion when Desktop + Mic are both loud.
#[inline]
fn soft_limit(x: f32) -> f32 {
    let a = x.abs();
    if a <= 0.70 {
        x
    } else {
        let t = ((a - 0.70) / 0.30).min(8.0);
        x.signum() * (0.70 + 0.30 * t.tanh())
    }
}

/// Commands and data packets sent from UI loop to background MP4 encoder thread
pub enum EncoderPacket {
    Frame {
        rgba: Vec<u8>,
        width: u32,
        height: u32,
    },
    Mixer {
        desk_volume: f32,
        desk_muted: bool,
        desk_enabled: bool,
        mic_volume: f32,
        mic_muted: bool,
        mic_enabled: bool,
    },
    Pause(bool),
    Stop,
}

pub struct Mp4RecordingWriter {
    sender: SyncSender<EncoderPacket>,
    is_running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Mp4RecordingWriter {
    pub fn start(
        output_path: PathBuf,
        target_width: u32,
        target_height: u32,
        fps: u32,
        bitrate_bps: u32,
    ) -> anyhow::Result<Self> {
        let (tx, rx) = sync_channel::<EncoderPacket>(32);
        let is_running = Arc::new(AtomicBool::new(true));
        let is_running_clone = is_running.clone();

        let handle = thread::Builder::new()
            .name("Mp4EncoderThread".to_string())
            .spawn(move || {
                run_mf_encoder_loop(
                    output_path,
                    target_width,
                    target_height,
                    fps,
                    bitrate_bps,
                    rx,
                    is_running_clone,
                );
            })?;

        Ok(Self {
            sender: tx,
            is_running,
            handle: Some(handle),
        })
    }

    pub fn push_frame(&self, rgba: Vec<u8>, width: u32, height: u32) {
        if self.is_running.load(Ordering::Relaxed) {
            let _ = self.sender.try_send(EncoderPacket::Frame { rgba, width, height });
        }
    }

    /// Mirrors the live Audio Mixer panel (volume faders, mute + device enable) into the recording pipeline.
    pub fn set_mixer(
        &self,
        desk_volume: f32,
        desk_muted: bool,
        desk_enabled: bool,
        mic_volume: f32,
        mic_muted: bool,
        mic_enabled: bool,
    ) {
        if self.is_running.load(Ordering::Relaxed) {
            let _ = self.sender.try_send(EncoderPacket::Mixer {
                desk_volume,
                desk_muted,
                desk_enabled,
                mic_volume,
                mic_muted,
                mic_enabled,
            });
        }
    }

    pub fn set_paused(&self, paused: bool) {
        if self.is_running.load(Ordering::Relaxed) {
            let _ = self.sender.try_send(EncoderPacket::Pause(paused));
        }
    }

    pub fn stop(&mut self) {
        if self.is_running.swap(false, Ordering::SeqCst) {
            let _ = self.sender.send(EncoderPacket::Stop);
            if let Some(h) = self.handle.take() {
                let _ = h.join();
            }
        }
    }
}

impl Drop for Mp4RecordingWriter {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(target_os = "windows")]
struct WasapiCaptureEndpoint {
    _client: Option<IAudioClient>,
    capture: Option<IAudioCaptureClient>,
    channels: usize,
    bits_per_sample: u16,
    sample_rate: u32,
}

#[cfg(target_os = "windows")]
impl WasapiCaptureEndpoint {
    unsafe fn open(dev: &IMMDevice, is_loopback: bool) -> Self {
        let mut client_opt = None;
        let mut capture_opt = None;
        let mut channels = 2usize;
        let mut bits_per_sample = 32u16;
        let mut sample_rate = MIX_RATE;

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
                            1_000_000, // 100ms buffer
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
                }
                CoTaskMemFree(Some(mix_fmt_ptr as *const _));
            }
        }

        Self {
            _client: client_opt,
            capture: capture_opt,
            channels,
            bits_per_sample,
            sample_rate,
        }
    }

    /// Drains every pending WASAPI packet into a continuous native-rate stereo FIFO.
    /// The FIFO is never truncated mid-waveform, so no sample discontinuity is introduced.
    unsafe fn drain_into(&self, queue: &mut VecDeque<(f32, f32)>) {
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

/// Linear-interpolating resampler from a native-rate stereo FIFO into the 48 kHz mix rate.
#[cfg(target_os = "windows")]
struct StereoResampler {
    frac: f64,
    step: f64,
    hold: (f32, f32),
    /// Multiplicative fade applied while the source queue is starved (avoids clicks).
    fade: f32,
}

#[cfg(target_os = "windows")]
impl StereoResampler {
    fn new(in_rate: u32) -> Self {
        Self {
            frac: 0.0,
            step: in_rate as f64 / MIX_RATE as f64,
            hold: (0.0, 0.0),
            fade: 0.0,
        }
    }

    /// Returns exactly one 48 kHz stereo frame, or `None` if the queue is still too short.
    fn next(&mut self, src: &mut VecDeque<(f32, f32)>) -> Option<(f32, f32)> {
        let n = src.len();
        if n < 2 {
            return None;
        }
        let i0 = self.frac.floor() as usize;
        if i0 + 1 >= n {
            return None;
        }
        let t = (self.frac - i0 as f64) as f32;
        let a = src[i0];
        let b = src[i0 + 1];
        let out = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);

        self.frac += self.step;
        while self.frac >= 1.0 && src.len() > 1 {
            src.pop_front();
            self.frac -= 1.0;
        }

        self.hold = out;
        self.fade = 1.0;
        Some(out)
    }

    /// Starvation fallback: holds the last sample and decays smoothly to silence.
    fn starved(&mut self) -> (f32, f32) {
        self.fade *= 0.995;
        if self.fade < 0.0005 {
            self.fade = 0.0;
        }
        (self.hold.0 * self.fade, self.hold.1 * self.fade)
    }
}

/// One-pole DC blocker / high-pass (~80 Hz) removing hum and rumble from the microphone.
#[cfg(target_os = "windows")]
struct DcBlocker {
    x1: (f32, f32),
    y1: (f32, f32),
}

#[cfg(target_os = "windows")]
impl DcBlocker {
    fn new() -> Self {
        Self {
            x1: (0.0, 0.0),
            y1: (0.0, 0.0),
        }
    }

    #[inline]
    fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        // coefficient 0.995 -> ~77 Hz corner at 48 kHz
        let yl = l - self.x1.0 + 0.995 * self.y1.0;
        let yr = r - self.x1.1 + 0.995 * self.y1.1;
        self.x1 = (l, r);
        self.y1 = (yl, yr);
        (yl, yr)
    }
}

/// Studio microphone conditioner: adaptive noise gate + hiss rolloff.
///
/// 1. `dc` removes mains hum / rumble / any DC offset.
/// 2. The noise floor is tracked continuously with a min-statistics envelope follower,
///    so the gate threshold adapts to whatever the microphone's real self-noise is
///    (laptop fan, room tone, preamp hiss) instead of relying on a fixed magic number.
/// 3. Gain glides open (5 ms) and closed (260 ms) — slow enough that ambience fades
///    out smoothly and never "chatters" into clicks.
/// 4. A gentle one-pole low-pass at ~11 kHz tames residual broadband hiss without
///    dulling speech intelligibility.
#[cfg(target_os = "windows")]
struct MicConditioner {
    dc: DcBlocker,
    noise_floor: f32,
    gain: f32,
    attack: f32,
    release: f32,
    lp_l: f32,
    lp_r: f32,
    lp_coeff: f32,
}

#[cfg(target_os = "windows")]
impl MicConditioner {
    fn new() -> Self {
        let fs = MIX_RATE as f32;
        // Open in ~4 ms, close over ~260 ms
        let attack = 1.0 - (-1.0f32 / (0.004 * fs)).exp();
        let release = 1.0 - (-1.0f32 / (0.260 * fs)).exp();
        // One-pole low-pass corner at 11 kHz
        let lp_coeff = 1.0 - (-(2.0 * std::f32::consts::PI * 11_000.0) / fs).exp();

        Self {
            dc: DcBlocker::new(),
            noise_floor: 0.0005,
            gain: 0.0,
            attack,
            release,
            lp_l: 0.0,
            lp_r: 0.0,
            lp_coeff,
        }
    }

    #[inline]
    fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        // 1. Remove DC offset / hum
        let (dl, dr) = self.dc.process(l, r);

        // 2. Gentle high-frequency rolloff to soften hiss
        self.lp_l += (dl - self.lp_l) * self.lp_coeff;
        self.lp_r += (dr - self.lp_r) * self.lp_coeff;
        let (fl, fr) = (self.lp_l, self.lp_r);

        // 3. Adaptive noise-floor tracking (min-statistics envelope)
        let level = fl.abs().max(fr.abs());
        let floor_coeff = if level < self.noise_floor {
            0.0015 // fall quickly toward true quiet
        } else {
            0.0000025 // rise extremely slowly so speech never lifts the floor
        };
        self.noise_floor += (level - self.noise_floor) * floor_coeff;
        self.noise_floor = self.noise_floor.clamp(0.00002, 0.05);

        // 4. Gate hysteresis: open ~11 dB above the estimated floor, close just below it
        let open_thresh = (self.noise_floor * 3.6).max(0.0035);
        let close_thresh = open_thresh * 0.82;
        let target = if self.gain > 0.35 {
            if level > close_thresh { 1.0 } else { 0.0 }
        } else if level > open_thresh {
            1.0
        } else {
            0.0
        };

        // 5. Smooth gain glide (no chattering)
        let coeff = if target > self.gain {
            self.attack
        } else {
            self.release
        };
        self.gain += (target - self.gain) * coeff;
        if self.gain < 0.0002 {
            self.gain = 0.0;
        }

        (fl * self.gain, fr * self.gain)
    }
}

/// Keeper of a silent playback stream so WASAPI loopback capture never starves
/// when no application is currently playing audio on the default endpoint.
#[cfg(target_os = "windows")]
struct LoopbackSilencePump {
    client: IAudioClient,
    render: IAudioRenderClient,
    block_align: u32,
    period_frames: u32,
}

#[cfg(target_os = "windows")]
impl LoopbackSilencePump {
    unsafe fn start(dev: &IMMDevice) -> Option<Self> {
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

    unsafe fn pump(&self) {
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

#[cfg(target_os = "windows")]
fn run_mf_encoder_loop(
    output_path: PathBuf,
    target_width: u32,
    target_height: u32,
    fps: u32,
    bitrate_bps: u32,
    rx: Receiver<EncoderPacket>,
    is_running: Arc<AtomicBool>,
) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        if MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET).is_err() {
            is_running.store(false, Ordering::Relaxed);
            return;
        }

        let out_url: HSTRING = output_path.to_string_lossy().as_ref().into();
        let writer_res: windows::core::Result<IMFSinkWriter> =
            MFCreateSinkWriterFromURL(&out_url, None, None);
        let Ok(writer) = writer_res else {
            is_running.store(false, Ordering::Relaxed);
            return;
        };

        // --- 1. Video Output Media Type (H.264) ---
        let Ok(out_type) = MFCreateMediaType() else {
            is_running.store(false, Ordering::Relaxed);
            return;
        };
        let _ = out_type.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video);
        let _ = out_type.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264);
        let _ = out_type.SetUINT32(&MF_MT_AVG_BITRATE, bitrate_bps);
        let _ = out_type.SetUINT32(
            &MF_MT_INTERLACE_MODE,
            MFVideoInterlace_Progressive.0 as u32,
        );
        let _ = out_type.SetUINT64(&MF_MT_FRAME_SIZE, pack_u32_pair(target_width, target_height));
        let _ = out_type.SetUINT64(&MF_MT_FRAME_RATE, pack_u32_pair(fps, 1));
        let _ = out_type.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack_u32_pair(1, 1));

        let Ok(v_stream_idx) = writer.AddStream(&out_type) else {
            is_running.store(false, Ordering::Relaxed);
            return;
        };

        // --- Video Input Media Type (RGB32 / BGR32 8bpc) ---
        let Ok(in_type) = MFCreateMediaType() else {
            is_running.store(false, Ordering::Relaxed);
            return;
        };
        let _ = in_type.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video);
        let _ = in_type.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32);
        let _ = in_type.SetUINT32(
            &MF_MT_INTERLACE_MODE,
            MFVideoInterlace_Progressive.0 as u32,
        );
        let _ = in_type.SetUINT64(&MF_MT_FRAME_SIZE, pack_u32_pair(target_width, target_height));
        let _ = in_type.SetUINT64(&MF_MT_FRAME_RATE, pack_u32_pair(fps, 1));
        let _ = in_type.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack_u32_pair(1, 1));

        if writer.SetInputMediaType(v_stream_idx, &in_type, None).is_err() {
            is_running.store(false, Ordering::Relaxed);
            return;
        }

        // --- 2. Audio Output Media Type (AAC-LC 48 kHz stereo 192 kbps) ---
        let mut a_stream_idx_opt: Option<u32> = None;
        if let Ok(a_out) = MFCreateMediaType() {
            let _ = a_out.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio);
            let _ = a_out.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_AAC);
            let _ = a_out.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, 2);
            let _ = a_out.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, MIX_RATE);
            let _ = a_out.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16);
            let _ = a_out.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, 192_000 / 8);
            let _ = a_out.SetUINT32(&MF_MT_AAC_PAYLOAD_TYPE, 0);
            let _ = a_out.SetUINT32(&MF_MT_AAC_AUDIO_PROFILE_LEVEL_INDICATION, 0x29);

            if let Ok(idx) = writer.AddStream(&a_out) {
                if let Ok(a_in) = MFCreateMediaType() {
                    let _ = a_in.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio);
                    let _ = a_in.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_PCM);
                    let _ = a_in.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, 2);
                    let _ = a_in.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, MIX_RATE);
                    let _ = a_in.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16);
                    let _ = a_in.SetUINT32(&MF_MT_AUDIO_BLOCK_ALIGNMENT, 4);
                    let _ = a_in.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, MIX_RATE * 4);

                    if writer.SetInputMediaType(idx, &a_in, None).is_ok() {
                        a_stream_idx_opt = Some(idx);
                    }
                }
            }
        }

        if writer.BeginWriting().is_err() {
            is_running.store(false, Ordering::Relaxed);
            return;
        }

        // --- 3. Live WASAPI capture endpoints (Desktop loopback + Microphone) ---
        let mut desk_cap: Option<WasapiCaptureEndpoint> = None;
        let mut mic_cap: Option<WasapiCaptureEndpoint> = None;
        let mut silence_pump: Option<LoopbackSilencePump> = None;

        if a_stream_idx_opt.is_some() {
            if let Ok(enumerator) =
                CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL)
            {
                if let Ok(render_dev) = enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
                    silence_pump = LoopbackSilencePump::start(&render_dev);
                    desk_cap = Some(WasapiCaptureEndpoint::open(&render_dev, true));
                }
                let cap_dev_res = enumerator
                    .GetDefaultAudioEndpoint(eCapture, eCommunications)
                    .or_else(|_| enumerator.GetDefaultAudioEndpoint(eCapture, eConsole));
                if let Ok(cap_dev) = cap_dev_res {
                    mic_cap = Some(WasapiCaptureEndpoint::open(&cap_dev, false));
                }
            }
        }

        // --- 4. Mixing pipeline state ---
        let desk_rate = desk_cap.as_ref().map(|c| c.sample_rate).unwrap_or(MIX_RATE);
        let mic_rate = mic_cap.as_ref().map(|c| c.sample_rate).unwrap_or(MIX_RATE);

        let mut desk_res = StereoResampler::new(desk_rate);
        let mut mic_res = StereoResampler::new(mic_rate);
        let mut mic_cond = MicConditioner::new();

        // Live mixer values mirrored from the Audio Mixer dock panel
        let mut desk_volume = 0.85f32;
        let mut desk_muted = false;
        let mut mic_volume = 0.90f32;
        let mut mic_muted = false;

        let mut desk_queue: VecDeque<(f32, f32)> = VecDeque::with_capacity(1 << 15);
        let mut mic_queue: VecDeque<(f32, f32)> = VecDeque::with_capacity(1 << 15);

        // Pre-roll: hold ~60 ms of audio before feeding the encoder so the very first
        // blocks are never starved (removes the start-of-recording "pop").
        let preroll_samples: i64 = (MIX_RATE as i64 * 60) / 1000;
        let mut primed = false;

        let frame_duration_100ns = 10_000_000i64 / (fps as i64);
        let mut target_frame_index: i64 = 0;
        let buf_size = (target_width * target_height * 4) as usize;
        let mut bgra_buf = vec![0u8; buf_size];

        let mut audio_samples_written: i64 = 0;
        let mut pcm_bytes: Vec<u8> = Vec::with_capacity(8192 * 4);

        let mut recording_start_instant: Option<Instant> = None;
        let mut total_pause_duration = std::time::Duration::ZERO;
        let mut pause_start_instant: Option<Instant> = None;

        while let Ok(packet) = rx.recv() {
            match packet {
                EncoderPacket::Stop => break,
                EncoderPacket::Pause(true) => {
                    if pause_start_instant.is_none() {
                        pause_start_instant = Some(Instant::now());
                    }
                }
                EncoderPacket::Pause(false) => {
                    if let Some(p_start) = pause_start_instant.take() {
                        total_pause_duration += p_start.elapsed();
                    }
                }
                EncoderPacket::Mixer {
                    desk_volume: dv,
                    desk_muted: dm,
                    desk_enabled: de,
                    mic_volume: mv,
                    mic_muted: mm,
                    mic_enabled: me,
                } => {
                    desk_volume = if de { dv.clamp(0.0, 1.5) } else { 0.0 };
                    desk_muted = dm || !de;
                    mic_volume = if me { mv.clamp(0.0, 1.5) } else { 0.0 };
                    mic_muted = mm || !me;
                }
                EncoderPacket::Frame { rgba, width, height } => {
                    if pause_start_instant.is_some() || width == 0 || height == 0 || rgba.is_empty()
                    {
                        continue;
                    }

                    let now = Instant::now();
                    let start_inst = *recording_start_instant.get_or_insert(now);
                    let elapsed_active = now
                        .duration_since(start_inst)
                        .saturating_sub(total_pause_duration);
                    let elapsed_secs = elapsed_active.as_secs_f64();

                    // Wall-clock frame pacing: keep the encoded timeline locked to real time.
                    let calculated_frame_idx = (elapsed_secs * (fps as f64)).round() as i64;
                    let frames_to_emit = if target_frame_index == 0 {
                        1
                    } else {
                        (calculated_frame_idx - target_frame_index).clamp(1, 30)
                    };

                    // Scale & convert RGBA to BGRA (upright orientation)
                    scale_rgba_to_target_bgra(
                        &rgba,
                        width,
                        height,
                        &mut bgra_buf,
                        target_width,
                        target_height,
                    );

                    for _ in 0..frames_to_emit {
                        if let Ok(buffer) = MFCreateMemoryBuffer(buf_size as u32) {
                            let mut ptr: *mut u8 = std::ptr::null_mut();
                            let mut max_len = 0u32;
                            let mut cur_len = 0u32;
                            if buffer
                                .Lock(&mut ptr, Some(&mut max_len), Some(&mut cur_len))
                                .is_ok()
                                && !ptr.is_null()
                            {
                                std::ptr::copy_nonoverlapping(bgra_buf.as_ptr(), ptr, buf_size);
                                let _ = buffer.Unlock();
                                let _ = buffer.SetCurrentLength(buf_size as u32);

                                if let Ok(sample) = MFCreateSample() {
                                    let _ = sample.AddBuffer(&buffer);
                                    let _ = sample.SetSampleTime(target_frame_index * frame_duration_100ns);
                                    let _ = sample.SetSampleDuration(frame_duration_100ns);

                                    let _ = writer.WriteSample(v_stream_idx, &sample);
                                    target_frame_index += 1;
                                }
                            }
                        }
                    }

                    // Keep the loopback endpoint clock alive
                    if let Some(ref pump) = silence_pump {
                        pump.pump();
                    }

                    // --- Continuous audio production ---
                    if let Some(a_stream_idx) = a_stream_idx_opt {
                        if let Some(ref d) = desk_cap {
                            d.drain_into(&mut desk_queue);
                        }
                        if let Some(ref m) = mic_cap {
                            m.drain_into(&mut mic_queue);
                        }

                        // Safety cap (1.5 s) so a stalled UI can never grow memory unbounded
                        let cap_len = (MIX_RATE as usize * 3) / 2;
                        while desk_queue.len() > cap_len {
                            desk_queue.pop_front();
                        }
                        while mic_queue.len() > cap_len {
                            mic_queue.pop_front();
                        }

                        if !primed {
                            if desk_queue.len() >= preroll_samples as usize || mic_queue.len() >= preroll_samples as usize {
                                primed = true;
                            }
                        }

                        // Samples demanded by the current video timeline position
                        let target_audio_samples =
                            (target_frame_index * MIX_RATE as i64) / (fps as i64);
                        let needed_samples =
                            (target_audio_samples - audio_samples_written).max(0) as usize;

                        if needed_samples > 0 && primed {
                            pcm_bytes.clear();
                            pcm_bytes.reserve(needed_samples * 4);

                            let desk_gain = if desk_muted { 0.0 } else { desk_volume };
                            let mic_gain = if mic_muted { 0.0 } else { mic_volume };

                            for _ in 0..needed_samples {
                                let (mut dl, mut dr) = match desk_res.next(&mut desk_queue) {
                                    Some(v) => v,
                                    None => desk_res.starved(),
                                };
                                let (ml, mr) = match mic_res.next(&mut mic_queue) {
                                    Some(v) => v,
                                    None => (0.0, 0.0),
                                };

                                // Mic conditioning: de-hum + hiss rolloff + adaptive noise gate
                                let (gl, gr) = mic_cond.process(ml, mr);
                                let (mut ml, mut mr) = (gl, gr);

                                dl *= desk_gain;
                                dr *= desk_gain;
                                ml *= mic_gain;
                                mr *= mic_gain;

                                // Sum then soft-limit (no hard clipping distortion)
                                let mix_l = soft_limit(dl + ml);
                                let mix_r = soft_limit(dr + mr);

                                let i_l = (mix_l * 32767.0).clamp(-32768.0, 32767.0) as i16;
                                let i_r = (mix_r * 32767.0).clamp(-32768.0, 32767.0) as i16;

                                pcm_bytes.extend_from_slice(&i_l.to_le_bytes());
                                pcm_bytes.extend_from_slice(&i_r.to_le_bytes());
                            }

                            if let Ok(a_buffer) = MFCreateMemoryBuffer(pcm_bytes.len() as u32) {
                                let mut a_ptr: *mut u8 = std::ptr::null_mut();
                                if a_buffer.Lock(&mut a_ptr, None, None).is_ok() && !a_ptr.is_null()
                                {
                                    std::ptr::copy_nonoverlapping(
                                        pcm_bytes.as_ptr(),
                                        a_ptr,
                                        pcm_bytes.len(),
                                    );
                                    let _ = a_buffer.Unlock();
                                    let _ = a_buffer.SetCurrentLength(pcm_bytes.len() as u32);

                                    if let Ok(a_sample) = MFCreateSample() {
                                        let _ = a_sample.AddBuffer(&a_buffer);
                                        let time_100ns =
                                            (audio_samples_written * 10_000_000) / MIX_RATE as i64;
                                        let dur_100ns =
                                            (needed_samples as i64 * 10_000_000) / MIX_RATE as i64;
                                        let _ = a_sample.SetSampleTime(time_100ns);
                                        let _ = a_sample.SetSampleDuration(dur_100ns);

                                        let _ = writer.WriteSample(a_stream_idx, &a_sample);
                                        audio_samples_written += needed_samples as i64;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let _ = writer.Finalize();
        is_running.store(false, Ordering::Relaxed);
    }
}

#[cfg(not(target_os = "windows"))]
fn run_mf_encoder_loop(
    _output_path: PathBuf,
    _target_width: u32,
    _target_height: u32,
    _fps: u32,
    _bitrate_bps: u32,
    _rx: Receiver<EncoderPacket>,
    is_running: Arc<AtomicBool>,
) {
    is_running.store(false, Ordering::Relaxed);
}

/// Converts source RGBA buffer to target BGRA buffer with upright orientation and nearest-neighbor scaling
fn scale_rgba_to_target_bgra(
    src_rgba: &[u8],
    src_w: u32,
    src_h: u32,
    dst_bgra: &mut [u8],
    dst_w: u32,
    dst_h: u32,
) {
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        return;
    }

    if src_w == dst_w && src_h == dst_h {
        for dy in 0..dst_h {
            let sy = dst_h - 1 - dy; // Invert vertically for top-down presentation
            let src_row = (sy * src_w * 4) as usize;
            let dst_row = (dy * dst_w * 4) as usize;

            for dx in 0..dst_w {
                let s = src_row + (dx * 4) as usize;
                let d = dst_row + (dx * 4) as usize;
                dst_bgra[d] = src_rgba[s + 2]; // B
                dst_bgra[d + 1] = src_rgba[s + 1]; // G
                dst_bgra[d + 2] = src_rgba[s]; // R
                dst_bgra[d + 3] = src_rgba[s + 3]; // A
            }
        }
        return;
    }

    // Fast nearest-neighbor scaler with vertical flip (sy = src_h - 1 - unscaled_y)
    for dy in 0..dst_h {
        let unscaled_y = (dy * src_h / dst_h).min(src_h - 1);
        let sy = src_h - 1 - unscaled_y;
        let src_row = (sy * src_w * 4) as usize;
        let dst_row = (dy * dst_w * 4) as usize;

        for dx in 0..dst_w {
            let sx = (dx * src_w / dst_w).min(src_w - 1);
            let s_idx = src_row + (sx * 4) as usize;
            let d_idx = dst_row + (dx * 4) as usize;

            dst_bgra[d_idx] = src_rgba[s_idx + 2]; // B
            dst_bgra[d_idx + 1] = src_rgba[s_idx + 1]; // G
            dst_bgra[d_idx + 2] = src_rgba[s_idx]; // R
            dst_bgra[d_idx + 3] = src_rgba[s_idx + 3]; // A
        }
    }
}
