//! Per-Source Audio Bus (OBS-style) specifications.
//!
//! Every audio-capable source in a scene (`Audio Input Capture`, `Audio Output Capture`,
//! `Application Audio Capture`) is described to the recording encoder as one independent
//! `SourceBusSpec`. The encoder opens one dedicated WASAPI stream per bus, applies its own
//! fader / mute / DSP, and sums all buses (plus the two global mixer buses) into a single
//! AAC track — so muting `Mic / Aux` can never silence an `Audio Input Capture`.
//!
//! The type is intentionally free of Windows types and *not* `Serialize`/`Deserialize`: it is a
//! transient runtime descriptor pushed to the encoder thread, never persisted to the project.
//! The filter chain it carries *is* serializable, and lives in `crate::audio::filters`.

use crate::audio::filters::AudioFilter;

/// Which Windows audio endpoint a source bus captures from.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceBusKind {
    /// Full-desktop WASAPI loopback on a **render** endpoint (`Audio Output Capture`).
    RenderLoopback,
    /// Direct **capture** endpoint input (`Audio Input Capture`).
    CaptureInput,
    /// Per-process loopback using `ActivateAudioInterfaceAsync` (`Application Audio Capture`).
    /// `pid == 0` means "unresolved" and falls back to full-desktop loopback.
    ProcessLoopback { pid: u32 },
}

impl SourceBusKind {
    /// Short stable tag used for UI badges and diagnostics.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::RenderLoopback => "OUT",
            Self::CaptureInput => "IN",
            Self::ProcessLoopback { .. } => "APP",
        }
    }

    /// Whether this bus is backed by a real microphone-style input.
    pub fn is_input(&self) -> bool {
        matches!(self, Self::CaptureInput)
    }
}

/// Runtime descriptor of one per-source audio bus handed to the encoder thread.
#[derive(Debug, Clone)]
pub struct SourceBusSpec {
    /// Stable identity of the bus (the source `Uuid` rendered as a simple string).
    pub bus_id: String,
    /// Human readable source name, used only for diagnostics.
    pub label: String,
    /// Endpoint flavour.
    pub kind: SourceBusKind,
    /// Friendly device name as selected in the UI (matched tolerantly by the encoder).
    pub device_name: String,
    /// Fader position, `0.0 ..= 1.5`.
    pub volume: f32,
    /// Mute flag.
    pub muted: bool,
    /// `false` when the source is hidden / not live, which forces the bus to silence.
    pub enabled: bool,
    /// Ordered filter chain configured on the source (already resolved, bypasses removed).
    pub filters: Vec<AudioFilter>,
}

impl SourceBusSpec {
    /// Effective linear gain applied while mixing (0.0 when muted or disabled).
    pub fn effective_gain(&self) -> f32 {
        if self.muted || !self.enabled {
            0.0
        } else {
            self.volume.clamp(0.0, 1.5)
        }
    }

    /// Cheap signature describing everything that requires (re)opening a WASAPI stream.
    /// Fader / mute / filter changes are excluded so they never force a device re-open.
    pub fn routing_signature(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.bus_id,
            self.kind.tag(),
            match self.kind {
                SourceBusKind::ProcessLoopback { pid } => pid,
                _ => 0,
            },
            self.device_name
        )
    }

    /// Runtime signature of the filter chain only.
    fn filters_signature(&self) -> String {
        crate::audio::filters::filters_signature(&self.filters)
    }

    /// Full signature covering *every* field that affects the mix, used by the UI to decide
    /// whether the encoder needs to be told about a change at all. A fader move changes this
    /// signature (so the new gain is delivered) without changing [`Self::routing_signature`]
    /// (so no device is re-opened).
    pub fn spec_signature(&self) -> String {
        format!(
            "{}|{:.4}|{}|{}|{}",
            self.routing_signature(),
            self.volume,
            self.muted,
            self.enabled,
            self.filters_signature()
        )
    }
}

/// Builds the signature of an entire bus list (order-sensitive), used by the UI to suppress
/// redundant `set_source_buses` sends without dropping real changes.
pub fn buses_signature(buses: &[SourceBusSpec]) -> String {
    buses
        .iter()
        .map(|b| b.spec_signature())
        .collect::<Vec<_>>()
        .join(";")
}
