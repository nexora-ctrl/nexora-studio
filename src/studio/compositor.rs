use crate::capture::LiveCaptureEngine;
use crate::studio::scene::Scene;
use crate::studio::source::{SourceItem, SourceKind};

/// Studio-grade Offscreen Canvas Compositor
///
/// Composes all visual sources in a scene directly from their native high-resolution capture
/// buffers (1080p Desktop Duplication, 1080p Window Capture, Webcam, Media files, Slides) into a
/// pristine, full-resolution RGBA target buffer matching the output video preset (1920x1080).
///
/// This eliminates the catastrophic double-degradation of screenshotting the small UI preview
/// (~600x338 px on screen) and nearest-neighbor upscaling it back to 1080p.
pub fn compose_scene_to_rgba(
    scene: &Scene,
    capture_engine: &LiveCaptureEngine,
    target_w: u32,
    target_h: u32,
    time_sec: f64,
) -> Option<Vec<u8>> {
    if target_w == 0 || target_h == 0 {
        return None;
    }

    let has_any_visible_source = scene.sources.iter().any(|s| s.visible);
    if !has_any_visible_source {
        return None;
    }

    let buf_len = (target_w * target_h * 4) as usize;
    // Dark stage surface background: [16, 20, 28, 255]
    let mut canvas = vec![0u8; buf_len];
    for p in canvas.chunks_exact_mut(4) {
        p[0] = 16;
        p[1] = 20;
        p[2] = 28;
        p[3] = 255;
    }

    let mut has_composed_any = false;

    // Draw from bottom of Source list (back layer) to top of Source list (front layer) - OBS Z-Order
    for src in scene.sources.iter().rev() {
        if !src.visible {
            continue;
        }

        let (vis_x, vis_y, vis_w, vis_h) = src.transform.visible_norm_rect();
        let dst_x = (vis_x * target_w as f32).round() as i32;
        let dst_y = (vis_y * target_h as f32).round() as i32;
        let dst_w = ((vis_w * target_w as f32).round() as u32).max(1);
        let dst_h = ((vis_h * target_h as f32).round() as u32).max(1);

        // Handle Solid Color Source
        if let SourceKind::ColorSource { color_rgba, .. } = &src.kind {
            fill_rect(
                &mut canvas,
                target_w,
                target_h,
                dst_x,
                dst_y,
                dst_w,
                dst_h,
                *color_rgba,
            );
            has_composed_any = true;
            continue;
        }

        // Resolve Capture Engine Key
        let key_opt = get_source_cache_key(src, time_sec);
        let Some(key) = key_opt else { continue };

        let Some(frame) = capture_engine.get_latest_frame(&key) else {
            continue;
        };

        if frame.width == 0 || frame.height == 0 || frame.rgba.is_empty() {
            continue;
        }

        let fw = frame.width;
        let fh = frame.height;

        let sc_x0 = ((src.transform.crop_left * fw as f32).round() as usize).min(fw.saturating_sub(1));
        let sc_y0 = ((src.transform.crop_top * fh as f32).round() as usize).min(fh.saturating_sub(1));
        let sc_x1 = ((fw as f32 * (1.0 - src.transform.crop_right)).round() as usize).clamp(sc_x0 + 1, fw);
        let sc_y1 = ((fh as f32 * (1.0 - src.transform.crop_bottom)).round() as usize).clamp(sc_y0 + 1, fh);
        let sc_w = sc_x1 - sc_x0;
        let sc_h = sc_y1 - sc_y0;

        let (corner_radius, border_thickness, border_color) = match &src.kind {
            SourceKind::WebcamDevice {
                corner_radius,
                border_thickness,
                border_color_rgba,
                ..
            } => (*corner_radius, *border_thickness, Some(*border_color_rgba)),
            _ => (0.0, 0.0, None),
        };

        // Fast path: Full screen 1:1 match without filters or special transforms
        if dst_x == 0
            && dst_y == 0
            && dst_w == target_w
            && dst_h == target_h
            && sc_w == target_w as usize
            && sc_h == target_h as usize
            && sc_x0 == 0
            && sc_y0 == 0
            && !src.filter.is_active()
            && corner_radius <= 0.0
            && border_thickness <= 0.0
        {
            let copy_len = (target_w * target_h * 4) as usize;
            if frame.rgba.len() >= copy_len {
                canvas[..copy_len].copy_from_slice(&frame.rgba[..copy_len]);
                has_composed_any = true;
                continue;
            }
        }

        // Bilinear scaling and blending path
        blit_source_frame(
            &mut canvas,
            target_w,
            target_h,
            &frame.rgba,
            fw,
            fh,
            sc_x0,
            sc_y0,
            sc_w,
            sc_h,
            dst_x,
            dst_y,
            dst_w,
            dst_h,
            &src.filter,
            corner_radius,
            border_thickness,
            border_color,
        );
        has_composed_any = true;
    }

    if has_composed_any {
        Some(canvas)
    } else {
        None
    }
}

