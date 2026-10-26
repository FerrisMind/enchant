//! The `liquidMetal` material on the CPU: Paper Shaders 0.0.80's fragment stage
//! (`shape: none`) plus its sizing vertex stage, and the sheet-crop mapping.
//!
//! This is the same arithmetic as `shaders/metal_fx.wgsl`; the GPU parity test
//! compares the two. The CPU copy exists because the glow hunts the brightest
//! point of the ring by sampling the material (the web reads the GL canvas
//! back), and because it drives the headless renderer.

use std::f32::consts::PI;

use super::preset::MetalPresetMode;

/// The web renders the material onto one square sheet of this many pixels
/// (`96 × DPR 2`) and crops a window onto each instance. Stripe anti-aliasing
/// is a finite difference over one texel of it.
pub const SHEET_RESOLUTION: f32 = 192.0;
/// `u_pixelRatio` of that sheet.
pub const SHEET_PIXEL_RATIO: f32 = 2.0;
/// Canonical pill the crop window is measured against (`CANONICAL_PILL_*`).
pub const CANONICAL_WIDTH: f32 = 140.0;
/// Canonical pill the crop window is measured against.
pub const CANONICAL_HEIGHT: f32 = 40.0;

/// How an instance maps onto the material sheet: `uv = origin + p * scale`
/// with `p` in box-local CSS px.
///
/// The web crops a window of `box / (140×40 · shaderScale)` of the sheet
/// (clamped to the whole sheet) centred on it (`copyShaderToInstance`).
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::SheetMapping;
///
/// let map = SheetMapping::new(140.0, 40.0, 1.0);
/// // A canonical pill at scale 1 shows the whole sheet.
/// assert_eq!(map.uv([0.0, 0.0]), [0.0, 0.0]);
/// assert_eq!(map.uv([140.0, 40.0]), [1.0, 1.0]);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SheetMapping {
    origin: [f32; 2],
    scale: [f32; 2],
}

impl SheetMapping {
    /// Mapping for a `width × height` box at `shader_scale`.
    #[must_use]
    pub fn new(width: f32, height: f32, shader_scale: f32) -> Self {
        let w = width.max(1.0);
        let h = height.max(1.0);
        let k = shader_scale.max(1e-3);
        let fx = (w / (CANONICAL_WIDTH * k)).min(1.0);
        let fy = (h / (CANONICAL_HEIGHT * k)).min(1.0);
        Self {
            origin: [0.5 - 0.5 * fx, 0.5 - 0.5 * fy],
            scale: [fx / w, fy / h],
        }
    }

    /// Sheet origin (uv of the box's top-left corner).
    #[must_use]
    pub fn origin(&self) -> [f32; 2] {
        self.origin
    }

    /// Sheet uv per CSS px.
    #[must_use]
    pub fn scale(&self) -> [f32; 2] {
        self.scale
    }

    /// Sheet uv of a box-local point.
    #[must_use]
    pub fn uv(&self, p: [f32; 2]) -> [f32; 2] {
        [
            self.origin[0] + p[0] * self.scale[0],
            self.origin[1] + p[1] * self.scale[1],
        ]
    }
}

type V2 = [f32; 2];

fn ss(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0).max(1e-6)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn fract(x: f32) -> f32 {
    x - x.floor()
}

fn gmod(x: f32, y: f32) -> f32 {
    x - y * (x / y).floor()
}

fn rotate2(uv: V2, th: f32) -> V2 {
    let (s, c) = th.sin_cos();
    [c * uv[0] - s * uv[1], s * uv[0] + c * uv[1]]
}

fn permute(x: [f32; 3]) -> [f32; 3] {
    x.map(|v| gmod((v * 34.0 + 1.0) * v, 289.0))
}

