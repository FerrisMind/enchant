//! Spring physics and CSS timing functions for the liquid-gooey port.
//!
//! Mirrors upstream `liquid-gooey/src/spring.ts` (the preset table and
//! `cubic-bezier` evaluation) together with `springStep` / `springSteps` from
//! `observer.ts` (semi-implicit Euler, wall-clock `dt` substepped at <= 1/60 s).
//!
//! Upstream compiles a spring into a CSS `linear()` sample list that the
//! compositor plays back; iced has no CSS transitions, so the same motion is
//! integrated here per frame. [`Transition`] is the per-item motion spec used
//! by the gooey engine for item position and scale changes, and [`Easing`]
//! covers explicit [`Transition::Timed`] durations.
//!
//! Physics helpers work in seconds and in the item's coordinate or scale
//! units. [`SpringConfig::mass`] is not a parameter of `step` / `advance`;
//! callers fold it into the rates they pass (`stiffness / mass`,
//! `damping / mass`) so the substep loop avoids a division per integration
//! step.

use std::time::Duration;

/// Largest stiffness / damping accepted by [`SpringConfig::new`].
const MAX_RATE: f32 = 1.0e6;

/// Smallest / largest mass accepted by [`SpringConfig::new`].
const MIN_MASS: f32 = 1.0e-3;
const MAX_MASS: f32 = 1.0e3;

/// Spring parameters for one axis of motion.
///
/// The fields are raw: physics is undefined for non-positive stiffness,
/// non-positive damping, or non-positive mass. Prefer [`SpringConfig::new`] or
/// [`TransitionPreset::config`] when values come from user input, so the
/// simulation cannot receive a degenerate configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpringConfig {
    /// Hooke constant `k` in force units per displacement unit.
    pub stiffness: f32,
    /// Viscous damping `c` in force units per velocity unit.
    pub damping: f32,
    /// Inertial mass `m`.
    pub mass: f32,
}

impl Default for SpringConfig {
    fn default() -> Self {
        Self {
            stiffness: 300.0,
            damping: 24.0,
            mass: 1.0,
        }
    }
}

impl SpringConfig {
    /// Sanitized constructor: clamps `stiffness` and `damping` to
    /// `0.0..=1e6` and `mass` to `1e-3..=1e3`.
    ///
    /// `NaN` maps to `0.0` for the rate fields (an inert spring) and to `1.0`
    /// for `mass` (neutral inertia); infinities clamp to the nearest bound.
    #[must_use]
    pub fn new(stiffness: f32, damping: f32, mass: f32) -> Self {
        Self {
            stiffness: sanitize(stiffness, 0.0, MAX_RATE, 0.0),
            damping: sanitize(damping, 0.0, MAX_RATE, 0.0),
            mass: sanitize(mass, MIN_MASS, MAX_MASS, 1.0),
        }
    }
}

fn sanitize(value: f32, min: f32, max: f32, nan: f32) -> f32 {
    if value.is_nan() {
        nan
    } else {
        value.clamp(min, max)
    }
}

/// Named spring tunings from upstream `spring.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TransitionPreset {
    /// Fast and firm: `480 / 34 / 1` (stiffness / damping / mass).
    Snappy,
    /// Upstream default: `190 / 26 / 1`.
    Smooth,
    /// Visible overshoot: `320 / 17 / 1`.
    Bouncy,
}

impl TransitionPreset {
    /// The upstream spring parameters for this preset.
    #[must_use]
    pub const fn config(self) -> SpringConfig {
        match self {
            Self::Snappy => SpringConfig {
                stiffness: 480.0,
                damping: 34.0,
                mass: 1.0,
            },
            Self::Smooth => SpringConfig {
                stiffness: 190.0,
                damping: 26.0,
                mass: 1.0,
            },
            Self::Bouncy => SpringConfig {
                stiffness: 320.0,
                damping: 17.0,
                mass: 1.0,
            },
        }
    }
}

