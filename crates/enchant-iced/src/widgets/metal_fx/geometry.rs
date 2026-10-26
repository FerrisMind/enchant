//! Rounded-rect outline sampling and perimeter maths, from
//! `engine/renderer/outline.ts` and `engine/glow/geometry.ts`.
//!
//! The ring, the glow's band mask and the rim all describe the same rounded
//! rectangle. On the GPU it is a signed-distance field; the explicit polygon
//! here is what a vector deformation (the bend) displaces, and what the
//! tests pin to the reference point counts.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

/// Number of arc segments per corner (`ARC_N`).
pub const ARC_SEGMENTS: usize = 14;
/// Spacing of points along straight edges, CSS px (`EDGE_STEP`).
pub const EDGE_STEP: f32 = 1.5;

/// Shape of the host (`kind` in the reference).
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::MetalFxVariant;
///
/// assert_eq!(MetalFxVariant::Circle.ring_px(), 2.0);
/// assert_eq!(MetalFxVariant::Button.shader_scale(), 1.6);
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MetalFxVariant {
    /// Pill-shaped baseline: 1 px ring, shader scale 1.6.
    #[default]
    Button,
    /// Compact circle baseline: 2 px ring, shader scale 1.3.
    Circle,
}

impl MetalFxVariant {
    /// Baseline ring thickness in CSS px (`1` / `2`).
    #[must_use]
    pub fn ring_px(self) -> f32 {
        match self {
            Self::Button => 1.0,
            Self::Circle => 2.0,
        }
    }

    /// Baseline shader sampling scale (`PILL_SHADER_SCALE` / `CIRCLE_SHADER_SCALE`).
    #[must_use]
    pub fn shader_scale(self) -> f32 {
        match self {
            Self::Button => 1.6,
            Self::Circle => 1.3,
        }
    }

    pub(crate) fn is_circle(self) -> bool {
        matches!(self, Self::Circle)
    }
}

/// A point displacement applied to outline points.
pub type Deform<'a> = &'a dyn Fn(f32, f32) -> [f32; 2];

/// Samples a rounded rect clockwise from the end of the top-left corner:
/// edges every ~1.5 CSS px, 14 segments per corner arc. `deform` (optional)
/// displaces each point.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::round_rect_outline;
///
/// let pts = round_rect_outline(0.0, 0.0, 140.0, 40.0, 20.0, None);
/// assert_eq!(pts[0], [20.0, 0.0]);
/// // All points stay inside the box.
/// assert!(pts.iter().all(|p| (0.0..=140.0).contains(&p[0]) && (0.0..=40.0).contains(&p[1])));
/// ```
#[must_use]
pub fn round_rect_outline(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    deform: Option<Deform<'_>>,
) -> Vec<[f32; 2]> {
    let r = r.min(w.min(h) / 2.0).max(0.0);
    let mut pts = Vec::with_capacity(
        4 * (ARC_SEGMENTS + 1) + ((2.0 * (w + h)) / EDGE_STEP).ceil() as usize + 8,
    );
    let mut push = |px: f32, py: f32| {
        pts.push(match deform {
            Some(f) => f(px, py),
            None => [px, py],
        });
    };
    let edge = |push: &mut dyn FnMut(f32, f32), x0: f32, y0: f32, x1: f32, y1: f32| {
        let len = (x1 - x0).hypot(y1 - y0);
        let k = ((len / EDGE_STEP).ceil() as usize).max(1);
        for i in 0..k {
            let t = i as f32 / k as f32;
            push(x0 + (x1 - x0) * t, y0 + (y1 - y0) * t);
        }
    };
    let arc = |push: &mut dyn FnMut(f32, f32), cx: f32, cy: f32, a0: f32, a1: f32| {
        for i in 0..=ARC_SEGMENTS {
            let a = a0 + (a1 - a0) * (i as f32 / ARC_SEGMENTS as f32);
            push(cx + r * a.cos(), cy + r * a.sin());
        }
    };
    edge(&mut push, x + r, y, x + w - r, y);
    arc(&mut push, x + w - r, y + r, -FRAC_PI_2, 0.0);
    edge(&mut push, x + w, y + r, x + w, y + h - r);
    arc(&mut push, x + w - r, y + h - r, 0.0, FRAC_PI_2);
    edge(&mut push, x + w - r, y + h, x + r, y + h);
    arc(&mut push, x + r, y + h - r, FRAC_PI_2, PI);
    edge(&mut push, x, y + h - r, x, y + r);
    arc(&mut push, x + r, y + r, PI, 1.5 * PI);
    pts
}

