use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

#[cfg(target_os = "windows")]
use crate::audio::bus::SourceBusKind;
use crate::audio::bus::SourceBusSpec;
#[cfg(target_os = "windows")]
use crate::audio::filters::AudioFilterChainRuntime;
#[cfg(target_os = "windows")]
use windows::core::*;
#[cfg(target_os = "windows")]
use windows::Win32::Media::Audio::*;
#[cfg(target_os = "windows")]
use windows::Win32::Media::MediaFoundation::*;
#[cfg(target_os = "windows")]
use windows::Win32::System::Com::*;

/// Internal mixing sample rate (AAC stream rate)
pub(crate) const MIX_RATE: u32 = 48_000;

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
        /// Endpoint friendly name selected in the Audio Mixer panel. Without this the recorder
        /// silently always used the Windows default device.
        desk_device: String,
        mic_volume: f32,
        mic_muted: bool,
        mic_enabled: bool,
        mic_device: String,
    },
    /// Per-source audio buses (OBS-style). Replaces the previous list wholesale; the encoder
    /// opens / closes dedicated WASAPI streams to match. Sent only when the routing changes.
    SourceBuses(Vec<SourceBusSpec>),
    Pause(bool),
    Stop,
}

pub struct Mp4RecordingWriter {
    sender: SyncSender<EncoderPacket>,
    is_running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
    /// Video frames that never reached the encoder thread.
    ///
    /// The channel is bounded (see `sync_channel` below), so when the encoder falls behind — a
    /// slow disk, a full CPU, or a capture rate above what the encoder can absorb — `try_send`
    /// fails and the frame is gone. Nothing used to count that, which made the "Drop" indicator in
    /// the status bar permanently read 0 even while frames were being lost. This counter is the
    /// only honest source for that number.
    dropped_frames: Arc<AtomicU64>,
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
        let dropped_frames = Arc::new(AtomicU64::new(0));

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
            dropped_frames,
        })
    }

    pub fn push_frame(&self, rgba: Vec<u8>, width: u32, height: u32) {
        if self.is_running.load(Ordering::Relaxed) {
            if self
                .sender
                .try_send(EncoderPacket::Frame { rgba, width, height })
                .is_err()
            {
                self.dropped_frames.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Number of video frames dropped because the encoder queue was full.
    pub fn dropped_frames(&self) -> u64 {
        self.dropped_frames.load(Ordering::Relaxed)
    }

    /// Mirrors the live Audio Mixer panel (volume faders, mute + device enable) into the recording pipeline.
    pub fn set_mixer(
        &self,
        desk_volume: f32,
        desk_muted: bool,
        desk_enabled: bool,
        desk_device: &str,
        mic_volume: f32,
        mic_muted: bool,
        mic_enabled: bool,
        mic_device: &str,
    ) {
        if self.is_running.load(Ordering::Relaxed) {
            let _ = self.sender.try_send(EncoderPacket::Mixer {
                desk_volume,
                desk_muted,
                desk_enabled,
                desk_device: desk_device.to_string(),
                mic_volume,
                mic_muted,
                mic_enabled,
                mic_device: mic_device.to_string(),
            });
        }
    }

    /// Replaces the per-source audio bus list. Only called when the routing actually changes
    /// (source added/removed, device switched, application changed) so the encoder thread is
    /// never asked to re-open WASAPI streams on every frame.
    pub fn set_source_buses(&self, buses: Vec<SourceBusSpec>) {
        if self.is_running.load(Ordering::Relaxed) {
            let _ = self.sender.try_send(EncoderPacket::SourceBuses(buses));
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
use crate::audio::endpoints::resolve_endpoint;
#[cfg(target_os = "windows")]
use crate::encoder::endpoint::{CaptureEndpoint, LoopbackSilencePump};

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

/// One live per-source audio bus: a dedicated WASAPI stream plus its own resampling FIFO and
/// user-configured filter chain, summed into the mix alongside the global buses.
#[cfg(target_os = "windows")]
struct ActiveSourceBus {
    spec: SourceBusSpec,
    endpoint: Option<CaptureEndpoint>,
    queue: VecDeque<(f32, f32)>,
    resampler: StereoResampler,
    chain: AudioFilterChainRuntime,
    /// `true` while this bus is backed by a full-desktop render loopback, so it needs the
    /// endpoint clock kept alive by a silence pump.
    needs_pump: bool,
}

#[cfg(target_os = "windows")]
impl ActiveSourceBus {
    unsafe fn open(spec: SourceBusSpec) -> Self {
        let endpoint = crate::encoder::endpoint::open_for_spec(&spec);
        let rate = endpoint.as_ref().map(|c| c.sample_rate()).unwrap_or(MIX_RATE);
        let needs_pump = matches!(spec.kind, SourceBusKind::RenderLoopback)
            || (matches!(spec.kind, SourceBusKind::ProcessLoopback { .. }) && endpoint.is_some());
        // No implicit conditioning: whatever the user configured is exactly what runs.
        let chain = AudioFilterChainRuntime::from_chain(&spec.filters);

        Self {
            spec,
            endpoint,
            queue: VecDeque::with_capacity(1 << 15),
            resampler: StereoResampler::new(rate),
            chain,
            needs_pump,
        }
    }

    /// Applies the *non-routing* half of a spec update (fader, mute, enable, filters) to an
    /// already-open bus, so editing a parameter never tears down the WASAPI stream.
    fn update_spec(&mut self, spec: SourceBusSpec) {
        if crate::audio::filters::filters_signature(&spec.filters)
            != crate::audio::filters::filters_signature(&self.spec.filters)
        {
            self.chain = AudioFilterChainRuntime::from_chain(&spec.filters);
        }
        self.spec = spec;
    }

    /// Pulls every pending packet, then drains exactly one 48 kHz stereo frame.
    /// Returns silence (fading to avoid clicks) when the stream has no data yet.
    unsafe fn next_frame(&mut self) -> (f32, f32) {
        if let Some(ref ep) = self.endpoint {
            ep.drain_into(&mut self.queue);
        }

        // Safety cap (1.5 s) so a stalled source can never grow memory unbounded.
        let cap_len = (MIX_RATE as usize * 3) / 2;
        while self.queue.len() > cap_len {
            self.queue.pop_front();
        }

        let (l, r) = match self.resampler.next(&mut self.queue) {
            Some(v) => v,
            None => self.resampler.starved(),
        };

        if self.chain.is_transparent() {
            (l, r)
        } else {
            self.chain.process(l, r)
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
        let mut desk_cap: Option<CaptureEndpoint> = None;
        let mut mic_cap: Option<CaptureEndpoint> = None;
        let mut silence_pump: Option<LoopbackSilencePump> = None;

        if a_stream_idx_opt.is_some() {
            if let Ok(enumerator) =
                CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL)
            {
                if let Ok(render_dev) = enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
                    silence_pump = LoopbackSilencePump::start(&render_dev);
                    desk_cap = Some(CaptureEndpoint::open_device(&render_dev, true));
                }
                let cap_dev_res = enumerator
                    .GetDefaultAudioEndpoint(eCapture, eCommunications)
                    .or_else(|_| enumerator.GetDefaultAudioEndpoint(eCapture, eConsole));
                if let Ok(cap_dev) = cap_dev_res {
                    mic_cap = Some(CaptureEndpoint::open_device(&cap_dev, false));
                }
            }
        }

        // --- 4. Mixing pipeline state ---
        let desk_rate = desk_cap.as_ref().map(|c| c.sample_rate()).unwrap_or(MIX_RATE);
        let mic_rate = mic_cap.as_ref().map(|c| c.sample_rate()).unwrap_or(MIX_RATE);

        let mut desk_res = StereoResampler::new(desk_rate);
        let mut mic_res = StereoResampler::new(mic_rate);

        // Live mixer values mirrored from the Audio Mixer dock panel
        let mut desk_volume = 0.85f32;
        let mut desk_muted = false;
        let mut mic_volume = 0.90f32;
        let mut mic_muted = false;

        let mut desk_queue: VecDeque<(f32, f32)> = VecDeque::with_capacity(1 << 15);
        let mut mic_queue: VecDeque<(f32, f32)> = VecDeque::with_capacity(1 << 15);

        // --- 4b. Per-source audio buses (OBS-style) ---
        // Each source bus owns its own WASAPI stream, FIFO, resampler and DSP chain, and is
        // summed after the two global buses. Muting `Mic / Aux` therefore cannot silence an
        // `Audio Input Capture`, which is the whole point of the per-source bus model.
        let mut source_buses: Vec<ActiveSourceBus> = Vec::new();
        let mut source_bus_signature: String = String::new();
        let mut rebuild_source_buses: Option<Vec<SourceBusSpec>> = None;
        let mut all_bus_silence_pumps: Vec<LoopbackSilencePump> = Vec::new();

        // Global bus device selections (now actually honoured by the recorder).
        let mut desk_device_name = String::new();
        let mut mic_device_name = String::new();
        let mut reopen_desk = false;
        let mut reopen_mic = false;

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
                    desk_device: dd,
                    mic_volume: mv,
                    mic_muted: mm,
                    mic_enabled: me,
                    mic_device: md,
                } => {
                    desk_volume = if de { dv.clamp(0.0, 1.5) } else { 0.0 };
                    desk_muted = dm || !de;
                    mic_volume = if me { mv.clamp(0.0, 1.5) } else { 0.0 };
                    mic_muted = mm || !me;

                    // The global buses follow the device chosen in the Audio Mixer panel.
                    // Re-open only when the selection actually changed.
                    if desk_device_name != dd {
                        desk_device_name = dd;
                        reopen_desk = true;
                    }
                    if mic_device_name != md {
                        mic_device_name = md;
                        reopen_mic = true;
                    }
                }
                EncoderPacket::SourceBuses(specs) => {
                    let signature = specs
                        .iter()
                        .map(|s| s.routing_signature())
                        .collect::<Vec<_>>()
                        .join(";");
                    if signature != source_bus_signature {
                        source_bus_signature = signature;
                        rebuild_source_buses = Some(specs);
                    } else {
                        // Routing unchanged (only faders / mutes / filters moved) — update in place
                        // so no WASAPI stream is needlessly torn down and re-opened.
                        for spec in specs {
                            if let Some(bus) = source_buses.iter_mut().find(|b| b.spec.bus_id == spec.bus_id) {
                                bus.update_spec(spec);
                            }
                        }
                    }
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

                    // Keep every loopback endpoint clock alive
                    if let Some(ref pump) = silence_pump {
                        pump.pump();
                    }
                    for pump in &all_bus_silence_pumps {
                        pump.pump();
                    }

                    // --- Continuous audio production ---
                    if let Some(a_stream_idx) = a_stream_idx_opt {
                        // Apply routing changes that arrived this frame. Global buses are
                        // re-opened only when the user actually picked another device.
                        if reopen_desk || reopen_mic {
                            if let Ok(enumerator) = CoCreateInstance::<_, IMMDeviceEnumerator>(
                                &MMDeviceEnumerator,
                                None,
                                CLSCTX_ALL,
                            ) {
                                if reopen_desk {
                                    reopen_desk = false;
                                    if let Some(dev) = resolve_endpoint(
                                        &enumerator,
                                        eRender,
                                        &desk_device_name,
                                    ) {
                                        silence_pump = LoopbackSilencePump::start(&dev);
                                        desk_cap = Some(CaptureEndpoint::open_device(&dev, true));
                                        let rate = desk_cap
                                            .as_ref()
                                            .map(|c| c.sample_rate())
                                            .unwrap_or(MIX_RATE);
                                        desk_res = StereoResampler::new(rate);
                                        desk_queue.clear();
                                    }
                                }
                                if reopen_mic {
                                    reopen_mic = false;
                                    if let Some(dev) = resolve_endpoint(
                                        &enumerator,
                                        eCapture,
                                        &mic_device_name,
                                    ) {
                                        mic_cap = Some(CaptureEndpoint::open_device(&dev, false));
                                        let rate = mic_cap
                                            .as_ref()
                                            .map(|c| c.sample_rate())
                                            .unwrap_or(MIX_RATE);
                                        mic_res = StereoResampler::new(rate);
                                        mic_queue.clear();
                                    }
                                }
                            } else {
                                reopen_desk = false;
                                reopen_mic = false;
                            }
                        }

                        // Rebuild the per-source bus set when its routing changed.
                        if let Some(specs) = rebuild_source_buses.take() {
                            source_buses.clear();
                            all_bus_silence_pumps.clear();
                            for spec in specs {
                                source_buses.push(ActiveSourceBus::open(spec));
                            }
                            // Any bus capturing a render endpoint needs its clock kept alive.
                            if let Ok(enumerator) = CoCreateInstance::<_, IMMDeviceEnumerator>(
                                &MMDeviceEnumerator,
                                None,
                                CLSCTX_ALL,
                            ) {
                                for bus in &source_buses {
                                    if !bus.needs_pump {
                                        continue;
                                    }
                                    let flow = if bus.spec.kind.is_input() {
                                        eCapture
                                    } else {
                                        eRender
                                    };
                                    if let Some(dev) = resolve_endpoint(
                                        &enumerator,
                                        flow,
                                        &bus.spec.device_name,
                                    ) {
                                        if let Some(pump) = LoopbackSilencePump::start(&dev) {
                                            all_bus_silence_pumps.push(pump);
                                        }
                                    }
                                }
                            }
                        }

                        // --- Suppress double capture: a source bus and a global bus resolving
                        // to the same endpoint must not both be summed (that would double the
                        // audio). The global bus yields, because the per-source bus is the one
                        // the user explicitly configured in the mixer.
                        let desk_id = desk_cap.as_ref().map(|c| c.endpoint_id()).unwrap_or("");
                        let mic_id = mic_cap.as_ref().map(|c| c.endpoint_id()).unwrap_or("");
                        let source_ids: Vec<&str> = source_buses
                            .iter()
                            .filter_map(|b| b.endpoint.as_ref().map(|e| e.endpoint_id()))
                            .filter(|id| !id.is_empty())
                            .collect();
                        let desk_suppressed = !desk_id.is_empty() && source_ids.contains(&desk_id);
                        let mic_suppressed = !mic_id.is_empty() && source_ids.contains(&mic_id);

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
                            let any_source_ready = source_buses
                                .iter()
                                .any(|b| b.queue.len() >= preroll_samples as usize);
                            if desk_queue.len() >= preroll_samples as usize
                                || mic_queue.len() >= preroll_samples as usize
                                || any_source_ready
                            {
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

                            let desk_gain = if desk_muted || desk_suppressed {
                                0.0
                            } else {
                                desk_volume
                            };
                            let mic_gain = if mic_muted || mic_suppressed {
                                0.0
                            } else {
                                mic_volume
                            };

                            for _ in 0..needed_samples {
                                let (dl, dr) = if desk_gain > 0.0 {
                                    match desk_res.next(&mut desk_queue) {
                                        Some(v) => v,
                                        None => desk_res.starved(),
                                    }
                                } else {
                                    (0.0, 0.0)
                                };

                                let (ml, mr) = match mic_res.next(&mut mic_queue) {
                                    Some(v) => v,
                                    None => (0.0, 0.0),
                                };

                                // Global buses stay raw, exactly like OBS: filtering is the job of
                                // the per-source chains, which keeps `Mic / Aux` a true device bus
                                // and makes sure a microphone is never processed twice.
                                let mut sum_l = dl * desk_gain + ml * mic_gain;
                                let mut sum_r = dr * desk_gain + mr * mic_gain;

                                // Per-source buses: each keeps its own isolation and filter chain.
                                for bus in &mut source_buses {
                                    let gain = bus.spec.effective_gain();
                                    let (sl, sr) = bus.next_frame();
                                    sum_l += sl * gain;
                                    sum_r += sr * gain;
                                }

                                // Sum then soft-limit (no hard clipping distortion)
                                let mix_l = soft_limit(sum_l);
                                let mix_r = soft_limit(sum_r);

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
