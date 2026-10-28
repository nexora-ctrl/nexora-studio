use std::path::Path;
use std::process::Command;

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