fn get_source_cache_key(src: &SourceItem, time_sec: f64) -> Option<String> {
    match &src.kind {
        SourceKind::DisplayCapture {
            monitor_id,
            capture_cursor,
            ..
        } => Some(format!("DISP:{}:{}", monitor_id, capture_cursor)),
        SourceKind::WindowCapture {
            window_hwnd,
            capture_cursor,
            ..
        } => {
            if *window_hwnd != 0 {
                Some(format!("WIN:{}:{}", window_hwnd, capture_cursor))
            } else {
                None
            }
        }
        SourceKind::GameCapture {
            window_hwnd,
            capture_cursor,
            ..
        } => {
            if *window_hwnd != 0 {
                Some(format!("WIN:{}:{}", window_hwnd, capture_cursor))
            } else {
                None
            }
        }
        SourceKind::WebcamDevice {
            device_id,
            resolution,
            ..
        } => Some(format!("CAM:{}:{}x{}", device_id, resolution.0, resolution.1)),
        SourceKind::MediaSource { file_path, .. } => {
            if !file_path.trim().is_empty() && !SourceKind::is_audio_only_media(file_path) {
                Some(format!("MEDIA:{}", file_path))
            } else {
                None
            }
        }
        SourceKind::ImageSource { file_path, .. } => {
            if !file_path.trim().is_empty() {
                Some(format!("IMG:{}", file_path))
            } else {
                None
            }
        }
        SourceKind::ImageSlideShow {
            files,
            slide_time_ms,
            loop_slides,
            ..
        } => {
            let valid_files: Vec<&String> = files.iter().filter(|f| !f.trim().is_empty()).collect();
            if !valid_files.is_empty() {
                let n = valid_files.len();
                let slide_ms = (*slide_time_ms).clamp(800, 60_000) as f64;
                let total_ms = time_sec * 1000.0;
                let raw_idx = (total_ms / slide_ms).floor() as usize;
                let cur_idx = if *loop_slides {
                    raw_idx % n
                } else {
                    raw_idx.min(n - 1)
                };
                Some(format!("IMG:{}", valid_files[cur_idx]))
            } else {
                None
            }
        }
        SourceKind::PowerPointSource {
            window_hwnd,
            slides,
            slide_index,
            ..
        } => {
            if *window_hwnd != 0 {
                Some(format!("WIN:{}:true", window_hwnd))
            } else if let Some(cur_slide) = slides.get(*slide_index) {
                Some(format!("IMG:{}", cur_slide))
            } else {
                None
            }
        }
        SourceKind::PdfSource {
            pages,
            page_index,
            ..
        } => {
            if let Some(cur_page) = pages.get(*page_index) {
                Some(format!("IMG:{}", cur_page))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn fill_rect(
    dst: &mut [u8],
    dst_w: u32,
    dst_h: u32,
    x0: i32,
    y0: i32,
    w: u32,
    h: u32,
    color: [u8; 4],
) {
    if color[3] == 0 {
        return;
    }
    let x_start = x0.max(0) as usize;
    let y_start = y0.max(0) as usize;
    let x_end = ((x0 + w as i32).max(0) as usize).min(dst_w as usize);
    let y_end = ((y0 + h as i32).max(0) as usize).min(dst_h as usize);

    if x_start >= x_end || y_start >= y_end {
        return;
    }

    let alpha = color[3] as u32;
    let inv_alpha = 255 - alpha;

    for y in y_start..y_end {
        let row_idx = (y * dst_w as usize + x_start) * 4;
        let row = &mut dst[row_idx..(y * dst_w as usize + x_end) * 4];

        if alpha == 255 {
            for px in row.chunks_exact_mut(4) {
                px[0] = color[0];
                px[1] = color[1];
                px[2] = color[2];
                px[3] = 255;
            }
        } else {
            for px in row.chunks_exact_mut(4) {
                px[0] = ((color[0] as u32 * alpha + px[0] as u32 * inv_alpha) / 255) as u8;
                px[1] = ((color[1] as u32 * alpha + px[1] as u32 * inv_alpha) / 255) as u8;
                px[2] = ((color[2] as u32 * alpha + px[2] as u32 * inv_alpha) / 255) as u8;
                px[3] = 255;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn blit_source_frame(
    dst: &mut [u8],
    dst_w: u32,
    dst_h: u32,
    src_rgba: &[u8],
    src_w: usize,
    _src_h: usize,
    sc_x0: usize,
    sc_y0: usize,
    sc_w: usize,
    sc_h: usize,
    dx0: i32,
    dy0: i32,
    dw: u32,
    dh: u32,
    filter: &crate::studio::source::SourceFilterConfig,
    corner_radius: f32,
    border_thickness: f32,
    border_color: Option<[u8; 4]>,
) {
    if dw == 0 || dh == 0 || sc_w == 0 || sc_h == 0 {
        return;
    }

    let dst_x_min = dx0.max(0);
    let dst_y_min = dy0.max(0);
    let dst_x_max = (dx0 + dw as i32).min(dst_w as i32);
    let dst_y_max = (dy0 + dh as i32).min(dst_h as i32);

    if dst_x_min >= dst_x_max || dst_y_min >= dst_y_max {
        return;
    }

    let filter_active = filter.is_active();
    let has_corners = corner_radius > 0.5;
    let has_border = border_thickness > 0.5 && border_color.is_some();
    let b_col = border_color.unwrap_or([255, 255, 255, 255]);

    let r_px = if corner_radius >= 150.0 {
        // Circle / Avatar mode
        (dw.min(dh) as f32) / 2.0
    } else {
        corner_radius * (dw as f32 / 300.0).clamp(0.5, 3.0)
    };

    let b_thick_px = border_thickness * (dw as f32 / 300.0).clamp(0.5, 3.0);

    let src_x_max = (sc_x0 + sc_w).saturating_sub(1);
    let src_y_max = (sc_y0 + sc_h).saturating_sub(1);

    for dy in dst_y_min..dst_y_max {
        let local_y = (dy - dy0) as f32;
        let v_frac = local_y / dh as f32;
        let sy_f = sc_y0 as f32 + v_frac * sc_h as f32;
        let sy0 = (sy_f.floor() as usize).clamp(sc_y0, src_y_max);
        let sy1 = (sy0 + 1).clamp(sc_y0, src_y_max);
        let fy = sy_f - sy0 as f32;

        let row0 = sy0 * src_w * 4;
        let row1 = sy1 * src_w * 4;
        let dst_row = (dy as usize * dst_w as usize) * 4;

        for dx in dst_x_min..dst_x_max {
            let local_x = (dx - dx0) as f32;

            // Optional Rounded Corner Masking
            let mut corner_alpha = 1.0f32;
            let mut is_border = false;

            if has_corners {
                let dist_x = if local_x < r_px {
                    r_px - local_x
                } else if local_x > dw as f32 - r_px {
                    local_x - (dw as f32 - r_px)
                } else {
                    0.0
                };
                let dist_y = if local_y < r_px {
                    r_px - local_y
                } else if local_y > dh as f32 - r_px {
                    local_y - (dh as f32 - r_px)
                } else {
                    0.0
                };

                if dist_x > 0.0 && dist_y > 0.0 {
                    let d = (dist_x * dist_x + dist_y * dist_y).sqrt();
                    if d > r_px {
                        continue; // Outside rounded corner
                    } else if d > r_px - 1.0 {
                        corner_alpha = (r_px - d).clamp(0.0, 1.0);
                    }
                    if has_border && d >= r_px - b_thick_px {
                        is_border = true;
                    }
                } else if has_border {
                    if local_x < b_thick_px
                        || local_x >= dw as f32 - b_thick_px
                        || local_y < b_thick_px
                        || local_y >= dh as f32 - b_thick_px
                    {
                        is_border = true;
                    }
                }
            } else if has_border {
                if local_x < b_thick_px
                    || local_x >= dw as f32 - b_thick_px
                    || local_y < b_thick_px
                    || local_y >= dh as f32 - b_thick_px
                {
                    is_border = true;
                }
            }

            let d_idx = dst_row + dx as usize * 4;

            if is_border {
                dst[d_idx] = b_col[0];
                dst[d_idx + 1] = b_col[1];
                dst[d_idx + 2] = b_col[2];
                dst[d_idx + 3] = 255;
                continue;
            }

            let u_frac = local_x / dw as f32;
            let sx_f = sc_x0 as f32 + u_frac * sc_w as f32;
            let sx0 = (sx_f.floor() as usize).clamp(sc_x0, src_x_max);
            let sx1 = (sx0 + 1).clamp(sc_x0, src_x_max);
            let fx = sx_f - sx0 as f32;

            let idx00 = row0 + sx0 * 4;
            let idx10 = row0 + sx1 * 4;
            let idx01 = row1 + sx0 * 4;
            let idx11 = row1 + sx1 * 4;

            if idx11 + 3 >= src_rgba.len() {
                continue;
            }

            let w00 = (1.0 - fx) * (1.0 - fy);
            let w10 = fx * (1.0 - fy);
            let w01 = (1.0 - fx) * fy;
            let w11 = fx * fy;

            let mut r = (src_rgba[idx00] as f32 * w00
                + src_rgba[idx10] as f32 * w10
                + src_rgba[idx01] as f32 * w01
                + src_rgba[idx11] as f32 * w11)
                .round() as u8;
            let mut g = (src_rgba[idx00 + 1] as f32 * w00
                + src_rgba[idx10 + 1] as f32 * w10
                + src_rgba[idx01 + 1] as f32 * w01
                + src_rgba[idx11 + 1] as f32 * w11)
                .round() as u8;
            let mut b = (src_rgba[idx00 + 2] as f32 * w00
                + src_rgba[idx10 + 2] as f32 * w10
                + src_rgba[idx01 + 2] as f32 * w01
                + src_rgba[idx11 + 2] as f32 * w11)
                .round() as u8;
            let mut a = (src_rgba[idx00 + 3] as f32 * w00
                + src_rgba[idx10 + 3] as f32 * w10
                + src_rgba[idx01 + 3] as f32 * w01
                + src_rgba[idx11 + 3] as f32 * w11)
                .round() as u8;

            if filter_active {
                let mut px_arr = [r, g, b, a];
                crate::capture::apply_key_filter_to_rgba(&mut px_arr, filter);
                r = px_arr[0];
                g = px_arr[1];
                b = px_arr[2];
                a = px_arr[3];
            }

            if corner_alpha < 0.999 {
                a = ((a as f32) * corner_alpha).round() as u8;
            }

            if a == 0 {
                continue;
            } else if a == 255 {
                dst[d_idx] = r;
                dst[d_idx + 1] = g;
                dst[d_idx + 2] = b;
                dst[d_idx + 3] = 255;
            } else {
                let alpha_val = a as u32;
                let inv_alpha = 255 - alpha_val;
                dst[d_idx] = ((r as u32 * alpha_val + dst[d_idx] as u32 * inv_alpha) / 255) as u8;
                dst[d_idx + 1] =
                    ((g as u32 * alpha_val + dst[d_idx + 1] as u32 * inv_alpha) / 255) as u8;
                dst[d_idx + 2] =
                    ((b as u32 * alpha_val + dst[d_idx + 2] as u32 * inv_alpha) / 255) as u8;
                dst[d_idx + 3] = 255;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compositor_empty_scene_returns_none() {
        let scene = Scene::new("Empty Scene");
        let engine = LiveCaptureEngine::new();
        let res = compose_scene_to_rgba(&scene, &engine, 1920, 1080, 0.0);
        assert!(res.is_none(), "empty scene should produce no frames");
    }

    #[test]
    fn test_compositor_color_source() {
        let mut scene = Scene::new("Color Scene");
        let mut item = SourceItem::new(
            "Background Color",
            SourceKind::ColorSource {
                color_rgba: [255, 0, 0, 255],
                width: 1920,
                height: 1080,
            },
        );
        item.transform.pos_x = 0.0;
        item.transform.pos_y = 0.0;
        item.transform.scale_x = 1.0;
        item.transform.scale_y = 1.0;
        scene.sources.push(item);

        let engine = LiveCaptureEngine::new();
        let res = compose_scene_to_rgba(&scene, &engine, 100, 100, 0.0);
        assert!(res.is_some());
        let buf = res.unwrap();
        assert_eq!(buf.len(), 100 * 100 * 4);
        // Center pixel should be red
        assert_eq!(&buf[0..4], &[255, 0, 0, 255]);
    }
}
