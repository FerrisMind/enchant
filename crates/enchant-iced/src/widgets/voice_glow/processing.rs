//! The glow's large-scale motion and the processing sweep.
//!
//! [`VoiceGlowMotion`] is `VoiceBeamMotion` (`types.ts`); [`VoiceProcessing`]
//! is `createVoiceProcessing` from `sites/home/src/studio/voice-processing.ts`:
//! once the voice is captured and something is working on it, the lobes gather
//! into one compact beam that travels its range left to right and back,
//! eased at each end and looped.

use std::fmt;
use std::sync::Arc;

use super::options::VoiceGlowType;

/// The glow's motion for one frame.
///
/// At rest (the default) the glow is the plain voice glow.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::VoiceGlowMotion;
///
/// let beam = VoiceGlowMotion::default().gather(1.0).offset(0.5).held_level(0.55);
/// assert_eq!(beam.gather, 1.0);
/// assert_eq!(VoiceGlowMotion::default().gather, 0.0);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VoiceGlowMotion {
    /// 0 the voice glow at rest ... 1 the lobes gathered into one beam.
    pub gather: f64,
    /// Where the beam sits, in half-widths of the lobe ring (0 = centre).
    pub offset: f64,
    /// Extra width while it moves, 0-1.
    pub stretch: f64,
    /// The level the glow is held at while gathered, 0-1.
    pub held_level: f64,
    /// How much the beam rides the element's corner arcs, 0-1.
    pub corner_follow: f64,
}

impl VoiceGlowMotion {
    /// Sets `gather`.
    #[must_use]
    pub fn gather(mut self, gather: f64) -> Self {
        self.gather = gather;
        self
    }

    /// Sets `offset`.
    #[must_use]
    pub fn offset(mut self, offset: f64) -> Self {
        self.offset = offset;
        self
    }

    /// Sets `stretch`.
    #[must_use]
    pub fn stretch(mut self, stretch: f64) -> Self {
        self.stretch = stretch;
        self
    }

    /// Sets `held_level`.
    #[must_use]
    pub fn held_level(mut self, held_level: f64) -> Self {
        self.held_level = held_level;
        self
    }

    /// Sets `corner_follow`.
    #[must_use]
    pub fn corner_follow(mut self, corner_follow: f64) -> Self {
        self.corner_follow = corner_follow;
        self
    }

    /// The driver's per-field clamping (`gather`, `stretch`, `heldLevel` and
    /// `cornerFollow` to 0-1; `offset` left alone). Non-finite fields rest.
    pub(crate) fn sanitized(self) -> Self {
        let unit = |v: f64| {
            if v.is_finite() {
                v.clamp(0.0, 1.0)
            } else {
                0.0
            }
        };
        Self {
            gather: unit(self.gather),
            offset: if self.offset.is_finite() {
                self.offset
            } else {
                0.0
            },
            stretch: unit(self.stretch),
            held_level: unit(self.held_level),
            corner_follow: unit(self.corner_follow),
        }
    }
}

type MotionFn = dyn Fn() -> Option<VoiceGlowMotion> + Send + Sync;

/// Where the glow's motion comes from (the `motion` prop).
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::{MotionSource, VoiceGlowMotion};
///
/// let fixed = MotionSource::fixed(VoiceGlowMotion::default().gather(0.5));
/// let live = MotionSource::getter(|| None);
/// assert_ne!(fixed, live);
/// ```
#[non_exhaustive]
#[derive(Clone, Default)]
pub enum MotionSource {
    /// At rest.
    #[default]
    Rest,
    /// A fixed motion.
    Fixed(VoiceGlowMotion),
    /// A getter sampled every frame; `None` is at rest.
    Getter(Arc<MotionFn>),
}

impl MotionSource {
    /// A fixed motion.
    #[must_use]
    pub fn fixed(motion: VoiceGlowMotion) -> Self {
        Self::Fixed(motion)
    }

    /// A per-frame getter.
    #[must_use]
    pub fn getter(f: impl Fn() -> Option<VoiceGlowMotion> + Send + Sync + 'static) -> Self {
        Self::Getter(Arc::new(f))
    }

    /// The motion now, if the source supplies one.
    pub(crate) fn sample(&self) -> Option<VoiceGlowMotion> {
        match self {
            Self::Rest => None,
            Self::Fixed(m) => Some(*m),
            Self::Getter(f) => f(),
        }
    }
}

