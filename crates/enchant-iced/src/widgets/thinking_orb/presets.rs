//! The shipped tunings: nine states, three sizes.
//!
//! `count` and `size` are multipliers over the base ("fine") mode profiles;
//! `speed` multiplies the shared clock. These are data, copied from the
//! reference library's `spec/orbs-spec.json`, not logic.

use std::fmt;

use super::modes::{
    ModeFrame, frame_braid, frame_globe, frame_morph, frame_orbits, frame_ribbon, frame_rubik,
    frame_wave, frame_web,
};
use super::options::{OrbKnob as K, OrbOptions};

/// What the orb is doing. Each state is a hand-tuned animation.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OrbState {
    /// Particles on tilted orbits.
    #[default]
    Working,
    /// A scan meridian sweeps a dotted globe.
    Searching,
    /// Bands scramble in quarter turns, then click back.
    Solving,
    /// A waveform rolls through latitude rings.
    Listening,
    /// A constellation wires itself, packets running the edges.
    Connecting,
    /// Three strands plait around the sphere.
    Weaving,
    /// An undulating multi-band sash.
    Composing,
    /// A face-on ring slowly morphing.
    Breathing,
    /// A dotted outline morphs circle, triangle, square.
    Shaping,
}

impl OrbState {
    /// All nine states, in the reference library's order.
    pub const ALL: [OrbState; 9] = [
        OrbState::Working,
        OrbState::Searching,
        OrbState::Solving,
        OrbState::Listening,
        OrbState::Connecting,
        OrbState::Weaving,
        OrbState::Composing,
        OrbState::Breathing,
        OrbState::Shaping,
    ];

    /// The state's name as used by the reference library (`"working"`, ...).
    pub const fn name(self) -> &'static str {
        match self {
            OrbState::Working => "working",
            OrbState::Searching => "searching",
            OrbState::Solving => "solving",
            OrbState::Listening => "listening",
            OrbState::Connecting => "connecting",
            OrbState::Weaving => "weaving",
            OrbState::Composing => "composing",
            OrbState::Breathing => "breathing",
            OrbState::Shaping => "shaping",
        }
    }

    /// Looks a state up by its reference-library name.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|state| state.name() == name)
    }

    /// The default accessibility label: a short spoken status.
    pub const fn label(self) -> &'static str {
        match self {
            OrbState::Working => "Working…",
            OrbState::Searching => "Searching…",
            OrbState::Solving => "Solving…",
            OrbState::Listening => "Listening…",
            OrbState::Connecting => "Connecting…",
            OrbState::Weaving => "Weaving…",
            OrbState::Composing => "Composing…",
            OrbState::Breathing => "Thinking…",
            OrbState::Shaping => "Shaping…",
        }
    }

    pub(crate) const fn mode(self) -> Mode {
        match self {
            OrbState::Working => Mode::Orbits,
            OrbState::Searching => Mode::Globe,
            OrbState::Solving => Mode::Rubik,
            OrbState::Listening => Mode::Wave,
            OrbState::Connecting => Mode::Web,
            OrbState::Weaving => Mode::Braid,
            OrbState::Composing => Mode::Ribbon,
            OrbState::Breathing => Mode::Ring,
            OrbState::Shaping => Mode::Morph,
        }
    }
}

impl fmt::Display for OrbState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Rendered size.
///
/// `Large` (64) and `Inline` (20) are hand-tuned designs, not a scale factor:
/// each carries its own dot count, dot size and speed. `Compact` (32) sits
/// between them and is interpolated from the two in log space; it reads
/// correctly but has not had a tuning pass of its own.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OrbSize {
    /// 64 logical px: chat-avatar scale.
    #[default]
    Large,
    /// 32 logical px: compact avatar scale.
    Compact,
    /// 20 logical px: inline-text scale.
    Inline,
}

impl OrbSize {
    /// All sizes, largest first.
    pub const ALL: [OrbSize; 3] = [OrbSize::Large, OrbSize::Compact, OrbSize::Inline];

    /// Side length in logical pixels.
    pub const fn px(self) -> u16 {
        match self {
            OrbSize::Large => 64,
            OrbSize::Compact => 32,
            OrbSize::Inline => 20,
        }
    }
}

impl fmt::Display for OrbSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}px", self.px())
    }
}

/// Engine mode behind a state. `Ring` shares the ribbon geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Orbits,
    Globe,
    Rubik,
    Wave,
    Web,
    Braid,
    Ribbon,
    Ring,
    Morph,
}

