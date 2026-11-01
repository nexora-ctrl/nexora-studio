use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingEntry {
    pub file_name: String,
    pub full_path: PathBuf,
    pub recorded_at: DateTime<Local>,
    pub size_bytes: u64,
    pub duration_label: String,
}

impl RecordingEntry {
    pub fn formatted_size(&self) -> String {
        let mb = self.size_bytes as f64 / (1024.0 * 1024.0);
        if mb >= 1024.0 {
            format!("{:.2} GB", mb / 1024.0)
        } else if mb >= 1.0 {
            format!("{:.1} MB", mb)
        } else {
            format!("{} KB", (self.size_bytes / 1024).max(1))
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MediaLibrary {
    pub entries: Vec<RecordingEntry>,
}

impl MediaLibrary {
    pub fn scan_directory(&mut self, dir: &Path) {
        self.entries.clear();
        let Ok(read_dir) = fs::read_dir(dir) else {
            return;
        };
        for item in read_dir.flatten() {
            let path = item.path();
            if !path.is_file() {
                continue;
            }
            let is_video = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "mp4" | "mkv" | "mov"))
                .unwrap_or(false);
            if !is_video {
                continue;
            }
            if let Ok(meta) = item.metadata() {
                let recorded_at: DateTime<Local> = meta
                    .modified()
                    .map(DateTime::<Local>::from)
                    .unwrap_or_else(|_| Local::now());
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("recording.mp4")
                    .to_string();
                // Reading the real duration costs one file open per entry, so it happens at scan
                // time (recording stopped, folder opened) rather than per UI frame.
                let duration_label =
                    read_mp4_duration_label(&path).unwrap_or_else(|| "--:--".to_string());
                self.entries.push(RecordingEntry {
                    file_name,
                    full_path: path,
                    recorded_at,
                    size_bytes: meta.len(),
                    duration_label,
                });
            }
        }
        self.entries
            .sort_by(|a, b| b.recorded_at.cmp(&a.recorded_at));
    }
}

/// Parses the duration out of an MP4/MOV `mvhd` box and formats it as `MM:SS` (or `H:MM:SS`).
///
/// ISO-BMFF files carry the movie duration as a timescale/duration pair inside the `moov` box.
/// Only the header path is walked and nothing is decoded, so this stays cheap even for large
/// recordings. The `moov` box is commonly written at the end of the file (that is what the
/// recorder does, since streaming it up front requires extra seeks), so the scan is a full walk of
/// the top-level box headers rather than a read of the first few kilobytes.
///
/// Returns `None` for formats or files this parser does not understand, so callers can fall back
/// to a placeholder instead of showing a wrong length.
fn read_mp4_duration_label(path: &Path) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(file);
    let seconds = read_mvhd_duration(&mut reader)?;
    Some(format_duration(seconds))
}

fn format_duration(total_seconds: f64) -> String {
    let secs = total_seconds.max(0.0).round() as u64;
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{:02}:{:02}", m, s)
    }
}

fn read_mvhd_duration<R: std::io::Read + std::io::Seek>(reader: &mut R) -> Option<f64> {
    use std::io::SeekFrom;

    // Walk the top-level boxes looking for `moov`, then for `mvhd` inside it.
    let total = reader.seek(SeekFrom::End(0)).ok()?;
    reader.seek(SeekFrom::Start(0)).ok()?;
    let mut offset = 0u64;
    while offset + 8 <= total {
        let mut header = [0u8; 8];
        reader.read_exact(&mut header).ok()?;
        let mut size = u32::from_be_bytes(header[0..4].try_into().ok()?) as u64;
        let kind = &header[4..8];
        let mut body_start = offset + 8;
        if size == 1 {
            let mut large = [0u8; 8];
            reader.read_exact(&mut large).ok()?;
            size = u64::from_be_bytes(large);
            body_start += 8;
        } else if size == 0 {
            size = total - offset;
        }
        if size < body_start - offset {
            return None;
        }

        if kind == b"moov" {
            return read_mvhd_in_moov(reader, body_start, size - (body_start - offset));
        }

        offset += size;
        reader.seek(SeekFrom::Start(offset)).ok()?;
    }
    None
}

fn read_mvhd_in_moov<R: std::io::Read + std::io::Seek>(
    reader: &mut R,
    moov_start: u64,
    moov_len: u64,
) -> Option<f64> {
    use std::io::SeekFrom;

    let moov_end = moov_start + moov_len;
    let mut offset = moov_start;
    while offset + 8 <= moov_end {
        reader.seek(SeekFrom::Start(offset)).ok()?;
        let mut header = [0u8; 8];
        reader.read_exact(&mut header).ok()?;
        let mut size = u32::from_be_bytes(header[0..4].try_into().ok()?) as u64;
        let kind = &header[4..8];
        let mut body_start = offset + 8;
        if size == 1 {
            let mut large = [0u8; 8];
            reader.read_exact(&mut large).ok()?;
            size = u64::from_be_bytes(large);
            body_start += 8;
        } else if size == 0 {
            size = moov_end - offset;
        }
        if size < 8 {
            break;
        }

        if kind == b"mvhd" {
            return read_mvhd_body(reader, body_start, size - 8);
        }
        offset += size;
    }
    None
}

