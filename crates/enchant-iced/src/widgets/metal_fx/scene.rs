//! One instance at one moment, ready to draw: the single source of truth the
//! GPU uniforms and the CPU reference rasteriser are both built from.
//!
//! The layer stack mirrors `styles.ts` / `MetalFx.tsx`:
//!
//! | layer | web element | z |
//! |-------|-------------|---|
//! | [`Layer::Surface`] | `.metal-fx-root` background + `::before` inset shadow | 0 / 2 |
//! | [`Layer::Ring`] | metal canvas + circle hairline (`.metal-fx-inner`) | 0 / 1 |
//! | [`Layer::Glow`] | glow canvas (`.metal-fx-glow-svg`) | 3 |
//! | [`Layer::Rim`] | inner-shadow rim canvas + `::after` hairline | 4 |
//!
//! [`Scene::shade`] is the CPU twin of `fs_main` in `shaders/metal_fx.wgsl`.

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};

use super::bake::GlowSprite;
use super::glow::GlowFrame;
use super::mask::{MetalFxMask, MetalGlowMode};
use super::material::{MetalMaterial, SHEET_PIXEL_RATIO, SHEET_RESOLUTION, SheetMapping};
use super::preset::{MetalFxResolvedTheme, MetalPresetMode};
use super::rim::MetalRimOptions;

/// Draw layers, in the order they are painted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Layer {
    Ring = 0,
    Glow = 1,
    Rim = 2,
    Surface = 3,
}

/// The halo's per-tick draw data.
#[derive(Clone, Debug)]
pub(crate) struct GlowDraw {
    pub frame: GlowFrame,
    pub halo: Arc<GlowSprite>,
    pub extra: Arc<GlowSprite>,
    /// Quad margin around the box, CSS px, enough for the halo's blur skirt.
    pub margin: f32,
}

#[derive(Clone, Debug)]
pub(crate) struct Scene {
    pub width: f32,
    pub height: f32,
    pub radius: f32,
    pub ring: f32,
    pub density: f32,
    pub circle: bool,
    pub theme: MetalFxResolvedTheme,
    pub mode: MetalPresetMode,
    pub u_time: f32,
    pub mapping: SheetMapping,
    /// `strength × shaderOpacity`.
    pub opacity_mul: f32,
    pub rim: Option<MetalRimOptions>,
    pub glow: Option<GlowDraw>,
    /// Straight RGBA, `None` for a transparent surface.
    pub surface: Option<[f32; 4]>,
    pub target_srgb: bool,
    pub dither: bool,
    /// Ring band vs sheet fill (`mask` on the web).
    pub mask: MetalFxMask,
    /// Glow placement when [`mask`](Self::mask) is a fill.
    pub glow_mode: MetalGlowMode,
}

/// GPU uniform block (matches `Uniforms` in the WGSL; every field is a vec4).
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(crate) struct Uniforms {
    pub rect: [f32; 4],
    pub metrics: [f32; 4],
    pub shape: [f32; 4],
    pub flags: [f32; 4],
    pub color_back: [f32; 4],
    pub color_tint: [f32; 4],
    pub mat0: [f32; 4],
    pub mat1: [f32; 4],
    pub sheet: [f32; 4],
    pub sizing0: [f32; 4],
    pub sizing1: [f32; 4],
    pub sizing2: [f32; 4],
    pub hair: [f32; 4],
    pub circ: [f32; 4],
    pub halo: [f32; 4],
    pub extra: [f32; 4],
    pub halo_tint: [f32; 4],
    pub extra_tint: [f32; 4],
    pub halo_sprite: [f32; 4],
    pub extra_sprite: [f32; 4],
    pub glow_misc: [f32; 4],
    pub rim: [f32; 4],
    pub rim_color: [f32; 4],
    pub surf: [f32; 4],
    pub shadow: [f32; 4],
    pub shadow_misc: [f32; 4],
}

/// Size of [`Uniforms`] in bytes (26 × vec4).
pub(crate) const UNIFORM_SIZE: u64 = 26 * 16;
const _: () = assert!(std::mem::size_of::<Uniforms>() as u64 == UNIFORM_SIZE);

const SHADOW_SIGMA: f32 = 25.0;
const GLOW_ALPHA_DARK: f32 = 0.7;
const GLOW_ALPHA_LIGHT: f32 = 0.2746;
/// Hairline overhang kept around the box by the non-glow layers.
const EDGE_MARGIN: f32 = 2.0;

