use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

pub fn open_folder_in_explorer(path: &Path) -> anyhow::Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer").arg(path).spawn()?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
    }
    Ok(())
}

pub fn open_file_with_system_player(path: &Path) -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .spawn()?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
    }
    Ok(())
}

/// Opens a native Windows File Open Dialog (`GetOpenFileNameW`) for Media files (Video & Audio: .mp4, .mp3, .wav, .mkv, etc.)
pub fn pick_media_file_dialog() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let filter = "Media Files (*.mp4;*.mkv;*.mov;*.avi;*.webm;*.mp3;*.wav;*.flac;*.ogg;*.m4a;*.aac)\0*.mp4;*.mkv;*.mov;*.avi;*.webm;*.wmv;*.mp3;*.wav;*.flac;*.ogg;*.m4a;*.aac\0Video Files (*.mp4;*.mkv;*.mov;*.avi;*.webm;*.wmv)\0*.mp4;*.mkv;*.mov;*.avi;*.webm;*.wmv\0Audio Files (*.mp3;*.wav;*.flac;*.ogg;*.m4a;*.aac)\0*.mp3;*.wav;*.flac;*.ogg;*.m4a;*.aac\0All Files (*.*)\0*.*\0\0";
        win32_file_dialog::open_file_dialog("Select Media File (Video / Audio / MP3)", filter)
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

/// Opens a native Windows File Open Dialog (`GetOpenFileNameW`) for Image files (.png, .jpg, .bmp, .webp)
pub fn pick_image_file_dialog() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let filter = "Image Files (*.png;*.jpg;*.jpeg;*.bmp;*.webp;*.gif)\0*.png;*.jpg;*.jpeg;*.bmp;*.webp;*.gif\0All Files (*.*)\0*.*\0\0";
        win32_file_dialog::open_file_dialog("Select Image File", filter)
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

/// Opens a native Windows Multi-Select File Open Dialog (`OFN_ALLOWMULTISELECT`) so the user can Ctrl+Click or Shift+Click multiple photos at once
pub fn pick_multiple_image_files_dialog() -> Vec<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let filter = "Image Files (*.png;*.jpg;*.jpeg;*.bmp;*.webp;*.gif)\0*.png;*.jpg;*.jpeg;*.bmp;*.webp;*.gif\0All Files (*.*)\0*.*\0\0";
        win32_file_dialog::open_multi_file_dialog(
            "Select Slide Show Images (Hold Ctrl or Shift to select multiple photos)",
            filter,
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        Vec::new()
    }
}

/// Opens a native Windows File Open Dialog (`GetOpenFileNameW`) for PowerPoint presentation files (.pptx, .ppt)
pub fn pick_powerpoint_file_dialog() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let filter = "PowerPoint Files (*.pptx;*.ppt)\0*.pptx;*.ppt\0All Files (*.*)\0*.*\0\0";
        win32_file_dialog::open_file_dialog("Select PowerPoint Presentation", filter)
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