/// Perimeter of a rounded rectangle.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::rr_perimeter;
///
/// // The reference pill: 2·(140−40) + 2π·20.
/// let p = rr_perimeter(140.0, 40.0, 20.0);
/// assert!((p - (200.0 + 2.0 * std::f32::consts::PI * 20.0)).abs() < 1e-4);
/// ```
#[must_use]
pub fn rr_perimeter(w: f32, h: f32, r: f32) -> f32 {
    let rr = r.min(w.min(h) / 2.0).max(0.0);
    2.0 * (w - 2.0 * rr).max(0.0) + 2.0 * (h - 2.0 * rr).max(0.0) + TAU * rr
}

/// Perimeter of the shape: the circle's circumference for
/// [`MetalFxVariant::Circle`], else [`rr_perimeter`].
#[must_use]
pub fn shape_perimeter(w: f32, h: f32, r: f32, kind: MetalFxVariant) -> f32 {
    match kind {
        MetalFxVariant::Circle => TAU * r.min(w.min(h) / 2.0).max(0.0),
        MetalFxVariant::Button => rr_perimeter(w, h, r),
    }
}

/// The point at arc length `s` along the outline, inset by `inset` and pushed
/// out by `outward` (`sampleAtArc`). The arc starts at the top-left corner
/// arc's end and runs clockwise; `s` wraps.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::{sample_at_arc, MetalFxVariant};
///
/// let p = sample_at_arc(0.0, 140.0, 40.0, 20.0, 1.5, 0.0, MetalFxVariant::Button);
/// assert_eq!(p, [20.0, 1.5]);
/// ```
#[must_use]
pub fn sample_at_arc(
    s: f32,
    w: f32,
    h: f32,
    r: f32,
    inset: f32,
    outward: f32,
    kind: MetalFxVariant,
) -> [f32; 2] {
    let rr = r.min(w.min(h) / 2.0).max(0.0);
    if kind == MetalFxVariant::Circle {
        let perim = TAU * rr;
        if perim <= 0.0001 {
            return [w * 0.5, h * 0.5];
        }
        let s = s.rem_euclid(perim);
        let theta = -FRAC_PI_2 + (s / perim) * TAU;
        let rad = (rr - inset + outward).max(0.0);
        return [w * 0.5 + rad * theta.cos(), h * 0.5 + rad * theta.sin()];
    }
    let top = (w - 2.0 * rr).max(0.0);
    let side = (h - 2.0 * rr).max(0.0);
    let arc_len = (PI * rr) / 2.0;
    let perim = 2.0 * (top + side) + 4.0 * arc_len;
    let mut d = s.rem_euclid(perim);
    let rad = (rr - inset + outward).max(0.0);
    let frac = |d: f32| if arc_len > 0.0 { d / arc_len } else { 0.0 };
    if d < top {
        return [rr + d, inset - outward];
    }
    d -= top;
    if d < arc_len {
        let t = -FRAC_PI_2 + frac(d) * FRAC_PI_2;
        return [(w - rr) + rad * t.cos(), rr + rad * t.sin()];
    }
    d -= arc_len;
    if d < side {
        return [w - inset + outward, rr + d];
    }
    d -= side;
    if d < arc_len {
        let t = frac(d) * FRAC_PI_2;
        return [(w - rr) + rad * t.cos(), (h - rr) + rad * t.sin()];
    }
    d -= arc_len;
    if d < top {
        return [w - rr - d, h - inset + outward];
    }
    d -= top;
    if d < arc_len {
        let t = FRAC_PI_2 + frac(d) * FRAC_PI_2;
        return [rr + rad * t.cos(), (h - rr) + rad * t.sin()];
    }
    d -= arc_len;
    if d < side {
        return [inset - outward, h - rr - d];
    }
    d -= side;
    let t = PI + frac(d) * FRAC_PI_2;
    [rr + rad * t.cos(), rr + rad * t.sin()]
}