fn read_mvhd_body<R: std::io::Read + std::io::Seek>(
    reader: &mut R,
    body_start: u64,
    body_len: u64,
) -> Option<f64> {
    use std::io::SeekFrom;

    reader.seek(SeekFrom::Start(body_start)).ok()?;
    let mut version = [0u8; 4];
    reader.read_exact(&mut version).ok()?;
    // Skip flags (3 bytes) + creation/modification time.
    let v1 = version[0] == 1;
    let header_len = if v1 { 4 + 16 + 4 } else { 4 + 8 + 4 };
    if body_len < header_len {
        return None;
    }
    reader.seek(SeekFrom::Start(body_start + 4 + if v1 { 16 } else { 8 })).ok()?;

    let mut timescale_buf = [0u8; 4];
    reader.read_exact(&mut timescale_buf).ok()?;
    let timescale = u32::from_be_bytes(timescale_buf) as f64;
    if timescale <= 0.0 {
        return None;
    }

    let mut duration_buf = [0u8; 8];
    if v1 {
        reader.read_exact(&mut duration_buf).ok()?;
    } else {
        reader.read_exact(&mut duration_buf[4..8]).ok()?;
    }
    let duration = u64::from_be_bytes(duration_buf) as f64;

    // A duration of u32::MAX / u64::MAX is the "unknown yet" sentinel written when the length is
    // not known up front; it must not be reported as a real length.
    if duration <= 0.0 || duration >= 4_294_967_295.0 {
        return None;
    }
    Some(duration / timescale)
}

#[cfg(test)]
mod tests {
    use super::{format_duration, read_mp4_duration_label};
    use std::io::Write;

    /// Builds a minimal but structurally valid ISO-BMFF header containing `moov`/`mvhd` v0.
    fn synthetic_mp4(timescale: u32, duration: u32) -> Vec<u8> {
        let mut mvhd_body = Vec::new();
        mvhd_body.extend_from_slice(&[0, 0, 0, 0]); // version 0 + flags
        mvhd_body.extend_from_slice(&[0u8; 8]); // creation + modification time
        mvhd_body.extend_from_slice(&timescale.to_be_bytes());
        mvhd_body.extend_from_slice(&duration.to_be_bytes());
        mvhd_body.extend_from_slice(&[0u8; 80]); // rest of the box, unused here

        let mut mvhd = Vec::new();
        mvhd.extend_from_slice(&((mvhd_body.len() + 8) as u32).to_be_bytes());
        mvhd.extend_from_slice(b"mvhd");
        mvhd.extend_from_slice(&mvhd_body);

        let mut moov = Vec::new();
        moov.extend_from_slice(&((mvhd.len() + 8) as u32).to_be_bytes());
        moov.extend_from_slice(b"moov");
        moov.extend_from_slice(&mvhd);

        // A leading `ftyp` box, as every real file has.
        let mut out = Vec::new();
        out.extend_from_slice(&16u32.to_be_bytes());
        out.extend_from_slice(b"ftyp");
        out.extend_from_slice(b"isom");
        out.extend_from_slice(&[0u8; 4]);
        out.extend_from_slice(&moov);
        out
    }

    #[test]
    fn duration_label_reports_real_length_instead_of_placeholder() {
        let dir = std::env::temp_dir().join("nexora_media_lib_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("clip.mp4");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(&synthetic_mp4(1000, 65_000)).unwrap(); // 65 s
        drop(f);

        assert_eq!(read_mp4_duration_label(&path).as_deref(), Some("01:05"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn duration_label_scales_to_hours() {
        assert_eq!(format_duration(3_725.0), "1:02:05");
        assert_eq!(format_duration(65.0), "01:05");
        assert_eq!(format_duration(5.0), "00:05");
    }

    /// A file that is not ISO-BMFF (or is truncated) must yield `None` so the UI keeps its
    /// placeholder rather than showing a fabricated length.
    #[test]
    fn duration_label_is_none_for_unparsable_files() {
        let dir = std::env::temp_dir().join("nexora_media_lib_test_bad");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("broken.mp4");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"not a real mp4 file at all").unwrap();
        drop(f);

        assert_eq!(read_mp4_duration_label(&path), None);
        let _ = std::fs::remove_file(&path);
    }
}