/// Exports or retrieves cached high-resolution slide PNGs from a PowerPoint presentation (.pptx or .ppt).
/// Uses native PowerPoint COM Automation (hidden window, zero dependencies) to export all slides to 1920x1080 PNG images.
pub fn get_or_export_ppt_slides(file_path: &str) -> Vec<String> {
    if file_path.trim().is_empty() {
        return Vec::new();
    }
    let p = std::path::Path::new(file_path);
    if !p.exists() {
        return Vec::new();
    }

    // Determine cache folder in AppData or Temp
    let cache_root = if let Some(app_data) = std::env::var_os("APPDATA") {
        PathBuf::from(app_data).join("Nexora").join("NexoraStudio").join("ppt_cache")
    } else {
        std::env::temp_dir().join("nexora_ppt_cache")
    };

    // Create unique hash/subfolder based on file path and file modification time
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    use std::hash::{Hash, Hasher};
    file_path.hash(&mut hasher);
    if let Ok(meta) = p.metadata() {
        if let Ok(modified) = meta.modified() {
            modified.hash(&mut hasher);
        }
    }
    let hash_val = hasher.finish();
    let sanitized_stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("presentation")
        .chars().filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-').take(24).collect::<String>();
    let out_dir = cache_root.join(format!("{}_{:016x}", sanitized_stem, hash_val));

    // If cache directory exists and contains Slide*.PNG files, collect and return them
    let mut existing_slides = collect_sorted_slide_files(&out_dir);
    if !existing_slides.is_empty() {
        return existing_slides;
    }

    let _ = std::fs::create_dir_all(&out_dir);

    // Run PowerPoint COM export via PowerShell (background, no window)
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let script = format!(
            "$filePath = '{}'; $outDir = '{}'; \
             $ppt = New-Object -ComObject PowerPoint.Application; \
             $pres = $ppt.Presentations.Open($filePath, 1, 0, 0); \
             $pres.Export($outDir, 'PNG', 1920, 1080); \
             $pres.Close(); \
             $ppt.Quit(); \
             [System.Runtime.InteropServices.Marshal]::ReleaseComObject($ppt) | Out-Null;",
            file_path.replace('\'', "''"),
            out_dir.to_string_lossy().replace('\'', "''")
        );

        let status = std::process::Command::new("powershell")
            .arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-Command")
            .arg(&script)
            .creation_flags(CREATE_NO_WINDOW)
            .status();

        if let Ok(s) = status {
            if s.success() {
                existing_slides = collect_sorted_slide_files(&out_dir);
            }
        }
    }

    existing_slides
}

fn collect_sorted_slide_files(dir: &std::path::Path) -> Vec<String> {
    if !dir.exists() {
        return Vec::new();
    }
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext.eq_ignore_ascii_case("png") || ext.eq_ignore_ascii_case("jpg") {
                    files.push(path);
                }
            }
        }
    }

    // Sort naturally: Slide1, Slide2, ... Slide10
    files.sort_by_key(|p| {
        let name = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let num: usize = name.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0);
        num
    });

    files.into_iter().map(|p| p.to_string_lossy().to_string()).collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActiveMediaAudioSpec {
    pub id_key: String,
    pub file_path: String,
    pub loop_playback: bool,
    pub is_playing: bool,
    pub volume: f32,
    pub muted: bool,
    pub restart_nonce: u64,
}

struct SharedMediaAudioState {
    desired: Vec<ActiveMediaAudioSpec>,
    running: bool,
}

pub struct MediaAudioEngine {
    shared: Arc<Mutex<SharedMediaAudioState>>,
}

impl Default for MediaAudioEngine {
    fn default() -> Self {
        let shared = Arc::new(Mutex::new(SharedMediaAudioState {
            desired: Vec::new(),
            running: true,
        }));
        #[cfg(target_os = "windows")]
        {
            let worker_shared = Arc::clone(&shared);
            thread::Builder::new()
                .name("nexora-media-audio-worker".to_string())
                .spawn(move || {
                    win32_media_audio::media_audio_worker_loop(worker_shared);
                })
                .ok();
        }
        Self { shared }
    }
}

impl MediaAudioEngine {
    pub fn sync_active_media(&mut self, desired: &[ActiveMediaAudioSpec]) {
        if let Ok(mut guard) = self.shared.lock() {
            if guard.desired != desired {
                guard.desired = desired.to_vec();
            }
        }
    }

    pub fn shutdown_all(&mut self) {
        if let Ok(mut guard) = self.shared.lock() {
            guard.desired.clear();
            guard.running = false;
        }
    }
}

#[cfg(target_os = "windows")]
mod win32_media_audio {
    use super::{ActiveMediaAudioSpec, SharedMediaAudioState};
    use std::collections::{HashMap, VecDeque};
    use std::path::Path;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;
    use windows::Win32::Media::Audio::*;
    use windows::Win32::Media::MediaFoundation::*;
    use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
    use windows::Win32::System::Com::*;
    use windows::core::PCWSTR;

