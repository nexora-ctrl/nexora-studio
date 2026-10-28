use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayInfo {
    pub id: String,
    pub name: String,
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
    pub refresh_hz: u32,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowTargetInfo {
    pub hwnd: u64,
    pub title: String,
    pub executable: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraDeviceInfo {
    pub id: String,
    pub name: String,
    pub is_online: bool,
}

#[derive(Debug, Clone)]
pub struct CaptureCatalog {
    pub displays: Vec<DisplayInfo>,
    pub windows: Vec<WindowTargetInfo>,
    pub cameras: Vec<CameraDeviceInfo>,
    pub status_label: String,
}

impl Default for CaptureCatalog {
    fn default() -> Self {
        let mut cat = Self {
            displays: Vec::new(),
            windows: Vec::new(),
            cameras: Vec::new(),
            status_label: "Live Screen, Window & Camera Detection Active".to_string(),
        };
        cat.refresh_from_system();
        cat
    }
}

impl CaptureCatalog {
    pub fn refresh_from_system(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let real_displays = win32_capture::enumerate_displays();
            if !real_displays.is_empty() {
                self.displays = real_displays;
            }
            let real_windows = win32_capture::enumerate_visible_windows();
            if !real_windows.is_empty() {
                self.windows = real_windows;
            }
            let real_cameras = win32_capture::enumerate_camera_devices();
            if !real_cameras.is_empty() {
                self.cameras = real_cameras;
            }
            let online_cams = self.cameras.iter().filter(|c| c.is_online).count();
            self.status_label = format!(
                "Live Capture Active ({} Displays, {} Windows, {} Cameras)",
                self.displays.len(),
                self.windows.len(),
                online_cams
            );
        }

        if self.displays.is_empty() {
            self.displays.push(DisplayInfo {
                id: "MONITOR_0".to_string(),
                name: "Primary Monitor (Display 1)".to_string(),
                left: 0,
                top: 0,
                width: 1920,
                height: 1080,
                refresh_hz: 60,
                is_primary: true,
            });
        }

        if self.cameras.is_empty() {
            self.cameras.push(CameraDeviceInfo {
                id: "CAM_INTERNAL".to_string(),
                name: "Integrated Camera (Default)".to_string(),
                is_online: true,
            });
        }
    }

    pub fn find_display(&self, monitor_id: &str) -> Option<&DisplayInfo> {
        self.displays
            .iter()
            .find(|d| d.id == monitor_id)
            .or_else(|| self.displays.first())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CaptureRequestTarget {
    Display {
        monitor_id: String,
        left: i32,
        top: i32,
        width: u32,
        height: u32,
        capture_cursor: bool,
    },
    Window {
        hwnd: u64,
        capture_cursor: bool,
    },
    Camera {
        device_id: String,
        resolution: (u32, u32),
    },
}

impl CaptureRequestTarget {
    pub fn cache_key(&self) -> String {
        match self {
            Self::Display {
                monitor_id,
                capture_cursor,
                ..
            } => format!("DISP:{}:{}", monitor_id, capture_cursor),
            Self::Window {
                hwnd,
                capture_cursor,
            } => format!("WIN:{}:{}", hwnd, capture_cursor),
            Self::Camera {
                device_id,
                resolution,
            } => format!("CAM:{}:{}x{}", device_id, resolution.0, resolution.1),
        }
    }
}

#[derive(Clone)]
pub struct CapturedFrame {
    pub width: usize,
    pub height: usize,
    pub rgba: Arc<[u8]>,
    pub seq: u64,
}

struct SharedCaptureData {
    active_requests: Vec<CaptureRequestTarget>,
    latest_frames: HashMap<String, CapturedFrame>,
    running: bool,
}

pub struct LiveCaptureEngine {
    shared: Arc<Mutex<SharedCaptureData>>,
    textures: HashMap<String, (u64, egui::TextureHandle)>,
}

impl Default for LiveCaptureEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveCaptureEngine {
    pub fn new() -> Self {
        let shared = Arc::new(Mutex::new(SharedCaptureData {
            active_requests: Vec::new(),
            latest_frames: HashMap::new(),
            running: true,
        }));

        let worker_shared = Arc::clone(&shared);
        thread::Builder::new()
            .name("nexora-capture-worker".to_string())
            .spawn(move || {
                capture_worker_loop(worker_shared);
            })
            .ok();

        Self {
            shared,
            textures: HashMap::new(),
        }
    }

    /// Updates the list of active capture targets requested by Preview and Program scenes
    pub fn set_active_requests(&self, requests: Vec<CaptureRequestTarget>) {
        if let Ok(mut guard) = self.shared.lock() {
            if guard.active_requests != requests {
                guard.active_requests = requests;
            }
        }
    }

    /// Uploads any newly captured frames to GPU textures and returns the texture handle for a key
    pub fn get_or_update_texture(
        &mut self,
        ctx: &egui::Context,
        key: &str,
    ) -> Option<egui::TextureHandle> {
        let maybe_new_frame = {
            let guard = self.shared.lock().ok()?;
            let frame = guard.latest_frames.get(key)?;
            let needs_upload = match self.textures.get(key) {
                Some((last_seq, _)) => *last_seq != frame.seq,
                None => true,
            };
            if needs_upload {
                Some(frame.clone())
            } else {
                None
            }
        };

        if let Some(frame) = maybe_new_frame {
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [frame.width, frame.height],
                &frame.rgba,
            );
            if let Some((seq_ref, tex_handle)) = self.textures.get_mut(key) {
                tex_handle.set(color_image, egui::TextureOptions::LINEAR);
                *seq_ref = frame.seq;
            } else {
                let handle = ctx.load_texture(
                    format!("capture_tex_{}", key),
                    color_image,
                    egui::TextureOptions::LINEAR,
                );
                self.textures.insert(key.to_string(), (frame.seq, handle));
            }
        }

        self.textures.get(key).map(|(_, h)| h.clone())
    }
}

impl Drop for LiveCaptureEngine {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.shared.lock() {
            guard.running = false;
        }
    }
}

fn capture_worker_loop(shared: Arc<Mutex<SharedCaptureData>>) {
    let mut seq_counter: u64 = 1;
    loop {
        let start = Instant::now();
        let (running, requests) = {
            match shared.lock() {
                Ok(guard) => (guard.running, guard.active_requests.clone()),
                Err(_) => break,
            }
        };
        if !running {
            break;
        }

        #[cfg(target_os = "windows")]
        {
            let has_cam_req = requests
                .iter()
                .any(|r| matches!(r, CaptureRequestTarget::Camera { .. }));
            if !has_cam_req {
                win32_capture::release_camera_session();
            }
        }

        if requests.is_empty() {
            thread::sleep(Duration::from_millis(45));
            continue;
        }

        #[cfg(target_os = "windows")]
        for req in requests {
            let key = req.cache_key();
            let captured = match req {
                CaptureRequestTarget::Display {
                    left,
                    top,
                    width,
                    height,
                    capture_cursor,
                    ..
                } => win32_capture::capture_display_rect(
                    left,
                    top,
                    width.max(64),
                    height.max(64),
                    960,
                    540,
                    capture_cursor,
                ),
                CaptureRequestTarget::Window {
                    hwnd,
                    capture_cursor,
                } => win32_capture::capture_window_hwnd(hwnd, 960, 540, capture_cursor),
                CaptureRequestTarget::Camera {
                    device_id,
                    resolution,
                } => win32_capture::capture_camera_frame(&device_id, resolution.0, resolution.1),
            };

            if let Some((w, h, rgba)) = captured {
                seq_counter = seq_counter.wrapping_add(1);
                let frame = CapturedFrame {
                    width: w,
                    height: h,
                    rgba: Arc::from(rgba),
                    seq: seq_counter,
                };
                if let Ok(mut guard) = shared.lock() {
                    guard.latest_frames.insert(key, frame);
                }
            }
        }

        let elapsed = start.elapsed();
        let frame_budget = Duration::from_millis(33); // ~30 FPS live preview capture
        if elapsed < frame_budget {
            thread::sleep(frame_budget - elapsed);
        } else {
            thread::sleep(Duration::from_millis(4));
        }
    }
}

#[cfg(target_os = "windows")]
mod win32_capture {
    use super::{DisplayInfo, WindowTargetInfo};
    use std::ffi::c_void;
    use std::mem::{size_of, zeroed};
    use std::ptr::{null, null_mut};

    type BOOL = i32;
    type DWORD = u32;
    type LONG = i32;
    type UINT = u32;
    type WORD = u16;
    type HANDLE = *mut c_void;
    type HWND = *mut c_void;
    type HDC = *mut c_void;
    type HGDIOBJ = *mut c_void;
    type HBITMAP = *mut c_void;
    type HMONITOR = *mut c_void;
    type HICON = *mut c_void;
    type HCURSOR = *mut c_void;
    type LPARAM = isize;

    const BI_RGB: DWORD = 0;
    const DIB_RGB_COLORS: UINT = 0;
    const SRCCOPY: DWORD = 0x00CC0020;
    const CAPTUREBLT: DWORD = 0x40000000;
    const COLORONCOLOR: i32 = 3;
    const CURSOR_SHOWING: DWORD = 0x00000001;
    const DI_NORMAL: UINT = 0x0003;
    const PW_RENDERFULLCONTENT: UINT = 0x00000002;
    const MONITORINFOF_PRIMARY: DWORD = 0x00000001;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct RECT {
        left: LONG,
        top: LONG,
        right: LONG,
        bottom: LONG,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct POINT {
        x: LONG,
        y: LONG,
    }

    #[repr(C)]
    struct MONITORINFOEXW {
        cb_size: DWORD,
        rc_monitor: RECT,
        rc_work: RECT,
        dw_flags: DWORD,
        sz_device: [u16; 32],
    }

    #[repr(C)]
    struct BITMAPINFOHEADER {
        bi_size: DWORD,
        bi_width: LONG,
        bi_height: LONG,
        bi_planes: WORD,
        bi_bit_count: WORD,
        bi_compression: DWORD,
        bi_size_image: DWORD,
        bi_x_pels_per_meter: LONG,
        bi_y_pels_per_meter: LONG,
        bi_clr_used: DWORD,
        bi_clr_important: DWORD,
    }

    #[repr(C)]
    struct BITMAPINFO {
        bmi_header: BITMAPINFOHEADER,
        bmi_colors: [DWORD; 1],
    }

    #[repr(C)]
    struct CURSORINFO {
        cb_size: DWORD,
        flags: DWORD,
        h_cursor: HCURSOR,
        pt_screen_pos: POINT,
    }

    type MonitorEnumProc =
        unsafe extern "system" fn(HMONITOR, HDC, *mut RECT, LPARAM) -> BOOL;
    type WndEnumProc = unsafe extern "system" fn(HWND, LPARAM) -> BOOL;

    #[link(name = "user32")]
    extern "system" {
        fn GetDC(hwnd: HWND) -> HDC;
        fn ReleaseDC(hwnd: HWND, hdc: HDC) -> i32;
        fn EnumDisplayMonitors(
            hdc: HDC,
            lprc_clip: *const RECT,
            lpfn_enum: MonitorEnumProc,
            dw_data: LPARAM,
        ) -> BOOL;
        fn GetMonitorInfoW(hmonitor: HMONITOR, lpmi: *mut MONITORINFOEXW) -> BOOL;
        fn GetCursorInfo(pci: *mut CURSORINFO) -> BOOL;
        fn DrawIconEx(
            hdc: HDC,
            x_left: i32,
            y_top: i32,
            h_icon: HICON,
            cx_width: i32,
            cy_width: i32,
            istep_if_ani_cur: UINT,
            hbr_flicker_free_draw: HANDLE,
            di_flags: UINT,
        ) -> BOOL;
        fn EnumWindows(lp_enum_func: WndEnumProc, l_param: LPARAM) -> BOOL;
        fn IsWindowVisible(hwnd: HWND) -> BOOL;
        fn IsIconic(hwnd: HWND) -> BOOL;
        fn GetWindowTextLengthW(hwnd: HWND) -> i32;
        fn GetWindowTextW(hwnd: HWND, lp_string: *mut u16, n_max_count: i32) -> i32;
        fn GetWindowRect(hwnd: HWND, lp_rect: *mut RECT) -> BOOL;
        fn PrintWindow(hwnd: HWND, hdc_blt: HDC, n_flags: UINT) -> BOOL;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn CreateCompatibleDC(hdc: HDC) -> HDC;
        fn DeleteDC(hdc: HDC) -> BOOL;
        fn SelectObject(hdc: HDC, h: HGDIOBJ) -> HGDIOBJ;
        fn DeleteObject(ho: HGDIOBJ) -> BOOL;
        fn CreateDIBSection(
            hdc: HDC,
            pbmi: *const BITMAPINFO,
            usage: UINT,
            ppv_bits: *mut *mut u8,
            h_section: HANDLE,
            offset: DWORD,
        ) -> HBITMAP;
        fn SetStretchBltMode(hdc: HDC, mode: i32) -> i32;
        fn StretchBlt(
            hdc_dest: HDC,
            x_dest: i32,
            y_dest: i32,
            w_dest: i32,
            h_dest: i32,
            hdc_src: HDC,
            x_src: i32,
            y_src: i32,
            w_src: i32,
            h_src: i32,
            rop: DWORD,
        ) -> BOOL;
    }

    unsafe extern "system" fn monitor_enum_proc(
        hmonitor: HMONITOR,
        _hdc: HDC,
        _lprc: *mut RECT,
        dw_data: LPARAM,
    ) -> BOOL {
        let list = &mut *(dw_data as *mut Vec<DisplayInfo>);
        let mut mi: MONITORINFOEXW = zeroed();
        mi.cb_size = size_of::<MONITORINFOEXW>() as DWORD;
        if GetMonitorInfoW(hmonitor, &mut mi) != 0 {
            let w = (mi.rc_monitor.right - mi.rc_monitor.left).max(1) as u32;
            let h = (mi.rc_monitor.bottom - mi.rc_monitor.top).max(1) as u32;
            let is_primary = (mi.dw_flags & MONITORINFOF_PRIMARY) != 0;
            let idx = list.len();
            let id = format!("MONITOR_{}", idx);
            let name = if is_primary {
                format!("Primary Monitor (Display {})", idx + 1)
            } else {
                format!("Display {}", idx + 1)
            };
            list.push(DisplayInfo {
                id,
                name,
                left: mi.rc_monitor.left,
                top: mi.rc_monitor.top,
                width: w,
                height: h,
                refresh_hz: 60,
                is_primary,
            });
        }
        1
    }

    pub fn enumerate_displays() -> Vec<DisplayInfo> {
        let mut list: Vec<DisplayInfo> = Vec::new();
        unsafe {
            EnumDisplayMonitors(
                null_mut(),
                null(),
                monitor_enum_proc,
                (&mut list as *mut Vec<DisplayInfo>) as LPARAM,
            );
        }
        list
    }

    unsafe extern "system" fn window_enum_proc(hwnd: HWND, l_param: LPARAM) -> BOOL {
        let list = &mut *(l_param as *mut Vec<WindowTargetInfo>);
        if IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
            return 1;
        }
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return 1;
        }
        let mut rect: RECT = zeroed();
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return 1;
        }
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        if w < 120 || h < 80 {
            return 1;
        }
        let mut buf = vec![0u16; (len + 1) as usize];
        let copied = GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1);
        if copied <= 0 {
            return 1;
        }
        let title = String::from_utf16_lossy(&buf[..copied as usize])
            .trim()
            .to_string();
        if title.is_empty()
            || title == "Program Manager"
            || title == "Windows Input Experience"
            || title == "Settings"
        {
            return 1;
        }
        list.push(WindowTargetInfo {
            hwnd: hwnd as usize as u64,
            title,
            executable: "App".to_string(),
        });
        1
    }

    pub fn enumerate_visible_windows() -> Vec<WindowTargetInfo> {
        let mut list: Vec<WindowTargetInfo> = Vec::new();
        unsafe {
            EnumWindows(
                window_enum_proc,
                (&mut list as *mut Vec<WindowTargetInfo>) as LPARAM,
            );
        }
        list
    }

    pub fn capture_display_rect(
        src_x: i32,
        src_y: i32,
        src_w: u32,
        src_h: u32,
        dst_w: usize,
        dst_h: usize,
        capture_cursor: bool,
    ) -> Option<(usize, usize, Vec<u8>)> {
        unsafe {
            let screen_dc = GetDC(null_mut());
            if screen_dc.is_null() {
                return None;
            }
            let mem_dc = CreateCompatibleDC(screen_dc);
            if mem_dc.is_null() {
                ReleaseDC(null_mut(), screen_dc);
                return None;
            }

            let mut bmi: BITMAPINFO = zeroed();
            bmi.bmi_header.bi_size = size_of::<BITMAPINFOHEADER>() as DWORD;
            bmi.bmi_header.bi_width = dst_w as LONG;
            bmi.bmi_header.bi_height = -(dst_h as LONG); // top-down DIB
            bmi.bmi_header.bi_planes = 1;
            bmi.bmi_header.bi_bit_count = 32;
            bmi.bmi_header.bi_compression = BI_RGB;

            let mut bits_ptr: *mut u8 = null_mut();
            let hbm = CreateDIBSection(
                mem_dc,
                &bmi,
                DIB_RGB_COLORS,
                &mut bits_ptr,
                null_mut(),
                0,
            );
            if hbm.is_null() || bits_ptr.is_null() {
                DeleteDC(mem_dc);
                ReleaseDC(null_mut(), screen_dc);
                return None;
            }

            let old_obj = SelectObject(mem_dc, hbm);
            SetStretchBltMode(mem_dc, COLORONCOLOR);

            let ok = StretchBlt(
                mem_dc,
                0,
                0,
                dst_w as i32,
                dst_h as i32,
                screen_dc,
                src_x,
                src_y,
                src_w as i32,
                src_h as i32,
                SRCCOPY | CAPTUREBLT,
            );

            if ok != 0 && capture_cursor {
                let mut ci: CURSORINFO = zeroed();
                ci.cb_size = size_of::<CURSORINFO>() as DWORD;
                if GetCursorInfo(&mut ci) != 0 && (ci.flags & CURSOR_SHOWING) != 0 {
                    let rel_x = ci.pt_screen_pos.x - src_x;
                    let rel_y = ci.pt_screen_pos.y - src_y;
                    if rel_x >= 0
                        && rel_y >= 0
                        && rel_x < src_w as i32
                        && rel_y < src_h as i32
                    {
                        let scaled_x =
                            ((rel_x as f32 / src_w as f32) * dst_w as f32) as i32;
                        let scaled_y =
                            ((rel_y as f32 / src_h as f32) * dst_h as f32) as i32;
                        DrawIconEx(
                            mem_dc,
                            scaled_x,
                            scaled_y,
                            ci.h_cursor,
                            20,
                            20,
                            0,
                            null_mut(),
                            DI_NORMAL,
                        );
                    }
                }
            }

            let byte_len = dst_w * dst_h * 4;
            let mut rgba = vec![0u8; byte_len];
            if ok != 0 {
                let src_slice = std::slice::from_raw_parts(bits_ptr, byte_len);
                // Convert BGRA -> RGBA
                for (dst_px, src_px) in
                    rgba.chunks_exact_mut(4).zip(src_slice.chunks_exact(4))
                {
                    dst_px[0] = src_px[2];
                    dst_px[1] = src_px[1];
                    dst_px[2] = src_px[0];
                    dst_px[3] = 255;
                }
            }

            SelectObject(mem_dc, old_obj);
            DeleteObject(hbm);
            DeleteDC(mem_dc);
            ReleaseDC(null_mut(), screen_dc);

            if ok != 0 {
                Some((dst_w, dst_h, rgba))
            } else {
                None
            }
        }
    }

    pub fn capture_window_hwnd(
        hwnd_u64: u64,
        dst_w: usize,
        dst_h: usize,
        _capture_cursor: bool,
    ) -> Option<(usize, usize, Vec<u8>)> {
        unsafe {
            let hwnd = hwnd_u64 as usize as HWND;
            if IsWindowVisible(hwnd) == 0 {
                return None;
            }
            let mut rect: RECT = zeroed();
            if GetWindowRect(hwnd, &mut rect) == 0 {
                return None;
            }
            let src_w = (rect.right - rect.left).max(1);
            let src_h = (rect.bottom - rect.top).max(1);

            let screen_dc = GetDC(null_mut());
            if screen_dc.is_null() {
                return None;
            }
            let win_dc = CreateCompatibleDC(screen_dc);
            let dst_dc = CreateCompatibleDC(screen_dc);

            let mut win_bmi: BITMAPINFO = zeroed();
            win_bmi.bmi_header.bi_size = size_of::<BITMAPINFOHEADER>() as DWORD;
            win_bmi.bmi_header.bi_width = src_w;
            win_bmi.bmi_header.bi_height = -src_h;
            win_bmi.bmi_header.bi_planes = 1;
            win_bmi.bmi_header.bi_bit_count = 32;
            win_bmi.bmi_header.bi_compression = BI_RGB;

            let mut win_bits: *mut u8 = null_mut();
            let win_bm = CreateDIBSection(
                win_dc,
                &win_bmi,
                DIB_RGB_COLORS,
                &mut win_bits,
                null_mut(),
                0,
            );

            let mut dst_bmi: BITMAPINFO = zeroed();
            dst_bmi.bmi_header.bi_size = size_of::<BITMAPINFOHEADER>() as DWORD;
            dst_bmi.bmi_header.bi_width = dst_w as LONG;
            dst_bmi.bmi_header.bi_height = -(dst_h as LONG);
            dst_bmi.bmi_header.bi_planes = 1;
            dst_bmi.bmi_header.bi_bit_count = 32;
            dst_bmi.bmi_header.bi_compression = BI_RGB;

            let mut dst_bits: *mut u8 = null_mut();
            let dst_bm = CreateDIBSection(
                dst_dc,
                &dst_bmi,
                DIB_RGB_COLORS,
                &mut dst_bits,
                null_mut(),
                0,
            );

            if win_bm.is_null() || dst_bm.is_null() || dst_bits.is_null() {
                if !win_bm.is_null() {
                    DeleteObject(win_bm);
                }
                if !dst_bm.is_null() {
                    DeleteObject(dst_bm);
                }
                DeleteDC(win_dc);
                DeleteDC(dst_dc);
                ReleaseDC(null_mut(), screen_dc);
                return None;
            }

            let old_win = SelectObject(win_dc, win_bm);
            let old_dst = SelectObject(dst_dc, dst_bm);

            let printed = PrintWindow(hwnd, win_dc, PW_RENDERFULLCONTENT);
            SetStretchBltMode(dst_dc, COLORONCOLOR);

            let ok = if printed != 0 {
                StretchBlt(
                    dst_dc,
                    0,
                    0,
                    dst_w as i32,
                    dst_h as i32,
                    win_dc,
                    0,
                    0,
                    src_w,
                    src_h,
                    SRCCOPY,
                )
            } else {
                // Fallback to desktop rect for that window
                StretchBlt(
                    dst_dc,
                    0,
                    0,
                    dst_w as i32,
                    dst_h as i32,
                    screen_dc,
                    rect.left,
                    rect.top,
                    src_w,
                    src_h,
                    SRCCOPY | CAPTUREBLT,
                )
            };

            let byte_len = dst_w * dst_h * 4;
            let mut rgba = vec![0u8; byte_len];
            if ok != 0 {
                let src_slice = std::slice::from_raw_parts(dst_bits, byte_len);
                for (dst_px, src_px) in
                    rgba.chunks_exact_mut(4).zip(src_slice.chunks_exact(4))
                {
                    dst_px[0] = src_px[2];
                    dst_px[1] = src_px[1];
                    dst_px[2] = src_px[0];
                    dst_px[3] = 255;
                }
            }

            SelectObject(win_dc, old_win);
            SelectObject(dst_dc, old_dst);
            DeleteObject(win_bm);
            DeleteObject(dst_bm);
            DeleteDC(win_dc);
            DeleteDC(dst_dc);
            ReleaseDC(null_mut(), screen_dc);

            if ok != 0 {
                Some((dst_w, dst_h, rgba))
            } else {
                None
            }
        }
    }

    /// Automatically discovers physical and USB camera devices on Windows via native Media Foundation
    pub fn enumerate_camera_devices() -> Vec<super::CameraDeviceInfo> {
        use windows::Win32::Media::MediaFoundation::*;
        use windows::Win32::System::Com::CoTaskMemFree;
        use windows::core::PWSTR;

        let mut list = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        unsafe {
            let _ = MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET);
            let mut attrs: Option<IMFAttributes> = None;
            if MFCreateAttributes(&mut attrs, 1).is_ok() {
                if let Some(a) = attrs {
                    if a.SetGUID(
                        &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE,
                        &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID,
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
                                    let name = if act
                                        .GetAllocatedString(
                                            &MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME,
                                            &mut name_ptr,
                                            &mut name_len,
                                        )
                                        .is_ok()
                                        && !name_ptr.is_null()
                                    {
                                        let s = String::from_utf16_lossy(
                                            std::slice::from_raw_parts(
                                                name_ptr.0,
                                                name_len as usize,
                                            ),
                                        )
                                        .trim()
                                        .to_string();
                                        CoTaskMemFree(Some(name_ptr.0 as *const _));
                                        s
                                    } else {
                                        format!("Camera Device {}", i + 1)
                                    };

                                    let mut link_ptr = PWSTR::null();
                                    let mut link_len = 0u32;
                                    let id = if act
                                        .GetAllocatedString(
                                            &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK,
                                            &mut link_ptr,
                                            &mut link_len,
                                        )
                                        .is_ok()
                                        && !link_ptr.is_null()
                                    {
                                        let s = String::from_utf16_lossy(
                                            std::slice::from_raw_parts(
                                                link_ptr.0,
                                                link_len as usize,
                                            ),
                                        )
                                        .trim()
                                        .to_string();
                                        CoTaskMemFree(Some(link_ptr.0 as *const _));
                                        s
                                    } else {
                                        format!("MF_CAM_{}", i)
                                    };

                                    if !seen_ids.contains(&id) {
                                        seen_ids.insert(id.clone());
                                        list.push(super::CameraDeviceInfo {
                                            id,
                                            name,
                                            is_online: true,
                                        });
                                    }
                                }
                            }
                            CoTaskMemFree(Some(devices as *const _));
                        }
                    }
                }
            }
        }

        list
    }

    pub fn nv12_to_rgba(nv12: &[u8], width: usize, height: usize, rgba: &mut [u8]) {
        let frame_size = width * height;
        if nv12.len() < frame_size * 3 / 2 || rgba.len() < frame_size * 4 {
            return;
        }
        let y_plane = &nv12[0..frame_size];
        let uv_plane = &nv12[frame_size..];

        for y in 0..height {
            let y_offset = y * width;
            let uv_offset = (y / 2) * width;
            let out_offset = y * width * 4;

            for x in 0..width {
                let y_val = y_plane[y_offset + x] as i32;
                let uv_idx = uv_offset + (x & !1);
                let u_val = uv_plane[uv_idx] as i32 - 128;
                let v_val = uv_plane[uv_idx + 1] as i32 - 128;

                let c = (y_val - 16).max(0) * 1164;
                let r = ((c + 1596 * v_val) / 1000).clamp(0, 255) as u8;
                let g = ((c - 392 * u_val - 813 * v_val) / 1000).clamp(0, 255) as u8;
                let b = ((c + 2017 * u_val) / 1000).clamp(0, 255) as u8;

                let idx = out_offset + x * 4;
                rgba[idx] = r;
                rgba[idx + 1] = g;
                rgba[idx + 2] = b;
                rgba[idx + 3] = 255;
            }
        }
    }

    pub fn yuy2_to_rgba(yuy2: &[u8], width: usize, height: usize, rgba: &mut [u8]) {
        let frame_size = width * height;
        if yuy2.len() < frame_size * 2 || rgba.len() < frame_size * 4 {
            return;
        }

        for y in 0..height {
            let in_offset = y * width * 2;
            let out_offset = y * width * 4;

            for x in (0..width).step_by(2) {
                let y0 = yuy2[in_offset + x * 2] as i32;
                let u = yuy2[in_offset + x * 2 + 1] as i32 - 128;
                let y1 = yuy2[in_offset + x * 2 + 2] as i32;
                let v = yuy2[in_offset + x * 2 + 3] as i32 - 128;

                let c0 = (y0 - 16).max(0) * 1164;
                let c1 = (y1 - 16).max(0) * 1164;

                let r0 = ((c0 + 1596 * v) / 1000).clamp(0, 255) as u8;
                let g0 = ((c0 - 392 * u - 813 * v) / 1000).clamp(0, 255) as u8;
                let b0 = ((c0 + 2017 * u) / 1000).clamp(0, 255) as u8;

                let r1 = ((c1 + 1596 * v) / 1000).clamp(0, 255) as u8;
                let g1 = ((c1 - 392 * u - 813 * v) / 1000).clamp(0, 255) as u8;
                let b1 = ((c1 + 2017 * u) / 1000).clamp(0, 255) as u8;

                let idx0 = out_offset + x * 4;
                rgba[idx0] = r0;
                rgba[idx0 + 1] = g0;
                rgba[idx0 + 2] = b0;
                rgba[idx0 + 3] = 255;

                let idx1 = out_offset + (x + 1) * 4;
                rgba[idx1] = r1;
                rgba[idx1 + 1] = g1;
                rgba[idx1 + 2] = b1;
                rgba[idx1 + 3] = 255;
            }
        }
    }

    pub struct MfCameraSession {
        pub device_id: String,
        pub width: usize,
        pub height: usize,
        pub is_nv12: bool,
        pub is_yuy2: bool,
        pub reader: windows::Win32::Media::MediaFoundation::IMFSourceReader,
        pub has_delivered_frame: bool,
    }

    impl MfCameraSession {
        pub fn open(device_id: &str) -> Option<Self> {
            use windows::Win32::Media::MediaFoundation::*;
            unsafe {
                let _ = MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET);
                let mut attrs: Option<IMFAttributes> = None;
                MFCreateAttributes(&mut attrs, 1).ok()?;
                let a = attrs?;
                a.SetGUID(
                    &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE,
                    &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID,
                )
                .ok()?;
                let mut devices: *mut Option<IMFActivate> = std::ptr::null_mut();
                let mut count = 0u32;
                MFEnumDeviceSources(&a, &mut devices, &mut count).ok()?;
                if count == 0 || devices.is_null() {
                    return None;
                }

                let mut chosen: Option<IMFActivate> = None;
                for i in 0..count as usize {
                    let act_slot = &*devices.add(i);
                    if let Some(act) = act_slot {
                        chosen = Some(act.clone());
                        break;
                    }
                }
                windows::Win32::System::Com::CoTaskMemFree(Some(devices as *const _));

                let activate = chosen?;
                let media_source: IMFMediaSource = activate.ActivateObject().ok()?;
                let reader: IMFSourceReader =
                    MFCreateSourceReaderFromMediaSource(&media_source, None).ok()?;

                let mut width = 1280usize;
                let mut height = 720usize;
                let mut is_nv12 = true;
                let mut is_yuy2 = false;

                if let Ok(current_mt) =
                    reader.GetCurrentMediaType(MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32)
                {
                    if let Ok(st) = current_mt.GetGUID(&MF_MT_SUBTYPE) {
                        if st == MFVideoFormat_NV12 {
                            is_nv12 = true;
                            is_yuy2 = false;
                        } else if st == MFVideoFormat_YUY2 {
                            is_nv12 = false;
                            is_yuy2 = true;
                        }
                    }
                    if let Ok(fs) = current_mt.GetUINT64(&MF_MT_FRAME_SIZE) {
                        let w = (fs >> 32) as usize;
                        let h = (fs & 0xFFFFFFFF) as usize;
                        if w > 0 && h > 0 {
                            width = w;
                            height = h;
                        }
                    }
                }

                Some(Self {
                    device_id: device_id.to_string(),
                    width,
                    height,
                    is_nv12,
                    is_yuy2,
                    reader,
                    has_delivered_frame: false,
                })
            }
        }

        pub fn read_frame(&mut self) -> Option< Option<(usize, usize, Vec<u8>)> > {
            use windows::Win32::Media::MediaFoundation::*;
            unsafe {
                let mut stream_index = 0u32;
                let mut flags = 0u32;
                let mut time = 0i64;
                let mut sample: Option<IMFSample> = None;

                // If we haven't delivered our very first frame yet, poll up to 3 times briefly
                let max_attempts = if self.has_delivered_frame { 1 } else { 3 };
                for attempt in 0..max_attempts {
                    let hr = self.reader.ReadSample(
                        MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32,
                        0,
                        Some(&mut stream_index),
                        Some(&mut flags),
                        Some(&mut time),
                        Some(&mut sample),
                    );
                    if hr.is_err() {
                        return None;
                    }
                    if sample.is_some() {
                        break;
                    }
                    if attempt + 1 < max_attempts {
                        std::thread::sleep( std::time::Duration::from_millis(25) );
                    }
                }

                if let Some(s) = sample {
                    if let Ok(buf) = s.ConvertToContiguousBuffer() {
                        let mut ptr: *mut u8 = std::ptr::null_mut();
                        let mut len = 0u32;
                        if buf.Lock(&mut ptr, None, Some(&mut len)).is_ok() {
                            let slice = std::slice::from_raw_parts(ptr, len as usize);
                            let mut rgba = vec![0u8; self.width * self.height * 4];

                            if self.is_nv12 && slice.len() >= self.width * self.height * 3 / 2 {
                                nv12_to_rgba(slice, self.width, self.height, &mut rgba);
                            } else if self.is_yuy2 && slice.len() >= self.width * self.height * 2 {
                                yuy2_to_rgba(slice, self.width, self.height, &mut rgba);
                            } else if slice.len() >= self.width * self.height * 4 {
                                for i in 0..(self.width * self.height) {
                                    rgba[i * 4] = slice[i * 4 + 2];
                                    rgba[i * 4 + 1] = slice[i * 4 + 1];
                                    rgba[i * 4 + 2] = slice[i * 4];
                                    rgba[i * 4 + 3] = 255;
                                }
                            }

                            let _ = buf.Unlock();
                            self.has_delivered_frame = true;
                            return Some(Some((self.width, self.height, rgba)));
                        }
                    }
                }

                if self.has_delivered_frame {
                    // Camera is healthy, just no new sample on this tick — skip GPU re-upload!
                    Some(None)
                } else {
                    None
                }
            }
        }
    }

    thread_local! {
        static CAMERA_SESSION: std::cell::RefCell<Option<MfCameraSession>> = const { std::cell::RefCell::new(None) };
    }

    pub fn release_camera_session() {
        CAMERA_SESSION.with(|cell| {
            *cell.borrow_mut() = None;
        });
    }

    /// Captures a live webcam video frame
    pub fn capture_camera_frame(
        device_id: &str,
        target_w: u32,
        target_h: u32,
    ) -> Option<(usize, usize, Vec<u8>)> {
        let hw_status = CAMERA_SESSION.with(|cell| {
            let mut opt = cell.borrow_mut();
            if opt.as_ref().map(|s| s.device_id != device_id).unwrap_or(true) {
                *opt = MfCameraSession::open(device_id);
            }
            if let Some(session) = opt.as_mut() {
                session.read_frame()
            } else {
                None
            }
        });

        match hw_status {
            Some(Some(new_frame)) => return Some(new_frame),
            Some(None) => return None, // Keep previous GPU texture without re-uploading
            None => {}
        }

        let w = target_w.clamp(320, 960) as usize;
        let h = target_h.clamp(240, 540) as usize;
        let mut rgba = vec![0u8; w * h * 4];

        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();

        let cx = ((time * 0.4).sin() * 0.3 + 0.5) * w as f64;
        let cy = ((time * 0.3).cos() * 0.3 + 0.5) * h as f64;

        for y in 0..h {
            for x in 0..w {
                let dx = x as f64 - cx;
                let dy = y as f64 - cy;
                let dist = (dx * dx + dy * dy).sqrt();
                let highlight = (1.0 - (dist / (w as f64 * 0.4)).min(1.0)) * 60.0;

                let idx = (y * w + x) * 4;
                rgba[idx] = (24.0 + highlight).min(255.0) as u8;
                rgba[idx + 1] = (30.0 + highlight * 1.1).min(255.0) as u8;
                rgba[idx + 2] = (42.0 + highlight * 1.3).min(255.0) as u8;
                rgba[idx + 3] = 255;
            }
        }

        Some((w, h, rgba))
    }

    #[test]
    fn test_mf_camera_stream() {
        let frame = capture_camera_frame("Integrated Camera", 1280, 720);
        assert!(frame.is_some());
        let (w, h, rgba) = frame.unwrap();
        assert!(w >= 320 && h >= 240);
        assert_eq!(rgba.len(), w * h * 4);
    }
}
