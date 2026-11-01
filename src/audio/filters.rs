//! Per-source audio filter chain (OBS-style).
//!
//! Unlike video keying — where exactly one key filter is active at a time — audio needs several
//! filters applied **in series**: a user typically wants Gain *and* Noise Gate *and* Noise
//! Suppression at once. The chain is therefore an ordered `Vec<AudioFilter>` that can be added
//! to, removed from, reordered, and individually bypassed.
//!
//! Every filter is **explicit**. Nothing in this module runs unless the user configured it,
//! which is what keeps a microphone from ending up double-gated by an invisible safety net.
//!
//! The chain always runs at the encoder's 48 kHz mix rate, i.e. *after* the bus resampler.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Sample rate the chain always runs at (the encoder's internal mix rate).
///
/// Declared locally rather than importing the encoder's constant so the audio module never has
/// to depend on the encoder module.
pub const FILTER_RATE: f32 = 48_000.0;

/// One-pole high-frequency rolloff corner used by [`AudioFilterKind::NoiseSuppression`].
///
/// Low enough to genuinely tame preamp hiss, high enough to stay inaudible on speech and
/// music.
const NS_ROLLOFF_HZ: f32 = 12_000.0;

#[inline]
fn db_to_lin(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

#[inline]
fn lin_to_db(lin: f32) -> f32 {
    20.0 * lin.max(1.0e-9).log10()
}

/// Per-sample smoothing coefficient for a time constant expressed in milliseconds.
#[inline]
fn smooth_coeff(ms: f32) -> f32 {
    let ms = ms.max(0.05);
    1.0 - (-1000.0 / (ms * FILTER_RATE)).exp()
}

/// One selectable audio filter kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AudioFilterKind {
    /// Static gain trim, `-30 dB .. +30 dB`.
    Gain,
    /// Hard gate: fully silences the signal while it stays below `threshold`.
    NoiseGate,
    /// Soft broadband expander that gently pushes hiss down instead of chopping it off.
    NoiseSuppression,
}

impl AudioFilterKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Gain => "Gain (Volume Boost / Cut)",
            Self::NoiseGate => "Noise Gate (Silence When Quiet)",
            Self::NoiseSuppression => "Noise Suppression (Reduce Hiss)",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Self::Gain => "Gain",
            Self::NoiseGate => "Noise Gate",
            Self::NoiseSuppression => "Noise Suppression",
        }
    }

    pub fn hover_explanation(&self) -> &'static str {
        match self {
            Self::Gain => {
                "Gain: Static volume trim for this source only.\n\u{2022} Best for: microphones that record too quietly (boost) or clip (cut).\n\u{2022} Range: -30 dB to +30 dB, applied before every other filter in the chain."
            }
            Self::NoiseGate => {
                "Noise Gate: Fully silences the source while it stays below the threshold.\n\u{2022} Best for: muting room tone, keyboard clatter, and fan noise between sentences.\n\u{2022} Tuning: raise the threshold until background noise disappears, then back off slightly so quiet speech still opens the gate.\n\u{2022} Warning: set it too high and the starts of words get clipped."
            }
            Self::NoiseSuppression => {
                "Noise Suppression: Downward expander that pushes constant hiss down smoothly.\n\u{2022} Best for: preamp hiss, air-conditioning hum, and steady broadband noise.\n\u{2022} Difference from Noise Gate: leaves the signal audible instead of cutting it off, so it never sounds choppy.\n\u{2022} Includes a gentle 12 kHz rolloff to tame high-frequency fizz."
            }
        }
    }

    pub fn all() -> &'static [AudioFilterKind] {
        &[Self::Gain, Self::NoiseGate, Self::NoiseSuppression]
    }
}