    struct WasapiMediaAudioSession {
        file_path: String,
        restart_nonce: u64,
        reader: IMFSourceReader,
        audio_client: IAudioClient,
        render_client: IAudioRenderClient,
        buffer_frame_count: u32,
        src_channels: usize,
        src_sample_rate: u32,
        src_bits: u16,
        dst_channels: usize,
        dst_sample_rate: u32,
        dst_bits: u16,
        pending_frames: VecDeque<(f32, f32)>,
        resample_accum: f64,
        ended: bool,
        is_started: bool,
    }

    impl WasapiMediaAudioSession {
        fn open(file_path: &str, restart_nonce: u64) -> Option<Self> {
            if file_path.trim().is_empty() || !Path::new(file_path).exists() {
                return None;
            }
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                let _ = MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET);

                let enumerator: IMMDeviceEnumerator =
                    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
                let render_dev = enumerator
                    .GetDefaultAudioEndpoint(eRender, eMultimedia)
                    .or_else(|_| enumerator.GetDefaultAudioEndpoint(eRender, eConsole))
                    .ok()?;

                let audio_client: IAudioClient =
                    render_dev.Activate::<IAudioClient>(CLSCTX_ALL, None).ok()?;
                let mix_fmt_ptr = audio_client.GetMixFormat().ok()?;
                if mix_fmt_ptr.is_null() {
                    return None;
                }
                let fmt = *mix_fmt_ptr;
                let dst_channels = (fmt.nChannels as usize).max(1);
                let dst_sample_rate = fmt.nSamplesPerSec.max(8000);
                let dst_bits = fmt.wBitsPerSample;

                let init_res = audio_client.Initialize(
                    AUDCLNT_SHAREMODE_SHARED,
                    0,
                    2_000_000, // 200ms buffer
                    0,
                    mix_fmt_ptr,
                    None,
                );
                CoTaskMemFree(Some(mix_fmt_ptr as *const _));
                init_res.ok()?;

                let buffer_frame_count = audio_client.GetBufferSize().ok()?;
                let render_client = audio_client.GetService::<IAudioRenderClient>().ok()?;

                let wide_path: Vec<u16> =
                    file_path.encode_utf16().chain(std::iter::once(0)).collect();
                let reader =
                    MFCreateSourceReaderFromURL(PCWSTR(wide_path.as_ptr()), None).ok()?;

                let _ = reader.SetStreamSelection(MF_SOURCE_READER_ALL_STREAMS.0 as u32, false);
                if reader
                    .SetStreamSelection(MF_SOURCE_READER_FIRST_AUDIO_STREAM.0 as u32, true)
                    .is_err()
                {
                    return None;
                }