impl fmt::Debug for MotionSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rest => f.write_str("Rest"),
            Self::Fixed(m) => f.debug_tuple("Fixed").field(m).finish(),
            Self::Getter(_) => f.write_str("Getter(..)"),
        }
    }
}

impl PartialEq for MotionSource {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Rest, Self::Rest) => true,
            (Self::Fixed(a), Self::Fixed(b)) => a == b,
            (Self::Getter(a), Self::Getter(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

/// Tuning of the processing sweep (`ProcessingTuning` upstream).
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::{ProcessingTuning, VoiceGlowType};
///
/// let mobile = ProcessingTuning::for_type(VoiceGlowType::Mobile);
/// assert_eq!((mobile.duration, mobile.level, mobile.travel), (1.05, 0.35, 1.0));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProcessingTuning {
    /// Seconds per pass (left to right, or back).
    pub duration: f64,
    /// How lit the glow is held while processing, 0-1.
    pub level: f64,
    /// How far the beam travels to each side, x half the lobe ring.
    pub travel: f64,
    /// How it eases into each turn: 1 constant speed, 2 smooth, higher dwells.
    pub curve: f64,
    /// How much the beam rides the element's corner arcs, 0-1.
    pub corner_follow: f64,
    /// Seconds the morph in and out takes.
    pub ease: f64,
}

impl ProcessingTuning {
    /// The tuning per host type (`PROCESSING_DEFAULTS` upstream).
    #[must_use]
    pub const fn for_type(kind: VoiceGlowType) -> Self {
        match kind {
            VoiceGlowType::Default => Self {
                duration: 1.1,
                level: 0.55,
                travel: 1.55,
                curve: 2.1,
                corner_follow: 0.45,
                ease: 0.6,
            },
            VoiceGlowType::Pill => Self {
                duration: 1.1,
                level: 0.55,
                travel: 2.0,
                curve: 2.1,
                corner_follow: 0.0,
                ease: 0.6,
            },
            VoiceGlowType::Mobile => Self {
                duration: 1.05,
                level: 0.35,
                travel: 1.0,
                curve: 2.1,
                corner_follow: 0.4,
                ease: 0.6,
            },
        }
    }
}

impl Default for ProcessingTuning {
    fn default() -> Self {
        Self::for_type(VoiceGlowType::Default)
    }
}

/// One-pole follower: fast up (attack), slow down (release).
pub(crate) fn follow(prev: f64, target: f64, dt: f64, attack: f64, release: f64) -> f64 {
    let tau = if target > prev { attack } else { release };
    prev + (target - prev) * (1.0 - (-dt / tau.max(0.001)).exp())
}

/// The processing driver: call [`VoiceProcessing::step`] every frame with
/// whether processing is on. A fresh start begins the pass at the centre,
/// heading right, so the beam grows out of the voice glow instead of jumping
/// to one end; it morphs in and out smoothly.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::{ProcessingTuning, VoiceGlowType, VoiceProcessing};
///
/// let tuning = ProcessingTuning::for_type(VoiceGlowType::Default);
/// let mut processing = VoiceProcessing::new();
/// let mut motion = processing.step(true, &tuning, false, 0.0);
/// for _ in 0..120 {
///     motion = processing.step(true, &tuning, false, 1.0 / 60.0);
/// }
/// assert!(motion.gather > 0.9);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VoiceProcessing {
    blend: f64,
    clock: f64,
    started: bool,
}

impl VoiceProcessing {
    /// A processing driver at rest.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `true` while the beam is on or still morphing out.
    #[must_use]
    pub fn is_engaged(&self) -> bool {
        self.blend >= 0.001 || self.clock != 0.0
    }

