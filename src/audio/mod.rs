use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioBusKind {
    DesktopSystem,
    MicrophoneInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioChannel {
    pub kind: AudioBusKind,
    pub label: String,
    pub selected_device: String,
    pub available_devices: Vec<String>,
    pub volume: f32, // 0.0 ..= 1.0
    pub muted: bool,
    #[serde(skip)]
    pub current_peak: f32,
    #[serde(skip)]
    pub peak_hold: f32,
    #[serde(skip)]
    pub peak_hold_timer: f32,
}

impl AudioChannel {
    pub fn is_disabled(&self) -> bool {
        self.selected_device.eq_ignore_ascii_case("Disabled")
    }

    pub fn volume_db(&self) -> f32 {
        if self.muted || self.is_disabled() || self.volume <= 0.0001 {
            -60.0
        } else {
            (20.0 * self.volume.log10()).max(-60.0)
        }
    }

    pub fn volume_percent(&self) -> u32 {
        if self.muted || self.is_disabled() {
            0
        } else {
            (self.volume.clamp(0.0, 1.0) * 100.0).round() as u32
        }
    }

    pub fn effective_level(&self) -> f32 {
        if self.muted || self.is_disabled() {
            0.0
        } else {
            self.current_peak.clamp(0.0, 1.0)
        }
    }

    /// Smoothly updates live meter ballistics with frame-independent delta time:
    /// - Fast responsive attack (~20ms) for snappy visual feedback
    /// - Graceful exponential decay (~350ms) matching broadcast audio meter standards
    /// - Peak hold that holds highest peak steadily for 1.2s before gently gliding down
    pub fn update_level(&mut self, hw_peak: f32, dt: f32) {
        if self.muted || self.is_disabled() {
            self.current_peak = 0.0;
            self.peak_hold = 0.0;
            self.peak_hold_timer = 0.0;
            return;
        }

        // Target level is hardware peak scaled by the channel volume slider (0.0 ..= 1.0)
        let target = (hw_peak * self.volume).clamp(0.0, 1.0);

        // Fast attack: jump up promptly to audio transients
        if target > self.current_peak {
            let attack_coeff = 1.0 - (-dt * 45.0).exp();
            self.current_peak += (target - self.current_peak) * attack_coeff;
        } else {
            // Smooth release: exponential decay over ~350ms
            let decay_coeff = (-dt * 5.0).exp();
            self.current_peak *= decay_coeff;
            if self.current_peak < 0.001 {
                self.current_peak = 0.0;
            }
        }

        // Peak hold ballistics
        if self.current_peak >= self.peak_hold {
            self.peak_hold = self.current_peak;
            self.peak_hold_timer = 1.2; // Hold steady for 1.2 seconds
        } else if self.peak_hold_timer > 0.0 {
            self.peak_hold_timer = (self.peak_hold_timer - dt).max(0.0);
        } else {
            // Gently glide peak hold down towards current peak
            let hold_decay = (-dt * 3.5).exp();
            self.peak_hold = (self.peak_hold * hold_decay).max(self.current_peak);
            if self.peak_hold < 0.001 {
                self.peak_hold = 0.0;
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioMixerState {
    pub desktop_audio: AudioChannel,
    pub microphone: AudioChannel,
    pub sample_rate_hz: u32,
    #[serde(skip)]
    last_meter_tick: f64,
}

impl Default for AudioMixerState {
    fn default() -> Self {
        let mut state = Self {
            desktop_audio: AudioChannel {
                kind: AudioBusKind::DesktopSystem,
                label: "Desktop Audio".to_string(),
                selected_device: "Default System Output (WASAPI)".to_string(),
                available_devices: vec![
                    "Default System Output (WASAPI)".to_string(),
                    "Disabled".to_string(),
                ],
                volume: 0.85,
                muted: false,
                current_peak: 0.0,
                peak_hold: 0.0,
                peak_hold_timer: 0.0,
            },
            microphone: AudioChannel {
                kind: AudioBusKind::MicrophoneInput,
                label: "Mic / Aux".to_string(),
                selected_device: "Default Microphone (WASAPI)".to_string(),
                available_devices: vec![
                    "Default Microphone (WASAPI)".to_string(),
                    "Disabled".to_string(),
                ],
                volume: 0.90,
                muted: false,
                current_peak: 0.0,
                peak_hold: 0.0,
                peak_hold_timer: 0.0,
            },
            sample_rate_hz: 48_000,
            last_meter_tick: 0.0,
        };
        state.refresh_devices_from_system();
        state
    }
}

impl AudioMixerState {
    /// Discovers real Windows WASAPI playback & recording audio endpoints
    pub fn refresh_devices_from_system(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let (render_devs, capture_devs) = wasapi_audio::enumerate_audio_endpoints();

            let mut desk_list = vec!["Default System Output (WASAPI)".to_string()];
            for d in render_devs {
                if !desk_list.contains(&d) {
                    desk_list.push(d);
                }
            }
            desk_list.push("Disabled".to_string());
            self.desktop_audio.available_devices = desk_list;
            if !self
                .desktop_audio
                .available_devices
                .contains(&self.desktop_audio.selected_device)
            {
                self.desktop_audio.selected_device =
                    self.desktop_audio.available_devices[0].clone();
            }

            let mut mic_list = vec!["Default Microphone (WASAPI)".to_string()];
            for d in capture_devs {
                if !mic_list.contains(&d) {
                    mic_list.push(d);
                }
            }
            mic_list.push("Disabled".to_string());
            self.microphone.available_devices = mic_list;
            if !self
                .microphone
                .available_devices
                .contains(&self.microphone.selected_device)
            {
                // Automatically select the first physical microphone if available
                if self.microphone.available_devices.len() > 2 {
                    self.microphone.selected_device =
                        self.microphone.available_devices[1].clone();
                } else {
                    self.microphone.selected_device =
                        self.microphone.available_devices[0].clone();
                }
            }

            wasapi_audio::reset_meter_session();
        }
    }

    pub fn reset_levels(&mut self) {
        self.desktop_audio.volume = 0.85;
        self.desktop_audio.muted = false;
        self.microphone.volume = 0.90;
        self.microphone.muted = false;
    }

    /// Updates live meter levels from Windows WASAPI streams with frame-rate independent ballistics
    pub fn tick_meters(&mut self, time_sec: f64) {
        let dt = if self.last_meter_tick <= 0.0 {
            0.016_f32
        } else {
            ((time_sec - self.last_meter_tick) as f32).clamp(0.001, 0.1)
        };
        self.last_meter_tick = time_sec;

        #[cfg(target_os = "windows")]
        let (hw_desk, hw_mic) = wasapi_audio::read_hardware_peaks(
            !self.desktop_audio.muted && !self.desktop_audio.is_disabled(),
            !self.microphone.muted && !self.microphone.is_disabled(),
        );

        #[cfg(not(target_os = "windows"))]
        let (hw_desk, hw_mic) = (0.0_f32, 0.0_f32);

        self.desktop_audio.update_level(hw_desk, dt);
        self.microphone.update_level(hw_mic, dt);
    }
}

#[cfg(target_os = "windows")]
mod wasapi_audio {
    use std::cell::RefCell;
    use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
    use windows::Win32::Media::Audio::*;
    use windows::Win32::Media::MediaFoundation::*;
    use windows::Win32::System::Com::*;
    use windows::core::PWSTR;

    struct ActiveEndpointStream {
        meter: Option<IAudioMeterInformation>,
        _client: Option<IAudioClient>,
        capture: Option<IAudioCaptureClient>,
        channels: usize,
        bits_per_sample: u16,
    }

    impl ActiveEndpointStream {
        unsafe fn open(dev: &IMMDevice, is_loopback: bool) -> Self {
            let meter = dev
                .Activate::<IAudioMeterInformation>(CLSCTX_ALL, None)
                .ok();
            let mut client_opt = None;
            let mut capture_opt = None;
            let mut channels = 2usize;
            let mut bits_per_sample = 32u16;

            if let Ok(client) = dev.Activate::<IAudioClient>(CLSCTX_ALL, None) {
                if let Ok(mix_fmt_ptr) = client.GetMixFormat() {
                    if !mix_fmt_ptr.is_null() {
                        let fmt = *mix_fmt_ptr;
                        channels = (fmt.nChannels as usize).max(1);
                        bits_per_sample = fmt.wBitsPerSample;

                        let flags = if is_loopback {
                            AUDCLNT_STREAMFLAGS_LOOPBACK
                        } else {
                            0
                        };

                        if client
                            .Initialize(
                                AUDCLNT_SHAREMODE_SHARED,
                                flags,
                                500_000, // 50ms buffer
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
                meter,
                _client: client_opt,
                capture: capture_opt,
                channels,
                bits_per_sample,
            }
        }

        /// Drains all pending WASAPI audio packets so the stream never stalls,
        /// and returns the highest peak amplitude (0.0..=1.0).
        unsafe fn poll_peak(&self) -> f32 {
            let mut max_sample: f32 = 0.0;

            if let Some(ref cap) = self.capture {
                let mut packet_len = cap.GetNextPacketSize().unwrap_or(0);
                let mut safety = 0;
                while packet_len > 0 && safety < 32 {
                    safety += 1;
                    let mut data_ptr: *mut u8 = std::ptr::null_mut();
                    let mut num_frames = 0u32;
                    let mut flags = 0u32;

                    if cap
                        .GetBuffer(&mut data_ptr, &mut num_frames, &mut flags, None, None)
                        .is_ok()
                    {
                        let silent = (flags & (AUDCLNT_BUFFERFLAGS_SILENT.0 as u32)) != 0;
                        if !silent && !data_ptr.is_null() && num_frames > 0 {
                            let total_samples = (num_frames as usize) * self.channels;
                            if self.bits_per_sample == 32 {
                                let floats = std::slice::from_raw_parts(
                                    data_ptr as *const f32,
                                    total_samples,
                                );
                                for &s in floats.iter().step_by(self.channels) {
                                    if s.is_finite() {
                                        let a = s.abs();
                                        if a > max_sample {
                                            max_sample = a;
                                        }
                                    }
                                }
                            } else if self.bits_per_sample == 16 {
                                let ints = std::slice::from_raw_parts(
                                    data_ptr as *const i16,
                                    total_samples,
                                );
                                for &s in ints.iter().step_by(self.channels) {
                                    let a = (s as f32 / 32768.0).abs();
                                    if a > max_sample {
                                        max_sample = a;
                                    }
                                }
                            }
                        }
                        let _ = cap.ReleaseBuffer(num_frames);
                    } else {
                        break;
                    }
                    packet_len = cap.GetNextPacketSize().unwrap_or(0);
                }
            }

            let meter_val = self
                .meter
                .as_ref()
                .and_then(|m| m.GetPeakValue().ok())
                .unwrap_or(0.0);

            max_sample.max(meter_val)
        }
    }

    struct WasapiMeterSession {
        render_stream: Option<ActiveEndpointStream>,
        capture_stream: Option<ActiveEndpointStream>,
    }

    impl WasapiMeterSession {
        fn new() -> Self {
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                let enumerator: Option<IMMDeviceEnumerator> =
                    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok();

                let mut render_stream = None;
                let mut capture_stream = None;

                if let Some(ref enum_ref) = enumerator {
                    if let Ok(render_dev) = enum_ref.GetDefaultAudioEndpoint(eRender, eConsole) {
                        render_stream = Some(ActiveEndpointStream::open(&render_dev, true));
                    }

                    // Try eCommunications first for microphone, fallback to eConsole
                    let cap_dev_res = enum_ref
                        .GetDefaultAudioEndpoint(eCapture, eCommunications)
                        .or_else(|_| enum_ref.GetDefaultAudioEndpoint(eCapture, eConsole));
                    if let Ok(cap_dev) = cap_dev_res {
                        capture_stream = Some(ActiveEndpointStream::open(&cap_dev, false));
                    }
                }

                Self {
                    render_stream,
                    capture_stream,
                }
            }
        }

        fn read_peaks(&self, read_desk: bool, read_mic: bool) -> (f32, f32) {
            unsafe {
                let desk = if read_desk {
                    self.render_stream
                        .as_ref()
                        .map(|s| s.poll_peak())
                        .unwrap_or(0.0)
                } else {
                    0.0
                };

                let mic = if read_mic {
                    self.capture_stream
                        .as_ref()
                        .map(|s| s.poll_peak())
                        .unwrap_or(0.0)
                } else {
                    0.0
                };

                // Convert linear amplitude to studio dB-like perceptual curve (-60dB..0dB mapped to 0.0..1.0)
                let to_meter = |amp: f32, boost: f32| -> f32 {
                    let boosted = (amp * boost).clamp(0.0, 1.0);
                    if boosted <= 0.0008 {
                        0.0
                    } else {
                        let db = (20.0 * boosted.log10()).clamp(-55.0, 0.0);
                        ((db + 55.0) / 55.0).clamp(0.0, 1.0)
                    }
                };

                (to_meter(desk, 1.1), to_meter(mic, 2.2))
            }
        }
    }

    thread_local! {
        static METER_SESSION: RefCell<Option<WasapiMeterSession>> = const { RefCell::new(None) };
    }

    pub fn reset_meter_session() {
        METER_SESSION.with(|cell| {
            *cell.borrow_mut() = None;
        });
    }

    pub fn read_hardware_peaks(read_desk: bool, read_mic: bool) -> (f32, f32) {
        METER_SESSION.with(|cell| {
            let mut borrow = cell.borrow_mut();
            if borrow.is_none() {
                *borrow = Some(WasapiMeterSession::new());
            }
            if let Some(ref session) = *borrow {
                session.read_peaks(read_desk, read_mic)
            } else {
                (0.0, 0.0)
            }
        })
    }

    pub fn enumerate_audio_endpoints() -> (Vec<String>, Vec<String>) {
        let mut render_list = Vec::new();
        let mut capture_list = Vec::new();

        unsafe {
            // 1. Enumerate Playback / Desktop Audio output devices via WinMM
            let num_out = waveOutGetNumDevs();
            for i in 0..num_out {
                let mut caps: WAVEOUTCAPSW = std::mem::zeroed();
                if waveOutGetDevCapsW(
                    i as usize,
                    &mut caps,
                    std::mem::size_of::<WAVEOUTCAPSW>() as u32,
                ) == 0
                {
                    let pname = caps.szPname;
                    let len = pname.iter().position(|&c| c == 0).unwrap_or(pname.len());
                    let name = String::from_utf16_lossy(&pname[..len]).trim().to_string();
                    if !name.is_empty() && !render_list.contains(&name) {
                        render_list.push(name);
                    }
                }
            }

            // 2. Enumerate Microphone / Audio Capture devices via Media Foundation (full friendly names)
            let _ = MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET);
            let mut attrs: Option<IMFAttributes> = None;
            if MFCreateAttributes(&mut attrs, 1).is_ok() {
                if let Some(a) = attrs {
                    if a.SetGUID(
                        &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE,
                        &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_AUDCAP_GUID,
                    )
                    .is_ok()
                    {
                        let mut devices: *mut Option<IMFActivate> = std::ptr::null_mut();
                        let mut count = 0u32;
                        if MFEnumDeviceSources(&a, &mut devices, &mut count).is_ok()
                            && count > 0
                            && !devices.is_null()
                        {
                            for i in 0..count as usize {
                                if let Some(act) = &*devices.add(i) {
                                    let mut name_ptr = PWSTR::null();
                                    let mut name_len = 0u32;
                                    if act
                                        .GetAllocatedString(
                                            &MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME,
                                            &mut name_ptr,
                                            &mut name_len,
                                        )
                                        .is_ok()
                                        && !name_ptr.is_null()
                                    {
                                        let name = String::from_utf16_lossy(
                                            std::slice::from_raw_parts(
                                                name_ptr.0,
                                                name_len as usize,
                                            ),
                                        )
                                        .trim()
                                        .to_string();
                                        CoTaskMemFree(Some(name_ptr.0 as *const _));
                                        if !name.is_empty() && !capture_list.contains(&name) {
                                            capture_list.push(name);
                                        }
                                    }
                                }
                            }
                            CoTaskMemFree(Some(devices as *const _));
                        }
                    }
                }
            }
        }

        (render_list, capture_list)
    }
}