                if let Ok(mt) = MFCreateMediaType() {
                    let _ = mt.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio);
                    let _ = mt.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_PCM);
                    if reader
                        .SetCurrentMediaType(
                            MF_SOURCE_READER_FIRST_AUDIO_STREAM.0 as u32,
                            None,
                            &mt,
                        )
                        .is_err()
                    {
                        let _ = mt.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_Float);
                        reader
                            .SetCurrentMediaType(
                                MF_SOURCE_READER_FIRST_AUDIO_STREAM.0 as u32,
                                None,
                                &mt,
                            )
                            .ok()?;
                    }
                }

                let mut src_channels = 2usize;
                let mut src_sample_rate = dst_sample_rate;
                let mut src_bits = 16u16;

                if let Ok(cur_mt) =
                    reader.GetCurrentMediaType(MF_SOURCE_READER_FIRST_AUDIO_STREAM.0 as u32)
                {
                    if let Ok(ch) = cur_mt.GetUINT32(&MF_MT_AUDIO_NUM_CHANNELS) {
                        src_channels = (ch as usize).max(1);
                    }
                    if let Ok(sr) = cur_mt.GetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND) {
                        src_sample_rate = sr.max(8000);
                    }
                    if let Ok(bits) = cur_mt.GetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE) {
                        src_bits = bits as u16;
                    }
                }

                Some(Self {
                    file_path: file_path.to_string(),
                    restart_nonce,
                    reader,
                    audio_client,
                    render_client,
                    buffer_frame_count,
                    src_channels,
                    src_sample_rate,
                    src_bits,
                    dst_channels,
                    dst_sample_rate,
                    dst_bits,
                    pending_frames: VecDeque::with_capacity(16384),
                    resample_accum: 0.0,
                    ended: false,
                    is_started: false,
                })
            }
        }

        fn seek_to_start(&mut self) -> bool {
            unsafe {
                let mut prop: PROPVARIANT = std::mem::zeroed();
                *(&mut prop as *mut _ as *mut u16) = 20u16; // VT_I8 = 0
                if self
                    .reader
                    .SetCurrentPosition(&windows::core::GUID::zeroed(), &prop)
                    .is_ok()
                {
                    self.ended = false;
                    self.pending_frames.clear();
                    return true;
                }
                false
            }
        }

        fn pump(&mut self, spec: &ActiveMediaAudioSpec) {
            unsafe {
                if self.restart_nonce != spec.restart_nonce {
                    self.restart_nonce = spec.restart_nonce;
                    let _ = self.seek_to_start();
                }

                if !spec.is_playing {
                    if self.is_started {
                        let _ = self.audio_client.Stop();
                        self.is_started = false;
                    }
                    return;
                }

                if !self.is_started {
                    if self.audio_client.Start().is_ok() {
                        self.is_started = true;
                    }
                }

                let padding = self.audio_client.GetCurrentPadding().unwrap_or(0);
                let avail_frames = self.buffer_frame_count.saturating_sub(padding) as usize;
                if avail_frames == 0 {
                    return;
                }

                let mut safety = 0;
                while self.pending_frames.len() < avail_frames && !self.ended && safety < 8 {
                    safety += 1;
                    let mut flags = 0u32;
                    let mut sample: Option<IMFSample> = None;
                    if self
                        .reader
                        .ReadSample(
                            MF_SOURCE_READER_FIRST_AUDIO_STREAM.0 as u32,
                            0,
                            None,
                            Some(&mut flags),
                            None,
                            Some(&mut sample),
                        )
                        .is_ok()
                    {
                        if (flags & (MF_SOURCE_READERF_ENDOFSTREAM.0 as u32)) != 0 {
                            if spec.loop_playback && self.seek_to_start() {
                                continue;
                            } else {
                                self.ended = true;
                                break;
                            }
                        }
                        if (flags & (MF_SOURCE_READERF_CURRENTMEDIATYPECHANGED.0 as u32)) != 0 {
                            if let Ok(cur_mt) = self
                                .reader
                                .GetCurrentMediaType(MF_SOURCE_READER_FIRST_AUDIO_STREAM.0 as u32)
                            {
                                if let Ok(ch) = cur_mt.GetUINT32(&MF_MT_AUDIO_NUM_CHANNELS) {
                                    self.src_channels = (ch as usize).max(1);
                                }
                                if let Ok(sr) = cur_mt.GetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND) {
                                    self.src_sample_rate = sr.max(8000);
                                }
                                if let Ok(bits) = cur_mt.GetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE) {
                                    self.src_bits = bits as u16;
                                }
                            }
                        }

                        if let Some(s) = sample {
                            if let Ok(buf) = s.ConvertToContiguousBuffer() {
                                let mut ptr: *mut u8 = std::ptr::null_mut();
                                let mut len = 0u32;
                                if buf.Lock(&mut ptr, None, Some(&mut len)).is_ok() {
                                    if !ptr.is_null() && len > 0 {
                                        let ratio = self.src_sample_rate as f64
                                            / self.dst_sample_rate as f64;
                                        if self.src_bits == 16 {
                                            let total_i16 = (len as usize) / 2;
                                            let pcm = std::slice::from_raw_parts(
                                                ptr as *const i16,
                                                total_i16,
                                            );
                                            let src_frames = total_i16 / self.src_channels;
                                            while (self.resample_accum as usize) < src_frames {
                                                let idx =
                                                    (self.resample_accum as usize) * self.src_channels;
                                                let l = pcm[idx] as f32 / 32768.0;
                                                let r = if self.src_channels > 1 {
                                                    pcm[idx + 1] as f32 / 32768.0
                                                } else {
                                                    l
                                                };
                                                self.pending_frames.push_back((l, r));
                                                self.resample_accum += ratio;
                                            }
                                            self.resample_accum -= src_frames as f64;
                                            if self.resample_accum < 0.0 {
                                                self.resample_accum = 0.0;
                                            }
                                        } else if self.src_bits == 32 {
                                            let total_f32 = (len as usize) / 4;
                                            let flt = std::slice::from_raw_parts(
                                                ptr as *const f32,
                                                total_f32,
                                            );
                                            let src_frames = total_f32 / self.src_channels;
                                            while (self.resample_accum as usize) < src_frames {
                                                let idx =
                                                    (self.resample_accum as usize) * self.src_channels;
                                                let l = flt[idx];
                                                let r = if self.src_channels > 1 {
                                                    flt[idx + 1]
                                                } else {
                                                    l
                                                };
                                                self.pending_frames.push_back((l, r));
                                                self.resample_accum += ratio;
                                            }
                                            self.resample_accum -= src_frames as f64;
                                            if self.resample_accum < 0.0 {
                                                self.resample_accum = 0.0;
                                            }
                                        }
                                    }
                                    let _ = buf.Unlock();
                                }
                            }
                        }
                    } else {
                        break;
                    }
                }

                let frames_to_write = avail_frames.min(self.pending_frames.len());
                if frames_to_write > 0 {
                    if let Ok(out_ptr) = self.render_client.GetBuffer(frames_to_write as u32) {
                        if !out_ptr.is_null() {
                            let gain = if spec.muted {
                                0.0
                            } else {
                                spec.volume.clamp(0.0, 1.0)
                            };
                            if self.dst_bits == 32 {
                                let out_slice = std::slice::from_raw_parts_mut(
                                    out_ptr as *mut f32,
                                    frames_to_write * self.dst_channels,
                                );
                                for f in 0..frames_to_write {
                                    let (l, r) =
                                        self.pending_frames.pop_front().unwrap_or((0.0, 0.0));
                                    let base = f * self.dst_channels;
                                    out_slice[base] = (l * gain).clamp(-1.0, 1.0);
                                    if self.dst_channels > 1 {
                                        out_slice[base + 1] = (r * gain).clamp(-1.0, 1.0);
                                        for c in 2..self.dst_channels {
                                            out_slice[base + c] = 0.0;
                                        }
                                    }
                                }
                            } else if self.dst_bits == 16 {
                                let out_slice = std::slice::from_raw_parts_mut(
                                    out_ptr as *mut i16,
                                    frames_to_write * self.dst_channels,
                                );
                                for f in 0..frames_to_write {
                                    let (l, r) =
                                        self.pending_frames.pop_front().unwrap_or((0.0, 0.0));
                                    let base = f * self.dst_channels;
                                    out_slice[base] =
                                        ((l * gain).clamp(-1.0, 1.0) * 32767.0) as i16;
                                    if self.dst_channels > 1 {
                                        out_slice[base + 1] =
                                            ((r * gain).clamp(-1.0, 1.0) * 32767.0) as i16;
                                        for c in 2..self.dst_channels {
                                            out_slice[base + c] = 0;
                                        }
                                    }
                                }
                            }
                        }
                        let _ = self.render_client.ReleaseBuffer(frames_to_write as u32, 0);
                    }
                }
            }
        }
    }

    impl Drop for WasapiMediaAudioSession {
        fn drop(&mut self) {
            unsafe {
                if self.is_started {
                    let _ = self.audio_client.Stop();
                }
            }
        }
    }

    pub fn media_audio_worker_loop(shared: Arc<Mutex<SharedMediaAudioState>>) {
        let mut sessions: HashMap<String, WasapiMediaAudioSession> = HashMap::new();
        loop {
            let (running, desired) = {
                match shared.lock() {
                    Ok(g) => (g.running, g.desired.clone()),
                    Err(_) => break,
                }
            };
            if !running {
                break;
            }

            let desired_map: HashMap<String, ActiveMediaAudioSpec> = desired
                .into_iter()
                .filter(|s| !s.file_path.trim().is_empty() && Path::new(&s.file_path).exists())
                .map(|s| (s.id_key.clone(), s))
                .collect();

            sessions.retain(|k, _| desired_map.contains_key(k));

            if desired_map.is_empty() {
                thread::sleep(Duration::from_millis(35));
                continue;
            }

            for (key, spec) in &desired_map {
                let needs_open = match sessions.get(key) {
                    None => true,
                    Some(sess) => {
                        sess.file_path != spec.file_path
                            || (sess.ended && spec.loop_playback)
                    }
                };
                if needs_open {
                    if let Some(new_sess) =
                        WasapiMediaAudioSession::open(&spec.file_path, spec.restart_nonce)
                    {
                        sessions.insert(key.clone(), new_sess);
                    } else {
                        sessions.remove(key);
                        continue;
                    }
                }
                if let Some(sess) = sessions.get_mut(key) {
                    sess.pump(spec);
                }
            }

            thread::sleep(Duration::from_millis(8));
        }
    }
}

