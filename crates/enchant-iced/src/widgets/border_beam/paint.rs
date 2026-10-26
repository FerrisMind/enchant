//! The float-plane primitives used by the software rasterizer: CSS radial and
//! conic gradients, rounded-rect distances and the
//! `hue-rotate() brightness() saturate()` filter chain.
//!
//! The plane/filter code follows the proven implementation in
//! `widgets::voice_glow::paint`; it is duplicated here so the two widgets can
//! evolve independently.

/// Premultiplied RGBA.
pub(crate) type Px = [f32; 4];

/// A float plane of premultiplied pixels with the sampling grid it lives on.
#[derive(Clone, Debug, Default)]
pub(crate) struct Plane {
    pub w: usize,
    pub h: usize,
    /// Pixels per host CSS px, per axis.
    pub kx: f32,
    pub ky: f32,
    pub px: Vec<Px>,
    /// Host-space position of the plane's top-left corner.
    pub ox: f32,
    pub oy: f32,
}

impl Plane {
    /// A transparent `w x h` plane covering a `host_w x host_h` box.
    pub(crate) fn reset(&mut self, w: usize, h: usize, host_w: f32, host_h: f32) {
        self.w = w;
        self.h = h;
        self.kx = if host_w > 0.0 { w as f32 / host_w } else { 1.0 };
        self.ky = if host_h > 0.0 { h as f32 / host_h } else { 1.0 };
        self.ox = 0.0;
        self.oy = 0.0;
        self.px.clear();
        self.px.resize(w * h, [0.0; 4]);
    }

    /// A transparent plane for a `w x h` box with `pad_x`/`pad_y` extra pixels
    /// on every side, so gradients and blurs keep the mass that spills past
    /// the box edge (CSS filters blur the whole shape, not the cropped part).
    pub(crate) fn reset_padded(
        &mut self,
        (w, h): (usize, usize),
        host: (f32, f32),
        (pad_x, pad_y): (usize, usize),
    ) {
        self.kx = if host.0 > 0.0 { w as f32 / host.0 } else { 1.0 };
        self.ky = if host.1 > 0.0 { h as f32 / host.1 } else { 1.0 };
        self.w = w + 2 * pad_x;
        self.h = h + 2 * pad_y;
        self.ox = -(pad_x as f32) / self.kx;
        self.oy = -(pad_y as f32) / self.ky;
        self.px.clear();
        self.px.resize(self.w * self.h, [0.0; 4]);
    }

    /// Host-space x of the centre of pixel column `i`.
    #[inline]
    pub(crate) fn x(&self, i: usize) -> f32 {
        self.ox + (i as f32 + 0.5) / self.kx
    }

    /// Host-space y of the centre of pixel row `j`.
    #[inline]
    pub(crate) fn y(&self, j: usize) -> f32 {
        self.oy + (j as f32 + 0.5) / self.ky
    }
}

/// Premultiplied source-over of `src` onto `dst`.
#[inline]
pub(crate) fn over(dst: &mut Px, src: Px) {
    let k = 1.0 - src[3];
    for c in 0..4 {
        dst[c] = src[c] + dst[c] * k;
    }
}

/// One colour stop of a gradient.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GradientStop {
    pub at: f32,
    pub color: RgbaLike,
}

/// A straight RGBA colour for gradient stops.
pub(crate) type RgbaLike = super::palette::Rgba;

/// Paints `radial-gradient(ellipse rx ry at (cx, cy), stops...)`.
///
/// `cx`/`cy`/`rx`/`ry` are logical pixels in the plane's host coordinates.
/// Colours are interpolated premultiplied, like CSS; beyond the last stop the
/// last colour is extended.
pub(crate) fn paint_radial(
    plane: &mut Plane,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    stops: &[GradientStop],
) {
    if rx <= 0.0 || ry <= 0.0 || stops.is_empty() {
        return;
    }
    let (kx, ky) = (plane.kx, plane.ky);
    let (ox, oy) = (plane.ox, plane.oy);
    let x0 = (((cx - rx - ox) * kx).floor().max(0.0)) as usize;
    let x1 = (((cx + rx - ox) * kx).ceil().max(0.0) as usize).min(plane.w);
    let y0 = (((cy - ry - oy) * ky).floor().max(0.0)) as usize;
    let y1 = (((cy + ry - oy) * ky).ceil().max(0.0) as usize).min(plane.h);
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    let (inv_rx, inv_ry) = (1.0 / rx, 1.0 / ry);
    for j in y0..y1 {
        let ny = (plane.y(j) - cy) * inv_ry;
        for i in x0..x1 {
            let nx = (plane.x(i) - cx) * inv_rx;
            let d = (nx * nx + ny * ny).sqrt();
            let c = sample_stops(stops, d);
            if c[3] > 0.0 {
                over(&mut plane.px[j * plane.w + i], c);
            }
        }
    }
}

