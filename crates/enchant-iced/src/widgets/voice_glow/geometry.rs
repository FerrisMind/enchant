//! Lobe geometry and the band line's maths (`styles.ts` lobes and the band
//! helpers of `voiceDriver.ts`), all in px for a ~350px-wide host.

/// One soft ellipse of the glow, at its resting place.
///
/// Seven fan out from the bottom centre: the centre lobe follows the low
/// band, its neighbours the mids, the outer pair the highs and the far pair
/// the mids again, so a voice makes the colours ripple outward instead of one
/// blob pumping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoiceLobe {
    /// Resting offset from centre, px.
    pub x: i32,
    /// Resting width, px.
    pub w: i32,
    /// Resting height, px.
    pub h: i32,
    /// The band that drives it: 0 low, 1 mid, 2 high.
    pub band: usize,
}

/// The seven lobes (`voiceLobes` upstream).
pub const LOBES: [VoiceLobe; 7] = [
    VoiceLobe {
        x: 0,
        w: 74,
        h: 46,
        band: 0,
    },
    VoiceLobe {
        x: -36,
        w: 54,
        h: 40,
        band: 1,
    },
    VoiceLobe {
        x: 36,
        w: 54,
        h: 40,
        band: 1,
    },
    VoiceLobe {
        x: -72,
        w: 48,
        h: 32,
        band: 2,
    },
    VoiceLobe {
        x: 72,
        w: 48,
        h: 32,
        band: 2,
    },
    VoiceLobe {
        x: -108,
        w: 42,
        h: 26,
        band: 1,
    },
    VoiceLobe {
        x: 108,
        w: 42,
        h: 26,
        band: 1,
    },
];

/// Resting distance between neighbouring lobes, px.
pub const LOBE_SPACING: f64 = 36.0;
/// Width of the ring the lobes travel around, px: one full turn of the flow.
pub const LOBE_SPAN: f64 = LOBE_SPACING * LOBES.len() as f64;

/// Half-width of the ceiling ellipse the band sits on (px at multiplier 1).
pub(crate) const CEILING_HALF_WIDTH: f64 = 170.0;
/// Height of the ceiling ellipse (px at multiplier 1).
pub(crate) const CEILING_HEIGHT: f64 = 64.0;
/// Segments of the band line (the line has one more point).
pub const BAND_SAMPLES: usize = 56;

/// Wrap a lobe offset into `[-span/2, span/2)`.
pub(crate) fn wrap_x(x: f64, span: f64) -> f64 {
    let half = span / 2.0;
    (((x + half) % span) + span) % span - half
}

/// How much of a lobe shows at offset `x`: full at centre, gone at the wrap
/// edge so a lobe never pops from one side to the other.
pub(crate) fn edge_envelope(x: f64, span: f64) -> f64 {
    let t = x / (span / 2.0 + 4.0);
    (1.0 - t * t).max(0.0)
}

/// The band's bell, normalised to 1 at the centre and exactly 0 at the ends:
/// `exp(-(|t| / sigma)^p)` with the tail value subtracted out. `p` below 2
/// gives the exponential, cusp-like rise of a bent surface; above 2 a flatter
/// top. `skew` widens one side and narrows the other.
pub(crate) fn bell(t: f64, p: f64, sigma: f64, skew: f64) -> f64 {
    let side = if t < 0.0 { 1.0 - skew } else { 1.0 + skew };
    let s = (sigma * side).max(0.05);
    let v = (-(t.abs() / s).powf(p)).exp();
    let tail = (-(1.0 / s).powf(p)).exp();
    ((v - tail) / (1.0 - tail)).max(0.0)
}

/// The band's tail lift: from `position` of the way from the beam's centre to
/// the element's edge, the line rises again to the full `lift` exactly at the
/// corner, with `curve` as the exponent.
pub(crate) fn tail_lift(dist: f64, edge: f64, lift: f64, position: f64, curve: f64) -> f64 {
    if lift <= 0.0 || edge <= 0.0 {
        return 0.0;
    }
    let start = edge * position.clamp(0.0, 0.98);
    if dist <= start {
        return 0.0;
    }
    let u = ((dist - start) / (edge - start).max(1.0)).min(1.0);
    lift * u.powf(curve.max(0.5))
}

/// The corner radius the host actually paints: CSS clamps a radius that would
/// overlap to half the box, so a pill's `border-radius: 106px` on a 150x44
/// element is really 22px.
pub(crate) fn painted_radius(radius: f64, cw: f64, ch: f64) -> f64 {
    radius.min(cw / 2.0).min(ch / 2.0).max(0.0)
}