/// Inverse of [`sample_at_arc`] at inset 0: the arc-length position nearest a
/// box-local point (`arcAtPoint`). Points off the outline project onto it.
#[must_use]
pub fn arc_at_point(x: f32, y: f32, w: f32, h: f32, r: f32, kind: MetalFxVariant) -> f32 {
    let rr = r.min(w.min(h) / 2.0).max(0.0);
    if kind == MetalFxVariant::Circle {
        let perim = TAU * rr;
        if perim <= 0.0001 {
            return 0.0;
        }
        let theta = (y - h / 2.0).atan2(x - w / 2.0);
        return (((theta + FRAC_PI_2) / TAU) * perim).rem_euclid(perim);
    }
    let top = (w - 2.0 * rr).max(0.0);
    let side = (h - 2.0 * rr).max(0.0);
    let arc_len = (PI * rr) / 2.0;
    let q = FRAC_PI_2;
    let s1 = top;
    let s2 = s1 + arc_len;
    let s3 = s2 + side;
    let s4 = s3 + arc_len;
    let s5 = s4 + top;
    let s6 = s5 + arc_len;
    let s7 = s6 + side;
    let in_x = x >= rr && x <= w - rr;
    let in_y = y >= rr && y <= h - rr;
    if in_x && in_y {
        let (dl, dr, dt, db) = (x, w - x, y, h - y);
        let m = dl.min(dr).min(dt).min(db);
        return if m == dt {
            x - rr
        } else if m == dr {
            s2 + (y - rr)
        } else if m == db {
            s4 + (w - rr - x)
        } else {
            s6 + (h - rr - y)
        };
    }
    if in_x {
        return if y < h / 2.0 {
            x - rr
        } else {
            s4 + (w - rr - x)
        };
    }
    if in_y {
        return if x > w / 2.0 {
            s2 + (y - rr)
        } else {
            s6 + (h - rr - y)
        };
    }
    if x > w / 2.0 && y < h / 2.0 {
        let t = (y - rr).atan2(x - (w - rr));
        return s1 + ((t + q) / q) * arc_len;
    }
    if x > w / 2.0 {
        let t = (y - (h - rr)).atan2(x - (w - rr));
        return s3 + (t / q) * arc_len;
    }
    if y > h / 2.0 {
        let t = (y - (h - rr)).atan2(x - rr);
        return s5 + ((t - q) / q) * arc_len;
    }
    let t = (y - rr).atan2(x - rr);
    s7 + ((t + PI) / q) * arc_len
}

/// Tangent angle of the outline at arc length `s` (`tangentAngleAtArc`).
#[must_use]
pub fn tangent_angle_at_arc(
    s: f32,
    w: f32,
    h: f32,
    r: f32,
    inset: f32,
    kind: MetalFxVariant,
) -> f32 {
    let eps = 0.1;
    let a = sample_at_arc(s - eps, w, h, r, inset, 0.0, kind);
    let b = sample_at_arc(s + eps, w, h, r, inset, 0.0, kind);
    (b[1] - a[1]).atan2(b[0] - a[0])
}

