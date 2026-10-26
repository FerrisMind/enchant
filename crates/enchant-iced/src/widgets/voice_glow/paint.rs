//! The pixel primitives of the software painter: float planes, CSS-style
//! radial gradients, the ellipse masks, rounded-rect distances and the
//! `hue-rotate() brightness() saturate()` filter chain.

use super::config::LobeLayer;
use super::frame::{LobeFrame, VoiceGlowFrame};
use super::palette::ColorFilter;

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
    /// Host-space position of the plane's top-left corner ((0, 0) unless
    /// the plane carries a margin around the host, see `reset_padded`).
    pub ox: f32,
    pub oy: f32,
}

impl Plane {
    /// A transparent `w x h` plane covering a `host_w x host_h` host.
    pub(crate) fn reset(&mut self, w: usize, h: usize, host_w: f64, host_h: f64) {
        self.w = w;
        self.h = h;
        self.kx = (w as f64 / host_w) as f32;
        self.ky = (h as f64 / host_h) as f32;
        self.ox = 0.0;
        self.oy = 0.0;
        self.px.clear();
        self.px.resize(w * h, [0.0; 4]);
    }

    /// A transparent plane for a `w x h` host raster with `pad_x` / `pad_y`
    /// extra pixels on every side: geometry hanging past the host still
    /// reaches a blur, like a canvas 2D filter (which blurs the whole shape,
    /// not just the part inside the canvas).
    pub(crate) fn reset_padded(
        &mut self,
        (w, h): (usize, usize),
        host: (f64, f64),
        (pad_x, pad_y): (usize, usize),
    ) {
        self.kx = (w as f64 / host.0) as f32;
        self.ky = (h as f64 / host.1) as f32;
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

    /// Bilinear sample of the plane at normalised coordinates `(u, v)` in
    /// `0..1` over the host, transparent outside.
    pub(crate) fn sample(&self, u: f32, v: f32) -> Px {
        let fx = u * self.w as f32 - 0.5;
        let fy = v * self.h as f32 - 0.5;
        let x0 = fx.floor();
        let y0 = fy.floor();
        let tx = fx - x0;
        let ty = fy - y0;
        let (x0, y0) = (x0 as isize, y0 as isize);
        // Edge pixels extend (clamp), so the picture does not fade at the
        // rim of an upscaled layer.
        let at = |x: isize, y: isize| -> Px {
            let x = x.clamp(0, self.w as isize - 1) as usize;
            let y = y.clamp(0, self.h as isize - 1) as usize;
            self.px[y * self.w + x]
        };
        let (a, b, c, d) = (
            at(x0, y0),
            at(x0 + 1, y0),
            at(x0, y0 + 1),
            at(x0 + 1, y0 + 1),
        );
        let mut out = [0.0; 4];
        for k in 0..4 {
            let top = a[k] + (b[k] - a[k]) * tx;
            let bottom = c[k] + (d[k] - c[k]) * tx;
            out[k] = top + (bottom - top) * ty;
        }
        out
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

/// Paints the seven lobes of one layer as radial gradients, the first lobe on
/// top (CSS stacks the first background layer highest). Each is
/// `radial-gradient(ellipse rx ry at (x, y), colour 0%, transparent fade%)`,
/// interpolated premultiplied.
pub(crate) fn paint_lobes(plane: &mut Plane, frame: &VoiceGlowFrame, layer: &LobeLayer) {
    let fade = (layer.fade_percent / 100.0) as f32;
    let host_h = frame.host.height as f32;
    for lobe in frame.lobes.iter().rev() {
        paint_lobe(plane, frame, layer, lobe, fade, host_h);
    }
}

fn paint_lobe(
    plane: &mut Plane,
    frame: &VoiceGlowFrame,
    layer: &LobeLayer,
    lobe: &LobeFrame,
    fade: f32,
    host_h: f32,
) {
    let (rx, ry) = lobe.radii(layer, frame);
    let (rx, ry) = (rx as f32, ry as f32);
    if rx <= 0.0 || ry <= 0.0 || fade <= 0.0 {
        return;
    }
    let cx = lobe.center_x as f32;
    let cy = host_h + layer.y as f32 + lobe.lift as f32;
    let [r, g, b] = lobe.color.to_unit().map(|v| v as f32);
    let alpha = layer.alpha as f32;
    let (kx, ky) = (plane.kx, plane.ky);
    let x0 = (((cx - rx * fade) * kx).floor().max(0.0)) as usize;
    let x1 = (((cx + rx * fade) * kx).ceil().max(0.0) as usize).min(plane.w);
    let y0 = (((cy - ry * fade) * ky).floor().max(0.0)) as usize;
    let y1 = (((cy + ry * fade) * ky).ceil().max(0.0) as usize).min(plane.h);
    let (inv_rx, inv_ry, inv_fade) = (1.0 / rx, 1.0 / ry, 1.0 / fade);
    for j in y0..y1 {
        let ny = (plane.y(j) - cy) * inv_ry;
        for i in x0..x1 {
            let nx = (plane.x(i) - cx) * inv_rx;
            let u = (nx * nx + ny * ny).sqrt() * inv_fade;
            if u >= 1.0 {
                continue;
            }
            let a = alpha * (1.0 - u);
            over(&mut plane.px[j * plane.w + i], [r * a, g * a, b * a, a]);
        }
    }
}

/// The ellipse every layer is masked to (`edgeMask`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct EdgeMask {
    pub cx: f32,
    pub cy: f32,
    pub rx: f32,
    pub ry: f32,
    /// Position of the half-alpha stop, 0-1.
    pub mid: f32,
    /// Alpha at 85%, when the gradient has that stop.
    pub tail: Option<f32>,
}

impl EdgeMask {
    /// The mask for a layer authored at `(half_width, height)`.
    pub(crate) fn new(
        frame: &VoiceGlowFrame,
        half_width: f64,
        height: f64,
        mid: f32,
        tail: Option<f32>,
    ) -> Self {
        let (cx, up, rx, ry) = frame.mask_ellipse(half_width, height);
        Self {
            cx: cx as f32,
            cy: (frame.host.height - up) as f32,
            rx: rx as f32,
            ry: ry as f32,
            mid,
            tail,
        }
    }

    /// Mask alpha at host point `(x, y)`.
    #[inline]
    pub(crate) fn at(&self, x: f32, y: f32) -> f32 {
        if self.rx <= 0.0 || self.ry <= 0.0 {
            return 0.0;
        }
        let nx = (x - self.cx) / self.rx;
        let ny = (y - self.cy) / self.ry;
        let d = (nx * nx + ny * ny).sqrt();
        if d >= 1.0 {
            return 0.0;
        }
        let lerp = |t: f32, a: f32, b: f32| a + (b - a) * t;
        if d <= self.mid {
            return lerp(d / self.mid.max(1e-6), 1.0, 0.5);
        }
        match self.tail {
            Some(tail) if d <= 0.85 => {
                lerp((d - self.mid) / (0.85 - self.mid).max(1e-6), 0.5, tail)
            }
            Some(tail) => lerp((d - 0.85) / 0.15, tail, 0.0),
            None => lerp((d - self.mid) / (1.0 - self.mid).max(1e-6), 0.5, 0.0),
        }
    }
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

/// The `hue-rotate() brightness() saturate()` chain with the clamps a
/// browser applies between stages, in `f32` for whole planes.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Fx {
    hue: [[f32; 3]; 3],
    brightness: f32,
    sat: [[f32; 3]; 3],
}

impl Fx {
    pub(crate) fn new(filter: &ColorFilter) -> Self {
        let (sin, cos) = filter.hue_degrees.to_radians().sin_cos();
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
        ]
        .map(|row| row.map(|v| v as f32));
        let s = filter.saturation;
        let sat = [
            [0.213 + 0.787 * s, 0.715 - 0.715 * s, 0.072 - 0.072 * s],
            [0.213 - 0.213 * s, 0.715 + 0.285 * s, 0.072 - 0.072 * s],
            [0.213 - 0.213 * s, 0.715 - 0.715 * s, 0.072 + 0.928 * s],
        ]
        .map(|row| row.map(|v| v as f32));
        Self {
            hue,
            brightness: filter.brightness as f32,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_runs_from_one_through_half_to_zero() {
        let m = EdgeMask {
            cx: 0.0,
            cy: 0.0,
            rx: 100.0,
            ry: 100.0,
            mid: 0.45,
            tail: Some(0.3),
        };
        assert!((m.at(0.0, 0.0) - 1.0).abs() < 1e-6);
        assert!((m.at(45.0, 0.0) - 0.5).abs() < 1e-5);
        assert!((m.at(85.0, 0.0) - 0.3).abs() < 1e-5);
        assert_eq!(m.at(100.0, 0.0), 0.0);
        assert_eq!(m.at(150.0, 0.0), 0.0);
    }

    #[test]
    fn rounded_rect_distance_signs() {
        assert!(sd_round_rect(50.0, 50.0, 0.0, 0.0, 100.0, 100.0, 16.0) < 0.0);
        assert!(sd_round_rect(150.0, 50.0, 0.0, 0.0, 100.0, 100.0, 16.0) > 0.0);
        // The corner arc cuts the corner point off.
        assert!(sd_round_rect(0.5, 0.5, 0.0, 0.0, 100.0, 100.0, 16.0) > 0.0);
    }

    /// Ground truth from Chromium: a solid colour under
    /// `filter: hue-rotate(h) brightness(b) saturate(s)`, read back from a
    /// screenshot (`tools/voice_cmp`, `filter_probe.cjs`). The stages clamp
    /// between each other; without those clamps the worst channel is off by 94.
    #[test]
    fn colour_filter_matches_chromium() {
        #[rustfmt::skip]
        // (hue-rotate deg, brightness, saturate, input rgb, chromium rgb)
        type Case = (f64, f64, f64, [u8; 3], [u8; 3]);
        let cases: &[Case] = &[
            (20.78, 1.15, 1.2, [255, 70, 120], [255, 82, 71]),
            (20.78, 1.15, 1.2, [60, 190, 255], [98, 205, 255]),
            (20.78, 1.15, 1.2, [175, 70, 255], [255, 57, 255]),
            (20.78, 1.15, 1.2, [60, 220, 130], [28, 255, 201]),
            (20.78, 1.15, 1.2, [255, 150, 40], [245, 193, 0]),
            (20.78, 1.15, 1.2, [90, 100, 255], [173, 92, 255]),
            (20.78, 1.15, 1.2, [40, 200, 190], [41, 225, 255]),
            (20.78, 1.15, 1.2, [255, 255, 255], [255, 255, 255]),
            (20.78, 1.15, 1.2, [250, 30, 30], [255, 43, 0]),
            (20.78, 1.15, 1.2, [0, 255, 0], [0, 255, 68]),
            (20.78, 1.15, 1.2, [10, 10, 255], [118, 0, 255]),
            (20.78, 1.15, 1.2, [200, 200, 20], [150, 255, 0]),
            (20.78, 1.15, 1.2, [128, 128, 128], [147, 147, 147]),
            (180.0, 1.6, 2.0, [255, 70, 120], [0, 255, 149]),
            (180.0, 1.6, 2.0, [60, 190, 255], [255, 233, 25]),
            (180.0, 1.6, 2.0, [175, 70, 255], [0, 255, 0]),
            (180.0, 1.6, 2.0, [60, 220, 130], [255, 213, 255]),
            (180.0, 1.6, 2.0, [255, 150, 40], [11, 255, 255]),
            (180.0, 1.6, 2.0, [90, 100, 255], [231, 199, 0]),
            (180.0, 1.6, 2.0, [40, 200, 190], [255, 198, 230]),
            (180.0, 1.6, 2.0, [255, 255, 255], [255, 255, 255]),
            (180.0, 1.6, 2.0, [250, 30, 30], [0, 240, 240]),
            (180.0, 1.6, 2.0, [0, 255, 0], [255, 153, 255]),
            (180.0, 1.6, 2.0, [10, 10, 255], [78, 78, 0]),
            (180.0, 1.6, 2.0, [200, 200, 20], [255, 255, 255]),
            (180.0, 1.6, 2.0, [128, 128, 128], [205, 205, 205]),
            (60.0, 0.8, 0.5, [255, 70, 120], [121, 86, 45]),
            (60.0, 0.8, 0.5, [60, 190, 255], [141, 124, 167]),
            (60.0, 0.8, 0.5, [175, 70, 255], [143, 63, 89]),
            (60.0, 0.8, 0.5, [60, 220, 130], [102, 152, 174]),
            (60.0, 0.8, 0.5, [255, 150, 40], [107, 145, 75]),
            (60.0, 0.8, 0.5, [90, 100, 255], [134, 70, 123]),
            (60.0, 0.8, 0.5, [40, 200, 190], [115, 131, 167]),
            (60.0, 0.8, 0.5, [255, 255, 255], [204, 204, 204]),
            (60.0, 0.8, 0.5, [250, 30, 30], [83, 66, 33]),
            (60.0, 0.8, 0.5, [0, 255, 0], [79, 178, 178]),
            (60.0, 0.8, 0.5, [10, 10, 255], [109, 23, 85]),
            (60.0, 0.8, 0.5, [200, 200, 20], [94, 170, 112]),
            (60.0, 0.8, 0.5, [128, 128, 128], [102, 102, 102]),
            (-24.0, 1.4, 1.8, [255, 70, 120], [255, 60, 255]),
            (-24.0, 1.4, 1.8, [60, 190, 255], [0, 255, 255]),
            (-24.0, 1.4, 1.8, [175, 70, 255], [162, 111, 255]),
            (-24.0, 1.4, 1.8, [60, 220, 130], [54, 255, 42]),
            (-24.0, 1.4, 1.8, [255, 150, 40], [255, 177, 64]),
            (-24.0, 1.4, 1.8, [90, 100, 255], [0, 183, 255]),
            (-24.0, 1.4, 1.8, [40, 200, 190], [0, 255, 178]),
            (-24.0, 1.4, 1.8, [255, 255, 255], [255, 255, 255]),
            (-24.0, 1.4, 1.8, [250, 30, 30], [255, 0, 194]),
            (-24.0, 1.4, 1.8, [0, 255, 0], [59, 255, 0]),
            (-24.0, 1.4, 1.8, [10, 10, 255], [0, 54, 255]),
            (-24.0, 1.4, 1.8, [200, 200, 20], [255, 255, 0]),
            (-24.0, 1.4, 1.8, [128, 128, 128], [179, 179, 179]),
        ];
        for &(hue, brightness, saturation, input, want) in cases {
            let fx = Fx::new(&ColorFilter::new(hue, brightness, saturation));
            let got = fx
                .apply(input.map(|v| f32::from(v) / 255.0))
                .map(|v| (v * 255.0).round() as i32);
            for c in 0..3 {
                assert!(
                    (got[c] - i32::from(want[c])).abs() <= 1,
                    "{input:?} under ({hue}, {brightness}, {saturation}): {got:?} vs {want:?}"
                );
            }
        }
    }

    #[test]
    fn identity_filter_is_identity() {
        let fx = Fx::new(&ColorFilter::new(0.0, 1.0, 1.0));
        let c = fx.apply([0.2, 0.5, 0.9]);
        for (a, b) in c.iter().zip([0.2, 0.5, 0.9]) {
            assert!((a - b).abs() < 1e-5);
        }
    }

    #[test]
    fn f32_filter_matches_the_model_filter() {
        use super::super::palette::Rgb;
        let cf = ColorFilter::new(17.0, 1.15, 1.2);
        let want = cf.apply(Rgb::new(255, 70, 120));
        let got = Fx::new(&cf).apply([1.0, 70.0 / 255.0, 120.0 / 255.0]);
        for (g, w) in got.iter().zip(want) {
            assert!((f64::from(*g) - w).abs() < 1e-4);
        }
    }
}
