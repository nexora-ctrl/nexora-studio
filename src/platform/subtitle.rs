use std::fs::File;
use std::io::Write;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
use windows::Win32::Media::Audio::*;
#[cfg(target_os = "windows")]
use windows::Win32::System::Com::*;

/// Request dispatched to Groq Cloud Whisper API
#[derive(Debug, Clone)]
pub struct SubtitleConfig {
    pub api_key: String,
    pub language: String,
    pub is_active: bool,
}

/// Service that captures microphone audio via WASAPI, performs VAD (voice activity detection),
/// and queries Groq's high-speed Whisper Large V3 API to generate live subtitles.
pub struct LiveSubtitleService {
    stop_signal: Arc<AtomicBool>,
    rx: Receiver<String>,
    /// Why the worker is not running, if it could not be started.
    ///
    /// Spawning a thread can fail for reasons outside the app's control (thread/handle exhaustion,
    /// security policy). Panicking there took down the whole studio — mid-recording — over a
    /// feature the user can simply do without, so the failure is surfaced as a status instead.
    startup_error: Option<String>,
}

impl LiveSubtitleService {
    pub fn new() -> Self {
        let stop_signal = Arc::new(AtomicBool::new(false));
        let (tx, rx) = channel();

        let stop_clone = stop_signal.clone();
        let startup_error = thread::Builder::new()
            .name("LiveSubtitleWorker".to_string())
            .spawn(move || {
                run_subtitle_capture_loop(stop_clone, tx);
            })
            .err()
            .map(|e| format!("Live subtitle worker unavailable: {}", e));

        Self {
            stop_signal,
            rx,
            startup_error,
        }
    }

    /// Returns why the live subtitle worker is not running, if it failed to start.
    ///
    /// `None` means the worker started (or already finished) normally.
    pub fn startup_error(&self) -> Option<&str> {
        self.startup_error.as_deref()
    }

    /// Checks if any newly transcribed subtitle text has arrived from Groq
    pub fn try_recv(&self) -> Option<String> {
        let mut last = None;
        while let Ok(msg) = self.rx.try_recv() {
            last = Some(msg);
        }
        last
    }
}

impl Drop for LiveSubtitleService {
    fn drop(&mut self) {
        self.stop_signal.store(true, Ordering::Relaxed);
    }
}

/// Global configuration for live subtitle capture
static SUBTITLE_CONFIG: std::sync::RwLock<Option<SubtitleConfig>> = std::sync::RwLock::new(None);

pub fn update_live_subtitle_config(cfg: Option<SubtitleConfig>) {
    if let Ok(mut lock) = SUBTITLE_CONFIG.write() {
        *lock = cfg;
    }
}

fn get_active_config() -> Option<SubtitleConfig> {
    SUBTITLE_CONFIG.read().ok().and_then(|c| c.clone())
}

