//! Raw engine knobs behind every thinking-orb state.
//!
//! The web library exposes these as a loose `opts` object. Here every knob is a
//! named [`OrbKnob`] variant and a set of overrides is an [`OrbOptions`] value,
//! so a typo is a compile error instead of a silently ignored key.

use std::collections::BTreeMap;
use std::fmt;

macro_rules! knobs {
    ($($(#[$doc:meta])* $variant:ident => $name:literal),+ $(,)?) => {
        /// One engine knob.
        ///
        /// A knob only does something in the states that read it; setting a key
        /// the current state does not read is harmless. The doc comment on each
        /// variant lists the states that read it.
        #[non_exhaustive]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum OrbKnob {
            $($(#[$doc])* $variant,)+
        }

        impl OrbKnob {
            /// Every knob, in declaration order.
            pub const ALL: &'static [OrbKnob] = &[$(OrbKnob::$variant,)+];

            /// The engine's own (camelCase) name for this knob, as used by the
            /// reference library's `spec/orbs-spec.json`.
            pub const fn name(self) -> &'static str {
                match self {
                    $(OrbKnob::$variant => $name,)+
                }
            }

            /// Looks a knob up by its engine name.
            pub fn from_name(name: &str) -> Option<Self> {
                match name {
                    $($name => Some(OrbKnob::$variant),)+
                    _ => None,
                }
            }
        }
    };
}

knobs! {
    /// Latitude rings of the lattice (searching, solving).
    LatRings => "latRings",
    /// Dots per equatorial ring (searching, solving, listening).
    LonDensity => "lonDensity",
    /// Latitude rings of the waveform sphere (listening).
    Rings => "rings",
    /// Parallel strands of the sash (composing, breathing).
    Lanes => "lanes",
    /// Dots per strand of the sash (composing, breathing).
    Segs => "segs",
    /// Number of tilted orbits (working).
    OrbitN => "orbitN",
    /// Dots of the faint guide sphere or orbit path (working, weaving, composing).
    GhostN => "ghostN",
    /// Nodes of the constellation (connecting).
    NodeN => "nodeN",
    /// Dots per strand of the plait (weaving).
    StrandN => "strandN",
    /// Packets running the constellation edges (connecting).
    Signals => "signals",
    /// Base dot radius (searching, solving, listening, weaving, composing, breathing).
    RBase => "rBase",
    /// Extra radius for near dots (searching, solving, listening, weaving, composing, breathing).
    RDepth => "rDepth",
    /// Extra radius of the band being turned (solving).
    RActive => "rActive",
    /// Dot radius as a fraction of the orb size (shaping).
    RDot => "rDot",
    /// Radius of the guide dots (working).
    GhostR => "ghostR",
    /// Radius of the travelling particles (working).
    PartR => "partR",
    /// Extra particle radius for near particles (working).
    PartRDepth => "partRDepth",
    /// Node radius (connecting).
    NodeR => "nodeR",
    /// Extra node radius for near nodes (connecting).
    NodeRDepth => "nodeRDepth",
    /// Size boost under the scan meridian (searching).
    RBoost => "rBoost",
    /// Ink value of the farthest dots (searching, solving).
    InkFar => "inkFar",
    /// Ink range between far and near dots (searching, solving).
    InkSpan => "inkSpan",
    /// Exponent of the size-to-radius scaling; lower shrinks dots less.
    RsPow => "rsPow",
    /// Radius floor applied to every dot.
    RMin => "rMin",
    /// Opacity of the guide path (working).
    GhostA => "ghostA",
    /// Particles per orbit (working).
    Particles => "particles",
    /// Quarter-turn moves in one scramble (solving).
    MoveCount => "moveCount",
    /// Rate multiplier of the scan meridian (searching).
    ScanMul => "scanMul",
    /// Opacity of un-scanned dots (searching).
    DimBase => "dimBase",
    /// Distance under which two nodes grow an edge (connecting).
    Thr => "thr",
    /// Edge stroke width (connecting).
    LineW => "lineW",
    /// Helix turns of each strand (weaving).
    Turns => "turns",
    /// Tumble speed of the sash; 0 freezes its orientation (composing, breathing).
    Spin => "spin",
    /// Strand-count multiplier of the sash (composing, breathing).
    BandMul => "bandMul",
    /// Undulation depth of the sash; 0 is a clean band (composing, breathing).
    WobMul => "wobMul",
    /// Non-zero makes the sash a face-on ring (breathing).
    FaceOn => "faceOn",
    /// Outline scale (connecting, shaping).
    Spread => "spread",
    /// 0, 1 or 2 holds the circle, triangle or square instead of cycling (shaping).
    Shape => "shape",
    /// Outline sampling density (shaping).
    IconD => "iconD",
    /// Accumulated radius multiplier used by spacing-derived radii (shaping).
    RSizeMul => "rSizeMul",
}

const KNOB_COUNT: usize = OrbKnob::ALL.len();

/// A sparse set of engine-knob values.
///
/// Unset knobs fall back to each mode's own default, exactly like an
/// `undefined` key in the reference library.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::{OrbKnob, OrbOptions};
///
/// let options = OrbOptions::new()
///     .with(OrbKnob::Thr, 0.6)
///     .with(OrbKnob::Signals, 9.0);
/// assert_eq!(options.get(OrbKnob::Thr), Some(0.6));
/// assert_eq!(options.get(OrbKnob::Turns), None);
/// ```
#[derive(Clone, PartialEq)]
pub struct OrbOptions {
    values: [Option<f64>; KNOB_COUNT],
}

impl Default for OrbOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for OrbOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let knobs: BTreeMap<&str, f64> = self
            .iter()
            .map(|(knob, value)| (knob.name(), value))
            .collect();
        f.debug_struct("OrbOptions").field("knobs", &knobs).finish()
    }
}

