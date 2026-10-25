//! A small software painter for finished [`OrbFrame`]s.
//!
//! The reference draws with Canvas2D, whose anti-aliasing is analytic: a dot
//! that covers 20% of a pixel contributes 20% alpha, and the many stacked,
//! translucent dots of the trail modes keep adding up. GPU multisampling
//! cannot reproduce that: coverage is quantised to a few samples and stacked
//! copies of one dot hit the same samples, so trails read thin and dim.
//! Orbs are tiny, so painting them on the CPU is cheap and gives the same
//! weight as the reference.

use iced::Color;

use super::frame::OrbFrame;
use super::paint::{ink_rgb, widen};

/// Sub-samples per pixel edge when measuring coverage.
const SUBSAMPLES: usize = 8;

/// An RGBA bitmap with straight alpha, `size * density` pixels square.
pub(crate) struct Bitmap {
    pub(crate) side: u32,
    pub(crate) rgba: Vec<u8>,
}

/// Paints `frame` (in logical units, `size` square) at `density` pixels per
/// unit. Edges go first, then nodes, source-over, in sRGB like a canvas.
pub(crate) fn render(
    frame: &OrbFrame,
    size: f32,
    density: f32,
    dark: bool,
    tint: Option<Color>,
) -> Bitmap {
    let side = (size * density).round().max(1.0) as usize;
    let mut acc = vec![[0.0_f32; 4]; side * side]; // premultiplied r, g, b, a
    let k = side as f32 / size;

    for line in frame.lines() {
        let (x1, y1) = line.start();
        let (x2, y2) = line.end();
        let (rgb, alpha) = ink_rgb(line.white(), line.alpha(), dark, tint);
        stroke(
            &mut acc,
            side,
            Segment {
                x1: x1 as f32 * k,
                y1: y1 as f32 * k,
                x2: x2 as f32 * k,
                y2: y2 as f32 * k,
                half: (line.width() as f32 * k / 2.0).max(0.0),
            },
            rgb,
            alpha,
        );
    }
    for dot in frame.dots() {
        let (rgb, alpha) = ink_rgb(dot.white(), dot.alpha(), dark, tint);
        let r = widen(dot.r() as f32 * k);
        fill_circle(
            &mut acc,
            side,
            dot.x() as f32 * k,
            dot.y() as f32 * k,
            r,
            rgb,
            alpha,
        );
    }

    let mut rgba = Vec::with_capacity(side * side * 4);
    for px in &acc {
        let a = px[3];
        if a <= 0.0 {
            rgba.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            for c in &px[..3] {
                rgba.push(((c / a) * 255.0).round().clamp(0.0, 255.0) as u8);
            }
            rgba.push((a * 255.0).round().clamp(0.0, 255.0) as u8);
        }
    }
    Bitmap {
        side: side as u32,
        rgba,
    }
}

/// Source-over of `rgb` at `alpha * coverage` onto one pixel.
fn blend(px: &mut [f32; 4], rgb: [f32; 3], alpha: f32, coverage: f32) {
    let a = alpha * coverage;
    if a <= 0.0 {
        return;
    }
    for (dst, src) in px.iter_mut().zip(rgb) {
        *dst = src * a + *dst * (1.0 - a);
    }
    px[3] = a + px[3] * (1.0 - a);
}

/// Pixel range `lo..hi` clipped to the bitmap.
fn span(lo: f32, hi: f32, side: usize) -> std::ops::Range<usize> {
    let start = lo.floor().max(0.0) as usize;
    let end = (hi.ceil().max(0.0) as usize).min(side);
    start.min(side)..end
}

/// Fraction of the pixel at `(px, py)` for which `inside` holds, by an even
/// sub-sample grid.
fn coverage(px: usize, py: usize, inside: impl Fn(f32, f32) -> bool) -> f32 {
    let mut hits = 0;
    for sy in 0..SUBSAMPLES {
        for sx in 0..SUBSAMPLES {
            let x = px as f32 + (sx as f32 + 0.5) / SUBSAMPLES as f32;
            let y = py as f32 + (sy as f32 + 0.5) / SUBSAMPLES as f32;
            if inside(x, y) {
                hits += 1;
            }
        }
    }
    hits as f32 / (SUBSAMPLES * SUBSAMPLES) as f32
}

fn fill_circle(
    acc: &mut [[f32; 4]],
    side: usize,
    cx: f32,
    cy: f32,
    r: f32,
    rgb: [f32; 3],
    alpha: f32,
) {
    let r2 = r * r;
    for py in span(cy - r, cy + r, side) {
        for px in span(cx - r, cx + r, side) {
            let c = coverage(px, py, |x, y| {
                let (dx, dy) = (x - cx, y - cy);
                dx * dx + dy * dy <= r2
            });
            blend(&mut acc[py * side + px], rgb, alpha, c);
        }
    }
}

struct Segment {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    half: f32,
}

/// A butt-capped stroke: the rectangle `half` either side of the segment.
fn stroke(acc: &mut [[f32; 4]], side: usize, seg: Segment, rgb: [f32; 3], alpha: f32) {
    let (dx, dy) = (seg.x2 - seg.x1, seg.y2 - seg.y1);
    let len = dx.hypot(dy);
    if len < 1e-6 {
        return;
    }
    let (ux, uy) = (dx / len, dy / len);
    let pad = seg.half + 1.0;
    let (lo_x, hi_x) = (seg.x1.min(seg.x2) - pad, seg.x1.max(seg.x2) + pad);
    let (lo_y, hi_y) = (seg.y1.min(seg.y2) - pad, seg.y1.max(seg.y2) + pad);
    for py in span(lo_y, hi_y, side) {
        for px in span(lo_x, hi_x, side) {
            let c = coverage(px, py, |x, y| {
                let (rx, ry) = (x - seg.x1, y - seg.y1);
                let along = rx * ux + ry * uy;
                let across = -rx * uy + ry * ux;
                (0.0..=len).contains(&along) && across.abs() <= seg.half
            });
            blend(&mut acc[py * side + px], rgb, alpha, c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::thinking_orb::Dot;

    fn one_dot(r: f64, alpha: f64) -> OrbFrame {
        OrbFrame::new(
            vec![Dot::new(5.5, 5.5, 0.0, r, 0.0).with_alpha(alpha)],
            vec![],
        )
    }

    #[test]
    fn a_big_dot_is_solid_in_the_middle_and_clear_outside() {
        let bmp = render(&one_dot(3.0, 1.0), 12.0, 1.0, false, None);
        let at = |x: usize, y: usize| bmp.rgba[(y * 12 + x) * 4 + 3];
        assert_eq!(at(5, 5), 255);
        assert_eq!(at(0, 0), 0);
    }

    #[test]
    fn stacked_translucent_dots_accumulate() {
        let single = render(&one_dot(0.45, 0.6), 12.0, 1.0, false, None);
        let frame = OrbFrame::new(
            (0..8)
                .map(|_| Dot::new(5.5, 5.5, 0.0, 0.45, 0.0).with_alpha(0.6))
                .collect(),
            vec![],
        );
        let stacked = render(&frame, 12.0, 1.0, false, None);
        let sum = |b: &Bitmap| b.rgba.chunks(4).map(|p| u32::from(p[3])).sum::<u32>();
        assert!(sum(&stacked) > sum(&single) * 2);
    }

    #[test]
    fn density_scales_the_bitmap() {
        let bmp = render(&one_dot(1.0, 1.0), 20.0, 2.0, false, None);
        assert_eq!(bmp.side, 40);
        assert_eq!(bmp.rgba.len(), 40 * 40 * 4);
    }
}