impl Mode {
    pub(crate) fn frame_fn(self) -> ModeFrame {
        match self {
            Mode::Orbits => frame_orbits,
            Mode::Globe => frame_globe,
            Mode::Rubik => frame_rubik,
            Mode::Wave => frame_wave,
            Mode::Web => frame_web,
            Mode::Braid => frame_braid,
            Mode::Ribbon | Mode::Ring => frame_ribbon,
            Mode::Morph => frame_morph,
        }
    }

    /// The base ("fine") profile of this mode, before preset multipliers.
    fn base_profile(self) -> OrbOptions {
        let pairs: &[(K, f64)] = match self {
            Mode::Globe => &[
                (K::LatRings, 17.0),
                (K::LonDensity, 44.0),
                (K::RBase, 0.6),
                (K::RDepth, 1.7),
                (K::RBoost, 1.0),
                (K::InkFar, 0.62),
                (K::InkSpan, 0.54),
                (K::RsPow, 0.6),
                (K::RMin, 0.3),
            ],
            Mode::Orbits => &[
                (K::OrbitN, 12.0),
                (K::GhostN, 40.0),
                (K::GhostR, 0.9),
                (K::GhostA, 0.5),
                (K::Particles, 3.0),
                (K::PartR, 1.2),
                (K::PartRDepth, 1.6),
                (K::RsPow, 0.6),
                (K::RMin, 0.3),
            ],
            Mode::Rubik => &[
                (K::LatRings, 15.0),
                (K::LonDensity, 40.0),
                (K::MoveCount, 14.0),
                (K::RBase, 0.6),
                (K::RDepth, 1.7),
                (K::RActive, 0.3),
                (K::InkFar, 0.62),
                (K::InkSpan, 0.54),
                (K::RsPow, 0.6),
                (K::RMin, 0.3),
            ],
            Mode::Wave => &[
                (K::Rings, 15.0),
                (K::LonDensity, 40.0),
                (K::RBase, 0.6),
                (K::RDepth, 1.7),
                (K::RsPow, 0.6),
                (K::RMin, 0.3),
            ],
            Mode::Web => &[
                (K::NodeN, 30.0),
                (K::Thr, 0.72),
                (K::Signals, 5.0),
                (K::NodeR, 1.4),
                (K::NodeRDepth, 1.8),
                (K::LineW, 0.8),
                (K::RsPow, 0.6),
                (K::RMin, 0.3),
            ],
            Mode::Braid => &[
                (K::StrandN, 52.0),
                (K::Turns, 3.0),
                (K::GhostN, 150.0),
                (K::RBase, 1.2),
                (K::RDepth, 1.8),
                (K::RsPow, 0.6),
                (K::RMin, 0.3),
            ],
            Mode::Ribbon => &[
                (K::Lanes, 5.0),
                (K::Segs, 88.0),
                (K::GhostN, 150.0),
                (K::RBase, 1.1),
                (K::RDepth, 1.7),
                (K::RsPow, 0.6),
                (K::RMin, 0.3),
            ],
            // ring shares ribbon's painter; faceOn cancels the camera tilt and
            // moves the undulation onto the radius, and there is no ghost sphere
            Mode::Ring => &[
                (K::Lanes, 5.0),
                (K::Segs, 88.0),
                (K::GhostN, 0.0),
                (K::FaceOn, 1.0),
                (K::RBase, 1.1),
                (K::RDepth, 1.7),
                (K::RsPow, 0.6),
                (K::RMin, 0.3),
            ],
            Mode::Morph => &[(K::RDot, 0.021), (K::IconD, 1.0), (K::RMin, 0.25)],
        };
        pairs.iter().copied().collect()
    }
}

#[derive(Clone, Copy)]
struct Preset {
    speed: f64,
    count: f64,
    size: f64,
    /// Extra knobs merged verbatim after scaling.
    extra: &'static [(K, f64)],
}

const fn preset(speed: f64, count: f64, size: f64) -> Preset {
    Preset {
        speed,
        count,
        size,
        extra: &[],
    }
}

const fn preset_with(speed: f64, count: f64, size: f64, extra: &'static [(K, f64)]) -> Preset {
    Preset {
        speed,
        count,
        size,
        extra,
    }
}