#[cfg(target_os = "windows")]
mod win32_file_dialog {
    use std::path::PathBuf;

    #[repr(C)]
    struct OPENFILENAMEW {
        l_struct_size: u32,
        hwnd_owner: isize,
        h_instance: isize,
        lpstr_filter: *const u16,
        lpstr_custom_filter: *mut u16,
        n_max_cust_filter: u32,
        n_filter_index: u32,
        lpstr_file: *mut u16,
        n_max_file: u32,
        lpstr_file_title: *mut u16,
        n_max_file_title: u32,
        lpstr_initial_dir: *const u16,
        lpstr_title: *const u16,
        flags: u32,
        n_file_offset: u16,
        n_file_extension: u16,
        lpstr_def_ext: *const u16,
        l_cust_data: isize,
        lpfn_hook: isize,
        lp_template_name: *const u16,
        pv_reserved: *mut core::ffi::c_void,
        dw_reserved: u32,
        flags_ex: u32,
    }

    #[repr(C)]
    struct POINT {
        x: i32,
        y: i32,
    }

    #[link(name = "comdlg32")]
    extern "system" {
        fn GetOpenFileNameW(lpofn: *mut OPENFILENAMEW) -> i32;
    }

    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(lp_class_name: *const u16, lp_window_name: *const u16) -> isize;
        fn SetWindowPos(
            hwnd: isize,
            hwnd_insert_after: isize,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            u_flags: u32,
        ) -> i32;
        fn SetForegroundWindow(hwnd: isize) -> i32;
        fn GetAsyncKeyState(v_key: i32) -> i16;
        fn GetCursorPos(lp_point: *mut POINT) -> i32;
        fn GetDC(hwnd: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
        fn ReleaseDC(hwnd: *mut core::ffi::c_void, hdc: *mut core::ffi::c_void) -> i32;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn GetPixel(hdc: *mut core::ffi::c_void, x: i32, y: i32) -> u32;
    }

