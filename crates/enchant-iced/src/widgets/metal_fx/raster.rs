//! Headless rendering: the metal at a fixed time and host size straight to
//! RGBA, for goldens, previews and numeric comparison with the GPU.
//!
//! This is the CPU twin of the shader ([`Scene::shade`] evaluates the same
//! arithmetic as `fs_main`) composed with the web's layer order and blend
//! modes: surface, ring, glow (`normal` on dark, `multiply` on light), rim.

use std::time::Duration;

use super::bake::{bake_extra, bake_halo};
use super::geometry::{rr_perimeter, shape_perimeter};
use super::glow::{GlowFrame, GlowState, GlowTick, MaterialSampler};
use super::glow_config::{MetalGlowConfig, glow_config};
use super::material::{MetalMaterial, SheetMapping};
use super::options::MetalFxOptions;
use super::preset::MetalFxResolvedTheme;
use super::scene::{GlowDraw, Layer, Scene};

const SIM_STEP_MS: f64 = 1000.0 / 60.0;
const SIM_MAX_STEPS: usize = 3000;
const REF_PERIMETER_W: f32 = 140.0;
const REF_PERIMETER_H: f32 = 40.0;
const REF_PERIMETER_R: f32 = 20.0;

/// Size of the host element, logical px.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::MetalFxHost;
///
/// let host = MetalFxHost::new(134.0, 40.0);
/// assert_eq!((host.width(), host.height()), (134.0, 40.0));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MetalFxHost {
    width: f32,
    height: f32,
}

impl MetalFxHost {
    /// A host of `width × height` logical px (clamped to `>= 1`).
    #[must_use]
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width: if width.is_finite() {
                width.max(1.0)
            } else {
                1.0
            },
            height: if height.is_finite() {
                height.max(1.0)
            } else {
                1.0
            },
        }
    }

    /// Width, logical px.
    #[must_use]
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Height, logical px.
    #[must_use]
    pub fn height(&self) -> f32 {
        self.height
    }
}

/// One compositing layer of the metal, see [`MetalRenderOptions::solo`].
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MetalFxLayerKind {
    /// The wrapper background and its inset shadow.
    Surface,
    /// The liquid-metal band (and the circle variant's hairline).
    Ring,
    /// The wandering halo.
    Glow,
    /// The inner-shadow rim and the `::after` hairline.
    Rim,
}

impl MetalFxLayerKind {
    fn layer(self) -> Layer {
        match self {
            Self::Surface => Layer::Surface,
            Self::Ring => Layer::Ring,
            Self::Glow => Layer::Glow,
            Self::Rim => Layer::Rim,
        }
    }
}

/// How [`MetalFxOptions::render_at`] paints.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::{MetalFxLayerKind, MetalRenderOptions};
///
/// let render = MetalRenderOptions::new().density(2.0).solo(MetalFxLayerKind::Ring);
/// assert_eq!(render.density_value(), 2.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
#[must_use = "render options do nothing unless passed to render_at"]
pub struct MetalRenderOptions {
    density: f32,
    background: Option<[u8; 4]>,
    solo: Option<MetalFxLayerKind>,
    dither: bool,
    seed: u64,
    config: Option<MetalGlowConfig>,
}

impl Default for MetalRenderOptions {
    fn default() -> Self {
        Self {
            density: 1.0,
            background: None,
            solo: None,
            dither: false,
            seed: 1,
            config: None,
        }
    }
}

impl MetalRenderOptions {
    /// Density 1, transparent background, every layer, no dither.
    pub fn new() -> Self {
        Self::default()
    }

    /// Device pixels per logical px (clamped to `0.5..=8`).
    pub fn density(mut self, density: f32) -> Self {
        if density.is_finite() {
            self.density = density.clamp(0.5, 8.0);
        }
        self
    }

    /// A straight RGBA colour filling the bitmap under every layer.
    pub fn background(mut self, rgba: [u8; 4]) -> Self {
        self.background = Some(rgba);
        self
    }

    /// Paints only `layer`.
    pub fn solo(mut self, layer: MetalFxLayerKind) -> Self {
        self.solo = Some(layer);
        self
    }

    /// Applies Paper's banding dither (off by default so renders compare
    /// exactly).
    pub fn dither(mut self, dither: bool) -> Self {
        self.dither = dither;
        self
    }