/// One configured filter inside a source's audio chain.
///
/// A single flat struct holds the parameters of every kind. That keeps the type trivially
/// serializable and stable across project files, at the cost of a few unused fields per entry —
/// acceptable for a chain that holds a handful of filters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioFilter {
    /// Stable identity used by the UI to reorder / remove an entry.
    pub id: Uuid,
    pub kind: AudioFilterKind,
    /// Per-filter bypass, independent of the source's own visibility.
    pub enabled: bool,

    // --- Gain ---
    /// `-30.0 ..= 30.0` dB.
    pub gain_db: f32,

    // --- Noise Gate ---
    /// `-80.0 ..= 0.0` dBFS.
    pub gate_threshold_db: f32,
    /// How fast the gate opens, milliseconds.
    pub gate_attack_ms: f32,
    /// How long the gate stays open after the signal drops, milliseconds.
    pub gate_hold_ms: f32,
    /// How fast the gate closes, milliseconds.
    pub gate_release_ms: f32,

    // --- Noise Suppression ---
    /// Maximum attenuation while the signal is well below the threshold, `0.0 ..= 30.0` dB.
    pub ns_reduction_db: f32,
    /// `0.0 ..= 1.0`; maps to a detection threshold of `-60 dB .. -15 dB`.
    pub ns_sensitivity: f32,
}

impl AudioFilter {
    /// A new filter of `kind` with sensible broadcast defaults.
    pub fn new(kind: AudioFilterKind) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            enabled: true,
            gain_db: 0.0,
            gate_threshold_db: -45.0,
            gate_attack_ms: 4.0,
            gate_hold_ms: 150.0,
            gate_release_ms: 280.0,
            ns_reduction_db: 12.0,
            ns_sensitivity: 0.5,
        }
    }

    /// Linear gain factor contributed by a [`AudioFilterKind::Gain`] entry.
    pub fn gain_factor(&self) -> f32 {
        if self.enabled && self.kind == AudioFilterKind::Gain {
            db_to_lin(self.gain_db.clamp(-30.0, 30.0))
        } else {
            1.0
        }
    }
}

/// The "Clean Mic" preset: a gentle expander followed by a gate.
///
/// This reproduces the conditioning the recorder used to apply invisibly to every microphone
/// (de-hum + hiss rolloff + adaptive gating), now as an explicit chain the user can inspect,
/// disable, or tune.
pub fn clean_mic_preset() -> Vec<AudioFilter> {
    let mut ns = AudioFilter::new(AudioFilterKind::NoiseSuppression);
    ns.ns_reduction_db = 12.0;
    ns.ns_sensitivity = 0.5;

    let mut gate = AudioFilter::new(AudioFilterKind::NoiseGate);
    gate.gate_threshold_db = -45.0;

    vec![ns, gate]
}