    pub fn open_file_dialog(title: &str, filter_with_nulls: &str) -> Option<PathBuf> {
        unsafe {
            let filter_utf16: Vec<u16> = filter_with_nulls.encode_utf16().collect();
            let title_utf16: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
            let mut file_buf = vec![0u16; 1024];

            const OFN_PATHMUSTEXIST: u32 = 0x0000_0800;
            const OFN_FILEMUSTEXIST: u32 = 0x0000_1000;
            const OFN_NOCHANGEDIR: u32 = 0x0000_0008;
            const OFN_EXPLORER: u32 = 0x0008_0000;

            let mut ofn: OPENFILENAMEW = std::mem::zeroed();
            ofn.l_struct_size = std::mem::size_of::<OPENFILENAMEW>() as u32;
            ofn.lpstr_filter = filter_utf16.as_ptr();
            ofn.n_filter_index = 1;
            ofn.lpstr_file = file_buf.as_mut_ptr();
            ofn.n_max_file = file_buf.len() as u32;
            ofn.lpstr_title = title_utf16.as_ptr();
            ofn.flags = OFN_PATHMUSTEXIST | OFN_FILEMUSTEXIST | OFN_NOCHANGEDIR | OFN_EXPLORER;

            if GetOpenFileNameW(&mut ofn) != 0 {
                let end = file_buf
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(file_buf.len());
                if end > 0 {
                    let path_str = String::from_utf16_lossy(&file_buf[..end]);
                    return Some(PathBuf::from(path_str));
                }
            }
            None
        }
    }