/// A CSS timing function, evaluated analytically.
///
/// Upstream uses these for explicit-duration transitions; the keyword curves
/// map to the same control points CSS defines.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Easing {
    /// `linear`: output equals `t`.
    Linear,
    /// `ease`: `cubic-bezier(0.25, 0.1, 0.25, 1)`.
    Ease,
    /// `ease-in`: `cubic-bezier(0.42, 0, 1, 1)`.
    EaseIn,
    /// `ease-out`: `cubic-bezier(0, 0, 0.58, 1)`.
    EaseOut,
    /// `ease-in-out`: `cubic-bezier(0.42, 0, 0.58, 1)`.
    EaseInOut,
    /// An explicit CSS cubic-bezier curve.
    ///
    /// `x1` and `x2` must lie in `0.0..=1.0` for CSS semantics (the `x(u)`
    /// curve has to be monotonic for the bisection to be meaningful). `y1`
    /// and `y2` may leave the unit interval to express overshoot.
    CubicBezier { x1: f32, y1: f32, x2: f32, y2: f32 },
}

impl Easing {
    /// Evaluate the curve at progress `t`.
    ///
    /// `t` is clamped to `0.0..=1.0` (CSS semantics); `NaN` evaluates as `0.0`
    /// and infinities clamp to the nearest endpoint. A [`Self::CubicBezier`]
    /// curve solves `x(u) = t` by 24-step bisection (same iteration count as
    /// upstream), then returns `y(u)`. Endpoints return exactly `0.0` / `1.0`.
    #[must_use]
    pub fn evaluate(self, t: f32) -> f32 {
        let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
        match self {
            Self::Linear => t,
            Self::Ease => cubic_bezier(t, 0.25, 0.1, 0.25, 1.0),
            Self::EaseIn => cubic_bezier(t, 0.42, 0.0, 1.0, 1.0),
            Self::EaseOut => cubic_bezier(t, 0.0, 0.0, 0.58, 1.0),
            Self::EaseInOut => cubic_bezier(t, 0.42, 0.0, 0.58, 1.0),
            Self::CubicBezier { x1, y1, x2, y2 } => cubic_bezier(t, x1, y1, x2, y2),
        }
    }
}