/// Signature covering every field that affects the rendered audio.
///
/// Deliberately excludes [`AudioFilter::id`] so reordering two identical filters, or an undo that
/// only reshuffles identities, does not look like a parameter change to the encoder.
pub fn filters_signature(filters: &[AudioFilter]) -> String {
    if filters.is_empty() {
        return "none".to_string();
    }
    filters
        .iter()
        .map(|f| {
            let tag = match f.kind {
                AudioFilterKind::Gain => "gain",
                AudioFilterKind::NoiseGate => "gate",
                AudioFilterKind::NoiseSuppression => "ns",
            };
            format!(
                "{}:{}:{:.3}:{:.2}:{:.1}:{:.1}:{:.1}:{:.2}:{:.3}",
                tag,
                u8::from(f.enabled),
                f.gain_db,
                f.gate_threshold_db,
                f.gate_attack_ms,
                f.gate_hold_ms,
                f.gate_release_ms,
                f.ns_reduction_db,
                f.ns_sensitivity,
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// One-pole DC blocker (~77 Hz) removing hum, rumble and any DC offset.
#[derive(Debug, Clone)]
struct DcBlocker {
    x1: (f32, f32),
    y1: (f32, f32),
}

impl DcBlocker {
    fn new() -> Self {
        Self {
            x1: (0.0, 0.0),
            y1: (0.0, 0.0),
        }
    }

    #[inline]
    fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        let yl = l - self.x1.0 + 0.995 * self.y1.0;
        let yr = r - self.x1.1 + 0.995 * self.y1.1;
        self.x1 = (l, r);
        self.y1 = (yl, yr);
        (yl, yr)
    }
}

/// One-pole low-pass, used for the hiss rolloff inside noise suppression.
#[derive(Debug, Clone)]
struct OnePole {
    l: f32,
    r: f32,
    coeff: f32,
}

impl OnePole {
    fn new_low_pass(hz: f32) -> Self {
        let coeff = 1.0 - (-2.0 * std::f32::consts::PI * hz / FILTER_RATE).exp();
        Self {
            l: 0.0,
            r: 0.0,
            coeff: coeff.clamp(0.0, 1.0),
        }
    }

    #[inline]
    fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        self.l += (l - self.l) * self.coeff;
        self.r += (r - self.r) * self.coeff;
        (self.l, self.r)
    }
}

/// Static gain trim with zipper-noise-free ramping.
#[derive(Debug, Clone)]
struct GainNode {
    target: f32,
    current: f32,
    coeff: f32,
}

impl GainNode {
    fn new(gain_db: f32) -> Self {
        let target = db_to_lin(gain_db.clamp(-30.0, 30.0));
        Self {
            target,
            // Start already at the target so the first block is not faded in.
            current: target,
            coeff: smooth_coeff(20.0),
        }
    }

    #[inline]
    fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        self.current += (self.target - self.current) * self.coeff;
        (l * self.current, r * self.current)
    }
}

/// Hysteretic noise gate with attack / hold / release ballistics.
#[derive(Debug, Clone)]
struct GateNode {
    dc: DcBlocker,
    /// Detection envelope (peak follower).
    det: f32,
    det_attack: f32,
    det_release: f32,
    /// Applied gain, ramped to avoid clicks.
    gain: f32,
    attack: f32,
    release: f32,
    open_thresh: f32,
    close_thresh: f32,
    open: bool,
    hold_samples: u32,
    hold_left: u32,
}

impl GateNode {
    fn new(threshold_db: f32, attack_ms: f32, hold_ms: f32, release_ms: f32) -> Self {
        let open_thresh = db_to_lin(threshold_db.clamp(-80.0, 0.0));
        // ~3 dB of hysteresis so a signal hovering at the threshold cannot chatter.
        let close_thresh = (open_thresh * 0.707).max(1.0e-7);
        Self {
            dc: DcBlocker::new(),
            det: 0.0,
            det_attack: smooth_coeff(1.0),
            det_release: smooth_coeff(50.0),
            gain: 0.0,
            attack: smooth_coeff(attack_ms),
            release: smooth_coeff(release_ms),
            open_thresh,
            close_thresh,
            open: false,
            hold_samples: ((hold_ms.max(0.0) / 1000.0) * FILTER_RATE) as u32,
            hold_left: 0,
        }
    }

    #[inline]
    fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        // A gate needs a stable floor to compare against, so de-hum first.
        let (dl, dr) = self.dc.process(l, r);

        let level = dl.abs().max(dr.abs());
        let dc = if level > self.det {
            self.det_attack
        } else {
            self.det_release
        };
        self.det += (level - self.det) * dc;

        if self.open {
            if self.det > self.close_thresh {
                self.hold_left = self.hold_samples;
            } else if self.hold_left > 0 {
                self.hold_left -= 1;
            } else {
                self.open = false;
            }
        } else if self.det > self.open_thresh {
            self.open = true;
            self.hold_left = self.hold_samples;
        }

        let target = if self.open { 1.0 } else { 0.0 };
        let coeff = if target > self.gain {
            self.attack
        } else {
            self.release
        };
        self.gain += (target - self.gain) * coeff;
        if self.gain < 1.0e-4 {
            self.gain = 0.0;
        }

        (dl * self.gain, dr * self.gain)
    }
}

/// Broadband downward expander: attenuates steadily-below-threshold signal by up to
/// `max_reduce_db`, so hiss fades instead of being chopped.
#[derive(Debug, Clone)]
struct NoiseSuppressionNode {
    lp: OnePole,
    det: f32,
    det_attack: f32,
    det_release: f32,
    gain: f32,
    attack: f32,
    release: f32,
    threshold: f32,
    max_reduce_db: f32,
}