/// Ashima simplex noise, as in Paper's `shader-utils`.
pub(crate) fn snoise(v: V2) -> f32 {
    const CX: f32 = 0.211_324_87;
    const CY: f32 = 0.366_025_42;
    const CZ: f32 = -0.577_350_26;
    const CW: f32 = 0.024_390_243;
    let s = (v[0] + v[1]) * CY;
    let mut i = [(v[0] + s).floor(), (v[1] + s).floor()];
    let t = (i[0] + i[1]) * CX;
    let x0 = [v[0] - i[0] + t, v[1] - i[1] + t];
    let i1 = if x0[0] > x0[1] {
        [1.0, 0.0]
    } else {
        [0.0, 1.0]
    };
    let x12 = [
        x0[0] + CX - i1[0],
        x0[1] + CX - i1[1],
        x0[0] + CZ,
        x0[1] + CZ,
    ];
    i = [gmod(i[0], 289.0), gmod(i[1], 289.0)];
    let py = permute([i[1], i[1] + i1[1], i[1] + 1.0]);
    let p = permute([py[0] + i[0], py[1] + i[0] + i1[0], py[2] + i[0] + 1.0]);
    let d = [
        x0[0] * x0[0] + x0[1] * x0[1],
        x12[0] * x12[0] + x12[1] * x12[1],
        x12[2] * x12[2] + x12[3] * x12[3],
    ];
    let mut m = d.map(|v| (0.5 - v).max(0.0));
    m = m.map(|v| v * v);
    m = m.map(|v| v * v);
    let x = p.map(|v| 2.0 * fract(v * CW) - 1.0);
    let h = x.map(|v| v.abs() - 0.5);
    let ox = x.map(|v| (v + 0.5).floor());
    let a0 = [x[0] - ox[0], x[1] - ox[1], x[2] - ox[2]];
    for k in 0..3 {
        m[k] *= 1.792_842_9 - 0.853_734_73 * (a0[k] * a0[k] + h[k] * h[k]);
    }
    let g = [
        a0[0] * x0[0] + h[0] * x0[1],
        a0[1] * x12[0] + h[1] * x12[1],
        a0[2] * x12[2] + h[2] * x12[3],
    ];
    130.0 * (m[0] * g[0] + m[1] * g[1] + m[2] * g[2])
}

/// Output of Paper's sizing vertex stage for one sheet point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SheetUvs {
    /// `v_objectUV`.
    pub object: [f32; 2],
    /// `v_responsiveUV`.
    pub responsive: [f32; 2],
    /// `v_responsiveBoxGivenSize`.
    pub given: [f32; 2],
}

fn box_size(box_ratio: f32, given: V2, res: V2, fit: f32) -> V2 {
    let mut x = box_ratio * (given[0] / box_ratio).min(given[1]);
    if fit == 1.0 {
        x = box_ratio * (res[0] / box_ratio).min(res[1]);
    } else if fit == 2.0 {
        x = box_ratio * (res[0] / box_ratio).max(res[1]);
    }
    [x, x / box_ratio]
}

/// Paper's sizing vertex stage (`vertex-shader.ts`), evaluated at `ndc`
/// (`gl_Position.xy`, y up) for a sheet of [`SHEET_RESOLUTION`] px.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::{sheet_uvs, MetalFxPreset, MetalFxResolvedTheme};
///
/// // The bundled presets use identity sizing: uv = ndc / 2.
/// let mode = MetalFxPreset::Silver.mode(MetalFxResolvedTheme::Dark);
/// let uvs = sheet_uvs(&mode, [1.0, -1.0]);
/// assert!((uvs.responsive[0] - 0.5).abs() < 1e-6);
/// assert!((uvs.responsive[1] + 0.5).abs() < 1e-6);
/// ```
#[must_use]
pub fn sheet_uvs(mode: &MetalPresetMode, ndc: [f32; 2]) -> SheetUvs {
    let res = [SHEET_RESOLUTION; 2];
    let uv = [ndc[0] * 0.5, ndc[1] * 0.5];
    let box_origin = [0.5 - mode.origin_x, mode.origin_y - 0.5];
    let given = [
        mode.world_width.max(1.0) * SHEET_PIXEL_RATIO,
        mode.world_height.max(1.0) * SHEET_PIXEL_RATIO,
    ];
    let r = mode.rotation * PI / 180.0;
    let offset = [-mode.offset_x, mode.offset_y];
    let fixed = [
        if mode.world_width == 0.0 {
            res[0]
        } else {
            given[0]
        },
        if mode.world_height == 0.0 {
            res[1]
        } else {
            given[1]
        },
    ];
    let fit = mode.fit.code();
    let scale = mode.scale;

    let obj_box = box_size(1.0, fixed, res, fit);
    let obj_scale = [res[0] / obj_box[0], res[1] / obj_box[1]];
    let mut o = [uv[0] * obj_scale[0], uv[1] * obj_scale[1]];
    o = [
        o[0] + box_origin[0] * (obj_scale[0] - 1.0) + offset[0],
        o[1] + box_origin[1] * (obj_scale[1] - 1.0) + offset[1],
    ];
    o = rotate2([o[0] / scale, o[1] / scale], r);

    let ratio = fixed[0] / fixed[1];
    let rbox = box_size(ratio, fixed, res, fit);
    let rscale = [res[0] / rbox[0], res[1] / rbox[1]];
    let mut q = [uv[0] * rscale[0], uv[1] * rscale[1]];
    q = [
        (q[0] + box_origin[0] * (rscale[0] - 1.0) + offset[0]) / scale,
        (q[1] + box_origin[1] * (rscale[1] - 1.0) + offset[1]) / scale,
    ];
    q[0] *= ratio;
    q = rotate2(q, r);
    q[0] /= ratio;

    SheetUvs {
        object: o,
        responsive: q,
        given: fixed,
    }
}