fn preset_for(mode: Mode, size: OrbSize) -> Preset {
    match (mode, size) {
        (Mode::Orbits, OrbSize::Large) => preset(1.885, 1.0, 1.0),
        (Mode::Orbits, OrbSize::Compact) => preset(2.9072, 0.4251, 1.6849),
        (Mode::Orbits, OrbSize::Inline) => preset(3.9, 0.238, 2.4),

        (Mode::Globe, OrbSize::Large) => {
            preset_with(2.015, 0.42, 1.15, &[(K::ScanMul, 4.08), (K::DimBase, 0.45)])
        }
        (Mode::Globe, OrbSize::Compact) => preset_with(
            2.3803,
            0.1839,
            1.4769,
            &[(K::ScanMul, 4.2301), (K::DimBase, 0.45)],
        ),
        (Mode::Globe, OrbSize::Inline) => preset_with(
            2.665,
            0.105,
            1.75,
            &[(K::ScanMul, 4.335), (K::DimBase, 0.45)],
        ),

        (Mode::Rubik, OrbSize::Large) => preset(1.82, 0.35, 1.05),
        (Mode::Rubik, OrbSize::Compact) => preset(1.8964, 0.1537, 1.4951),
        (Mode::Rubik, OrbSize::Inline) => preset(1.95, 0.088, 1.9),

        (Mode::Wave, OrbSize::Large) => preset(4.388, 0.341, 1.0),
        (Mode::Wave, OrbSize::Compact) => preset(4.1512, 0.169, 1.3232),
        (Mode::Wave, OrbSize::Inline) => preset(3.998, 0.105, 1.6),

        (Mode::Web, OrbSize::Large) => preset(3.315, 1.35, 0.95),
        (Mode::Web, OrbSize::Compact) => preset(5.0104, 0.4942, 1.2571),
        (Mode::Web, OrbSize::Inline) => preset(6.63, 0.25, 1.52),

        (Mode::Braid, OrbSize::Large) => preset(1.625, 0.5, 1.0),
        (Mode::Braid, OrbSize::Compact) => preset(2.2234, 0.2056, 1.2011),
        (Mode::Braid, OrbSize::Inline) => preset(2.75, 0.1125, 1.36),

        (Mode::Ribbon, OrbSize::Large) => preset_with(
            2.34,
            0.25,
            0.85,
            &[(K::Spin, 0.0), (K::BandMul, 3.9), (K::WobMul, 1.0)],
        ),
        (Mode::Ribbon, OrbSize::Compact) => preset_with(
            2.7776,
            0.0969,
            0.9766,
            &[(K::Spin, 0.0), (K::BandMul, 4.49), (K::WobMul, 1.0)],
        ),
        (Mode::Ribbon, OrbSize::Inline) => preset_with(
            3.12,
            0.051,
            1.073,
            &[(K::Spin, 0.0), (K::BandMul, 4.94), (K::WobMul, 1.0)],
        ),

        (Mode::Ring, OrbSize::Large) => preset_with(
            3.24,
            0.25,
            0.956,
            &[(K::Spin, 0.0), (K::BandMul, 3.627), (K::WobMul, 0.368)],
        ),
        (Mode::Ring, OrbSize::Compact) => preset_with(
            3.5517,
            0.0678,
            1.31,
            &[(K::Spin, 0.0), (K::BandMul, 3.8265), (K::WobMul, 0.4751)],
        ),
        (Mode::Ring, OrbSize::Inline) => preset_with(
            3.78,
            0.028,
            1.622,
            &[(K::Spin, 0.0), (K::BandMul, 3.968), (K::WobMul, 0.565)],
        ),

        (Mode::Morph, OrbSize::Large) => preset_with(2.405, 0.702, 0.395, &[(K::Spread, 1.45)]),
        (Mode::Morph, OrbSize::Compact) => {
            preset_with(2.2057, 0.5937, 0.6916, &[(K::Spread, 1.45)])
        }
        (Mode::Morph, OrbSize::Inline) => preset_with(2.08, 0.53, 1.011, &[(K::Spread, 1.45)]),
    }
}

// 2-D lattices (rings x dots-per-ring) come in pairs: each side takes
// sqrt(scale) so the TOTAL dot count scales by `scale`; flat lists scale
// linearly. `iconD` sets the morph outline's sampling density.
const COUNT_PAIRS: [(K, K); 3] = [
    (K::LatRings, K::LonDensity),
    (K::Rings, K::LonDensity),
    (K::Lanes, K::Segs),
];
const COUNT_KEYS: [K; 5] = [K::OrbitN, K::GhostN, K::NodeN, K::StrandN, K::Signals];

// Every knob that sets a dot's rendered radius: scaling all of them keeps a
// dot's near/far falloff intact while shrinking or growing the mark.
const RADIUS_KEYS: [K; 9] = [
    K::RBase,
    K::RDepth,
    K::RActive,
    K::RDot,
    K::GhostR,
    K::PartR,
    K::PartRDepth,
    K::NodeR,
    K::NodeRDepth,
];