/// Paints `conic-gradient(from <angle>, stops...)` centred on `(cx, cy)`.
pub(crate) fn paint_conic(
    plane: &mut Plane,
    cx: f32,
    cy: f32,
    angle_degrees: f32,
    stops: &[GradientStop],
) {
    if stops.is_empty() {
        return;
    }
    let angle = angle_degrees.rem_euclid(360.0);
    for j in 0..plane.h {
        let dy = plane.y(j) - cy;
        for i in 0..plane.w {
            let dx = plane.x(i) - cx;
            let deg = dx.atan2(-dy).to_degrees().rem_euclid(360.0);
            let p = ((deg - angle).rem_euclid(360.0)) / 360.0;
            let c = sample_stops(stops, p);
            if c[3] > 0.0 {
                over(&mut plane.px[j * plane.w + i], c);
            }
        }
    }
}

fn sample_stops(stops: &[GradientStop], d: f32) -> Px {
    let d = d.max(0.0);
    if d <= stops[0].at {
        return premul(stops[0].color);
    }
    for pair in stops.windows(2) {
        let a = pair[0];
        let b = pair[1];
        if d <= b.at {
            let t = if b.at > a.at {
                ((d - a.at) / (b.at - a.at)).clamp(0.0, 1.0)
            } else {
                1.0
            };
            return lerp_premul(a.color, b.color, t);
        }
    }
    premul(stops[stops.len() - 1].color)
}

#[inline]
fn premul(c: RgbaLike) -> Px {
    [c.r * c.a, c.g * c.a, c.b * c.a, c.a]
}