/// Everything the stripe stage needs from one sheet point.
struct Core {
    edge_o: f32,
    edge: f32,
    direction: f32,
    bump: f32,
    diag_tl: f32,
    dr: f32,
    db: f32,
}

fn edge_raw(mode: &MetalPresetMode, s: V2) -> f32 {
    let su = sheet_uvs(mode, [2.0 * s[0] - 1.0, 1.0 - 2.0 * s[1]]);
    let border = [su.responsive[0] + 0.5, su.responsive[1] + 0.5];
    let mask = [
        border[0].min(1.0 - border[0]),
        border[1].min(1.0 - border[1]),
    ];
    let pt = [
        (250.0 / su.given[0]).min(0.5),
        (250.0 / su.given[1]).min(0.5),
    ];
    let mx = ss(0.0, pt[0], mask[0]).powf(0.25);
    let my = ss(0.0, pt[1], mask[1]).powf(0.25);
    (1.0 - mx * my).clamp(0.0, 1.0)
}

fn liquid_core(mode: &MetalPresetMode, s: V2, t: f32, fw_edge: f32) -> Core {
    let su = sheet_uvs(mode, [2.0 * s[0] - 1.0, 1.0 - 2.0 * s[1]]);
    let uv_obj = [su.object[0] + 0.5, 1.0 - (su.object[1] + 0.5)];
    let angle = (-mode.angle + 70.0) * PI / 180.0;
    let (sa, ca) = angle.sin_cos();
    let ro = [uv_obj[0] - 0.5, uv_obj[1] - 0.5];
    let rotated = [ro[0] * ca - ro[1] * sa + 0.5, ro[0] * sa + ro[1] * ca + 0.5];

    let raw = edge_raw(mode, s);
    let ratio = su.given[0] / su.given[1];
    let mut uv = su.responsive;
    if ratio > 1.0 {
        uv[1] /= ratio;
    } else {
        uv[0] *= ratio;
    }
    uv = [uv[0] + 0.5, 1.0 - (uv[1] + 0.5)];
    let cycle_width = mode.repetition * 2.0;

    let contour = mode.contour;
    let edge_o = mix(
        ss(0.9 - 2.0 * fw_edge, 0.9, raw),
        raw,
        ss(0.0, 0.4, contour),
    );
    let mut edge = 1.2 * edge_o;

    let diag_bl = rotated[0] - rotated[1];
    let diag_tl = rotated[0] + rotated[1];

    let mut grad = [uv[0] - 0.5, uv[1] - 0.5];
    let dist = (grad[0] * grad[0] + (grad[1] + 0.2 * diag_bl).powi(2)).sqrt();
    grad = rotate2(grad, (0.25 - 0.2 * diag_bl) * PI);
    let mut direction = grad[0];

    let uy = uv[1].max(0.0);
    let mut bump = 1.0 - (1.8 * dist).powf(1.2);
    bump *= uy.powf(0.3);

    let noise = snoise([uv[0] - t, uv[1] - t]);
    edge += (1.0 - edge) * mode.distortion * noise;

    direction += diag_bl;
    let se = ss(0.0, 1.0, edge);
    direction -= 2.0 * noise * diag_bl * (se * (1.0 - se));
    let c51 = ss(0.5, 1.0, contour);
    direction *= mix(1.0, 1.0 - edge, c51);
    direction -= 1.7 * edge * c51;
    direction += 0.2 * contour.powi(4) * (1.0 - se);

    bump *= uy.powf(0.1).clamp(0.3, 1.0);
    direction *= 0.1 + (1.1 - edge) * bump;

    direction *= 0.4 + 0.6 * (1.0 - ss(0.5, 1.0, edge));
    direction += 0.18 * (ss(0.1, 0.2, uv[1]) * (1.0 - ss(0.2, 0.4, uv[1])));
    direction += 0.03 * (ss(0.1, 0.2, 1.0 - uv[1]) * (1.0 - ss(0.2, 0.4, 1.0 - uv[1])));

    direction *= 0.5 + 0.5 * uv[1] * uv[1];
    direction *= cycle_width;
    direction -= t;

    let dispersion = (1.0 - bump).clamp(0.0, 1.0);
    let mut dr = dispersion;
    dr += 0.03 * bump * noise;
    dr += 5.0
        * (ss(-0.1, 0.2, uv[1]) * (1.0 - ss(0.1, 0.5, uv[1])))
        * (ss(0.4, 0.6, bump) * (1.0 - ss(0.4, 1.0, bump)));
    dr -= diag_bl;

    let mut db = dispersion * 1.3;
    db += (ss(0.0, 0.4, uv[1]) * (1.0 - ss(0.1, 0.8, uv[1])))
        * (ss(0.4, 0.6, bump) * (1.0 - ss(0.4, 0.8, bump)));
    db -= 0.2 * edge;

    dr *= mode.shift_red / 20.0;
    db *= mode.shift_blue / 20.0;

    Core {
        edge_o,
        edge,
        direction,
        bump,
        diag_tl,
        dr,
        db,
    }
}