    /// Seed of the glow's deterministic wander.
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Glow tuning for this render; defaults to the live configuration.
    pub fn glow_config(mut self, config: MetalGlowConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// The density.
    #[must_use]
    pub fn density_value(&self) -> f32 {
        self.density
    }
}

/// A rendered frame: straight (non-premultiplied) RGBA8, display-referred.
///
/// The bitmap covers the host box plus whatever the layers overhang by
/// (`left`/`top` are negative offsets from the box's top-left, logical px).
#[derive(Clone, Debug, PartialEq)]
pub struct MetalFxBitmap {
    width: u32,
    height: u32,
    left: f32,
    top: f32,
    density: f32,
    rgba: Vec<u8>,
}

impl MetalFxBitmap {
    /// Width in device px.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Height in device px.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Offset of the bitmap's left edge from the box's, logical px (`<= 0`).
    #[must_use]
    pub fn left(&self) -> f32 {
        self.left
    }

    /// Offset of the bitmap's top edge from the box's, logical px (`<= 0`).
    #[must_use]
    pub fn top(&self) -> f32 {
        self.top
    }

    /// Device px per logical px.
    #[must_use]
    pub fn density(&self) -> f32 {
        self.density
    }

    /// Straight RGBA8, row-major.
    #[must_use]
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    /// One pixel, by device-px coordinates.
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }

    /// The pixel at a box-local logical-px point (its containing device px).
    #[must_use]
    pub fn pixel_at(&self, x: f32, y: f32) -> [u8; 4] {
        let px = (((x - self.left) * self.density).floor().max(0.0) as u32).min(self.width - 1);
        let py = (((y - self.top) * self.density).floor().max(0.0) as u32).min(self.height - 1);
        self.pixel(px, py)
    }
}

/// Everything besides the options that fixes a [`Scene`].
pub(crate) struct SceneParams<'a> {
    pub host: MetalFxHost,
    pub theme: MetalFxResolvedTheme,
    pub density: f32,
    pub elapsed_seconds: f32,
    pub glow: Option<GlowFrame>,
    pub config: &'a MetalGlowConfig,
    pub target_srgb: bool,
    pub bend: Option<super::bend::MetalBendField>,
}

impl MetalFxOptions {
    /// Builds the instance's [`Scene`].
    pub(crate) fn scene(&self, params: &SceneParams<'_>) -> Scene {
        let SceneParams {
            host,
            theme,
            density,
            elapsed_seconds,
            glow,
            config,
            target_srgb,
            bend,
        } = *params;
        let (w, h) = (host.width, host.height);
        let mode = self.resolved_mode(theme);
        let material = MetalMaterial::new(mode);
        let radius = self.resolved_radius(w, h);
        let ratio = shape_perimeter(w, h, radius, self.variant)
            / rr_perimeter(REF_PERIMETER_W, REF_PERIMETER_H, REF_PERIMETER_R);
        let glow = glow.filter(|_| self.glow).map(|frame| {
            let s = self.scale;
            let halo = bake_halo((config.halo_half_len * ratio).max(1.0), s, config);
            let extra = bake_extra((config.extra_half_len * ratio).max(0.6), s, config);
            let margin =
                (halo.anchor().1.max(extra.anchor().1) + config.extra_outward * ratio * s + 2.0)
                    .ceil();
            GlowDraw {
                frame,
                halo,
                extra,
                margin,
            }
        });
        Scene {
            width: w,
            height: h,
            radius,
            ring: self.resolved_ring_px(),
            density,
            circle: self.variant.is_circle(),
            theme,
            mode,
            u_time: material.u_time(elapsed_seconds),
            mapping: SheetMapping::new(w, h, self.resolved_shader_scale()),
            opacity_mul: self.strength * mode.shader_opacity,
            rim: self.inner_shadow,
            glow,
            surface: self.resolved_surface(theme).map(|c| [c.r, c.g, c.b, c.a]),
            target_srgb,
            dither: false,
            mask: self.mask.clone(),
            glow_mode: self.glow_mode,
            bend,
        }
    }