/// GLSL `smoothstep` with the reference's degenerate-edge handling.
#[must_use]
pub fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    if a == b {
        return if x < a { 0.0 } else { 1.0 };
    }
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_baselines() {
        assert_eq!(MetalFxVariant::Button.ring_px(), 1.0);
        assert_eq!(MetalFxVariant::Circle.ring_px(), 2.0);
        assert_eq!(MetalFxVariant::Button.shader_scale(), 1.6);
        assert_eq!(MetalFxVariant::Circle.shader_scale(), 1.3);
    }

    #[test]
    fn outline_point_counts_follow_the_reference() {
        let pts = round_rect_outline(0.0, 0.0, 140.0, 40.0, 20.0, None);
        // 2 straight 100px edges -> ceil(100/1.5)=67 points each; the 20px
        // side edges have zero length -> 1 point each; 4 arcs of 15 points.
        assert_eq!(pts.len(), 67 * 2 + 2 + 4 * 15);
        let circle = round_rect_outline(0.0, 0.0, 32.0, 32.0, 16.0, None);
        assert_eq!(circle.len(), 4 + 4 * 15);
    }

    #[test]
    fn outline_applies_the_deform() {
        let shifted =
            round_rect_outline(0.0, 0.0, 40.0, 40.0, 8.0, Some(&|x, y| [x + 3.0, y - 1.0]));
        let plain = round_rect_outline(0.0, 0.0, 40.0, 40.0, 8.0, None);
        for (a, b) in shifted.iter().zip(&plain) {
            assert_eq!(*a, [b[0] + 3.0, b[1] - 1.0]);
        }
    }

    #[test]
    fn outline_clamps_the_radius() {
        let pts = round_rect_outline(0.0, 0.0, 30.0, 10.0, 99.0, None);
        assert!(pts.iter().all(|p| p[1] >= -1e-4 && p[1] <= 10.0 + 1e-4));
    }

    #[test]
    fn perimeters() {
        assert!((rr_perimeter(140.0, 40.0, 20.0) - (200.0 + TAU * 20.0)).abs() < 1e-3);
        assert!(
            (shape_perimeter(32.0, 32.0, 16.0, MetalFxVariant::Circle) - TAU * 16.0).abs() < 1e-4
        );
        // Over-large radius clamps to a stadium/circle.
        assert!((rr_perimeter(40.0, 40.0, 100.0) - TAU * 20.0).abs() < 1e-3);
    }

    #[test]
    fn arc_sampling_walks_the_outline() {
        let (w, h, r) = (140.0, 40.0, 20.0);
        // Top edge start, then each quarter boundary.
        assert_eq!(
            sample_at_arc(0.0, w, h, r, 0.0, 0.0, MetalFxVariant::Button),
            [20.0, 0.0]
        );
        let top = w - 2.0 * r;
        let arc = PI * r / 2.0;
        let right = sample_at_arc(top + arc, w, h, r, 0.0, 0.0, MetalFxVariant::Button);
        assert!(
            (right[0] - w).abs() < 1e-3 && (right[1] - r).abs() < 1e-3,
            "{right:?}"
        );
        let bottom = sample_at_arc(top + 2.0 * arc, w, h, r, 0.0, 0.0, MetalFxVariant::Button);
        assert!(
            (bottom[0] - (w - r)).abs() < 1e-3 && (bottom[1] - h).abs() < 1e-3,
            "{bottom:?}"
        );
        // Wraps.
        let perim = rr_perimeter(w, h, r);
        let a = sample_at_arc(10.0, w, h, r, 1.5, 0.0, MetalFxVariant::Button);
        let b = sample_at_arc(10.0 + perim, w, h, r, 1.5, 0.0, MetalFxVariant::Button);
        assert!((a[0] - b[0]).abs() < 1e-3 && (a[1] - b[1]).abs() < 1e-3);
        // Inset moves the point inward, outward moves it out.
        let inward = sample_at_arc(30.0, w, h, r, 3.0, 0.0, MetalFxVariant::Button);
        let outward = sample_at_arc(30.0, w, h, r, 0.0, 2.0, MetalFxVariant::Button);
        assert_eq!(inward[1], 3.0);
        assert_eq!(outward[1], -2.0);
    }

    #[test]
    fn circle_arc_starts_at_the_top() {
        let p = sample_at_arc(0.0, 32.0, 32.0, 16.0, 0.0, 0.0, MetalFxVariant::Circle);
        assert!((p[0] - 16.0).abs() < 1e-4 && p[1].abs() < 1e-4, "{p:?}");
        let q = sample_at_arc(
            TAU * 16.0 / 4.0,
            32.0,
            32.0,
            16.0,
            0.0,
            0.0,
            MetalFxVariant::Circle,
        );
        assert!(
            (q[0] - 32.0).abs() < 1e-3 && (q[1] - 16.0).abs() < 1e-3,
            "{q:?}"
        );
    }

    #[test]
    fn arc_at_point_inverts_sample_at_arc() {
        for kind in [MetalFxVariant::Button, MetalFxVariant::Circle] {
            let (w, h, r) = if kind == MetalFxVariant::Circle {
                (40.0, 40.0, 20.0)
            } else {
                (140.0, 40.0, 20.0)
            };
            let perim = shape_perimeter(w, h, r, kind);
            for i in 0..60 {
                let s = perim * (i as f32 + 0.37) / 60.0;
                let p = sample_at_arc(s, w, h, r, 0.0, 0.0, kind);
                let back = arc_at_point(p[0], p[1], w, h, r, kind);
                let d = (back - s).rem_euclid(perim);
                let d = d.min(perim - d);
                assert!(d < 0.35, "{kind:?} s={s} back={back} p={p:?}");
            }
        }
    }

    #[test]
    fn tangent_runs_clockwise() {
        let t = tangent_angle_at_arc(30.0, 140.0, 40.0, 20.0, 1.5, MetalFxVariant::Button);
        assert!(t.abs() < 1e-3, "top edge heads +x");
        let side = rr_perimeter(140.0, 40.0, 20.0) / 2.0;
        let _ = side;
        let down = tangent_angle_at_arc(
            100.0 + 20.0 * FRAC_PI_2 + 1.0,
            140.0,
            40.0,
            20.0,
            1.5,
            MetalFxVariant::Button,
        );
        assert!(down.is_finite());
    }

    #[test]
    fn smoothstep_handles_degenerate_edges() {
        assert_eq!(smoothstep(1.0, 1.0, 0.5), 0.0);
        assert_eq!(smoothstep(1.0, 1.0, 1.5), 1.0);
        assert_eq!(smoothstep(0.0, 1.0, 0.5), 0.5);
    }
}