fn cubic_bezier(t: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }

    let mut lo = 0.0_f32;
    let mut hi = 1.0_f32;
    for _ in 0..24 {
        let mid = 0.5 * (lo + hi);
        if bezier_axis(mid, x1, x2) < t {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    bezier_axis(0.5 * (lo + hi), y1, y2)
}

/// One axis of a cubic bezier with endpoints fixed at 0 and 1.
fn bezier_axis(u: f32, p1: f32, p2: f32) -> f32 {
    let v = 1.0 - u;
    3.0 * u * v * v * p1 + 3.0 * u * u * v * p2 + u * u * u
}

/// Motion spec for component-driven item x / y / scale changes.
///
/// One transition drives every animated property of an item, matching
/// upstream's single `transition` prop. [`Transition::Smooth`] is the default
/// and is equivalent to `Preset(TransitionPreset::Smooth)`.
///
/// The enum is [`Copy`] so resolved specs can be stored per item without
/// allocation.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[non_exhaustive]
pub enum Transition {
    /// Upstream default: [`TransitionPreset::Smooth`] (`190 / 26 / 1`).
    #[default]
    Smooth,
    /// A named upstream preset.
    Preset(TransitionPreset),
    /// A custom spring.
    Spring(SpringConfig),
    /// An explicit duration with a CSS-style easing curve.
    ///
    /// A zero [`Duration`] completes immediately, matching reduced-motion
    /// resolution upstream.
    Timed { duration: Duration, easing: Easing },
}

impl From<TransitionPreset> for Transition {
    fn from(preset: TransitionPreset) -> Self {
        Self::Preset(preset)
    }
}

/// One semi-implicit Euler step towards `target`.
///
/// `dt` is in seconds and should be finite and non-negative; the caller owns
/// substepping (see [`advance`]). `stiffness` and `damping` are the
/// mass-normalized rates: pass `k / m` and `c / m` when mass is not `1.0`.
#[must_use]
pub(crate) fn step(
    cur: f32,
    vel: f32,
    target: f32,
    stiffness: f32,
    damping: f32,
    dt: f32,
) -> (f32, f32) {
    let acceleration = stiffness * (target - cur) - damping * vel;
    let velocity = vel + acceleration * dt;
    (cur + velocity * dt, velocity)
}

/// Advance a spring over a wall-clock `dt`, substepping at <= 1/60 s.
///
/// Matches upstream `springSteps`: `n = max(1, ceil(dt * 60))` substeps of
/// `h = dt / n`, so a long frame cannot destabilize the integration. This is
/// the per-frame call the engine makes for item x / y / scale springs.
///
/// A non-finite or non-positive `dt` is a no-op (state is returned unchanged).
/// Finite frame gaps are integrated in full so motion follows the wall clock;
/// the engine clips its own frame delta upstream (0.25 s) before calling.
/// `stiffness` / `damping` are mass-normalized, as in [`step`].
#[must_use]
pub(crate) fn advance(
    cur: f32,
    vel: f32,
    target: f32,
    stiffness: f32,
    damping: f32,
    dt: f32,
) -> (f32, f32) {
    if !dt.is_finite() || dt <= 0.0 {
        return (cur, vel);
    }
    let substeps = (dt * 60.0).ceil().max(1.0) as u32;
    let h = dt / substeps as f32;

    let mut cur = cur;
    let mut vel = vel;
    for _ in 0..substeps {
        (cur, vel) = step(cur, vel, target, stiffness, damping, h);
    }
    (cur, vel)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1.0e-6;

    /// Run for a whole number of frames, tracking the peak value reached.
    fn simulate(config: SpringConfig, target: f32, seconds: f32, hz: f32) -> (f32, f32, f32) {
        let dt = 1.0 / hz;
        let frames = (seconds * hz).round() as usize;
        let mut cur = 0.0_f32;
        let mut vel = 0.0_f32;
        let mut peak = cur;
        for _ in 0..frames {
            (cur, vel) = advance(
                cur,
                vel,
                target,
                config.stiffness / config.mass,
                config.damping / config.mass,
                dt,
            );
            peak = peak.max(cur);
        }
        (cur, vel, peak)
    }

    #[test]
    fn presets_match_upstream() {
        assert_eq!(
            TransitionPreset::Snappy.config(),
            SpringConfig {
                stiffness: 480.0,
                damping: 34.0,
                mass: 1.0
            }
        );
        assert_eq!(
            TransitionPreset::Smooth.config(),
            SpringConfig {
                stiffness: 190.0,
                damping: 26.0,
                mass: 1.0
            }
        );
        assert_eq!(
            TransitionPreset::Bouncy.config(),
            SpringConfig {
                stiffness: 320.0,
                damping: 17.0,
                mass: 1.0
            }
        );
    }

    #[test]
    fn default_config_matches_upstream() {
        assert_eq!(
            SpringConfig::default(),
            SpringConfig {
                stiffness: 300.0,
                damping: 24.0,
                mass: 1.0
            }
        );
    }

    #[test]
    fn new_sanitizes_inputs() {
        let clamped = SpringConfig::new(-10.0, f32::INFINITY, 0.0);
        assert_eq!(clamped.stiffness, 0.0);
        assert_eq!(clamped.damping, MAX_RATE);
        assert_eq!(clamped.mass, MIN_MASS);

        let nan = SpringConfig::new(f32::NAN, 12.0, f32::NAN);
        assert_eq!(nan.stiffness, 0.0);
        assert_eq!(nan.damping, 12.0);
        assert_eq!(nan.mass, 1.0);
    }

    #[test]
    fn spring_converges_and_settles() {
        let config = SpringConfig::default();
        let (cur, vel, _) = simulate(config, 1.0, 4.0, 240.0);
        assert!((cur - 1.0).abs() < 1.0e-3, "cur = {cur}");
        assert!(vel.abs() < 0.02, "vel = {vel}");
    }

    #[test]
    fn critically_damped_does_not_overshoot_much() {
        let config = SpringConfig {
            stiffness: 100.0,
            damping: 20.0,
            mass: 1.0,
        };
        let (cur, vel, peak) = simulate(config, 1.0, 4.0, 240.0);
        assert!((cur - 1.0).abs() < 1.0e-3, "cur = {cur}");
        assert!(vel.abs() < 0.02, "vel = {vel}");
        assert!(peak <= 1.0 + 1.0e-3, "peak = {peak}");
    }

    #[test]
    fn advance_with_large_frame_dt_is_stable() {
        let config = TransitionPreset::Bouncy.config();
        let mut cur = 0.0_f32;
        let mut vel = 0.0_f32;
        let mut peak = cur;
        for _ in 0..5 {
            (cur, vel) = advance(
                cur,
                vel,
                1.0,
                config.stiffness / config.mass,
                config.damping / config.mass,
                0.2,
            );
            assert!(cur.is_finite(), "cur = {cur}");
            assert!(vel.is_finite(), "vel = {vel}");
            peak = peak.max(cur);
        }
        assert!(peak < 1.2, "peak = {peak}");
        assert!((cur - 1.0).abs() < 1.0e-3, "cur = {cur}");
    }

    #[test]
    fn advance_sanitizes_bad_dt() {
        let untouched = advance(0.25, 1.5, 1.0, 300.0, 24.0, f32::NAN);
        assert_eq!(untouched, (0.25, 1.5));
        assert_eq!(
            advance(0.25, 1.5, 1.0, 300.0, 24.0, f32::INFINITY),
            (0.25, 1.5)
        );
        assert_eq!(
            advance(0.25, 1.5, 1.0, 300.0, 24.0, -1.0 / 60.0),
            (0.25, 1.5)
        );
        assert_eq!(advance(0.25, 1.5, 1.0, 300.0, 24.0, 0.0), (0.25, 1.5));
    }

    #[test]
    fn cubic_bezier_identity_is_linear() {
        let easing = Easing::CubicBezier {
            x1: 0.0,
            y1: 0.0,
            x2: 1.0,
            y2: 1.0,
        };
        for t in [0.1_f32, 0.25, 0.5, 0.75, 0.9] {
            assert!(
                (easing.evaluate(t) - t).abs() < 1.0e-4,
                "t = {t}, got {}",
                easing.evaluate(t)
            );
        }
    }

    #[test]
    fn ease_in_out_midpoint_is_half() {
        assert!(
            (Easing::EaseInOut.evaluate(0.5) - 0.5).abs() < 1.0e-4,
            "got {}",
            Easing::EaseInOut.evaluate(0.5)
        );
    }

    #[test]
    fn evaluate_clamps_outside_unit_interval() {
        for easing in [
            Easing::Linear,
            Easing::Ease,
            Easing::EaseIn,
            Easing::EaseOut,
            Easing::EaseInOut,
            Easing::CubicBezier {
                x1: 0.42,
                y1: 0.0,
                x2: 0.58,
                y2: 1.0,
            },
        ] {
            assert_eq!(easing.evaluate(-0.5), 0.0, "{easing:?}");
            assert_eq!(easing.evaluate(1.5), 1.0, "{easing:?}");
            assert_eq!(easing.evaluate(f32::NEG_INFINITY), 0.0, "{easing:?}");
            assert_eq!(easing.evaluate(f32::INFINITY), 1.0, "{easing:?}");
            assert_eq!(easing.evaluate(f32::NAN), 0.0, "{easing:?}");
        }
        assert!((Easing::Linear.evaluate(0.25) - 0.25).abs() < EPSILON);
    }

    #[test]
    fn smooth_transition_is_default() {
        assert_eq!(Transition::default(), Transition::Smooth);
        assert_eq!(
            Transition::from(TransitionPreset::Smooth),
            Transition::Preset(TransitionPreset::Smooth)
        );
    }
}