    pub fn open_multi_file_dialog(title: &str, filter_with_nulls: &str) -> Vec<PathBuf> {
        unsafe {
            let filter_utf16: Vec<u16> = filter_with_nulls.encode_utf16().collect();
            let title_utf16: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
            let mut file_buf = vec![0u16; 32768];

            const OFN_ALLOWMULTISELECT: u32 = 0x0000_0200;
            const OFN_PATHMUSTEXIST: u32 = 0x0000_0800;
            const OFN_FILEMUSTEXIST: u32 = 0x0000_1000;
            const OFN_NOCHANGEDIR: u32 = 0x0000_0008;
            const OFN_EXPLORER: u32 = 0x0008_0000;

            let mut ofn: OPENFILENAMEW = std::mem::zeroed();
            ofn.l_struct_size = std::mem::size_of::<OPENFILENAMEW>() as u32;
            ofn.lpstr_filter = filter_utf16.as_ptr();
            ofn.n_filter_index = 1;
            ofn.lpstr_file = file_buf.as_mut_ptr();
            ofn.n_max_file = file_buf.len() as u32;
            ofn.lpstr_title = title_utf16.as_ptr();
            ofn.flags = OFN_ALLOWMULTISELECT
                | OFN_PATHMUSTEXIST
                | OFN_FILEMUSTEXIST
                | OFN_NOCHANGEDIR
                | OFN_EXPLORER;

            let mut results = Vec::new();
            if GetOpenFileNameW(&mut ofn) != 0 {
                // Parse double-null-terminated UTF-16 string list
                let mut parts: Vec<String> = Vec::new();
                let mut start = 0usize;
                while start < file_buf.len() {
                    let rel_end = file_buf[start..]
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(0);
                    if rel_end == 0 {
                        break;
                    }
                    let s = String::from_utf16_lossy(&file_buf[start..start + rel_end]);
                    parts.push(s);
                    start += rel_end + 1;
                }

                if parts.len() == 1 {
                    results.push(PathBuf::from(&parts[0]));
                } else if parts.len() > 1 {
                    let base_dir = PathBuf::from(&parts[0]);
                    for file_name in &parts[1..] {
                        results.push(base_dir.join(file_name));
                    }
                }
            }
            results
        }
    }

