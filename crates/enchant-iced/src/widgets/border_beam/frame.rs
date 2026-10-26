//! Time sampling: the rotating angle, hue drift, line keyframes and pulse
//! oscillators.

use std::f32::consts::TAU;
use std::time::Duration;

use super::options::BorderBeamSpec;
use super::palette::{BeamHost, BorderBeamSize};

/// One sampled animation state.
///
/// Obtain it from [`BorderBeamOptions::frame_at`](super::BorderBeamOptions::frame_at)
/// and draw it with [`BorderBeamRaster::render`](super::BorderBeamRaster::render).
/// # Examples
///
/// ```rust
/// use enchant_iced::{BeamHost, BorderBeamOptions, ResolvedBeamTheme};
/// use std::time::Duration;
///
/// let frame = BorderBeamOptions::new().frame_at(
///     Duration::from_millis(490),
///     BeamHost::new(348.0, 122.0),
///     ResolvedBeamTheme::Dark,
/// );
/// assert!((frame.angle_degrees() - 90.0).abs() < 0.1);
/// ```
#[must_use = "a frame does nothing unless rasterized"]
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BorderBeamFrame {
    pub(crate) spec: BorderBeamSpec,
    pub(crate) host: BeamHost,
    pub(crate) elapsed: Duration,
    pub(crate) angle_degrees: f32,
    pub(crate) hue_degrees: f32,
    pub(crate) line: Option<LineFrame>,
    pub(crate) pulse: Option<PulseFrame>,
}

impl BorderBeamFrame {
    /// The resolved style this frame belongs to.
    pub fn spec(&self) -> &BorderBeamSpec {
        &self.spec
    }

    /// The host size at sampling time.
    pub fn host(&self) -> BeamHost {
        self.host
    }

    /// The clock sampled for this frame. Reduced motion freezes it at a
    /// representative phase, so every time-derived value stops together.
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// The rotating conic angle in degrees (`md`/`sm`; `0` otherwise).
    pub fn angle_degrees(&self) -> f32 {
        self.angle_degrees
    }

