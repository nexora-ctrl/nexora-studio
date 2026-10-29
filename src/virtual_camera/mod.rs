//! NEXORA Studio Native Virtual Camera Driver & Shared Memory IPC
//!
//! Provides DirectShow / Windows Media Foundation integration so that
//! "NEXORA Virtual Camera" appears natively in Zoom, MS Teams, Google Meet,
//! Discord, OBS Studio, and WebRTC browsers.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use windows::core::w;
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::System::Memory::{
    CreateFileMappingW, MapViewOfFile, UnmapViewOfFile, FILE_MAP_ALL_ACCESS, PAGE_READWRITE,
};

/// 32-byte frame header required by OBS VirtualCam DirectShow filter
pub const FRAME_HEADER_SIZE: usize = 32;

pub const SHARED_QUEUE_STATE_INVALID: u32 = 0;
pub const SHARED_QUEUE_STATE_STARTING: u32 = 1;
pub const SHARED_QUEUE_STATE_READY: u32 = 2;
pub const SHARED_QUEUE_STATE_STOPPING: u32 = 3;

/// DirectShow Shared Memory Queue Header (OBSVirtualCam protocol)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct QueueHeader {
    pub write_idx: u32,
    pub read_idx: u32,
    pub state: u32, // 0 = stopped, 1 = starting, 2 = ready/running
    pub offsets: [u32; 3],
    pub video_type: u32, // 0 = NV12
    pub cx: u32,
    pub cy: u32,
    pub interval: u64, // 100ns units (e.g. 333333 for 30fps)
    pub reserved: [u32; 8],
}

/// Windows Registry helper to ensure "NEXORA Virtual Camera" is registered
/// in DirectShow Video Input Category under HKCU (no Admin required).
pub fn ensure_nexora_virtual_cam_registered() -> bool {
    use windows_sys::Win32::System::Registry::*;

    unsafe {
        let sub_key = windows_sys::core::w!(
            "Software\\Classes\\CLSID\\{860BB310-5D01-11d0-BD3B-00A0C911CE86}\\Instance\\{2B462C23-1F84-4829-9F8E-0062C1FE0E1A}"
        );

        let mut hkey: HKEY = std::ptr::null_mut();
        let res = RegCreateKeyW(HKEY_CURRENT_USER, sub_key, &mut hkey);
        if res != 0 {
            return false;
        }

        // FriendlyName: "NEXORA Virtual Camera"
        let friendly_name = windows_sys::core::w!("NEXORA Virtual Camera");
        let name_bytes = (22 * 2) as u32; // UTF-16 length including null
        RegSetValueExW(
            hkey,
            windows_sys::core::w!("FriendlyName"),
            0,
            REG_SZ,
            friendly_name as *const _ as *const u8,
            name_bytes,
        );

        // CLSID pointing to virtual camera filter DLL COM class
        let clsid_str = windows_sys::core::w!("{A3FCE0F5-3493-419F-958A-ABA1250EC20B}");
        let clsid_bytes = (39 * 2) as u32;
        RegSetValueExW(
            hkey,
            windows_sys::core::w!("CLSID"),
            0,
            REG_SZ,
            clsid_str as *const _ as *const u8,
            clsid_bytes,
        );

        // FilterData for DirectShow video capture pin (NV12 format)
        let filter_data: [u8; 88] = [
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x30, 0x70, 0x69, 0x33, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x74,
            0x79, 0x33, 0x00, 0x00, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00, 0x48, 0x00, 0x00, 0x00,
            0x76, 0x69, 0x64, 0x73, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38,
            0x9B, 0x71, 0x4E, 0x56, 0x31, 0x32, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA,
            0x00, 0x38, 0x9B, 0x71,
        ];

        RegSetValueExW(
            hkey,
            windows_sys::core::w!("FilterData"),
            0,
            REG_BINARY,
            filter_data.as_ptr(),
            filter_data.len() as u32,
        );

        RegCloseKey(hkey);
        true
    }
}