#[inline]
fn lerp_premul(a: RgbaLike, b: RgbaLike, t: f32) -> Px {
    let (a, b) = (premul(a), premul(b));
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

/// Signed distance from `(x, y)` to the rounded rectangle `[x0, x1] x
/// [y0, y1]` with corner radius `r` (negative inside).
#[inline]
pub(crate) fn sd_round_rect(x: f32, y: f32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> f32 {
    let (hx, hy) = ((x1 - x0) * 0.5, (y1 - y0) * 0.5);
    let r = r.min(hx).min(hy).max(0.0);
    let qx = (x - (x0 + x1) * 0.5).abs() - (hx - r);
    let qy = (y - (y0 + y1) * 0.5).abs() - (hy - r);
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    outside + qx.max(qy).min(0.0) - r
}

/// One device-pixel-wide analytic coverage for `sd` (negative inside).
#[inline]
pub(crate) fn coverage(sd: f32, k: f32) -> f32 {
    (0.5 - sd * k).clamp(0.0, 1.0)
}

/// The `hue-rotate() brightness() saturate()` chain with the clamps a browser
/// applies between stages, in `f32` for whole planes.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Fx {
    hue: [[f32; 3]; 3],
    brightness: f32,
    sat: [[f32; 3]; 3],
}

impl Fx {
    pub(crate) fn new(hue_degrees: f32, brightness: f32, saturation: f32) -> Self {
        let (sin, cos) = (hue_degrees.to_radians()).sin_cos();
        let hue = [
            [
                0.213 + cos * 0.787 - sin * 0.213,
                0.715 - cos * 0.715 - sin * 0.715,
                0.072 - cos * 0.072 + sin * 0.928,
            ],
            [
                0.213 - cos * 0.213 + sin * 0.143,
                0.715 + cos * 0.285 + sin * 0.140,
                0.072 - cos * 0.072 - sin * 0.283,
            ],
            [
                0.213 - cos * 0.213 - sin * 0.787,
                0.715 - cos * 0.715 + sin * 0.715,
                0.072 + cos * 0.928 + sin * 0.072,
            ],
        ];
        let s = saturation;
        let sat = [
            [0.213 + 0.787 * s, 0.715 - 0.715 * s, 0.072 - 0.072 * s],
            [0.213 - 0.213 * s, 0.715 + 0.285 * s, 0.072 - 0.072 * s],
            [0.213 - 0.213 * s, 0.715 - 0.715 * s, 0.072 + 0.928 * s],
        ];
        Self {
            hue,
            brightness,
            sat,
        }
    }

    #[inline]
    fn mul(m: &[[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
        [
            m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
            m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
            m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
        ]
    }

    /// Filters a straight colour.
    #[inline]
    pub(crate) fn apply(&self, c: [f32; 3]) -> [f32; 3] {
        let rotated = Self::mul(&self.hue, c).map(|v| v.clamp(0.0, 1.0));
        let lit = rotated.map(|v| (v * self.brightness).clamp(0.0, 1.0));
        Self::mul(&self.sat, lit).map(|v| v.clamp(0.0, 1.0))
    }

    /// Filters every pixel of a premultiplied plane.
    pub(crate) fn apply_plane(&self, px: &mut [Px]) {
        for p in px {
            if p[3] <= 1e-6 {
                continue;
            }
            let inv = 1.0 / p[3];
            let c = self.apply([p[0] * inv, p[1] * inv, p[2] * inv]);
            p[0] = c[0] * p[3];
            p[1] = c[1] * p[3];
            p[2] = c[2] * p[3];
        }
    }
}

/// Builds a conic-gradient stop.
pub(crate) const fn stop(at: f32, r: u8, g: u8, b: u8, a: f32) -> GradientStop {
    GradientStop {
        at,
        color: super::palette::rgba(r, g, b, a),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::border_beam::palette::rgba;

    #[test]
    fn radial_gradient_interpolates_premultiplied() {
        let mut plane = Plane::default();
        plane.reset(20, 20, 20.0, 20.0);
        paint_radial(
            &mut plane,
            10.0,
            10.0,
            10.0,
            10.0,
            &[
                GradientStop {
                    at: 0.0,
                    color: rgba(255, 0, 0, 1.0),
                },
                GradientStop {
                    at: 1.0,
                    color: rgba(255, 0, 0, 0.0),
                },
            ],
        );
        let centre = plane.px[10 * 20 + 10];
        assert!(centre[0] > 0.9);
        let edge = plane.px[10 * 20 + 19];
        assert!(edge[0] < 0.1, "{edge:?}");
    }

    #[test]
    fn rounded_rect_distance_signs() {
        assert!(sd_round_rect(50.0, 50.0, 0.0, 0.0, 100.0, 100.0, 16.0) < 0.0);
        assert!(sd_round_rect(150.0, 50.0, 0.0, 0.0, 100.0, 100.0, 16.0) > 0.0);
        assert!(sd_round_rect(0.5, 0.5, 0.0, 0.0, 100.0, 100.0, 16.0) > 0.0);
    }

    #[test]
    fn conic_starts_at_the_top_and_runs_clockwise() {
        let mut plane = Plane::default();
        plane.reset(100, 100, 100.0, 100.0);
        paint_conic(
            &mut plane,
            50.0,
            50.0,
            0.0,
            &[
                GradientStop {
                    at: 0.0,
                    color: rgba(255, 0, 0, 1.0),
                },
                GradientStop {
                    at: 0.5,
                    color: rgba(0, 255, 0, 1.0),
                },
                GradientStop {
                    at: 1.0,
                    color: rgba(0, 0, 255, 1.0),
                },
            ],
        );
        let top = plane.px[50];
        let right = plane.px[50 * 100 + 98];
        let bottom = plane.px[98 * 100 + 50];
        let left = plane.px[50 * 100 + 1];
        assert!(top[1] < 0.2 && top[0] > 0.8, "top {top:?}");
        assert!(
            (right[0] - 0.5).abs() < 0.05 && (right[1] - 0.5).abs() < 0.05,
            "right {right:?}"
        );
        assert!(bottom[1] > 0.8, "bottom {bottom:?}");
        assert!(
            (left[1] - 0.5).abs() < 0.05 && (left[2] - 0.5).abs() < 0.05,
            "left {left:?}"
        );
    }

    #[test]
    fn filter_matches_chromium_samples() {
        // Ground truth from Chromium, same table as voice_glow.
        let fx = Fx::new(20.78, 1.15, 1.2);
        let got = fx
            .apply([1.0, 70.0 / 255.0, 120.0 / 255.0])
            .map(|v| (v * 255.0).round() as i32);
        assert!((got[0] - 255).abs() <= 1, "{got:?}");
        assert!((got[1] - 82).abs() <= 1, "{got:?}");
        assert!((got[2] - 71).abs() <= 1, "{got:?}");
    }
}