    /// Runs the glow from rest for `elapsed` (60 Hz ticks) and returns its last
    /// frame; deterministic for a given `seed`.
    pub(crate) fn simulate_glow(
        &self,
        host: MetalFxHost,
        theme: MetalFxResolvedTheme,
        elapsed: Duration,
        seed: u64,
        config: &MetalGlowConfig,
    ) -> Option<GlowFrame> {
        if !self.glow {
            return None;
        }
        let mode = self.resolved_mode(theme);
        let material = MetalMaterial::new(mode);
        let radius = self.resolved_radius(host.width, host.height);
        let mapping = SheetMapping::new(host.width, host.height, self.resolved_shader_scale());
        let mut state = GlowState::new(seed);
        state.configure(
            host.width,
            host.height,
            radius,
            self.variant,
            self.scale,
            config,
        );
        let total_ms = elapsed.as_secs_f64() * 1000.0;
        let steps = ((total_ms / SIM_STEP_MS).ceil() as usize).clamp(1, SIM_MAX_STEPS);
        let strength = (self.strength * self.glow_gain).clamp(0.0, 1.0);
        let mut last = None;
        for i in 1..=steps {
            let now = if steps == SIM_MAX_STEPS {
                total_ms * i as f64 / steps as f64
            } else {
                (i as f64 * SIM_STEP_MS).min(total_ms.max(SIM_STEP_MS))
            };
            let sampler =
                MaterialSampler::new(material, mapping, material.u_time((now / 1000.0) as f32));
            last = state.tick(&GlowTick {
                now_ms: now,
                strength,
                theme,
                sampler: &sampler,
                config,
                cursor_light: None,
            });
        }
        last
    }

    /// Renders the effect at `elapsed` into straight RGBA8, without a window.
    ///
    /// Layers composite as on the web: surface, ring, glow (`multiply` in light
    /// mode), rim. The glow is simulated from rest, so the same `elapsed` and
    /// seed always produce the same bitmap. The clock is the caller's:
    /// [`paused`](Self::paused) is a widget concern, pass the frozen time.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use enchant_iced::widgets::metal_fx::{
    ///     MetalFxHost, MetalFxOptions, MetalFxResolvedTheme, MetalFxTheme, MetalRenderOptions,
    /// };
    ///
    /// let bitmap = MetalFxOptions::new()
    ///     .theme(MetalFxTheme::Dark)
    ///     .glow(false)
    ///     .render_at(
    ///         Duration::from_millis(1500),
    ///         MetalFxHost::new(134.0, 40.0),
    ///         MetalFxResolvedTheme::Dark,
    ///         &MetalRenderOptions::new(),
    ///     );
    /// // The top edge of the band is metal; the middle of the box is the surface.
    /// assert!(bitmap.pixel_at(67.0, 0.5)[3] > 0);
    /// assert_eq!(bitmap.pixel_at(67.0, 20.0)[3], 255);
    /// ```
    pub fn render_at(
        &self,
        elapsed: Duration,
        host: MetalFxHost,
        theme: MetalFxResolvedTheme,
        render: &MetalRenderOptions,
    ) -> MetalFxBitmap {
        let config = render.config.unwrap_or_else(glow_config);
        let glow = self.simulate_glow(host, theme, elapsed, render.seed, &config);
        let mut scene = self.scene(&SceneParams {
            host,
            theme,
            density: render.density,
            elapsed_seconds: elapsed.as_secs_f32(),
            glow,
            config: &config,
            target_srgb: false,
            bend: None,
        });
        scene.dither = render.dither;
        let layers: Vec<Layer> = match render.solo {
            Some(kind) => vec![kind.layer()],
            None => vec![Layer::Surface, Layer::Ring, Layer::Glow, Layer::Rim],
        };
        compose(&scene, &layers, render.background)
    }
}