/// Fast conversion from 32-bit RGBA (or RGBX) to NV12 format.
///
/// NV12 layout:
/// - Y plane: width * height bytes (luminance)
/// - UV plane: (width * height) / 2 bytes (interleaved U and V, 2x2 subsampled)
pub fn rgba_to_nv12(rgba: &[u8], width: usize, height: usize, nv12_out: &mut [u8]) {
    let y_plane_size = width * height;
    let expected_nv12_size = y_plane_size + (y_plane_size / 2);
    if rgba.len() < width * height * 4 || nv12_out.len() < expected_nv12_size {
        return;
    }

    let (y_plane, uv_plane) = nv12_out.split_at_mut(y_plane_size);

    // Compute Y for every pixel
    for y in 0..height {
        let row_rgba_offset = y * width * 4;
        let row_y_offset = y * width;
        for x in 0..width {
            let px_offset = row_rgba_offset + (x * 4);
            let r = rgba[px_offset] as i32;
            let g = rgba[px_offset + 1] as i32;
            let b = rgba[px_offset + 2] as i32;

            // BT.601 standard full range Y calculation
            let y_val = ((66 * r + 129 * g + 25 * b + 128) >> 8) + 16;
            y_plane[row_y_offset + x] = y_val.clamp(16, 235) as u8;
        }
    }

    // Compute U and V for 2x2 blocks (interleaved UV)
    let half_h = height / 2;
    let half_w = width / 2;
    for hy in 0..half_h {
        let y_top = hy * 2;
        let uv_row_offset = hy * width; // width bytes per row in interleaved UV
        for hx in 0..half_w {
            let x_left = hx * 2;
            let px_offset = (y_top * width + x_left) * 4;
            let r = rgba[px_offset] as i32;
            let g = rgba[px_offset + 1] as i32;
            let b = rgba[px_offset + 2] as i32;

            let u_val = ((-38 * r - 74 * g + 112 * b + 128) >> 8) + 128;
            let v_val = ((112 * r - 94 * g - 18 * b + 128) >> 8) + 128;

            let uv_idx = uv_row_offset + (hx * 2);
            uv_plane[uv_idx] = u_val.clamp(16, 240) as u8;
            uv_plane[uv_idx + 1] = v_val.clamp(16, 240) as u8;
        }
    }
}

/// Shared Memory Producer for OBS Virtual Camera / NEXORA Virtual Camera
pub struct VCamProducer {
    handle: HANDLE,
    view: *mut u8,
    _total_size: usize,
    width: u32,
    height: u32,
    write_idx: u32,
    start_time: Instant,
    scratch_nv12: Vec<u8>,
}

unsafe impl Send for VCamProducer {}
unsafe impl Sync for VCamProducer {}