    /// The current hue rotation in degrees (`0` when colours are static).
    pub fn hue_degrees(&self) -> f32 {
        self.hue_degrees
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LineFrame {
    pub x: f32,
    pub w: f32,
    pub h: f32,
    pub spike: f32,
    pub spike2: f32,
    pub edge: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PulseFrame {
    pub vars: PulseVars,
    /// Per-axis glow scale for `pulse-outside` (1 for `pulse-inner`).
    pub glow_scale: (f32, f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PulseVars {
    pub bw: [f32; 3],
    pub bh: [f32; 3],
    pub bx: [f32; 3],
    pub by: [f32; 3],
    pub bgh: f32,
    pub bop: [f32; 4],
}

/// Samples the whole effect at `elapsed`.
pub(crate) fn frame_at(
    spec: &BorderBeamSpec,
    host: BeamHost,
    elapsed: Duration,
) -> BorderBeamFrame {
    let t = sampled_time(spec, elapsed);
    let duration = spec.duration.as_secs_f32().max(0.001);
    let angle_degrees = if spec.size.is_pulse() {
        0.0
    } else {
        ((t / duration).fract() * 360.0).rem_euclid(360.0)
    };
    let hue_degrees = if spec.static_colors {
        0.0
    } else if spec.size.is_pulse() {
        let period = pulse_params(spec).hue_period.max(0.001);
        (t / period).fract() * 360.0
    } else {
        let range = spec.hue_range;
        -range + 2.0 * range * ping_pong(t / 12.0)
    };
    let line = (spec.size == BorderBeamSize::Line).then(|| line_frame(spec, t));
    let pulse = spec.size.is_pulse().then(|| PulseFrame {
        vars: pulse_vars(spec, t),
        glow_scale: glow_scale(spec, host),
    });
    BorderBeamFrame {
        spec: *spec,
        host,
        elapsed: Duration::from_secs_f32(t),
        angle_degrees,
        hue_degrees,
        line,
        pulse,
    }
}

/// Reduced motion is rendered at a representative phase instead of a blank
/// first frame (the reference's reduced-motion pulse rule leaves its opacity at
/// zero, which is not useful).
fn sampled_time(spec: &BorderBeamSpec, elapsed: Duration) -> f32 {
    if !spec.reduced_motion {
        return elapsed.as_secs_f32();
    }
    let duration = spec.duration.as_secs_f32();
    match spec.size {
        BorderBeamSize::Line => duration * 0.5,
        BorderBeamSize::PulseInner | BorderBeamSize::PulseOutside => duration * 0.348,
        BorderBeamSize::Sm | BorderBeamSize::Md => duration * 0.25,
    }
}

// ── line keyframes ──────────────────────────────────────────────────────────

#[rustfmt::skip]
const TRAVEL_X: [(f32, f32); 11] = [
    (0.0, 0.06), (10.0, 0.15), (20.0, 0.25), (30.0, 0.35), (40.0, 0.44),
    (50.0, 0.50), (60.0, 0.56), (70.0, 0.65), (80.0, 0.75), (90.0, 0.85),
    (100.0, 0.94),
];

#[rustfmt::skip]
const TRAVEL_W: [(f32, f32); 11] = [
    (0.0, 0.5), (10.0, 0.8), (20.0, 1.1), (30.0, 1.3), (40.0, 1.45),
    (50.0, 1.5), (60.0, 1.45), (70.0, 1.3), (80.0, 1.1), (90.0, 0.8),
    (100.0, 0.5),
];

#[rustfmt::skip]
const EDGE_FADE: [(f32, f32); 6] = [
    (0.0, 0.0), (12.5, 0.0), (32.5, 1.0), (67.5, 1.0), (87.5, 0.0),
    (100.0, 0.0),
];

#[rustfmt::skip]
const BREATHE: [(f32, f32); 5] = [(0.0, 0.8), (25.0, 1.25), (55.0, 0.85), (80.0, 1.3), (100.0, 0.8)];

#[rustfmt::skip]
const SPIKE: [(f32, f32); 5] = [(0.0, 0.8), (25.0, 1.3), (50.0, 0.9), (75.0, 1.4), (100.0, 0.8)];

#[rustfmt::skip]
const SPIKE2: [(f32, f32); 5] = [(0.0, 1.2), (25.0, 0.7), (50.0, 1.4), (75.0, 0.8), (100.0, 1.2)];

fn line_frame(spec: &BorderBeamSpec, t: f32) -> LineFrame {
    let duration = spec.duration.as_secs_f32().max(0.001);
    let cycle = |scale: f32| (t / (duration * scale)).rem_euclid(1.0);
    LineFrame {
        x: sample(&TRAVEL_X, cycle(1.0), false),
        w: sample(&TRAVEL_W, cycle(1.0), false),
        h: sample(&BREATHE, cycle(1.3), true),
        spike: sample(&SPIKE, cycle(1.33), true),
        spike2: sample(&SPIKE2, cycle(1.7), true),
        edge: sample(&EDGE_FADE, cycle(1.0), false),
    }
}

/// Samples a percentage keyframe table at phase `p` in `0..1`.
fn sample(table: &[(f32, f32)], p: f32, ease: bool) -> f32 {
    let pct = p.clamp(0.0, 1.0) * 100.0;
    if pct <= table[0].0 {
        return table[0].1;
    }
    for pair in table.windows(2) {
        let (p0, v0) = pair[0];
        let (p1, v1) = pair[1];
        if pct <= p1 {
            let f = if p1 > p0 { (pct - p0) / (p1 - p0) } else { 1.0 };
            let f = if ease { ease_in_out(f) } else { f };
            return v0 + (v1 - v0) * f;
        }
    }
    table[table.len() - 1].1
}

/// CSS `cubic-bezier(0.42, 0, 0.58, 1)`.
fn ease_in_out(x: f32) -> f32 {
    cubic_bezier(x, 0.42, 0.0, 0.58, 1.0)
}

/// CSS `ease` = `cubic-bezier(0.25, 0.1, 0.25, 1)`.
pub(crate) fn css_ease(x: f32) -> f32 {
    cubic_bezier(x, 0.25, 0.1, 0.25, 1.0)
}

/// Solves the x(t)=x equation of a cubic Bezier and returns y(t).
fn cubic_bezier(x: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let curve = |t: f32, p1: f32, p2: f32| {
        let mt = 1.0 - t;
        3.0 * mt * mt * t * p1 + 3.0 * mt * t * t * p2 + t * t * t
    };
    let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
    for _ in 0..24 {
        let mid = (lo + hi) * 0.5;
        if curve(mid, x1, x2) < x {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    curve((lo + hi) * 0.5, y1, y2).clamp(0.0, 1.0)
}

#[inline]
fn ping_pong(phase: f32) -> f32 {
    (1.0 - (TAU * phase).cos()) * 0.5
}

// ── pulse oscillators ──────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
struct PulseParams {
    sp: f32,
    dr: f32,
    op: f32,
    gh: f32,
    bs: f32,
    ss: f32,
    ghs: f32,
    hue_period: f32,
}

fn pulse_params(spec: &BorderBeamSpec) -> PulseParams {
    let duration = spec.duration.as_secs_f32().max(0.05);
    let scale = duration / 2.3;
    let dark = spec.theme.is_dark();
    if spec.size == BorderBeamSize::PulseInner {
        PulseParams {
            sp: 0.28,
            dr: if dark { 33.0 } else { 40.0 },
            op: if dark { 0.48 } else { 0.45 },
            gh: if dark { 0.34 } else { 0.22 },
            bs: (if dark { 1.9 } else { 2.6 }) * scale,
            ss: (if dark { 2.6 } else { 4.6 }) * scale,
            ghs: (if dark { 2.4 } else { 5.5 }) * scale,
            hue_period: 16.0,
        }
    } else {
        PulseParams {
            sp: if dark { 0.28 } else { 0.36 },
            dr: if dark { 14.0 } else { 19.0 },
            op: if dark { 0.46 } else { 0.0 },
            gh: if dark { 0.16 } else { 0.58 },
            bs: (if dark { 2.3 } else { 3.7 }) * scale,
            ss: (if dark { 6.4 } else { 4.6 }) * scale,
            ghs: (if dark { 2.4 } else { 3.8 }) * scale,
            hue_period: 14.0,
        }
    }
}

fn pulse_vars(spec: &BorderBeamSpec, t: f32) -> PulseVars {
    let p = pulse_params(spec);
    let (sp, dr, op) = (p.sp, p.dr, p.op);
    let osc = |a: f32, b: f32, period: f32, delay: f32| {
        let phase = if period > 0.0 {
            (t - delay) / period
        } else {
            0.0
        };
        a + (b - a) * ping_pong(phase)
    };
    PulseVars {
        bw: [
            osc(1.0 - sp, 1.0 + sp * 1.1, p.ss * 0.9, 0.0),
            osc(1.0 + sp, 1.0 - sp * 0.85, p.ss * 1.1, 0.0),
            osc(1.0 - sp * 0.6, 1.0 + sp * 1.15, p.ss * 0.98, 0.0),
        ],
        bh: [
            osc(1.0 + sp * 0.9, 1.0 - sp * 0.85, p.ss * 1.26, 0.0),
            osc(1.0 - sp * 0.8, 1.0 + sp * 1.05, p.ss * 0.81, 0.0),
            osc(1.0 + sp * 0.75, 1.0 - sp, p.ss * 1.4, 0.0),
        ],
        bx: [
            osc(-dr, dr * 0.9, p.bs * 1.6, 0.0),
            osc(dr * 0.8, -dr * 0.9, p.bs * 1.88, 0.0),
            osc(-dr * 0.6, dr, p.bs * 1.45, 0.0),
        ],
        by: [
            osc(dr * 0.55, -dr * 0.7, p.bs * 1.6, 0.0),
            osc(-dr, dr * 0.65, p.bs * 1.88, 0.0),
            osc(-dr * 0.85, dr * 0.45, p.bs * 1.45, 0.0),
        ],
        bgh: osc(1.0 - p.gh, 1.0 + p.gh, p.ghs, 0.0),
        bop: [
            osc(1.0 - op, 1.0, p.bs, 0.0),
            osc(1.0 - op, 1.0, p.bs * 1.32, p.bs * 0.28),
            osc(1.0 - op, 1.0, p.bs * 0.84, p.bs * 0.55),
            osc(1.0 - op, 1.0, p.bs * 1.58, p.bs * 0.83),
        ],
    }
}

fn glow_scale(spec: &BorderBeamSpec, host: BeamHost) -> (f32, f32) {
    if spec.size != BorderBeamSize::PulseOutside {
        return (1.0, 1.0);
    }
    let clamp = |v: f32| ((v / 350.0).clamp(0.35, 4.0) * 1000.0).round() / 1000.0;
    let sx = clamp(host.width());
    let sy = ((host.height() / 140.0).clamp(0.35, 4.0) * 1000.0).round() / 1000.0;
    (sx, sy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::border_beam::options::BorderBeamOptions;
    use crate::widgets::border_beam::palette::ResolvedBeamTheme;

    #[test]
    fn ping_pong_hits_its_endpoints() {
        assert!((ping_pong(0.0)).abs() < 1e-6);
        assert!((ping_pong(0.5) - 1.0).abs() < 1e-6);
        assert!((ping_pong(1.0)).abs() < 1e-6);
    }

    #[test]
    fn line_keyframes_match_the_reference_halves() {
        let spec = BorderBeamOptions::new()
            .size(BorderBeamSize::Line)
            .spec(ResolvedBeamTheme::Dark);
        let frame = line_frame(&spec, 1.55);
        assert!((frame.x - 0.5).abs() < 1e-5, "x {}", frame.x);
        assert!((frame.w - 1.5).abs() < 1e-5, "w {}", frame.w);
        assert!((frame.edge - 1.0).abs() < 1e-6);
    }

    #[test]
    fn md_angle_matches_the_frozen_parity_time() {
        let spec = BorderBeamOptions::new().spec(ResolvedBeamTheme::Dark);
        let frame = frame_at(
            &spec,
            BeamHost::new(348.0, 122.0),
            Duration::from_millis(490),
        );
        assert!((frame.angle_degrees - 90.0).abs() < 0.1);
        assert!(
            (frame.hue_degrees - (-29.018)).abs() < 0.05,
            "hue {}",
            frame.hue_degrees
        );
    }

    #[test]
    fn pulse_scale_is_measured_against_350x140() {
        let spec = BorderBeamOptions::new()
            .size(BorderBeamSize::PulseOutside)
            .spec(ResolvedBeamTheme::Dark);
        let frame = frame_at(&spec, BeamHost::new(348.0, 122.0), Duration::ZERO);
        let (sx, sy) = frame.pulse.unwrap().glow_scale;
        assert!((sx - 0.994).abs() < 1e-6, "sx {sx}");
        assert!((sy - 0.871).abs() < 1e-6, "sy {sy}");
    }

    #[test]
    fn reduced_motion_freezes_every_time_derived_value() {
        let spec = BorderBeamOptions::new()
            .size(BorderBeamSize::Line)
            .reduced_motion(true)
            .spec(ResolvedBeamTheme::Dark);
        let host = BeamHost::new(348.0, 122.0);
        let a = frame_at(&spec, host, Duration::ZERO);
        let b = frame_at(&spec, host, Duration::from_secs(30));
        assert_eq!(a, b);
        assert_eq!(a.elapsed(), Duration::from_secs_f32(1.55));
    }

    #[test]
    fn cubic_ease_matches_css_endpoints_and_symmetry() {
        assert_eq!(css_ease(0.0), 0.0);
        assert_eq!(css_ease(1.0), 1.0);
        let a = ease_in_out(0.25);
        let b = ease_in_out(0.75);
        assert!((a + b - 1.0).abs() < 1e-4);
    }
}