impl Scene {
    /// The `(origin, size)` of the quad `layer` is drawn into, box-local CSS px.
    pub fn quad(&self, layer: Layer) -> ([f32; 2], [f32; 2]) {
        let margin = match layer {
            Layer::Surface => 0.0,
            Layer::Ring | Layer::Rim => EDGE_MARGIN,
            Layer::Glow => self.glow.as_ref().map_or(0.0, |g| g.margin),
        };
        (
            [-margin, -margin],
            [self.width + 2.0 * margin, self.height + 2.0 * margin],
        )
    }

    /// Whether `layer` paints anything.
    pub fn has_layer(&self, layer: Layer) -> bool {
        match layer {
            Layer::Surface | Layer::Ring => true,
            Layer::Glow => self.glow.is_some(),
            Layer::Rim => true,
        }
    }

    fn after_hairline(&self) -> [f32; 4] {
        match self.theme {
            MetalFxResolvedTheme::Dark => [1.0, 1.0, 1.0, 0.1],
            MetalFxResolvedTheme::Light => [0.0, 0.0, 0.0, 0.06],
        }
    }

    fn after_hairline_width(&self) -> f32 {
        if self.circle { 2.0 } else { 1.0 }
    }

    fn circle_hairline(&self) -> [f32; 4] {
        match (self.circle, self.theme) {
            (true, MetalFxResolvedTheme::Dark) => [0.0, 0.0, 0.0, 0.45],
            _ => [0.0; 4],
        }
    }

    fn glow_ring_inset(&self) -> f32 {
        if self.circle { 2.0 } else { 1.0 }
    }

    fn glow_filter(&self) -> (f32, f32) {
        match (self.theme, self.circle) {
            (MetalFxResolvedTheme::Dark, _) => (1.0, 1.0),
            (MetalFxResolvedTheme::Light, false) => (5.355, 0.78),
            (MetalFxResolvedTheme::Light, true) => (7.5, 0.6),
        }
    }

    fn glow_group_alpha(&self) -> f32 {
        match self.theme {
            MetalFxResolvedTheme::Dark => GLOW_ALPHA_DARK,
            MetalFxResolvedTheme::Light => GLOW_ALPHA_LIGHT,
        }
    }

    fn shadow_color(&self) -> [f32; 4] {
        match self.theme {
            MetalFxResolvedTheme::Dark => [1.0, 1.0, 1.0, 0.02],
            MetalFxResolvedTheme::Light => [0.0, 0.0, 0.0, 0.02],
        }
    }

    /// Uniform block for `layer`.
    pub fn uniforms(&self, layer: Layer) -> Uniforms {
        let m = &self.mode;
        let (origin, size) = self.quad(layer);
        let b = |v: bool| if v { 1.0 } else { 0.0 };
        let (sat, bright) = self.glow_filter();
        let mut u = Uniforms::zeroed();
        u.rect = [origin[0], origin[1], size[0], size[1]];
        u.metrics = [self.width, self.height, self.density, layer as u32 as f32];
        u.shape = [
            self.radius,
            self.ring,
            self.after_hairline_width(),
            self.glow_ring_inset(),
        ];
        u.flags = [
            b(self.circle),
            b(self.theme.is_light()),
            b(self.target_srgb),
            b(self.dither),
        ];
        u.color_back = m.color_back;
        u.color_tint = m.color_tint;
        u.mat0 = [m.repetition, m.softness, m.shift_red, m.shift_blue];
        u.mat1 = [m.distortion, m.contour, m.angle, self.u_time];
        let (so, ss) = (self.mapping.origin(), self.mapping.scale());
        u.sheet = [so[0], so[1], ss[0], ss[1]];
        u.sizing0 = [m.origin_x, m.origin_y, m.world_width, m.world_height];
        u.sizing1 = [m.fit.code(), m.scale, m.rotation, m.offset_x];
        u.sizing2 = [
            m.offset_y,
            SHEET_PIXEL_RATIO,
            SHEET_RESOLUTION,
            self.opacity_mul,
        ];
        u.hair = self.after_hairline();
        u.circ = self.circle_hairline();
        u.glow_misc = [sat, bright, b(self.glow.is_some()), b(self.mask.is_fill())];
        if let Some(g) = &self.glow {
            let f = &g.frame;
            u.halo = [f.position[0], f.position[1], f.tangent, f.halo_opacity];
            u.extra = [
                f.extra_position[0],
                f.extra_position[1],
                f.tangent,
                f.extra_opacity,
            ];
            u.halo_tint = [f.halo_tint[0], f.halo_tint[1], f.halo_tint[2], f.envelope];
            u.extra_tint = [
                f.extra_tint[0],
                f.extra_tint[1],
                f.extra_tint[2],
                self.glow_group_alpha(),
            ];
            let (hw, hh) = g.halo.size();
            let (ha, hb) = g.halo.anchor();
            u.halo_sprite = [hw, hh, ha, hb];
            let (ew, eh) = g.extra.size();
            let (ea, eb) = g.extra.anchor();
            u.extra_sprite = [ew, eh, ea, eb];
        }
        if let Some(r) = &self.rim {
            u.rim = [r.offset_y_px(), r.blur_px(), r.alpha_value(), 1.0];
            let c = r.color_rgb();
            u.rim_color = [c[0], c[1], c[2], 1.0];
        }
        u.surf = self.surface.unwrap_or([0.0; 4]);
        u.shadow = self.shadow_color();
        u.shadow_misc = [SHADOW_SIGMA, 0.0, 0.0, 0.0];
        u
    }
}