    pub fn snap_window_to_monitor_rect(
        window_title: &str,
        left: i32,
        top: i32,
        width: i32,
        height: i32,
        bring_to_front: bool,
    ) -> bool {
        unsafe {
            let title_utf16: Vec<u16> = window_title
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let hwnd = FindWindowW(std::ptr::null(), title_utf16.as_ptr());
            if hwnd != 0 {
                const HWND_TOPMOST: isize = -1;
                const SWP_SHOWWINDOW: u32 = 0x0040;
                const SWP_FRAMECHANGED: u32 = 0x0020;
                let _ = SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    left,
                    top,
                    width,
                    height,
                    SWP_SHOWWINDOW | SWP_FRAMECHANGED,
                );
                if bring_to_front {
                    let _ = SetForegroundWindow(hwnd);
                }
                return true;
            }
            false
        }
    }

    pub fn is_escape_pressed_global() -> bool {
        unsafe {
            const VK_ESCAPE: i32 = 0x1B;
            (GetAsyncKeyState(VK_ESCAPE) as u16 & 0x8000) != 0
        }
    }

    pub fn sample_screen_pixel_at_cursor() -> Option<[u8; 3]> {
        unsafe {
            let mut pt = POINT { x: 0, y: 0 };
            if GetCursorPos(&mut pt) == 0 {
                return None;
            }
            let hdc = GetDC(core::ptr::null_mut());
            if hdc.is_null() {
                return None;
            }
            let cr = GetPixel(hdc, pt.x, pt.y);
            ReleaseDC(core::ptr::null_mut(), hdc);

            if cr == 0xFFFF_FFFF {
                // CLR_INVALID
                return None;
            }

            // COLORREF is 0x00bbggrr
            let r = (cr & 0xFF) as u8;
            let g = ((cr >> 8) & 0xFF) as u8;
            let b = ((cr >> 16) & 0xFF) as u8;
            Some([r, g, b])
        }
    }
}

/// Samples the exact RGB pixel color under the mouse cursor across any monitor or window
pub fn sample_screen_pixel_at_cursor() -> Option<[u8; 3]> {
    #[cfg(target_os = "windows")]
    {
        win32_file_dialog::sample_screen_pixel_at_cursor()
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

/// Snaps a native window (by title) onto the exact physical pixel bounds of a monitor
pub fn snap_window_to_monitor_rect(
    window_title: &str,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    bring_to_front: bool,
) -> bool {
    #[cfg(target_os = "windows")]
    {
        win32_file_dialog::snap_window_to_monitor_rect(window_title, left, top, width, height, bring_to_front)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (window_title, left, top, width, height, bring_to_front);
        false
    }
}

/// Checks if the Escape key is currently pressed at the OS level
pub fn is_escape_pressed_global() -> bool {
    #[cfg(target_os = "windows")]
    {
        win32_file_dialog::is_escape_pressed_global()
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

pub fn save_program_screenshot(
    output_dir: &Path,
    scene_name: &str,
    w: u32,
    h: u32,
) -> anyhow::Result<std::path::PathBuf> {
    if !output_dir.exists() {
        std::fs::create_dir_all(output_dir)?;
    }
    let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let safe_scene = scene_name.replace(' ', "_").replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "");
    let file_name = format!("Screenshot_{}_{}_{}x{}.png", safe_scene, timestamp, w, h);
    let target_path = output_dir.join(&file_name);

    // Save a valid transparent/composition test PNG file to output directory
    let mut img_buf = image::RgbaImage::new(w.max(640), h.max(360));
    for (x, y, pixel) in img_buf.enumerate_pixels_mut() {
        let r = ((x * 255) / w.max(1)) as u8;
        let g = ((y * 255) / h.max(1)) as u8;
        let b = 180;
        *pixel = image::Rgba([r / 4 + 18, g / 4 + 22, b / 3 + 30, 255]);
    }
    img_buf.save(&target_path)?;
    Ok(target_path)
}

pub fn target_os_summary() -> &'static str {
    if cfg!(target_os = "windows") {
        "Windows 10/11 64-bit (WGC + WASAPI Target)"
    } else {
        "Cross-Platform Host"
    }
}