fn color_changes(
    colors: [f32; 2],
    p: f32,
    w: [f32; 3],
    blur: f32,
    bump: f32,
    tint: f32,
    tint_a: f32,
) -> f32 {
    let [c1, c2] = colors;
    let blur = blur.max(1e-5);
    let mut ch = mix(c2, c1, ss(0.0, 2.0 * blur, p));
    let mut border = w[0];
    ch = mix(ch, c2, ss(border, border + 2.0 * blur, p));
    border = w[0] + 0.4 * (1.0 - bump) * w[1];
    ch = mix(ch, c1, ss(border, border + 2.0 * blur, p));
    border = w[0] + 0.5 * (1.0 - bump) * w[1];
    ch = mix(ch, c2, ss(border, border + 2.0 * blur, p));
    border = w[0] + w[1];
    ch = mix(ch, c1, ss(border, border + 2.0 * blur, p));
    let gradient_t = (p - w[0] - w[1]) / w[2];
    let gradient = mix(c1, c2, ss(0.0, 1.0, gradient_t));
    ch = mix(ch, gradient, ss(border, border + 0.5 * blur, p));
    mix(ch, 1.0 - ((1.0 - ch) / tint.max(0.0001)).min(1.0), tint_a)
}

/// The liquid-metal material of one preset mode.
///
/// [`shade`](Self::shade) is the fragment stage: it takes a point of the
/// 192 px sheet (`0..=1`, y down) and returns the premultiplied colour Paper's
/// shader writes there. Stripe anti-aliasing uses finite differences over one
/// sheet texel where the GLSL uses `fwidth`.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::{MetalFxPreset, MetalFxResolvedTheme, MetalMaterial};
///
/// let material = MetalMaterial::new(MetalFxPreset::Silver.mode(MetalFxResolvedTheme::Dark));
/// let px = material.shade([0.5, 0.5], 0.0, None);
/// assert!(px[3] > 0.99, "the interior of the sheet is opaque");
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MetalMaterial {
    mode: MetalPresetMode,
}

impl MetalMaterial {
    /// Wraps a preset mode.
    #[must_use]
    pub fn new(mode: MetalPresetMode) -> Self {
        Self { mode }
    }

    /// The parameters.
    #[must_use]
    pub fn mode(&self) -> &MetalPresetMode {
        &self.mode
    }

    /// `u_time` for `elapsed_seconds` of animation (the preset's speed applied).
    #[must_use]
    pub fn u_time(&self, elapsed_seconds: f32) -> f32 {
        elapsed_seconds * self.mode.speed
    }