/// How far above the bottom edge the host's outline sits at `x`: 0 along the
/// straight run, rising along the corner's circle inside the last `radius` px
/// on either side. `influence` is how far the thing at `x` spreads sideways.
pub(crate) fn corner_lift(x: f64, cw: f64, radius: f64, influence: f64) -> f64 {
    if radius <= 0.0 {
        return 0.0;
    }
    let d = x.min(cw - x) - influence;
    if d >= radius {
        return 0.0;
    }
    if d <= 0.0 {
        return radius;
    }
    let dx = radius - d;
    radius - (radius * radius - dx * dx).max(0.0).sqrt()
}

/// Cosine ease-in-out in `[0, 1]`: 0 at phase 0/1, 1 at phase 0.5.
pub(crate) fn ping_pong(phase: f64) -> f64 {
    (1.0 - (std::f64::consts::TAU * phase).cos()) / 2.0
}

/// Noise gate then soft saturation, so a shout rounds off instead of clipping.
pub(crate) fn shape(raw: f64, threshold: f64) -> f64 {
    if raw <= threshold {
        return 0.0;
    }
    let t = (raw - threshold) / (1.0 - threshold).max(0.001);
    ((1.0 - (-3.0 * t).exp()) / (1.0 - (-3.0_f64).exp())).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_stays_in_the_ring() {
        let span = LOBE_SPAN;
        assert_eq!(wrap_x(0.0, span), 0.0);
        assert!((wrap_x(span / 2.0, span) + span / 2.0).abs() < 1e-12);
        assert!((wrap_x(-span / 2.0 - 1.0, span) - (span / 2.0 - 1.0)).abs() < 1e-12);
        assert!((wrap_x(3.5 * span + 10.0, span) - (10.0 - span / 2.0)).abs() < 1e-9);
    }

    #[test]
    fn edge_envelope_fades_lobes_at_the_wrap() {
        assert_eq!(edge_envelope(0.0, LOBE_SPAN), 1.0);
        assert_eq!(edge_envelope(LOBE_SPAN, LOBE_SPAN), 0.0);
        // At the wrap edge itself 1 - (126/130)^2 is all that is left.
        assert!(
            (edge_envelope(LOBE_SPAN / 2.0, LOBE_SPAN) - (1.0 - (126.0_f64 / 130.0).powi(2))).abs()
                < 1e-12
        );
    }

    #[test]
    fn shape_gates_and_saturates() {
        assert_eq!(shape(0.015, 0.015), 0.0);
        assert_eq!(shape(0.0, 0.015), 0.0);
        assert!((shape(1.0, 0.015) - 1.0).abs() < 1e-12);
        // t = 0.5: (1 - e^-1.5) / (1 - e^-3)
        let want = (1.0 - (-1.5_f64).exp()) / (1.0 - (-3.0_f64).exp());
        assert!((shape(0.5, 0.0) - want).abs() < 1e-12);
        assert_eq!(shape(5.0, 0.0), 1.0);
    }

    #[test]
    fn bell_is_one_at_centre_and_zero_at_ends() {
        assert!((bell(0.0, 1.75, 0.87, 0.12) - 1.0).abs() < 1e-12);
        assert!(bell(1.0, 1.75, 0.87, 0.12).abs() < 1e-12);
        assert!(bell(-1.0, 1.75, 0.87, 0.12).abs() < 1e-12);
        // Positive skew widens the right side.
        assert!(bell(0.5, 1.75, 0.87, 0.12) > bell(-0.5, 1.75, 0.87, 0.12));
    }

    #[test]
    fn tail_lift_reaches_full_at_the_corner() {
        assert_eq!(tail_lift(10.0, 100.0, 0.5, 0.67, 2.4), 0.0);
        assert!((tail_lift(100.0, 100.0, 0.5, 0.67, 2.4) - 0.5).abs() < 1e-12);
        assert_eq!(tail_lift(100.0, 100.0, 0.0, 0.67, 2.4), 0.0);
    }

    #[test]
    fn corner_lift_follows_the_arc() {
        assert_eq!(corner_lift(100.0, 200.0, 16.0, 0.0), 0.0);
        assert_eq!(corner_lift(0.0, 200.0, 16.0, 0.0), 16.0);
        assert_eq!(corner_lift(-5.0, 200.0, 16.0, 0.0), 16.0);
        let mid = corner_lift(8.0, 200.0, 16.0, 0.0);
        assert!((mid - (16.0 - (256.0_f64 - 64.0).sqrt())).abs() < 1e-12);
        assert_eq!(painted_radius(106.0, 150.0, 44.0), 22.0);
    }

    #[test]
    fn ping_pong_is_a_cosine_ease() {
        assert!(ping_pong(0.0).abs() < 1e-12);
        assert!((ping_pong(0.5) - 1.0).abs() < 1e-12);
        assert!(ping_pong(1.0).abs() < 1e-12);
    }
}