impl OrbOptions {
    /// An empty set: every knob at its mode default.
    pub fn new() -> Self {
        Self {
            values: [None; KNOB_COUNT],
        }
    }

    /// Sets `knob` to `value`. Non-finite values are ignored.
    pub fn with(mut self, knob: OrbKnob, value: f64) -> Self {
        self.set(knob, value);
        self
    }

    /// In-place form of [`OrbOptions::with`].
    pub fn set(&mut self, knob: OrbKnob, value: f64) {
        if value.is_finite() {
            self.values[knob as usize] = Some(value);
        }
    }

    /// Clears `knob` back to its mode default.
    pub fn unset(&mut self, knob: OrbKnob) {
        self.values[knob as usize] = None;
    }

    /// The value of `knob`, if set.
    pub fn get(&self, knob: OrbKnob) -> Option<f64> {
        self.values[knob as usize]
    }

    /// `true` when no knob is set.
    pub fn is_empty(&self) -> bool {
        self.values.iter().all(Option::is_none)
    }

    /// Every set knob and its value, in [`OrbKnob::ALL`] order.
    pub fn iter(&self) -> impl Iterator<Item = (OrbKnob, f64)> + '_ {
        OrbKnob::ALL
            .iter()
            .filter_map(|&knob| self.get(knob).map(|value| (knob, value)))
    }

    /// The value of `knob`, or `default` when unset (`o.knob ?? default`).
    pub(crate) fn or(&self, knob: OrbKnob, default: f64) -> f64 {
        self.get(knob).unwrap_or(default)
    }

    /// A count-like knob, clamped so a stray override cannot exhaust memory.
    pub(crate) fn count(&self, knob: OrbKnob, default: f64) -> f64 {
        self.or(knob, default).clamp(0.0, MAX_COUNT)
    }

    /// `other` laid over `self`, knob by knob.
    pub(crate) fn merged(&self, other: &OrbOptions) -> OrbOptions {
        let mut out = self.clone();
        for (knob, value) in other.iter() {
            out.set(knob, value);
        }
        out
    }
}

/// Upper bound of any per-axis count, so `OrbOptions` overrides stay safe.
const MAX_COUNT: f64 = 1024.0;

impl FromIterator<(OrbKnob, f64)> for OrbOptions {
    fn from_iter<I: IntoIterator<Item = (OrbKnob, f64)>>(iter: I) -> Self {
        let mut options = Self::new();
        options.extend(iter);
        options
    }
}

impl Extend<(OrbKnob, f64)> for OrbOptions {
    fn extend<I: IntoIterator<Item = (OrbKnob, f64)>>(&mut self, iter: I) {
        for (knob, value) in iter {
            self.set(knob, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for &knob in OrbKnob::ALL {
            assert_eq!(OrbKnob::from_name(knob.name()), Some(knob));
        }
        assert_eq!(OrbKnob::from_name("nope"), None);
    }

    #[test]
    fn set_ignores_non_finite_and_unset_clears() {
        let mut options = OrbOptions::new();
        options.set(OrbKnob::Thr, f64::NAN);
        assert!(options.is_empty());
        options.set(OrbKnob::Thr, 0.5);
        assert_eq!(options.get(OrbKnob::Thr), Some(0.5));
        options.unset(OrbKnob::Thr);
        assert!(options.is_empty());
    }

    #[test]
    fn count_is_clamped() {
        let options = OrbOptions::new().with(OrbKnob::NodeN, 1e9);
        assert_eq!(options.count(OrbKnob::NodeN, 3.0), MAX_COUNT);
        assert_eq!(options.count(OrbKnob::Signals, 5.0), 5.0);
    }

    #[test]
    fn merged_prefers_the_overlay() {
        let base = OrbOptions::new()
            .with(OrbKnob::Thr, 0.7)
            .with(OrbKnob::LineW, 1.0);
        let top = OrbOptions::new().with(OrbKnob::Thr, 0.2);
        let merged = base.merged(&top);
        assert_eq!(merged.get(OrbKnob::Thr), Some(0.2));
        assert_eq!(merged.get(OrbKnob::LineW), Some(1.0));
    }

    #[test]
    fn debug_is_never_empty() {
        assert!(!format!("{:?}", OrbOptions::new()).is_empty());
        assert!(format!("{:?}", OrbOptions::new().with(OrbKnob::Thr, 0.5)).contains("thr"));
    }

    #[test]
    fn collects_from_iterator() {
        let options: OrbOptions = [(OrbKnob::Turns, 4.0), (OrbKnob::Spin, 0.0)]
            .into_iter()
            .collect();
        assert_eq!(options.iter().count(), 2);
    }
}