impl VCamProducer {
    /// Creates or opens the shared memory ring buffer at the requested resolution.
    pub fn new(width: u32, height: u32, fps: u32) -> windows::core::Result<Self> {
        let interval = (10_000_000 / fps.max(1) as u64) as u64; // 100ns units

        // Sync %APPDATA%\obs-virtualcam.txt so DirectShow filter reads the exact active dimensions
        if let Ok(appdata) = std::env::var("APPDATA") {
            let txt_path = std::path::Path::new(&appdata).join("obs-virtualcam.txt");
            let _ = std::fs::write(&txt_path, format!("{}x{}x{}\n", width, height, interval));
        }

        let align_32 = |v: usize| -> usize { (v + 31) & !31 };
        let header_size = align_32(std::mem::size_of::<QueueHeader>());
        let frame_data_size = (width as usize * height as usize * 3) / 2;
        let slot_size = align_32(FRAME_HEADER_SIZE + frame_data_size);
        let total_size = header_size + 3 * slot_size;

        unsafe {
            let handle = CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                None,
                PAGE_READWRITE,
                0,
                total_size as u32,
                w!("OBSVirtualCamVideo"),
            )?;

            let view = MapViewOfFile(handle, FILE_MAP_ALL_ACCESS, 0, 0, total_size).Value as *mut u8;

            if view.is_null() {
                let _ = CloseHandle(handle);
                return Err(windows::core::Error::from_thread());
            }

            // Initialize QueueHeader with 32-byte aligned offsets
            let offsets = [
                header_size as u32,
                (header_size + slot_size) as u32,
                (header_size + 2 * slot_size) as u32,
            ];

            let header = QueueHeader {
                write_idx: 0,
                read_idx: 0,
                state: SHARED_QUEUE_STATE_READY, // 2 = Active & Ready
                offsets,
                video_type: 0, // NV12
                cx: width,
                cy: height,
                interval,
                reserved: [0; 8],
            };

            std::ptr::copy_nonoverlapping(
                &header as *const QueueHeader as *const u8,
                view,
                std::mem::size_of::<QueueHeader>(),
            );

            let nv12_buf_len = (width as usize * height as usize * 3) / 2;

            let mut producer = Self {
                handle,
                view,
                _total_size: total_size,
                width,
                height,
                write_idx: 0,
                start_time: Instant::now(),
                scratch_nv12: vec![0u8; nv12_buf_len],
            };

            // Pre-seed first frame immediately so consumer clients have a valid initial frame
            producer.write_placeholder("NEXORA Studio - Live Virtual Camera");

            Ok(producer)
        }
    }

    /// Writes an RGBA video frame into the shared memory queue.
    pub fn write_rgba_frame(&mut self, rgba: &[u8], width: usize, height: usize) {
        if self.view.is_null() {
            return;
        }

        // Convert RGBA -> NV12
        rgba_to_nv12(rgba, width, height, &mut self.scratch_nv12);

        unsafe {
            let header_ptr = self.view as *mut QueueHeader;
            let current_slot = self.write_idx as usize;
            let slot_offset = (*header_ptr).offsets[current_slot] as usize;

            let frame_ptr = self.view.add(slot_offset);

            // Write 32-byte frame header (timestamp in 100ns units)
            let elapsed_100ns =
                (self.start_time.elapsed().as_nanos() / 100) as u64;
            std::ptr::copy_nonoverlapping(
                &elapsed_100ns as *const u64 as *const u8,
                frame_ptr,
                8,
            );

            // Write NV12 pixel data immediately following the 32-byte header
            let data_dest = frame_ptr.add(FRAME_HEADER_SIZE);
            std::ptr::copy_nonoverlapping(
                self.scratch_nv12.as_ptr(),
                data_dest,
                self.scratch_nv12.len(),
            );

            // Advance write index (triple buffering: 0 -> 1 -> 2 -> 0)
            self.write_idx = (self.write_idx + 1) % 3;
            (*header_ptr).write_idx = self.write_idx;
            (*header_ptr).state = SHARED_QUEUE_STATE_READY;
        }
    }

    /// Generates a crisp placeholder pattern when idle or offline.
    pub fn write_placeholder(&mut self, _text: &str) {
        let w = self.width as usize;
        let h = self.height as usize;
        let mut rgba = vec![24u8; w * h * 4];

        // Draw a clean dark gradient background
        for y in 0..h {
            let row_offset = y * w * 4;
            let grad_val = (18 + (y * 22 / h)) as u8;
            for x in 0..w {
                let px = row_offset + x * 4;
                rgba[px] = grad_val;        // R
                rgba[px + 1] = grad_val + 3; // G
                rgba[px + 2] = grad_val + 8; // B
                rgba[px + 3] = 255;          // A
            }
        }

        self.write_rgba_frame(&rgba, w, h);
    }

    /// Stops the queue and marks state as stopping.
    pub fn stop(&mut self) {
        if !self.view.is_null() {
            unsafe {
                let header_ptr = self.view as *mut QueueHeader;
                (*header_ptr).state = SHARED_QUEUE_STATE_STOPPING;
            }
        }
    }
}

impl Drop for VCamProducer {
    fn drop(&mut self) {
        self.stop();
        if !self.view.is_null() {
            unsafe {
                let _ = UnmapViewOfFile(windows::Win32::System::Memory::MEMORY_MAPPED_VIEW_ADDRESS {
                    Value: self.view as *mut _,
                });
                let _ = CloseHandle(self.handle);
            }
            self.view = std::ptr::null_mut();
        }
    }
}

/// Latest captured studio frame (full viewport screenshot + canvas crop region in physical pixels).
/// Crop & scale to 1080p is performed on the Virtual Camera thread to keep the UI thread light.
struct PendingCanvasFrame {
    image: Arc<egui::ColorImage>,
    crop: [usize; 4], // x, y, w, h
    seq: u64,
}