    /// Advances by `dt` seconds (the first call ever uses 0, later ones are
    /// clamped to `0..=0.05` like upstream) and returns the motion.
    pub fn step(
        &mut self,
        active: bool,
        tuning: &ProcessingTuning,
        reduced_motion: bool,
        dt: f64,
    ) -> VoiceGlowMotion {
        let dt = if self.started {
            dt.clamp(0.0, 0.05)
        } else {
            0.0
        };
        self.started = true;
        let duration = tuning.duration.max(0.05);
        // A fresh start begins the pass at the centre, heading right.
        if active && self.blend < 0.001 && self.clock == 0.0 {
            self.clock = duration / 2.0;
        }
        let ease = tuning.ease.max(0.05);
        self.blend = follow(
            self.blend,
            if active { 1.0 } else { 0.0 },
            dt,
            ease * 0.9,
            ease * 0.8,
        );
        if active {
            self.clock += dt;
        } else if self.blend < 0.001 {
            self.clock = 0.0;
        }

        // The morph runs on an eased copy of the blend, slow to start, slow
        // to settle, so the gather, the narrowing and the travel read as one.
        let morph = self.blend * self.blend * (3.0 - 2.0 * self.blend);
        let passes = self.clock / duration;
        let index = passes.floor();
        let u = passes - index;
        let k = tuning.curve.max(1.0);
        let eased = if u < 0.5 {
            0.5 * (2.0 * u).powf(k)
        } else {
            1.0 - 0.5 * (2.0 - 2.0 * u).powf(k)
        };
        let even = (index as i64).rem_euclid(2) == 0;
        let pass = if reduced_motion {
            0.0
        } else if even {
            2.0 * eased - 1.0
        } else {
            1.0 - 2.0 * eased
        };
        VoiceGlowMotion {
            gather: morph,
            offset: morph * tuning.travel * pass,
            // A little wider mid-pass than at the turns.
            stretch: 1.0 - pass * pass,
            held_level: tuning.level,
            corner_follow: tuning.corner_follow,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f64 = 1.0 / 60.0;

    #[test]
    fn fresh_start_begins_at_the_centre_heading_right() {
        let t = ProcessingTuning::default();
        let mut p = VoiceProcessing::new();
        let first = p.step(true, &t, false, DT);
        assert_eq!(first.gather, 0.0);
        assert!(first.offset.abs() < 1e-12, "starts at the centre");
        let mut last = first;
        for _ in 0..10 {
            last = p.step(true, &t, false, DT);
        }
        assert!(last.offset > 0.0, "heads right");
    }

    #[test]
    fn gathers_fully_then_releases_and_resets() {
        let t = ProcessingTuning::for_type(VoiceGlowType::Pill);
        let mut p = VoiceProcessing::new();
        let mut m = VoiceGlowMotion::default();
        for _ in 0..240 {
            m = p.step(true, &t, false, DT);
        }
        assert!(m.gather > 0.99);
        assert_eq!(m.held_level, 0.55);
        assert_eq!(m.corner_follow, 0.0);
        for _ in 0..600 {
            m = p.step(false, &t, false, DT);
        }
        assert!(m.gather < 1e-6 && m.offset.abs() < 1e-6);
        assert!(!p.is_engaged());
    }

    #[test]
    fn offset_stays_within_travel_and_reverses() {
        let t = ProcessingTuning::default();
        let mut p = VoiceProcessing::new();
        let (mut lo, mut hi) = (f64::MAX, f64::MIN);
        for _ in 0..600 {
            let m = p.step(true, &t, false, DT);
            lo = lo.min(m.offset);
            hi = hi.max(m.offset);
            assert!((0.0..=1.0).contains(&m.stretch));
        }
        assert!(hi <= t.travel + 1e-9 && lo >= -t.travel - 1e-9);
        assert!(hi > 0.9 * t.travel && lo < -0.9 * t.travel);
    }

    #[test]
    fn reduced_motion_pins_the_beam_to_the_centre() {
        let t = ProcessingTuning::default();
        let mut p = VoiceProcessing::new();
        let mut m = VoiceGlowMotion::default();
        for _ in 0..200 {
            m = p.step(true, &t, true, DT);
        }
        assert!(m.gather > 0.9);
        assert_eq!(m.offset, 0.0);
        assert_eq!(m.stretch, 1.0);
    }

    #[test]
    fn sanitized_clamps_like_the_driver() {
        let m = VoiceGlowMotion {
            gather: 2.0,
            offset: -3.0,
            stretch: -1.0,
            held_level: f64::NAN,
            corner_follow: 0.5,
        }
        .sanitized();
        assert_eq!(
            (m.gather, m.offset, m.stretch, m.held_level, m.corner_follow),
            (1.0, -3.0, 0.0, 0.0, 0.5)
        );
    }
}