    /// Premultiplied RGBA at sheet point `s` for `u_time`.
    ///
    /// `dither` is the fragment position in device px for Paper's banding fix
    /// (`None` skips it, which is what comparisons want).
    #[must_use]
    pub fn shade(&self, s: [f32; 2], u_time: f32, dither: Option<[f32; 2]>) -> [f32; 4] {
        let mode = &self.mode;
        let t = 0.3 * (u_time + 2.8);
        let texel = 1.0 / SHEET_RESOLUTION;
        let sx = [s[0] + texel, s[1]];
        let sy = [s[0], s[1] + texel];

        let e0 = edge_raw(mode, s);
        let fw_edge = (edge_raw(mode, sx) - e0).abs() + (edge_raw(mode, sy) - e0).abs();

        let c = liquid_core(mode, s, t, fw_edge);
        let cx = liquid_core(mode, sx, t, fw_edge);
        let cy = liquid_core(mode, sy, t, fw_edge);

        let xr = |k: &Core| k.direction + k.dr;
        let xg = |k: &Core| k.direction;
        let xb = |k: &Core| k.direction - k.db;
        let fw = |f: &dyn Fn(&Core) -> f32| (f(&cx) - f(&c)).abs() + (f(&cy) - f(&c)).abs();
        let (fw_r, fw_g, fw_b) = (fw(&xr), fw(&xg), fw(&xb));

        let mut opacity = 1.0 - ss(0.9 - 2.0 * fw_edge, 0.9, c.edge_o);

        let color1 = [0.98, 0.98, 1.0];
        let color2 = [0.1, 0.1, 0.1 + 0.1 * ss(0.7, 1.3, c.diag_tl)];

        let cw = mode.repetition * 2.0;
        let thin1 = 0.12 / cw * (1.0 - 0.4 * c.bump);
        let thin2 = 0.07 / cw * (1.0 + 0.4 * c.bump);
        let wide = 1.0 - thin1 - thin2;
        let mut w = [cw * thin1, cw * thin2, wide];
        w[1] -= 0.02 * ss(0.0, 1.0, c.edge + c.bump);

        let blur = mode.softness / 15.0;
        let tint = mode.color_tint;
        let r = color_changes(
            [color1[0], color2[0]],
            fract(xr(&c)),
            w,
            blur + fw_r,
            c.bump,
            tint[0],
            tint[3],
        );
        let g = color_changes(
            [color1[1], color2[1]],
            fract(xg(&c)),
            w,
            blur + fw_g,
            c.bump,
            tint[1],
            tint[3],
        );
        let b = color_changes(
            [color1[2], color2[2]],
            fract(xb(&c)),
            w,
            blur + fw_b,
            c.bump,
            tint[2],
            tint[3],
        );

        let back = mode.color_back;
        let mut color = [r * opacity, g * opacity, b * opacity];
        for (k, ch) in color.iter_mut().enumerate() {
            *ch += back[k] * back[3] * (1.0 - opacity);
        }
        opacity += back[3] * (1.0 - opacity);

        if let Some(frag) = dither {
            let d = (0.014 * frag[0]) * 12.9898 + (0.014 * frag[1]) * 78.233;
            let n = fract(d.sin() * 43_758.547) - 0.5;
            for ch in &mut color {
                *ch += n / 256.0;
            }
        }
        [color[0], color[1], color[2], opacity]
    }