fn compose(scene: &Scene, layers: &[Layer], background: Option<[u8; 4]>) -> MetalFxBitmap {
    let margin = layers
        .iter()
        .filter(|l| scene.has_layer(**l))
        .map(|l| -scene.quad(*l).0[0])
        .fold(0.0_f32, f32::max);
    let d = scene.density;
    let left = -margin;
    let w = ((scene.width + 2.0 * margin) * d).ceil() as u32;
    let h = ((scene.height + 2.0 * margin) * d).ceil() as u32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let bg = background.map_or([0.0; 4], |c| {
        let a = f32::from(c[3]) / 255.0;
        [
            f32::from(c[0]) / 255.0 * a,
            f32::from(c[1]) / 255.0 * a,
            f32::from(c[2]) / 255.0 * a,
            a,
        ]
    });
    for py in 0..h {
        for px in 0..w {
            let p = [left + (px as f32 + 0.5) / d, left + (py as f32 + 0.5) / d];
            let frag = [px as f32 + 0.5, py as f32 + 0.5];
            let mut acc = bg;
            for layer in layers {
                if !scene.has_layer(*layer) {
                    continue;
                }
                let (origin, size) = scene.quad(*layer);
                if p[0] < origin[0]
                    || p[1] < origin[1]
                    || p[0] >= origin[0] + size[0]
                    || p[1] >= origin[1] + size[1]
                {
                    continue;
                }
                let c = scene.shade(*layer, p, frag);
                if *layer == Layer::Glow && scene.theme.is_light() {
                    // CSS `mix-blend-mode: multiply`, source premultiplied.
                    let k = 1.0 - c[3];
                    acc = [
                        c[0] * acc[0] + acc[0] * k,
                        c[1] * acc[1] + acc[1] * k,
                        c[2] * acc[2] + acc[2] * k,
                        acc[3],
                    ];
                } else {
                    let k = 1.0 - c[3];
                    acc = [
                        c[0] + acc[0] * k,
                        c[1] + acc[1] * k,
                        c[2] + acc[2] * k,
                        c[3] + acc[3] * k,
                    ];
                }
            }
            let i = ((py * w + px) * 4) as usize;
            let a = acc[3];
            let un = |v: f32| {
                if a > 0.0 {
                    ((v / a).clamp(0.0, 1.0) * 255.0).round() as u8
                } else {
                    0
                }
            };
            rgba[i] = un(acc[0]);
            rgba[i + 1] = un(acc[1]);
            rgba[i + 2] = un(acc[2]);
            rgba[i + 3] = (a.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
    MetalFxBitmap {
        width: w,
        height: h,
        left,
        top: left,
        density: d,
        rgba,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::metal_fx::preset::MetalFxTheme;

    fn dark() -> MetalFxOptions {
        MetalFxOptions::new().theme(MetalFxTheme::Dark)
    }

    fn render(
        o: &MetalFxOptions,
        theme: MetalFxResolvedTheme,
        ms: u64,
        r: &MetalRenderOptions,
    ) -> MetalFxBitmap {
        o.render_at(
            Duration::from_millis(ms),
            MetalFxHost::new(134.0, 40.0),
            theme,
            r,
        )
    }

    /// Sum of RGB over the 134x40 box plus a 2 px margin (bitmaps differ in
    /// size when one of them carries a glow).
    fn region_sum(b: &MetalFxBitmap) -> u64 {
        let mut sum = 0;
        for y in -2..42 {
            for x in -2..136 {
                let p = b.pixel_at(x as f32 + 0.5, y as f32 + 0.5);
                sum += u64::from(p[0]) + u64::from(p[1]) + u64::from(p[2]);
            }
        }
        sum
    }

    #[test]
    fn host_clamps() {
        let h = MetalFxHost::new(-3.0, f32::NAN);
        assert_eq!((h.width(), h.height()), (1.0, 1.0));
    }

    #[test]
    fn render_is_deterministic() {
        let o = dark();
        let r = MetalRenderOptions::new();
        let a = render(&o, MetalFxResolvedTheme::Dark, 900, &r);
        let b = render(&o, MetalFxResolvedTheme::Dark, 900, &r);
        assert_eq!(a, b);
    }

    #[test]
    fn solo_ring_has_only_the_band() {
        let o = dark().glow(false);
        let r = MetalRenderOptions::new().solo(MetalFxLayerKind::Ring);
        let b = render(&o, MetalFxResolvedTheme::Dark, 500, &r);
        assert!(b.pixel_at(67.0, 0.5)[3] > 100);
        assert_eq!(b.pixel_at(67.0, 20.0)[3], 0);
        // Margin of 2 px around the box for hairlines.
        assert_eq!((b.left(), b.top()), (-2.0, -2.0));
        assert_eq!(b.width(), 138);
    }

    #[test]
    fn layers_compose_in_order() {
        let o = dark().glow(false);
        let full = render(
            &o,
            MetalFxResolvedTheme::Dark,
            500,
            &MetalRenderOptions::new(),
        );
        // Interior: the surface only (the ring does not reach it), plus the
        // 2 % `::before` inset shadow, which is ~2 levels at the centre of a
        // 40 px tall box (50 px blur).
        let centre = full.pixel_at(67.0, 20.0);
        assert!((0x28..=0x2b).contains(&centre[0]), "{centre:?}");
        assert_eq!(centre[3], 255);
        let ring = render(
            &o,
            MetalFxResolvedTheme::Dark,
            500,
            &MetalRenderOptions::new().solo(MetalFxLayerKind::Ring),
        );
        // On the band the metal sits over the surface at its own alpha.
        let m = ring.pixel_at(67.0, 0.5);
        let f = full.pixel_at(67.0, 0.5);
        assert_eq!(f[3], 255);
        let a = f32::from(m[3]) / 255.0;
        // ...and the `::after` hairline (white 10 %) sits over both.
        let under = f32::from(m[0]) * a + 41.0 * (1.0 - a);
        let expect = under * 0.9 + 255.0 * 0.1;
        assert!(
            (f32::from(f[0]) - expect).abs() < 2.5,
            "{f:?} vs ring {m:?}, expected {expect}"
        );
    }

    #[test]
    fn strength_dims_the_metal() {
        let r = MetalRenderOptions::new().solo(MetalFxLayerKind::Ring);
        let full = render(&dark().glow(false), MetalFxResolvedTheme::Dark, 500, &r);
        let half = render(
            &dark().glow(false).strength(0.4),
            MetalFxResolvedTheme::Dark,
            500,
            &r,
        );
        let (a, b) = (full.pixel_at(67.0, 0.5)[3], half.pixel_at(67.0, 0.5)[3]);
        assert!(
            b < a && (f32::from(b) - f32::from(a) * 0.4).abs() < 3.0,
            "{a} -> {b}"
        );
    }

    #[test]
    fn time_moves_the_stripes_and_paused_time_freezes_them() {
        let o = dark().glow(false);
        let r = MetalRenderOptions::new().solo(MetalFxLayerKind::Ring);
        let a = render(&o, MetalFxResolvedTheme::Dark, 0, &r);
        let b = render(&o, MetalFxResolvedTheme::Dark, 4000, &r);
        let c = render(&o, MetalFxResolvedTheme::Dark, 4000, &r);
        assert_ne!(a.rgba(), b.rgba());
        assert_eq!(b.rgba(), c.rgba());
    }

    #[test]
    fn glow_adds_light_around_the_ring() {
        let r = MetalRenderOptions::new();
        let with = render(&dark(), MetalFxResolvedTheme::Dark, 2000, &r);
        let without = render(&dark().glow(false), MetalFxResolvedTheme::Dark, 2000, &r);
        assert_ne!(with.rgba(), without.rgba());
        // Glow never darkens the page in dark mode: compare total luminance.
        assert!(region_sum(&with) >= region_sum(&without));
    }

    #[test]
    fn light_glow_multiplies_and_darkens() {
        let o = MetalFxOptions::new().theme(MetalFxTheme::Light);
        let r = MetalRenderOptions::new().background([255, 255, 255, 255]);
        let with = render(&o, MetalFxResolvedTheme::Light, 2000, &r);
        let without = render(
            &o.clone().glow(false),
            MetalFxResolvedTheme::Light,
            2000,
            &r,
        );
        assert!(
            region_sum(&with) < region_sum(&without),
            "multiply glow darkens the page"
        );
    }

    #[test]
    fn inner_shadow_brightens_the_top_edge() {
        let r = MetalRenderOptions::new().solo(MetalFxLayerKind::Rim);
        let rim = render(
            &dark().glow(false).inner_shadow(true).ring_px(3.0),
            MetalFxResolvedTheme::Dark,
            100,
            &r,
        );
        let none = render(
            &dark().glow(false).ring_px(3.0),
            MetalFxResolvedTheme::Dark,
            100,
            &r,
        );
        assert!(rim.pixel_at(67.0, 0.5)[3] > none.pixel_at(67.0, 0.5)[3] + 100);
    }

    #[test]
    fn density_scales_the_bitmap() {
        let o = dark().glow(false);
        let one = render(
            &o,
            MetalFxResolvedTheme::Dark,
            0,
            &MetalRenderOptions::new(),
        );
        let two = render(
            &o,
            MetalFxResolvedTheme::Dark,
            0,
            &MetalRenderOptions::new().density(2.0),
        );
        assert_eq!(two.width(), one.width() * 2);
        assert_eq!(two.density(), 2.0);
    }

    #[test]
    fn background_shows_through_transparent_pixels() {
        let o = dark().glow(false);
        let b = render(
            &o,
            MetalFxResolvedTheme::Dark,
            0,
            &MetalRenderOptions::new()
                .solo(MetalFxLayerKind::Ring)
                .background([10, 20, 30, 255]),
        );
        assert_eq!(b.pixel_at(67.0, 20.0), [10, 20, 30, 255]);
    }

    #[test]
    fn simulated_glow_converges_to_a_visible_frame() {
        let o = dark();
        let f = o
            .simulate_glow(
                MetalFxHost::new(134.0, 40.0),
                MetalFxResolvedTheme::Dark,
                Duration::from_millis(2500),
                1,
                &MetalGlowConfig::DEFAULTS,
            )
            .unwrap();
        assert!((f.envelope - 1.0).abs() < 1e-3);
        assert!(f.halo_opacity > 0.3);
        assert!(
            o.clone()
                .glow(false)
                .simulate_glow(
                    MetalFxHost::new(134.0, 40.0),
                    MetalFxResolvedTheme::Dark,
                    Duration::from_millis(10),
                    1,
                    &MetalGlowConfig::DEFAULTS
                )
                .is_none()
        );
    }
}