/// Rescales every count knob by `scale` with the paired scaler the size
/// presets use, so density changes keep the mode's balance.
pub(crate) fn scale_counts(opts: &OrbOptions, scale: f64) -> OrbOptions {
    let mut out = opts.clone();
    let mut done: Vec<K> = Vec::new();
    let rt = scale.sqrt();
    for (a, b) in COUNT_PAIRS {
        if let (Some(va), Some(vb)) = (out.get(a), out.get(b))
            && !done.contains(&a)
            && !done.contains(&b)
        {
            out.set(a, (va * rt).round().max(2.0));
            out.set(b, (vb * rt).round().max(2.0));
            done.push(a);
            done.push(b);
        }
    }
    for k in COUNT_KEYS {
        // 0 means the mode opted out of that layer entirely (ring has no ghost
        // sphere): scaling must not resurrect it as a single stray dot
        if let Some(v) = out.get(k)
            && v != 0.0
            && !done.contains(&k)
        {
            out.set(k, (v * scale).round().max(1.0));
        }
    }
    if let Some(v) = out.get(K::IconD) {
        out.set(K::IconD, (v * scale).max(0.02));
    }
    out
}

/// Rescales every radius knob by `scale`.
pub(crate) fn scale_radii(opts: &OrbOptions, scale: f64) -> OrbOptions {
    let mut out = opts.clone();
    for k in RADIUS_KEYS {
        if let Some(v) = out.get(k) {
            out.set(k, v * scale);
        }
    }
    // remember the multiplier itself: spacing-derived radii use it, since
    // they aren't based on any single radius key
    out.set(K::RSizeMul, out.or(K::RSizeMul, 1.0) * scale);
    out
}

/// A `(state, size)` pair resolved to its mode, clock speed and options.
#[derive(Clone, Debug)]
pub(crate) struct Resolved {
    pub(crate) mode: Mode,
    pub(crate) speed: f64,
    pub(crate) opts: OrbOptions,
}

pub(crate) fn resolve_preset(state: OrbState, size: OrbSize) -> Resolved {
    let mode = state.mode();
    let preset = preset_for(mode, size);
    let mut opts = mode.base_profile();
    if preset.count != 1.0 {
        opts = scale_counts(&opts, preset.count);
    }
    if preset.size != 1.0 {
        opts = scale_radii(&opts, preset.size);
    }
    opts.extend(preset.extra.iter().copied());
    Resolved {
        mode,
        speed: preset.speed,
        opts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for state in OrbState::ALL {
            assert_eq!(OrbState::from_name(state.name()), Some(state));
            assert_eq!(state.to_string(), state.name());
            assert!(!state.label().is_empty());
        }
    }

    #[test]
    fn sizes_have_the_tuned_pixels() {
        assert_eq!(OrbSize::Large.px(), 64);
        assert_eq!(OrbSize::Compact.px(), 32);
        assert_eq!(OrbSize::Inline.px(), 20);
        assert_eq!(OrbSize::Inline.to_string(), "20px");
    }

    #[test]
    fn ring_keeps_its_ghost_sphere_off_after_scaling() {
        let resolved = resolve_preset(OrbState::Breathing, OrbSize::Large);
        assert_eq!(resolved.opts.get(K::GhostN), Some(0.0));
    }

    #[test]
    fn working_inline_matches_the_reference_resolution() {
        let r = resolve_preset(OrbState::Working, OrbSize::Inline);
        assert_eq!(r.opts.get(K::OrbitN), Some(3.0));
        assert_eq!(r.opts.get(K::GhostN), Some(10.0));
        assert!((r.opts.or(K::GhostR, 0.0) - 2.16).abs() < 1e-9);
        assert!((r.opts.or(K::RSizeMul, 0.0) - 2.4).abs() < 1e-9);
    }

    #[test]
    fn scale_counts_pairs_share_the_square_root() {
        let base = Mode::Globe.base_profile();
        let scaled = scale_counts(&base, 0.25);
        // 17 * 0.5 = 8.5 -> 9 (round half up), 44 * 0.5 = 22
        assert_eq!(scaled.get(K::LatRings), Some(9.0));
        assert_eq!(scaled.get(K::LonDensity), Some(22.0));
    }

    #[test]
    fn scale_radii_accumulates_the_size_multiplier() {
        let once = scale_radii(&Mode::Morph.base_profile(), 2.0);
        let twice = scale_radii(&once, 3.0);
        assert_eq!(twice.get(K::RSizeMul), Some(6.0));
    }
}