    /// Rec. 709 luminance of the material at a sheet point (premultiplied
    /// colour, no dither).
    #[must_use]
    pub fn luminance(&self, s: [f32; 2], u_time: f32) -> f32 {
        let c = self.shade(s, u_time, None);
        0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::metal_fx::preset::{MetalFxPreset, MetalFxResolvedTheme};

    fn silver() -> MetalMaterial {
        MetalMaterial::new(MetalFxPreset::Silver.mode(MetalFxResolvedTheme::Dark))
    }

    #[test]
    fn sheet_mapping_follows_copy_shader_to_instance() {
        // 70x40 at 1.6: window 70/(140*1.6)=0.3125 wide, 40/(40*1.6)=0.625 tall.
        let m = SheetMapping::new(70.0, 40.0, 1.6);
        let fx = 70.0 / (140.0 * 1.6);
        let fy = 40.0 / (40.0 * 1.6);
        assert!((m.origin()[0] - (0.5 - 0.5 * fx)).abs() < 1e-6);
        assert!((m.origin()[1] - (0.5 - 0.5 * fy)).abs() < 1e-6);
        let end = m.uv([70.0, 40.0]);
        assert!((end[0] - (0.5 + 0.5 * fx)).abs() < 1e-6);
        assert!((end[1] - (0.5 + 0.5 * fy)).abs() < 1e-6);
        // Big boxes clamp to the whole sheet.
        let big = SheetMapping::new(800.0, 400.0, 1.6);
        assert_eq!(big.origin(), [0.0, 0.0]);
        assert!((big.uv([800.0, 400.0])[0] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn default_sizing_is_identity() {
        let mode = MetalFxPreset::Chromatic.mode(MetalFxResolvedTheme::Dark);
        for (ndc, expect) in [
            ([-1.0, -1.0], [-0.5, -0.5]),
            ([0.0, 0.0], [0.0, 0.0]),
            ([1.0, 1.0], [0.5, 0.5]),
            ([0.5, -0.25], [0.25, -0.125]),
        ] {
            let u = sheet_uvs(&mode, ndc);
            for (k, want) in expect.iter().enumerate() {
                assert!((u.object[k] - want).abs() < 1e-5, "object {ndc:?}");
                assert!((u.responsive[k] - want).abs() < 1e-5, "responsive {ndc:?}");
            }
            assert_eq!(u.given, [SHEET_RESOLUTION; 2]);
        }
    }

    #[test]
    fn sizing_scale_and_offset_apply() {
        let mut mode = MetalFxPreset::Silver.mode(MetalFxResolvedTheme::Dark);
        mode.scale = 2.0;
        let u = sheet_uvs(&mode, [1.0, 0.0]);
        assert!((u.object[0] - 0.25).abs() < 1e-5);
        mode.scale = 1.0;
        mode.offset_x = 0.25;
        let u = sheet_uvs(&mode, [0.0, 0.0]);
        assert!(
            (u.object[0] + 0.25).abs() < 1e-5,
            "graphicOffset.x = -offsetX"
        );
    }

    #[test]
    fn simplex_noise_is_bounded_and_deterministic() {
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for i in 0..200 {
            for j in 0..200 {
                let v = snoise([i as f32 * 0.137 - 3.0, j as f32 * 0.091 - 2.0]);
                assert_eq!(v, snoise([i as f32 * 0.137 - 3.0, j as f32 * 0.091 - 2.0]));
                min = min.min(v);
                max = max.max(v);
            }
        }
        assert!(min >= -1.05 && max <= 1.05, "range {min}..{max}");
        assert!(
            min < -0.7 && max > 0.7,
            "noise should span the range, got {min}..{max}"
        );
        assert_eq!(snoise([0.0, 0.0]), 0.0);
    }

    #[test]
    fn sheet_interior_is_opaque_and_edge_sliver_is_not() {
        let m = silver();
        assert!(m.shade([0.5, 0.5], 1.0, None)[3] > 0.999);
        assert!(m.shade([0.5, 0.2], 1.0, None)[3] > 0.999);
        // The outermost sheet pixels fall in the edge >= .9 sliver.
        assert!(m.shade([0.0, 0.5], 1.0, None)[3] < 0.5);
    }

    #[test]
    fn material_is_bounded_and_animates() {
        let m = MetalMaterial::new(MetalFxPreset::Chromatic.mode(MetalFxResolvedTheme::Dark));
        let mut changed = false;
        for i in 0..32 {
            for j in 0..32 {
                let s = [0.05 + i as f32 / 36.0, 0.05 + j as f32 / 36.0];
                let a = m.shade(s, 0.0, None);
                let b = m.shade(s, 2.0, None);
                for ch in a.iter().chain(b.iter()) {
                    assert!(
                        ch.is_finite() && *ch > -0.02 && *ch < 1.02,
                        "{s:?} -> {a:?}"
                    );
                }
                changed |= (0..3).any(|k| (a[k] - b[k]).abs() > 0.05);
            }
        }
        assert!(changed, "time must move the stripes");
    }

    #[test]
    fn tint_burns_the_channels_it_targets() {
        let mut gold = MetalFxPreset::Gold.mode(MetalFxResolvedTheme::Dark);
        let mut silver = gold;
        gold.color_tint = [1.0, 0.8, 0.33, 0.8];
        silver.color_tint = [1.0, 1.0, 1.0, 0.0];
        let g = MetalMaterial::new(gold);
        let s = MetalMaterial::new(silver);
        let (mut gb, mut sb) = (0.0, 0.0);
        for i in 0..20 {
            let p = [0.2 + i as f32 * 0.03, 0.5];
            gb += g.shade(p, 0.0, None)[2];
            sb += s.shade(p, 0.0, None)[2];
        }
        assert!(
            gb < sb * 0.9,
            "gold must burn the blue channel ({gb} vs {sb})"
        );
    }

    #[test]
    fn dither_perturbs_by_less_than_a_level() {
        let m = silver();
        let a = m.shade([0.4, 0.45], 3.0, None);
        let b = m.shade([0.4, 0.45], 3.0, Some([17.0, 9.0]));
        for k in 0..3 {
            assert!((a[k] - b[k]).abs() <= 0.5 / 256.0 + 1e-6);
        }
        assert_eq!(a[3], b[3]);
    }

    #[test]
    fn u_time_applies_speed() {
        let gold = MetalMaterial::new(MetalFxPreset::Gold.mode(MetalFxResolvedTheme::Dark));
        assert!((gold.u_time(2.0) - 1.7).abs() < 1e-6);
    }

    fn error_stats(case: &str, mode: MetalPresetMode, time: f32) -> (usize, usize, usize, f32) {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("fixtures/paper_liquid_metal.json"))
                .expect("fixture json");
        let material = MetalMaterial::new(mode);
        let points = fixture[case].as_array().expect("case");
        let (mut n, mut close, mut near, mut worst_close) = (0, 0, 0, 0.0_f32);
        for p in points {
            let v: Vec<f32> = p
                .as_array()
                .expect("point")
                .iter()
                .map(|x| x.as_f64().expect("num") as f32)
                .collect();
            let uv = [
                (v[0] + 0.5) / SHEET_RESOLUTION,
                (v[1] + 0.5) / SHEET_RESOLUTION,
            ];
            let got = material.shade(uv, time, None);
            let err = (0..4)
                .map(|k| (got[k].clamp(0.0, 1.0) * 255.0 - v[2 + k]).abs())
                .fold(0.0, f32::max);
            n += 1;
            if err <= 3.0 {
                close += 1;
                worst_close = worst_close.max(err);
            }
            if err <= 12.0 {
                near += 1;
            }
        }
        (n, close, near, worst_close)
    }

    /// The decisive check: Paper's real GLSL, run in a browser, against this
    /// port. 81 sheet points per preset; only stripe-edge pixels (where the
    /// GLSL's `fwidth` anti-aliasing differs from a finite difference by a
    /// fraction of a texel) may deviate.
    #[test]
    fn matches_the_real_paper_glsl() {
        let cases = [
            (
                "chromaticDark",
                MetalFxPreset::Chromatic.mode(MetalFxResolvedTheme::Dark),
                1.25,
            ),
            (
                "goldDark",
                MetalFxPreset::Gold.mode(MetalFxResolvedTheme::Dark),
                3.1 * 0.85,
            ),
            (
                "chromaticLight",
                MetalFxPreset::Chromatic.mode(MetalFxResolvedTheme::Light),
                0.4,
            ),
        ];
        for (name, mode, time) in cases {
            let (n, close, near, worst) = error_stats(name, mode, time);
            eprintln!(
                "paper parity {name}: {close}/{n} within 3/255, {near}/{n} within 12/255 (worst close {worst:.1})"
            );
            // Measured: 86-94 % within 3/255 and 95-99 % within 12/255; the
            // rest are stripe-edge pixels (same colours, slightly different
            // transition width).
            assert!(
                close * 100 >= n * 84,
                "{name}: only {close}/{n} points within 3/255 of the GLSL"
            );
            assert!(
                near * 100 >= n * 93,
                "{name}: only {near}/{n} points within 12/255 of the GLSL"
            );
        }
    }

    #[test]
    fn luminance_is_bounded() {
        let m = MetalMaterial::new(MetalFxPreset::Chromatic.mode(MetalFxResolvedTheme::Dark));
        let lum = m.luminance([0.31, 0.62], 1.25);
        assert!((0.0..=1.0).contains(&lum));
    }
}