// ─── CPU twin of the fragment shader ────────────────────────────────────────

fn clamp01(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

fn sd_rr(p: [f32; 2], half: [f32; 2], r: f32) -> f32 {
    let rr = r.clamp(0.0, half[0].min(half[1]).max(0.0));
    let q = [p[0].abs() - half[0] + rr, p[1].abs() - half[1] + rr];
    let outside = q[0].max(0.0).hypot(q[1].max(0.0));
    outside + q[0].max(q[1]).min(0.0) - rr
}

fn over(top: [f32; 4], bottom: [f32; 4]) -> [f32; 4] {
    let k = 1.0 - top[3];
    [
        top[0] + bottom[0] * k,
        top[1] + bottom[1] * k,
        top[2] + bottom[2] * k,
        top[3] + bottom[3] * k,
    ]
}

fn premul(c: [f32; 4]) -> [f32; 4] {
    [c[0] * c[3], c[1] * c[3], c[2] * c[3], c[3]]
}

fn erf_approx(x: f32) -> f32 {
    let s = x.signum();
    let a = x.abs();
    let t = 1.0 / (1.0 + 0.327_591_1 * a);
    let y = 1.0
        - ((((1.061_405_4 * t - 1.453_152_1) * t + 1.421_413_8) * t - 0.284_496_72) * t
            + 0.254_829_6)
            * t
            * (-a * a).exp();
    s * y
}

fn phi(x: f32) -> f32 {
    0.5 * (1.0 + erf_approx(x * std::f32::consts::FRAC_1_SQRT_2))
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

impl Scene {
    fn cover(&self, sd: f32) -> f32 {
        clamp01(0.5 - sd * self.density)
    }

    fn box_sd(&self, p: [f32; 2]) -> f32 {
        let half = [0.5 * self.width, 0.5 * self.height];
        sd_rr([p[0] - half[0], p[1] - half[1]], half, self.radius)
    }

    fn inset_sd(&self, p: [f32; 2], inset: f32) -> f32 {
        let half = [0.5 * self.width, 0.5 * self.height];
        sd_rr(
            [p[0] - half[0], p[1] - half[1]],
            [(half[0] - inset).max(0.0), (half[1] - inset).max(0.0)],
            (self.radius - inset).max(0.0),
        )
    }

    fn band_cover(&self, p: [f32; 2], inset: f32) -> f32 {
        self.cover(self.box_sd(p)) * (1.0 - self.cover(self.inset_sd(p, inset)))
    }

    /// Premultiplied colour of `layer` at box-local point `p`, before the
    /// sRGB-target conversion. `frag_px` is the fragment position inside the
    /// quad in device px (for dither).
    pub fn shade_layer(&self, layer: Layer, p: [f32; 2], frag_px: [f32; 2]) -> [f32; 4] {
        match layer {
            Layer::Ring => self.shade_ring(p, frag_px),
            Layer::Glow => self.shade_glow(p),
            Layer::Rim => self.shade_rim(p),
            Layer::Surface => self.shade_surface(p),
        }
    }

    /// Final colour as the shader writes it: [`shade_layer`](Self::shade_layer)
    /// plus the linear conversion for sRGB targets.
    pub fn shade(&self, layer: Layer, p: [f32; 2], frag_px: [f32; 2]) -> [f32; 4] {
        let c = self.shade_layer(layer, p, frag_px);
        if self.target_srgb && c[3] > 0.0 {
            let a = c[3];
            let lin = |v: f32| srgb_to_linear(clamp01(v / a)) * a;
            return [lin(c[0]), lin(c[1]), lin(c[2]), a];
        }
        c
    }

    fn mask_cover(&self, p: [f32; 2]) -> f32 {
        match &self.mask {
            MetalFxMask::Ring | MetalFxMask::Fill => 1.0,
            MetalFxMask::Custom(m) => m.sample(p[0], p[1]),
        }
    }

    fn shade_ring(&self, p: [f32; 2], frag_px: [f32; 2]) -> [f32; 4] {
        let sd_o = self.box_sd(p);
        let co = self.cover(sd_o);
        let a = if self.mask.is_fill() {
            co * self.mask_cover(p)
        } else {
            co * (1.0 - self.cover(self.inset_sd(p, self.ring)))
        };
        let mut out = [0.0; 4];
        if a > 0.0 {
            let s = self.mapping.uv(p);
            let m =
                MetalMaterial::new(self.mode).shade(s, self.u_time, self.dither.then_some(frag_px));
            let k = self.opacity_mul * a;
            out = [m[0] * k, m[1] * k, m[2] * k, m[3] * k];
        }
        let circ = self.circle_hairline();
        if !self.mask.is_fill() && self.circle && circ[3] > 0.0 {
            let outer = (self.cover(sd_o - 1.0) - co).max(0.0);
            out = over(premul([circ[0], circ[1], circ[2], circ[3] * outer]), out);
        }
        out
    }

    fn sprite_alpha(sprite: &GlowSprite, p: [f32; 2], pos: [f32; 2], tangent: f32) -> f32 {
        let d = [p[0] - pos[0], p[1] - pos[1]];
        let (s, c) = tangent.sin_cos();
        sprite.sample(c * d[0] + s * d[1], -s * d[0] + c * d[1])
    }

    fn shade_glow(&self, p: [f32; 2]) -> [f32; 4] {
        let Some(g) = &self.glow else {
            return [0.0; 4];
        };
        let f = &g.frame;
        let co = self.cover(self.box_sd(p));
        let ci = self.cover(self.inset_sd(p, self.glow_ring_inset()));
        let m = if self.mask.is_fill() && self.glow_mode == MetalGlowMode::Mask {
            // Clip the halo to the custom mask (metal text).
            self.mask_cover(p)
        } else {
            (1.0 - co) * 0.5 + co * (1.0 - ci)
        };
        let ah = Self::sprite_alpha(&g.halo, p, f.position, f.tangent) * f.halo_opacity * m;
        let ae = Self::sprite_alpha(&g.extra, p, f.extra_position, f.tangent) * f.extra_opacity * m;
        let a = ae + ah * (1.0 - ae);
        if a <= 0.0 {
            return [0.0; 4];
        }
        let straight_at = |k: usize| (f.extra_tint[k] * ae + f.halo_tint[k] * ah * (1.0 - ae)) / a;
        let mut straight = [straight_at(0), straight_at(1), straight_at(2)];
        let (sat, bright) = self.glow_filter();
        if sat != 1.0 || bright != 1.0 {
            let l = straight[0] * 0.213 + straight[1] * 0.715 + straight[2] * 0.072;
            for c in &mut straight {
                *c = clamp01(l + (*c - l) * sat);
                *c = clamp01(*c * bright);
            }
        }
        let alpha = a * f.envelope * self.glow_group_alpha();
        [
            straight[0] * alpha,
            straight[1] * alpha,
            straight[2] * alpha,
            alpha,
        ]
    }

    fn rim_raw(&self, p: [f32; 2], ring: f32, offset_y: f32) -> f32 {
        (self.band_cover(p, ring) - self.band_cover([p[0], p[1] - offset_y], ring)).max(0.0)
    }

    fn shade_rim(&self, p: [f32; 2]) -> [f32; 4] {
        let ring = self.ring;
        let mut out = [0.0; 4];
        if let Some(rim) = &self.rim {
            let a0 = self.band_cover(p, ring);
            if a0 > 0.0 {
                let sigma = rim.blur_px() * self.density;
                let off = rim.offset_y_px();
                let blurred = if sigma > 0.05 {
                    let radius = (3.0 * sigma).ceil().min(3.0) as i32;
                    let (mut acc, mut wsum) = (0.0, 0.0);
                    for j in -radius..=radius {
                        for i in -radius..=radius {
                            let d = [i as f32, j as f32];
                            let w = (-(d[0] * d[0] + d[1] * d[1]) / (2.0 * sigma * sigma)).exp();
                            acc += w * self.rim_raw(
                                [p[0] + d[0] / self.density, p[1] + d[1] / self.density],
                                ring,
                                off,
                            );
                            wsum += w;
                        }
                    }
                    acc / wsum
                } else {
                    self.rim_raw(p, ring, off)
                };
                let a = (blurred * a0 * rim.alpha_value()).min(1.0);
                let c = rim.color_rgb();
                out = [c[0] * a, c[1] * a, c[2] * a, a];
            }
        }
        let hair = self.after_hairline();
        let after = self.band_cover(p, self.after_hairline_width()) * hair[3];
        over(
            [hair[0] * after, hair[1] * after, hair[2] * after, after],
            out,
        )
    }

    fn shade_surface(&self, p: [f32; 2]) -> [f32; 4] {
        let co = self.cover(self.box_sd(p));
        if co <= 0.0 {
            return [0.0; 4];
        }
        let mut out = self.surface.map_or([0.0; 4], premul);
        let sigma = SHADOW_SIGMA;
        let rx = phi(p[0] / sigma) - phi((p[0] - self.width) / sigma);
        let ry = phi(p[1] / sigma) - phi((p[1] - self.height) / sigma);
        let sh = self.shadow_color();
        let sa = sh[3] * (1.0 - rx * ry);
        out = over([sh[0] * sa, sh[1] * sa, sh[2] * sa, sa], out);
        [out[0] * co, out[1] * co, out[2] * co, out[3] * co]
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::widgets::metal_fx::glow_config::MetalGlowConfig;
    use crate::widgets::metal_fx::preset::MetalFxPreset;

    pub(crate) fn scene(theme: MetalFxResolvedTheme, circle: bool) -> Scene {
        let (w, h) = if circle { (36.0, 36.0) } else { (134.0, 40.0) };
        let mode = MetalFxPreset::Chromatic.mode(theme);
        Scene {
            width: w,
            height: h,
            radius: w.min(h) / 2.0,
            ring: if circle { 2.0 } else { 1.0 },
            density: 1.0,
            circle,
            theme,
            mode,
            u_time: 1.0,
            mapping: SheetMapping::new(w, h, if circle { 1.3 } else { 1.6 }),
            opacity_mul: mode.shader_opacity,
            rim: None,
            glow: None,
            surface: Some([0.153, 0.153, 0.153, 1.0]),
            target_srgb: false,
            dither: false,
            mask: MetalFxMask::Ring,
            glow_mode: MetalGlowMode::Mask,
        }
    }

    #[test]
    fn uniform_block_is_26_vec4() {
        assert_eq!(std::mem::size_of::<Uniforms>(), 416);
        assert_eq!(bytemuck::bytes_of(&Uniforms::zeroed()).len(), 416);
    }

    #[test]
    fn ring_is_a_band_around_the_box() {
        let s = scene(MetalFxResolvedTheme::Dark, false);
        let inside = s.shade(Layer::Ring, [67.0, 20.0], [0.0; 2]);
        assert_eq!(inside, [0.0; 4], "the interior is punched out");
        let top = s.shade(Layer::Ring, [67.0, 0.5], [0.0; 2]);
        assert!(top[3] > 0.5, "the band is metal, alpha {}", top[3]);
        let outside = s.shade(Layer::Ring, [67.0, -1.5], [0.0; 2]);
        assert_eq!(outside, [0.0; 4]);
    }

    #[test]
    fn strength_scales_the_ring_alpha() {
        let mut s = scene(MetalFxResolvedTheme::Dark, false);
        let full = s.shade(Layer::Ring, [67.0, 0.5], [0.0; 2]);
        s.opacity_mul *= 0.5;
        let half = s.shade(Layer::Ring, [67.0, 0.5], [0.0; 2]);
        for k in 0..4 {
            assert!((half[k] - full[k] * 0.5).abs() < 1e-6);
        }
    }

    #[test]
    fn circle_variant_has_an_outer_dark_hairline_in_dark_only() {
        let dark = scene(MetalFxResolvedTheme::Dark, true);
        let ring = dark.shade(Layer::Ring, [18.0, -0.5], [0.0; 2]);
        assert!(ring[3] > 0.3, "hairline just outside the box, got {ring:?}");
        assert!(ring[0] < 0.01);
        let light = scene(MetalFxResolvedTheme::Light, true);
        assert_eq!(light.shade(Layer::Ring, [18.0, -0.5], [0.0; 2]), [0.0; 4]);
        let button = scene(MetalFxResolvedTheme::Dark, false);
        assert_eq!(button.shade(Layer::Ring, [67.0, -0.5], [0.0; 2]), [0.0; 4]);
    }

    #[test]
    fn after_hairline_is_a_translucent_band() {
        let dark = scene(MetalFxResolvedTheme::Dark, false);
        let c = dark.shade(Layer::Rim, [67.0, 0.5], [0.0; 2]);
        assert!(
            (c[3] - 0.1).abs() < 1e-5 && (c[0] - 0.1).abs() < 1e-5,
            "{c:?}"
        );
        let light = scene(MetalFxResolvedTheme::Light, false);
        let c = light.shade(Layer::Rim, [67.0, 0.5], [0.0; 2]);
        assert!((c[3] - 0.06).abs() < 1e-5 && c[0] == 0.0, "{c:?}");
        // The circle variant's hairline is 2 px wide.
        let circle = scene(MetalFxResolvedTheme::Dark, true);
        assert!(circle.shade(Layer::Rim, [18.0, 1.5], [0.0; 2])[3] > 0.09);
        assert!(dark.shade(Layer::Rim, [67.0, 1.5], [0.0; 2])[3] < 1e-6);
    }

    #[test]
    fn inner_shadow_lights_only_the_top_of_the_band() {
        let mut s = scene(MetalFxResolvedTheme::Dark, false);
        s.ring = 3.0;
        s.rim = Some(MetalRimOptions::new().blur(0.0));
        let top = s.shade_layer(Layer::Rim, [67.0, 0.5], [0.0; 2]);
        let bottom = s.shade_layer(Layer::Rim, [67.0, 39.5], [0.0; 2]);
        assert!(
            top[3] > 0.8,
            "the upper edge catches the light, got {top:?}"
        );
        assert!(
            bottom[3] < 0.2,
            "the lower outer edge does not, got {bottom:?}"
        );
        // The inner (hole-side) edge of the lower half of the band is lit:
        // the 3 px band spans y = 37..40.
        let inner_lower = s.shade_layer(Layer::Rim, [67.0, 37.5], [0.0; 2]);
        assert!(inner_lower[3] > 0.8, "{inner_lower:?}");
        // Nothing inside the hole.
        assert_eq!(s.shade_layer(Layer::Rim, [67.0, 20.0], [0.0; 2])[3], 0.0);
    }

    #[test]
    fn surface_fills_the_box_and_the_inset_shadow_is_faint() {
        let s = scene(MetalFxResolvedTheme::Dark, false);
        let mid = s.shade(Layer::Surface, [67.0, 20.0], [0.0; 2]);
        assert!(mid[3] > 0.999);
        let edge = s.shade(Layer::Surface, [67.0, 2.0], [0.0; 2]);
        // The 2 % white inset shadow brightens the edge slightly over the middle.
        assert!(
            edge[0] > mid[0] && edge[0] - mid[0] < 0.02,
            "{edge:?} vs {mid:?}"
        );
        assert_eq!(s.shade(Layer::Surface, [-5.0, 20.0], [0.0; 2]), [0.0; 4]);
        let mut t = s.clone();
        t.surface = None;
        let clear = t.shade(Layer::Surface, [67.0, 2.0], [0.0; 2]);
        assert!(clear[3] > 0.0 && clear[3] < 0.02, "shadow only: {clear:?}");
    }

    #[test]
    fn srgb_targets_get_linear_output() {
        let mut s = scene(MetalFxResolvedTheme::Dark, false);
        let plain = s.shade(Layer::Surface, [67.0, 20.0], [0.0; 2]);
        s.target_srgb = true;
        let lin = s.shade(Layer::Surface, [67.0, 20.0], [0.0; 2]);
        assert!(lin[0] < plain[0]);
        assert!((lin[0] - srgb_to_linear(plain[0])).abs() < 1e-4);
        assert_eq!(lin[3], plain[3]);
    }

    #[test]
    fn quads_cover_their_layers() {
        let mut s = scene(MetalFxResolvedTheme::Dark, false);
        let (o, size) = s.quad(Layer::Surface);
        assert_eq!((o, size), ([0.0, 0.0], [134.0, 40.0]));
        let (o, size) = s.quad(Layer::Ring);
        assert_eq!((o, size), ([-2.0, -2.0], [138.0, 44.0]));
        assert!(!s.has_layer(Layer::Glow));
        assert_eq!(s.quad(Layer::Glow).1, [134.0, 40.0]);
        s.glow = None;
    }

    #[test]
    fn uniforms_carry_the_scene() {
        let mut s = scene(MetalFxResolvedTheme::Light, true);
        s.rim = Some(MetalRimOptions::new());
        s.target_srgb = true;
        let u = s.uniforms(Layer::Rim);
        assert_eq!(u.metrics, [36.0, 36.0, 1.0, 2.0]);
        assert_eq!(u.flags, [1.0, 1.0, 1.0, 0.0]);
        assert_eq!(u.shape[0], 18.0);
        assert_eq!(u.rim, [1.0, 0.5, 0.9, 1.0]);
        assert_eq!(u.hair, [0.0, 0.0, 0.0, 0.06]);
        assert_eq!(u.glow_misc[0], 7.5);
        assert_eq!(u.sizing2[1], 2.0);
        assert_eq!(u.sizing2[2], 192.0);
        assert_eq!(u.mat0[0], s.mode.repetition);
        assert_eq!(u.rect, [-2.0, -2.0, 40.0, 40.0]);
    }

    pub(crate) fn glow_scene(theme: MetalFxResolvedTheme) -> Scene {
        use crate::widgets::metal_fx::bake::{bake_extra, bake_halo};
        let mut s = scene(theme, false);
        let cfg = MetalGlowConfig::DEFAULTS;
        let halo = bake_halo(7.8 * 1.0, 1.0, &cfg);
        let extra = bake_extra(cfg.extra_half_len, 1.0, &cfg);
        s.glow = Some(GlowDraw {
            frame: GlowFrame {
                position: [67.0, 1.5],
                extra_position: [67.0, 0.5],
                tangent: 0.0,
                halo_opacity: 1.0,
                extra_opacity: 1.0,
                envelope: 1.0,
                halo_tint: [0.6, 0.8, 1.0],
                extra_tint: [1.0; 3],
                animating: false,
            },
            margin: 42.0,
            halo,
            extra,
        });
        s
    }

    #[test]
    fn glow_is_clipped_to_band_and_half_strength_outside() {
        let s = glow_scene(MetalFxResolvedTheme::Dark);
        let band = s.shade(Layer::Glow, [67.0, 0.5], [0.0; 2]);
        let outside = s.shade(Layer::Glow, [67.0, -3.0], [0.0; 2]);
        let hole = s.shade(Layer::Glow, [67.0, 20.0], [0.0; 2]);
        assert!(band[3] > 0.3, "{band:?}");
        assert!(outside[3] > 0.05, "halo spills outside: {outside:?}");
        assert_eq!(hole[3], 0.0, "no glow inside the hole");
        // Dark glow carries the 0.7 group alpha.
        assert!(band[3] <= 0.7 + 1e-5);
    }

    #[test]
    fn light_glow_is_much_fainter_and_saturated() {
        let dark = glow_scene(MetalFxResolvedTheme::Dark);
        let light = glow_scene(MetalFxResolvedTheme::Light);
        let d = dark.shade(Layer::Glow, [67.0, 0.5], [0.0; 2]);
        let l = light.shade(Layer::Glow, [67.0, 0.5], [0.0; 2]);
        assert!(l[3] < d[3] * 0.5);
        assert!(l[3] <= 0.2746 + 1e-5);
    }

    #[test]
    fn glow_envelope_scales_alpha() {
        let mut s = glow_scene(MetalFxResolvedTheme::Dark);
        let full = s.shade(Layer::Glow, [67.0, 0.5], [0.0; 2]);
        if let Some(g) = s.glow.as_mut() {
            g.frame.envelope = 0.5;
        }
        let half = s.shade(Layer::Glow, [67.0, 0.5], [0.0; 2]);
        assert!((half[3] - full[3] * 0.5).abs() < 1e-5);
    }
}