/// Global Background Virtual Camera Manager Thread
pub struct VirtualCameraService {
    is_running: Arc<AtomicBool>,
    current_frame: Arc<Mutex<Option<PendingCanvasFrame>>>,
    frame_seq: u64,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl VirtualCameraService {
    pub fn start() -> Self {
        let is_running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&is_running);
        let current_frame: Arc<Mutex<Option<PendingCanvasFrame>>> = Arc::new(Mutex::new(None));
        let frame_clone = Arc::clone(&current_frame);

        // Ensure registry entry is present on startup
        ensure_nexora_virtual_cam_registered();

        let thread_handle = std::thread::spawn(move || {
            let mut producer = match VCamProducer::new(1920, 1080, 30) {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!("Failed to open Virtual Camera shared memory: {:?}", e);
                    return;
                }
            };

            let frame_dur = Duration::from_millis(33); // ~30 fps
            let mut last_seq: u64 = 0;
            let mut cached_rgba: Option<Vec<u8>> = None;

            while running_clone.load(Ordering::Relaxed) {
                let start = Instant::now();

                // Pick up the newest canvas frame (if any) and rescale it to 1920x1080
                let pending = frame_clone.lock().ok().and_then(|guard| {
                    guard.as_ref().filter(|f| f.seq != last_seq).map(|f| (Arc::clone(&f.image), f.crop, f.seq))
                });
                if let Some((image, crop, seq)) = pending {
                    last_seq = seq;
                    cached_rgba = Some(crop_and_scale_color_image(
                        &image, crop[0], crop[1], crop[2], crop[3], 1920, 1080,
                    ));
                }

                // Write the last live canvas frame (repeat it when UI is idle); placeholder until first frame
                match cached_rgba.as_ref() {
                    Some(rgba) => producer.write_rgba_frame(rgba, 1920, 1080),
                    None => producer.write_placeholder("NEXORA Studio - Live Virtual Camera"),
                }

                let elapsed = start.elapsed();
                if elapsed < frame_dur {
                    std::thread::sleep(frame_dur - elapsed);
                }
            }

            producer.stop();
        });

        Self {
            is_running,
            current_frame,
            frame_seq: 0,
            thread_handle: Some(thread_handle),
        }
    }

    /// Pushes a full viewport screenshot plus the canvas crop rect (physical pixels).
    /// The Virtual Camera thread crops & scales it to 1920x1080 NV12.
    pub fn push_canvas_screenshot(&mut self, image: Arc<egui::ColorImage>, crop: [usize; 4]) {
        if crop[2] < 2 || crop[3] < 2 {
            return;
        }
        self.frame_seq = self.frame_seq.wrapping_add(1).max(1);
        if let Ok(mut guard) = self.current_frame.lock() {
            *guard = Some(PendingCanvasFrame {
                image,
                crop,
                seq: self.frame_seq,
            });
        }
    }

    pub fn stop(&mut self) {
        self.is_running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

/// Crops a sub-rectangle from an `egui::ColorImage` and scales it to `out_w x out_h` RGBA8
pub fn crop_and_scale_color_image(
    image: &egui::ColorImage,
    crop_x: usize,
    crop_y: usize,
    crop_w: usize,
    crop_h: usize,
    out_w: usize,
    out_h: usize,
) -> Vec<u8> {
    let mut out = vec![0u8; out_w * out_h * 4];
    let img_w = image.width();
    let img_h = image.height();

    if img_w == 0 || img_h == 0 || crop_w == 0 || crop_h == 0 {
        return out;
    }

    for oy in 0..out_h {
        let sy = crop_y + (oy * crop_h) / out_h;
        let sy = sy.min(img_h.saturating_sub(1));
        let row_in = sy * img_w;
        let row_out = oy * out_w * 4;

        for ox in 0..out_w {
            let sx = crop_x + (ox * crop_w) / out_w;
            let sx = sx.min(img_w.saturating_sub(1));
            let px = image.pixels[row_in + sx];

            let out_idx = row_out + ox * 4;
            out[out_idx] = px.r();
            out[out_idx + 1] = px.g();
            out[out_idx + 2] = px.b();
            out[out_idx + 3] = 255;
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crop_and_scale_color_image() {
        let img = egui::ColorImage::new([100, 100], egui::Color32::from_rgb(255, 0, 0));
        let res = crop_and_scale_color_image(&img, 10, 10, 50, 50, 1920, 1080);
        assert_eq!(res.len(), 1920 * 1080 * 4);
        assert_eq!(res[0], 255);
        assert_eq!(res[1], 0);
        assert_eq!(res[2], 0);
        assert_eq!(res[3], 255);
    }
}

