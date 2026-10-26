//! Thin drawing helpers over `tiny-skia`.
//!
//! The renderer composes a frame as a `Pixmap`, which is then handed to iced
//! as an image. This module keeps the small affine type the port's maths was
//! written in, plus the handful of wrappers used all over the renderer:
//! filling and stroking a path, building a coverage mask, blitting a sprite,
//! and converting one of the crate's [`Rgba`] colours into a paint.

use tiny_skia::{
    FillRule, Mask, Paint, Path, Pixmap, PixmapPaint, PixmapRef, Stroke, Transform,
};

use super::color::Rgba;

/// A 2×3 affine matrix in the same layout the reference uses:
/// `x' = a·x + c·y + e`, `y' = b·x + d·y + f`.
///
/// Composition is [`then`](Self::then): `a.then(b)` applies `a` first, as
/// the reference's `mulAffine(a, b)` does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Affine(pub [f32; 6]);

impl Affine {
    /// The identity.
    pub(crate) const IDENTITY: Self = Self([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    /// A translation.
    pub(crate) const fn translate(x: f32, y: f32) -> Self {
        Self([1.0, 0.0, 0.0, 1.0, x, y])
    }

    /// A scale.
    pub(crate) const fn scale(x: f32, y: f32) -> Self {
        Self([x, 0.0, 0.0, y, 0.0, 0.0])
    }

    /// A rotation, in radians.
    pub(crate) fn rotate(radians: f32) -> Self {
        let (s, c) = radians.sin_cos();
        Self([c, s, -s, c, 0.0, 0.0])
    }

    /// Apply `self`, then `next`.
    #[must_use]
    pub(crate) fn then(self, next: Self) -> Self {
        let a = self.0;
        let b = next.0;
        Self([
            b[0] * a[0] + b[2] * a[1],
            b[1] * a[0] + b[3] * a[1],
            b[0] * a[2] + b[2] * a[3],
            b[1] * a[2] + b[3] * a[3],
            b[0] * a[4] + b[2] * a[5] + b[4],
            b[1] * a[4] + b[3] * a[5] + b[5],
        ])
    }

    /// Transform a point.
    #[must_use]
    pub(crate) fn point(self, x: f32, y: f32) -> (f32, f32) {
        let m = self.0;
        (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
    }

    /// The inverse, or the identity for a singular matrix.
    #[must_use]
    pub(crate) fn inverse(self) -> Self {
        let m = self.0;
        let det = m[0] * m[3] - m[1] * m[2];
        if det.abs() < 1e-9 {
            return Self::IDENTITY;
        }
        let a = m[3] / det;
        let b = -m[1] / det;
        let c = -m[2] / det;
        let d = m[0] / det;
        Self([a, b, c, d, -(m[4] * a + m[5] * c), -(m[4] * b + m[5] * d)])
    }

    /// The same matrix for `tiny-skia`.
    #[must_use]
    pub(crate) fn transform(self) -> Transform {
        let m = self.0;
        Transform::from_row(m[0], m[1], m[2], m[3], m[4], m[5])
    }
}

/// Compose two affines, applying `a` first. A free function to mirror the
/// reference's `mulAffine` at call sites that read like the port.
#[must_use]
pub(crate) fn mul(a: Affine, b: Affine) -> Affine {
    a.then(b)
}

/// A paint of one opaque colour.
#[must_use]
pub(crate) fn paint(color: Rgba) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.r, color.g, color.b, color.a);
    paint.anti_alias = true;
    paint
}

/// A paint of one colour with an explicit alpha multiplier (0–1).
#[must_use]
pub(crate) fn paint_alpha(color: Rgba, alpha: f32) -> Paint<'static> {
    let a = (f32::from(color.a) / 255.0 * alpha).clamp(0.0, 1.0);
    paint(color.with_alpha(a))
}

/// Fill a path.
pub(crate) fn fill(
    pixmap: &mut Pixmap,
    path: &Path,
    paint: &Paint<'_>,
    transform: Affine,
    mask: Option<&Mask>,
) {
    pixmap.fill_path(
        path,
        paint,
        FillRule::Winding,
        transform.transform(),
        mask,
    );
}

/// Stroke a path with round caps and joins, the reference's default.
pub(crate) fn stroke(
    pixmap: &mut Pixmap,
    path: &Path,
    paint: &Paint<'_>,
    width: f32,
    transform: Affine,
    mask: Option<&Mask>,
) {
    let stroke = Stroke {
        width,
        line_cap: tiny_skia::LineCap::Round,
        line_join: tiny_skia::LineJoin::Round,
        ..Stroke::default()
    };
    pixmap.stroke_path(path, paint, &stroke, transform.transform(), mask);
}


/// Stroke a path with butt caps and round joins, for a run of segment arcs
/// that should not grow past its ends.
pub(crate) fn stroke_butt(
    pixmap: &mut Pixmap,
    path: &Path,
    paint: &Paint<'_>,
    width: f32,
    transform: Affine,
    mask: Option<&Mask>,
) {
    let stroke = Stroke {
        width,
        line_cap: tiny_skia::LineCap::Butt,
        line_join: tiny_skia::LineJoin::Round,
        ..Stroke::default()
    };
    pixmap.stroke_path(path, paint, &stroke, transform.transform(), mask);
}

