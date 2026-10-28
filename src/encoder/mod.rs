use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncoderBackendKind {
    WindowsMediaFoundationH264,
    FfmpegCliPipe,
}

impl EncoderBackendKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::WindowsMediaFoundationH264 => "Windows Media Foundation H.264 / AAC (.mp4)",
            Self::FfmpegCliPipe => "FFmpeg H.264 / AAC Subprocess Pipe (.mp4)",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncoderConfig {
    pub backend: EncoderBackendKind,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub video_bitrate_kbps: u32,
    pub audio_bitrate_kbps: u32,
}

impl Default for EncoderConfig {
    fn default() -> Self {
        Self {
            backend: EncoderBackendKind::WindowsMediaFoundationH264,
            width: 1920,
            height: 1080,
            fps: 60,
            video_bitrate_kbps: 12_000,
            audio_bitrate_kbps: 192,
        }
    }
}