impl NoiseSuppressionNode {
    fn new(reduction_db: f32, sensitivity: f32) -> Self {
        // sensitivity 0.0 -> -60 dBFS (only suppresses very quiet floor)
        // sensitivity 1.0 -> -15 dBFS (aggressive)
        let threshold_db = -60.0 + 45.0 * sensitivity.clamp(0.0, 1.0);
        Self {
            lp: OnePole::new_low_pass(NS_ROLLOFF_HZ),
            det: 0.0,
            det_attack: smooth_coeff(1.0),
            det_release: smooth_coeff(80.0),
            gain: 1.0,
            attack: smooth_coeff(5.0),
            release: smooth_coeff(120.0),
            threshold: db_to_lin(threshold_db),
            max_reduce_db: reduction_db.clamp(0.0, 30.0),
        }
    }

    #[inline]
    fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        // Gentle high-frequency rolloff tames broadband hiss without dulling speech.
        let (fl, fr) = self.lp.process(l, r);

        let level = fl.abs().max(fr.abs());
        let dc = if level > self.det {
            self.det_attack
        } else {
            self.det_release
        };
        self.det += (level - self.det) * dc;

        // How far below the threshold we currently are, in dB.
        let below_db = (lin_to_db(self.threshold) - lin_to_db(self.det)).max(0.0);
        // 1 dB below the threshold costs 1 dB extra attenuation (2:1 expander),
        // capped so the signal never disappears entirely.
        let reduce_db = below_db.min(self.max_reduce_db);
        let target = db_to_lin(-reduce_db);

        let coeff = if target > self.gain {
            self.attack
        } else {
            self.release
        };
        self.gain += (target - self.gain) * coeff;
        self.gain = self.gain.clamp(1.0e-6, 1.0);

        (fl * self.gain, fr * self.gain)
    }
}

/// One instantiated filter in a running chain.
#[derive(Debug, Clone)]
enum FilterNode {
    Gain(GainNode),
    Gate(GateNode),
    Ns(NoiseSuppressionNode),
}

/// Live chain instantiated on the encoder thread for one audio bus.
///
/// Rebuilt whenever [`filters_signature`] changes; parameter edits therefore take effect without
/// re-opening the WASAPI stream.
#[derive(Debug, Clone, Default)]
pub struct AudioFilterChainRuntime {
    nodes: Vec<FilterNode>,
}

impl AudioFilterChainRuntime {
    /// Instantiates only the filters that are actually enabled.
    pub fn from_chain(filters: &[AudioFilter]) -> Self {
        let nodes = filters
            .iter()
            .filter(|f| f.enabled)
            .map(|f| match f.kind {
                AudioFilterKind::Gain => FilterNode::Gain(GainNode::new(f.gain_db)),
                AudioFilterKind::NoiseGate => FilterNode::Gate(GateNode::new(
                    f.gate_threshold_db,
                    f.gate_attack_ms,
                    f.gate_hold_ms,
                    f.gate_release_ms,
                )),
                AudioFilterKind::NoiseSuppression => FilterNode::Ns(NoiseSuppressionNode::new(
                    f.ns_reduction_db,
                    f.ns_sensitivity,
                )),
            })
            .collect();
        Self { nodes }
    }

    /// `true` when no filter is enabled, letting the caller skip the call entirely.
    pub fn is_transparent(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Runs one stereo frame through the whole chain, in configured order.
    #[inline]
    pub fn process(&mut self, mut l: f32, mut r: f32) -> (f32, f32) {
        for node in &mut self.nodes {
            let (nl, nr) = match node {
                FilterNode::Gain(n) => n.process(l, r),
                FilterNode::Gate(n) => n.process(l, r),
                FilterNode::Ns(n) => n.process(l, r),
            };
            l = nl;
            r = nr;
        }
        (l, r)
    }
}