/// A fresh coverage mask the size of the pixmap.
#[must_use]
pub(crate) fn mask(pixmap: &Pixmap) -> Mask {
    Mask::new(pixmap.width(), pixmap.height()).expect("non-zero canvas")
}

/// Fill a path into a mask.
pub(crate) fn mask_fill(mask: &mut Mask, path: &Path, transform: Affine) {
    mask.fill_path(path, FillRule::Winding, true, transform.transform());
}

/// Blit a sprite, optionally through a mask.
pub(crate) fn blit(
    pixmap: &mut Pixmap,
    sprite: PixmapRef<'_>,
    transform: Affine,
    quality: tiny_skia::FilterQuality,
    mask: Option<&Mask>,
) {
    let paint = PixmapPaint {
        opacity: 1.0,
        blend_mode: tiny_skia::BlendMode::SourceOver,
        quality,
    };
    pixmap.draw_pixmap(0, 0, sprite, &paint, transform.transform(), mask);
}

/// One rounded rectangle path.
#[must_use]
pub(crate) fn round_rect(x: f32, y: f32, w: f32, h: f32, radius: f32) -> Option<Path> {
    let r = radius.max(0.0).min(w.abs() / 2.0).min(h.abs() / 2.0);
    let mut b = tiny_skia::PathBuilder::new();
    if r <= 0.0 {
        b.push_rect(tiny_skia::Rect::from_xywh(x, y, w, h)?);
    } else {
        let k = r * 0.552_284_8;
        b.move_to(x + r, y);
        b.line_to(x + w - r, y);
        b.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
        b.line_to(x + w, y + h - r);
        b.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);
        b.line_to(x + r, y + h);
        b.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);
        b.line_to(x, y + r);
        b.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
        b.close();
    }
    b.finish()
}

/// One ellipse path centred on `(cx, cy)`.
#[must_use]
pub(crate) fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> Option<Path> {
    let mut b = tiny_skia::PathBuilder::new();
    let kx = rx * 0.552_284_8;
    let ky = ry * 0.552_284_8;
    b.move_to(cx + rx, cy);
    b.cubic_to(cx + rx, cy + ky, cx + kx, cy + ry, cx, cy + ry);
    b.cubic_to(cx - kx, cy + ry, cx - rx, cy + ky, cx - rx, cy);
    b.cubic_to(cx - rx, cy - ky, cx - kx, cy - ry, cx, cy - ry);
    b.cubic_to(cx + kx, cy - ry, cx + rx, cy - ky, cx + rx, cy);
    b.close();
    b.finish()
}

/// One polygon path from `(x, y)` pairs.
#[must_use]
pub(crate) fn polygon(points: &[(f32, f32)]) -> Option<Path> {
    if points.len() < 2 {
        return None;
    }
    let mut b = tiny_skia::PathBuilder::new();
    b.move_to(points[0].0, points[0].1);
    for &(x, y) in &points[1..] {
        b.line_to(x, y);
    }
    b.close();
    b.finish()
}

/// The coverage of a path as a fresh mask.
#[must_use]
pub(crate) fn path_mask(pixmap: &Pixmap, path: &Path, transform: Affine) -> Mask {
    let mut m = mask(pixmap);
    mask_fill(&mut m, path, transform);
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affine_composes_in_application_order() {
        let t = Affine::translate(10.0, 0.0).then(Affine::scale(2.0, 2.0));
        assert_eq!(t.point(1.0, 1.0), (22.0, 2.0));
        let back = t.inverse();
        let (x, y) = back.point(22.0, 2.0);
        assert!((x - 1.0).abs() < 1e-4 && (y - 1.0).abs() < 1e-4);
    }

    #[test]
    fn mask_fill_accumulates_subpaths() {
        let mut pixmap = Pixmap::new(16, 16).unwrap();
        let a = polygon(&[(2.0, 2.0), (6.0, 2.0), (6.0, 6.0), (2.0, 6.0)]).unwrap();
        let b = polygon(&[(10.0, 10.0), (14.0, 10.0), (14.0, 14.0), (10.0, 14.0)]).unwrap();
        let mut m = mask(&pixmap);
        mask_fill(&mut m, &a, Affine::IDENTITY);
        mask_fill(&mut m, &b, Affine::IDENTITY);
        let at = |x: usize, y: usize| m.data()[y * 16 + x];
        assert!(at(4, 4) > 200);
        assert!(at(12, 12) > 200);
        assert_eq!(at(8, 8), 0);
        let p = paint(Rgba::from_rgb(255, 0, 0));
        fill(&mut pixmap, &a, &p, Affine::IDENTITY, Some(&m));
        assert!(pixmap.pixel(4, 4).unwrap().red() > 200);
        assert_eq!(pixmap.pixel(12, 12).unwrap().alpha(), 0);
    }

    #[test]
    fn round_rect_and_ellipse_finish() {
        assert!(round_rect(0.0, 0.0, 10.0, 8.0, 2.0).is_some());
        assert!(ellipse(5.0, 5.0, 4.0, 3.0).is_some());
        assert!(polygon(&[(0.0, 0.0)]).is_none());
    }
}