#[cfg(target_os = "windows")]
fn run_subtitle_capture_loop(stop_signal: Arc<AtomicBool>, tx: Sender<String>) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    let is_busy_transcribing = Arc::new(AtomicBool::new(false));

    while !stop_signal.load(Ordering::Relaxed) {
        let cfg = get_active_config();
        let (api_key, language) = match cfg {
            Some(ref c) if c.is_active && !c.api_key.trim().is_empty() => {
                (c.api_key.trim().to_string(), c.language.trim().to_string())
            }
            _ => {
                thread::sleep(Duration::from_millis(200));
                continue;
            }
        };

        // Open WASAPI microphone stream
        let mut mic_stream = match open_wasapi_mic_stream() {
            Some(s) => s,
            None => {
                thread::sleep(Duration::from_millis(1000));
                continue;
            }
        };

        let mut audio_buffer: Vec<i16> = Vec::with_capacity(16000 * 8);
        let mut in_speech = false;
        let mut speech_start = Instant::now();
        let mut last_voice = Instant::now();

        while !stop_signal.load(Ordering::Relaxed) {
            // Check if config changed
            if let Some(cur_cfg) = get_active_config() {
                if !cur_cfg.is_active || cur_cfg.api_key.trim().is_empty() {
                    break;
                }
            } else {
                break;
            }

            // Read audio buffer from WASAPI
            let (frames, peak) = unsafe { mic_stream.read_samples_16k_mono() };
            if !frames.is_empty() {
                audio_buffer.extend_from_slice(&frames);

                // Peak threshold for human speech on laptop mic
                if peak > 0.015 {
                    if !in_speech {
                        in_speech = true;
                        speech_start = Instant::now();
                    }
                    last_voice = Instant::now();
                }

                if in_speech {
                    let silence_ms = last_voice.elapsed().as_millis();
                    let total_speech_ms = speech_start.elapsed().as_millis();

                    // Natural phrase completion:
                    // 1) User paused for 350ms after speaking at least 500ms
                    // 2) Or continuous speaking reached 3.8s (chunk boundary to prevent latency build-up)
                    if (silence_ms >= 350 && total_speech_ms >= 500) || total_speech_ms >= 3800 {
                        in_speech = false;
                        if audio_buffer.len() >= 16000 * 5 / 10 {
                            if !is_busy_transcribing.load(Ordering::Relaxed) {
                                is_busy_transcribing.store(true, Ordering::Relaxed);
                                let samples = std::mem::take(&mut audio_buffer);
                                let api_key_c = api_key.clone();
                                let lang_c = language.clone();
                                let tx_c = tx.clone();
                                let busy_flag = is_busy_transcribing.clone();

                                thread::spawn(move || {
                                    if let Some(text) = transcribe_with_groq(&samples, &api_key_c, &lang_c) {
                                        let clean = text.trim();
                                        if !clean.is_empty()
                                            && clean != "."
                                            && clean != "..."
                                            && clean != "Terima kasih."
                                            && clean != "Terima kasih telah menonton!"
                                        {
                                            let _ = tx_c.send(clean.to_string());
                                        }
                                    }
                                    busy_flag.store(false, Ordering::Relaxed);
                                });
                            } else {
                                audio_buffer.clear();
                            }
                        } else {
                            audio_buffer.clear();
                        }
                    }
                } else {
                    // Pre-roll lead-in: preserve ~250ms of audio before speech starts
                    // so the first syllable ("ha-lo...") is never clipped!
                    let keep_len = 16000 * 25 / 100;
                    if audio_buffer.len() > keep_len {
                        let drain_count = audio_buffer.len() - keep_len;
                        audio_buffer.drain(0..drain_count);
                    }
                }
            }

            thread::sleep(Duration::from_millis(25));
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn run_subtitle_capture_loop(_stop_signal: Arc<AtomicBool>, _tx: Sender<String>) {}

/// Encodes 16kHz mono i16 PCM samples to a standard WAV file and sends it to Groq Whisper
fn transcribe_with_groq(samples: &[i16], api_key: &str, language: &str) -> Option<String> {
    let wav_bytes = encode_wav_16k_mono(samples);
    let temp_path = std::env::temp_dir().join(format!("nx_st_{}.wav", uuid::Uuid::new_v4().simple()));

    if let Ok(mut f) = File::create(&temp_path) {
        if f.write_all(&wav_bytes).is_err() {
            let _ = std::fs::remove_file(&temp_path);
            return None;
        }
    } else {
        return None;
    }

    let temp_str = format!("@{}", temp_path.display());
    let auth_header = format!("Authorization: Bearer {}", api_key);
    let file_arg = format!("file={}", temp_str);
    let mut args = vec![
        "-s",
        "-m", "6",
        "-X", "POST",
        "https://api.groq.com/openai/v1/audio/transcriptions",
        "-H", &auth_header,
        "-F", &file_arg,
        "-F", "model=whisper-large-v3-turbo",
        "-F", "temperature=0.0",
        "-F", "prompt=Halo teman-teman, selamat datang di live streaming studio siaran ini.",
        "-F", "response_format=json",
    ];

    let lang_arg;
    if !language.is_empty() && language != "auto" {
        lang_arg = format!("language={}", language);
        args.push("-F");
        args.push(&lang_arg);
    }

    let output = Command::new("curl.exe").args(&args).output();
    let _ = std::fs::remove_file(&temp_path);

    if let Ok(out) = output {
        if out.status.success() {
            if let Ok(json_str) = String::from_utf8(out.stdout) {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    if let Some(text) = parsed.get("text").and_then(|v| v.as_str()) {
                        return Some(text.trim().to_string());
                    }
                }
            }
        }
    }

    None
}

/// Builds a 16kHz mono 16-bit PCM WAV byte buffer
fn encode_wav_16k_mono(samples: &[i16]) -> Vec<u8> {
    let sample_rate = 16000u32;
    let num_channels = 1u16;
    let bits_per_sample = 16u16;
    let byte_rate = sample_rate * (num_channels as u32) * (bits_per_sample as u32 / 8);
    let block_align = num_channels * (bits_per_sample / 8);
    let data_len = (samples.len() * 2) as u32;
    let riff_len = 36 + data_len;

    let mut buf = Vec::with_capacity(44 + data_len as usize);
    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&riff_len.to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    // fmt subchunk
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // Subchunk1Size (16 for PCM)
    buf.extend_from_slice(&1u16.to_le_bytes());  // AudioFormat (1 for PCM)
    buf.extend_from_slice(&num_channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&bits_per_sample.to_le_bytes());
    // data subchunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_len.to_le_bytes());

    for &s in samples {
        buf.extend_from_slice(&s.to_le_bytes());
    }

    buf
}

#[cfg(target_os = "windows")]
struct WasapiMicStream {
    _client: IAudioClient,
    capture: IAudioCaptureClient,
    native_sample_rate: u32,
    channels: usize,
    bits_per_sample: u16,
    resample_accum: f64,
}

#[cfg(target_os = "windows")]
impl WasapiMicStream {
    unsafe fn read_samples_16k_mono(&mut self) -> (Vec<i16>, f32) {
        let mut out_samples = Vec::new();
        let mut max_peak: f32 = 0.0;

        let mut packet_len = self.capture.GetNextPacketSize().unwrap_or(0);
        let mut loops = 0;

        while packet_len > 0 && loops < 32 {
            loops += 1;
            let mut data_ptr: *mut u8 = std::ptr::null_mut();
            let mut num_frames = 0u32;
            let mut flags = 0u32;

            if self
                .capture
                .GetBuffer(&mut data_ptr, &mut num_frames, &mut flags, None, None)
                .is_ok()
            {
                let silent = (flags & (AUDCLNT_BUFFERFLAGS_SILENT.0 as u32)) != 0;
                if !silent && !data_ptr.is_null() && num_frames > 0 {
                    let total_samples = (num_frames as usize) * self.channels;
                    let step = (self.native_sample_rate as f64) / 16000.0;

                    if self.bits_per_sample == 32 {
                        let floats = std::slice::from_raw_parts(data_ptr as *const f32, total_samples);
                        for frame_idx in 0..num_frames as usize {
                            let mut sum = 0.0f32;
                            for c in 0..self.channels {
                                let s = floats[frame_idx * self.channels + c];
                                if s.is_finite() {
                                    sum += s;
                                    let a = s.abs();
                                    if a > max_peak {
                                        max_peak = a;
                                    }
                                }
                            }
                            let mono_s = sum / (self.channels as f32);

                            self.resample_accum += 1.0;
                            while self.resample_accum >= step {
                                self.resample_accum -= step;
                                let clamped = (mono_s * 32767.0).clamp(-32768.0, 32767.0) as i16;
                                out_samples.push(clamped);
                            }
                        }
                    } else if self.bits_per_sample == 16 {
                        let ints = std::slice::from_raw_parts(data_ptr as *const i16, total_samples);
                        for frame_idx in 0..num_frames as usize {
                            let mut sum = 0.0f32;
                            for c in 0..self.channels {
                                let s = ints[frame_idx * self.channels + c] as f32;
                                sum += s;
                                let a = (s / 32768.0).abs();
                                if a > max_peak {
                                    max_peak = a;
                                }
                            }
                            let mono_s = sum / (self.channels as f32);

                            self.resample_accum += 1.0;
                            while self.resample_accum >= step {
                                self.resample_accum -= step;
                                let clamped = mono_s.clamp(-32768.0, 32767.0) as i16;
                                out_samples.push(clamped);
                            }
                        }
                    }
                }
                let _ = self.capture.ReleaseBuffer(num_frames);
            } else {
                break;
            }
            packet_len = self.capture.GetNextPacketSize().unwrap_or(0);
        }

        (out_samples, max_peak)
    }
}

#[cfg(target_os = "windows")]
fn open_wasapi_mic_stream() -> Option<WasapiMicStream> {
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;

        let cap_dev = enumerator
            .GetDefaultAudioEndpoint(eCapture, eCommunications)
            .or_else(|_| enumerator.GetDefaultAudioEndpoint(eCapture, eConsole))
            .ok()?;

        let client: IAudioClient = cap_dev.Activate(CLSCTX_ALL, None).ok()?;
        let mix_fmt_ptr = client.GetMixFormat().ok()?;
        if mix_fmt_ptr.is_null() {
            return None;
        }

        let fmt = *mix_fmt_ptr;
        let channels = (fmt.nChannels as usize).max(1);
        let bits_per_sample = fmt.wBitsPerSample;
        let native_sample_rate = fmt.nSamplesPerSec;

        let init_res = client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            0,
            1_000_000, // 100ms buffer
            0,
            mix_fmt_ptr,
            None,
        );
        CoTaskMemFree(Some(mix_fmt_ptr as *const _));

        if init_res.is_err() {
            return None;
        }

        let capture: IAudioCaptureClient = client.GetService().ok()?;
        if client.Start().is_err() {
            return None;
        }

        Some(WasapiMicStream {
            _client: client,
            capture,
            native_sample_rate,
            channels,
            bits_per_sample,
            resample_accum: 0.0,
        })
    }
}
