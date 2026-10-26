//! Resolved configuration: what `VoiceBeam.tsx` computes from its props
//! (`driverConfig`, the theme and type presets) and what `styles.ts` bakes
//! into the stylesheet (layer shapes, opacities, fades, palettes).

use super::options::{ResolvedTheme, VoiceGlow, VoiceGlowType};
use super::palette::{BandPalette, ColorVariant, LobePalette, palette};
use super::presets::{resolve_defaults, resolve_style, theme_preset};
use super::processing::ProcessingTuning;

/// The theme-and-type independent default corner radius (`DEFAULT_RADIUS`).
pub const DEFAULT_RADIUS: f64 = 16.0;
/// Width of the edge stroke ring, px (`BORDER_WIDTH`).
pub const BORDER_WIDTH: f64 = 1.0;

/// JS `Math.round`.
fn js_round(v: f64) -> f64 {
    (v + 0.5).floor()
}

/// `Math.round(v * 10) / 10`: the stylesheet's `px()`.
fn px(v: f64) -> f64 {
    js_round(v * 10.0) / 10.0
}

/// `Number(v.toFixed(2))`.
fn fixed2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// How one lobe layer (stroke, inner light or bloom) shapes its gradients.
///
/// A lobe's ellipse is `round(lobe.w * width_scale) * frame.width_scale` wide
/// and `round(lobe.h * height_scale) * frame.height_scale * lobe.amplitude`
/// tall, centred `y` px below the bottom edge, fading to transparent at
/// `fade_percent` of its radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LobeLayer {
    /// Multiplier on the lobe's resting width.
    pub width_scale: f64,
    /// Multiplier on the lobe's resting height.
    pub height_scale: f64,
    /// Vertical offset of the lobe centre from the bottom edge, px (positive
    /// is below).
    pub y: f64,
    /// Colour alpha of each lobe (1 for the crisp stroke).
    pub alpha: f64,
    /// Where the gradient fades out, 0-100.
    pub fade_percent: f64,
}

/// The hot white spot at the centre of the edge (the stroke layer's
/// `highlight` gradient): black on the light theme so the edge still reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HotSpot {
    /// Horizontal radius at resting width, px.
    pub rx: f64,
    /// Vertical radius at resting height, px.
    pub ry: f64,
    /// `true` for ink-black on the light theme, white on dark.
    pub black: bool,
    /// Gradient stops after the centre: `(position 0-1, alpha)`. The last
    /// one is the point where the gradient reaches transparent.
    pub stops: [(f64, f64); 2],
    /// Alpha at the very centre.
    pub centre_alpha: f64,
}

/// The epicentre wash (`[data-voice-beam-core]`): a soft white wash under the
/// band line. Past `core_light` 1 the solid core widens and the whole wash
/// grows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoreLightStyle {
    /// Horizontal radius at resting width, px.
    pub rx: f64,
    /// Vertical radius at resting height, px.
    pub ry: f64,
    /// Solid white until this percentage of the radius.
    pub solid_percent: f64,
    /// The mid stop's position, percent.
    pub mid_percent: f64,
    /// The mid stop's alpha.
    pub mid_alpha: f64,
    /// Where it reaches transparent, percent.
    pub end_percent: f64,
    /// `min(1, core_light)`, to two decimals.
    pub opacity_base: f64,
    /// The opacity gain, `1.6 + 1.4 * boost`, to two decimals.
    pub opacity_gain: f64,
    /// Blur radius of the wash, px.
    pub blur: f64,
}

/// Everything the stylesheet fixes once per config.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct VoiceGlowStyle {
    /// The seven lobe colours, custom slots applied.
    pub colors: LobePalette,
    /// Final opacity multiplier of the stroke layer (theme x prop x mono).
    pub stroke_opacity: f64,
    /// Final opacity multiplier of the inner light.
    pub inner_opacity: f64,
    /// Final opacity multiplier of the bloom.
    pub bloom_opacity: f64,
    /// Brightness of the filter chain.
    pub brightness: f64,
    /// Saturation of the filter chain.
    pub saturation: f64,
    /// The theme's resting hue shift in degrees, added to the drift.
    pub hue_base: f64,
    /// The inner inset shadow as `(r, g, b, alpha)`.
    pub inner_shadow: (u8, u8, u8, f64),
    /// Blur radius of the inset shadow, px.
    pub inset_blur: f64,
    /// Corner fade of the inner layers' masks, px.
    pub corner_fade: f64,
    /// Corner radius, px.
    pub border_radius: f64,
    /// Corner radius of the stroke ring, px.
    pub inner_radius: f64,
    /// Stroke ring width, px.
    pub border_width: f64,
    /// Percent where lobe gradients fade out (`softness`).
    pub fade_percent: f64,
    /// Blur radius of the bloom, px.
    pub bloom_blur: f64,
    /// The crisp edge stroke's lobes.
    pub stroke: LobeLayer,
    /// The soft inner light's lobes.
    pub inner: LobeLayer,
    /// The bloom's lobes.
    pub bloom: LobeLayer,
    /// The hot spot at the centre of the edge.
    pub hot_spot: HotSpot,
    /// The epicentre wash, when `core_light` is above 0.
    pub core_light: Option<CoreLightStyle>,
    /// Base frequencies `(x, y)` of the distortion's fractal noise
    /// (two octaves, seed 7).
    pub noise_frequency: (f64, f64),
}

/// The driver's resolved configuration (`VoiceDriverConfig`), with the
/// component's pixel-scaling already applied.
///
/// Fields are public because this is a read-only snapshot: build it with
/// [`VoiceGlow::config`].
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct VoiceGlowConfig {
    /// The resolved theme.
    pub theme: ResolvedTheme,
    /// The host preset.
    pub kind: VoiceGlowType,
    /// The colour variant.
    pub color_variant: ColorVariant,
    /// Input gain.
    pub sensitivity: f64,
    /// Noise gate, 0-0.95.
    pub threshold: f64,
    /// Attack, seconds.
    pub attack: f64,
    /// Release, seconds.
    pub release: f64,
    /// Resting presence, 0-1.
    pub idle: f64,
    /// Idle breathing period, seconds (at least 0.2).
    pub breathe_duration: f64,
    /// Height growth at full level.
    pub reach: f64,
    /// Width growth at full level.
    pub spread: f64,
    /// Independent bands on.
    pub bands: bool,
    /// Flow speed, px/s at full level (already times `scale`).
    pub flow: f64,
    /// Multiplier on the resting distance between lobes (times `scale`).
    pub lobe_spacing: f64,
    /// Bend height, px (times `scale`).
    pub bend: f64,
    /// Band opacity multiplier.
    pub band_strength: f64,
    /// Band thickness (times `scale`).
    pub band_width: f64,
    /// Band peak as a fraction of the ceiling.
    pub band_position: f64,
    /// Bell exponent.
    pub band_curve: f64,
    /// Bell width.
    pub band_spread: f64,
    /// Bell skew, -0.9..0.9.
    pub band_skew: f64,
    /// Vertical shift, px (times `scale`).
    pub band_offset: f64,
    /// Tail lift, 0-1.5.
    pub band_tail: f64,
    /// Where the tail starts, 0-0.98.
    pub band_tail_position: f64,
    /// Tail exponent.
    pub band_tail_curve: f64,
    /// Px the band runs past each side (times `scale`).
    pub band_tail_overflow: f64,
    /// Chromatic split, 0-1.
    pub band_aberration: f64,
    /// Ceiling width multiplier (times `scale`).
    pub range_width: f64,
    /// Ceiling height multiplier (times `scale`).
    pub range_height: f64,
    /// The band's colours.
    pub band_colors: BandPalette,
    /// Distortion, 0-1.
    pub distortion: f64,
    /// Epicentre wash opacity, 0-3.
    pub core_light: f64,
    /// The effect's overall scale (at least 0.05).
    pub scale: f64,
    /// The host's corner radius, px.
    pub radius: f64,
    /// Hue drift range, degrees.
    pub hue_range: f64,
    /// Hue drift period, seconds (at least 0.5).
    pub hue_duration: f64,
    /// Hue drift off.
    pub static_colors: bool,
    /// Reduced motion.
    pub reduced_motion: bool,
    /// Frozen.
    pub paused: bool,
    /// Effect on.
    pub active: bool,
    /// Overall strength, 0-1.
    pub strength: f64,
    /// Opacity multiplier of the band (--voice-band-opacity).
    pub band_opacity: f64,
    /// Opacity multiplier of the epicentre wash (--voice-core-light-opacity).
    pub core_light_opacity: f64,
    /// The processing sweep is on.
    pub processing: bool,
    /// The processing sweep's tuning.
    pub processing_tuning: ProcessingTuning,
    /// The baked layer style.
    pub style: VoiceGlowStyle,
}

impl VoiceGlow {
    /// Resolves the props the way `VoiceBeam.tsx` does: the type preset
    /// supplies geometry defaults, an explicit setter wins, `scale` then
    /// multiplies every pixel dimension as one, and the theme (with `Auto`
    /// taken from `system`) supplies colour defaults.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use enchant_iced::widgets::voice_glow::{ResolvedTheme, VoiceGlow, VoiceGlowType};
    ///
    /// let config = VoiceGlow::new().kind(VoiceGlowType::Pill).config(ResolvedTheme::Dark);
    /// // `bend` 23 times `scale` 0.45.
    /// assert!((config.bend - 23.0 * 0.45).abs() < 1e-12);
    /// assert_eq!(config.style.brightness, 1.35);
    /// ```
    #[must_use]
    pub fn config(&self, system: ResolvedTheme) -> VoiceGlowConfig {
        let theme = self.theme.resolve(system);
        let dark = theme.is_dark();
        let d = self.geometry.apply(resolve_defaults(self.kind, theme));
        let sc = d.scale.max(0.05);
        let preset = theme_preset(theme);
        let type_style = resolve_style(self.kind, theme);

        let strength = self
            .strength
            .or(type_style.strength)
            .or(preset.strength)
            .unwrap_or(1.0);
        let hue_range = self
            .hue_range
            .map(|d| d.get())
            .or(preset.hue_range)
            .unwrap_or(24.0);
        let hue_duration = self
            .hue_duration
            .map(|d| d.as_secs_f64())
            .or(preset.hue_duration)
            .unwrap_or(12.0);
        let brightness = self
            .brightness
            .or(type_style.brightness)
            .unwrap_or(preset.brightness);
        let saturation = self
            .saturation
            .or(type_style.saturation)
            .unwrap_or(preset.saturation);
        let radius = self.border_radius.unwrap_or(DEFAULT_RADIUS);

        let glow_width = d.glow_width * sc;
        let glow_height = d.glow_height * sc;
        let range_width = d.range_width * sc;
        let range_height = d.range_height * sc;
        let core_size = d.core_size * sc;
        let core_light = d.core_light.clamp(0.0, 3.0);
        let distortion_detail = d.distortion_detail / sc;

        let band_defaults = BandPalette::theme_default(theme);
        let band_colors = BandPalette {
            core: self.band_colors.core.unwrap_or(band_defaults.core),
            above: self.band_colors.above.unwrap_or(band_defaults.above),
            mid: self.band_colors.mid.unwrap_or(band_defaults.mid),
            below: self.band_colors.below.unwrap_or(band_defaults.below),
        };

        // ── styles.ts: generateVoiceBeamCSS ──────────────────────────────
        let is_mono = self.color_variant == ColorVariant::Mono;
        let mono_mul = if is_mono { 0.6 } else { 1.0 };
        let mut colors = palette(self.color_variant, theme);
        for (slot, custom) in colors.iter_mut().zip(self.colors) {
            if let Some(c) = custom {
                *slot = c;
            }
        }
        let fade = js_round((70.0 * d.softness).clamp(40.0, 95.0));
        let glow_size = (d.glow_size * sc).max(0.0);
        // `scaleBlur` (two decimals, never below 0.5) as the stylesheet then
        // prints it through `px()` (one decimal).
        let blur = |base: f64| px(((base * glow_size * 100.0).round() / 100.0).max(0.5));
        let layer = |sw: f64, sh: f64, y: f64, alpha: f64, fade_percent: f64| LobeLayer {
            width_scale: glow_width * sw,
            height_scale: glow_height * sh,
            y,
            alpha,
            fade_percent,
        };
        let hot_spot = if dark {
            HotSpot {
                rx: px(30.0 * core_size),
                ry: px(30.0 * core_size),
                black: false,
                stops: [(0.30, 0.14), (0.65, 0.0)],
                centre_alpha: 0.45,
            }
        } else {
            HotSpot {
                rx: px(40.0 * core_size),
                ry: px(30.0 * core_size),
                black: true,
                stops: [(0.35, 0.22), (0.70, 0.0)],
                centre_alpha: 0.55,
            }
        };
        let core_light_style = (core_light > 0.0).then(|| {
            // Past 1 the wash keeps growing: white is already white, so
            // "brighter" means a wider solid core, a later fade and a larger
            // ellipse. Two stops of boost, to 3.
            let boost = (core_light - 1.0).clamp(0.0, 2.0);
            let b1 = boost.min(1.0);
            let b2 = (boost - 1.0).max(0.0);
            let grow = 1.0 + 0.3 * boost;
            CoreLightStyle {
                rx: px(120.0 * d.core_light_width * grow * sc),
                ry: px(70.0 * d.core_light_height * grow * sc),
                solid_percent: px(45.0 * b1 + 27.0 * b2),
                mid_percent: px(40.0 + 25.0 * b1 + 15.0 * b2),
                mid_alpha: fixed2((0.55 + 0.35 * b1 + 0.1 * b2).min(1.0)),
                end_percent: px(72.0 + 14.0 * b1 + 8.0 * b2),
                opacity_base: fixed2(core_light.min(1.0)),
                opacity_gain: fixed2(1.6 + 1.4 * boost),
                blur: blur(8.0),
            }
        });
        let style = VoiceGlowStyle {
            colors,
            stroke_opacity: fixed2(preset.stroke_opacity * d.stroke_opacity * mono_mul),
            inner_opacity: fixed2(preset.inner_opacity * d.inner_opacity * mono_mul),
            bloom_opacity: fixed2(preset.bloom_opacity * d.bloom_opacity * mono_mul),
            brightness: fixed2(brightness),
            saturation: fixed2(saturation),
            hue_base: self
                .hue_base
                .map(|d| d.get())
                .or(preset.hue_base)
                .unwrap_or(0.0),
            inner_shadow: preset.inner_shadow,
            inset_blur: js_round(9.0 * sc * 10.0) / 10.0,
            corner_fade: js_round(28.0 * sc * 10.0) / 10.0,
            border_radius: radius,
            inner_radius: (radius - BORDER_WIDTH).max(0.0),
            border_width: BORDER_WIDTH,
            fade_percent: fade,
            bloom_blur: blur(10.0),
            stroke: layer(d.stroke_scale, d.stroke_scale, 2.0, 1.0, fade),
            inner: layer(
                0.9 * d.inner_scale,
                0.9 * d.inner_scale * d.inner_height,
                0.0,
                0.46,
                fade,
            ),
            bloom: layer(
                1.15 * d.bloom_scale,
                1.5 * d.bloom_scale * d.bloom_height,
                0.0,
                if dark { 0.9 } else { 0.7 },
                (fade + 2.0).min(95.0),
            ),
            hot_spot,
            core_light: core_light_style,
            noise_frequency: (
                (0.012 * distortion_detail * 1e4).round() / 1e4,
                (0.05 * distortion_detail * 1e4).round() / 1e4,
            ),
        };

        VoiceGlowConfig {
            theme,
            kind: self.kind,
            color_variant: self.color_variant,
            sensitivity: self.sensitivity.max(0.0),
            threshold: self.threshold.clamp(0.0, 0.95),
            attack: self.attack.as_secs_f64(),
            release: self.release.as_secs_f64(),
            idle: d.idle.clamp(0.0, 1.0),
            breathe_duration: self.breathe_duration.as_secs_f64().max(0.2),
            reach: d.reach.max(0.0),
            spread: d.spread.max(0.0),
            bands: self.bands,
            flow: d.flow * sc,
            lobe_spacing: (d.lobe_spacing * sc).max(0.1),
            bend: (d.bend * sc).max(0.0),
            band_strength: d.band_strength.max(0.0),
            band_width: (d.band_width * sc).max(0.0),
            band_position: d.band_position.max(0.0),
            band_curve: d.band_curve.max(0.3),
            band_spread: d.band_spread.max(0.05),
            band_skew: d.band_skew.clamp(-0.9, 0.9),
            band_offset: d.band_offset * sc,
            band_tail: d.band_tail.clamp(0.0, 1.5),
            band_tail_position: d.band_tail_position.clamp(0.0, 0.98),
            band_tail_curve: d.band_tail_curve.max(0.5),
            band_tail_overflow: (d.band_tail_overflow * sc).max(0.0),
            band_aberration: d.band_aberration.clamp(0.0, 1.0),
            range_width,
            range_height,
            band_colors,
            distortion: d.distortion.clamp(0.0, 1.0),
            core_light,
            scale: sc,
            radius,
            hue_range: hue_range.max(0.0),
            hue_duration: hue_duration.max(0.5),
            static_colors: is_mono || self.static_colors,
            reduced_motion: self.reduced_motion,
            paused: self.paused,
            active: self.active,
            strength: strength.clamp(0.0, 1.0),
            band_opacity: self.band_opacity.max(0.0),
            core_light_opacity: self.core_light_opacity.max(0.0),
            processing: self.processing,
            processing_tuning: self
                .processing_tuning
                .unwrap_or_else(|| ProcessingTuning::for_type(self.kind)),
            style,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::voice_glow::{Degrees, Rgb};

    #[test]
    fn dark_default_matches_the_component() {
        let c = VoiceGlow::new().config(ResolvedTheme::Dark);
        assert_eq!(c.sensitivity, 3.1);
        assert_eq!(c.attack, 0.325);
        assert_eq!(c.release, 0.86);
        assert_eq!(c.idle, 0.18);
        assert_eq!(c.hue_range, 24.0);
        assert_eq!(c.hue_duration, 12.0);
        assert_eq!(c.strength, 1.0);
        assert_eq!(c.radius, 16.0);
        assert_eq!(c.style.brightness, 1.15);
        assert_eq!(c.style.saturation, 1.2);
        assert_eq!(c.style.stroke_opacity, 1.16);
        assert_eq!(c.style.fade_percent, 75.0);
        assert!(c.style.core_light.is_none());
    }

    #[test]
    fn light_default_uses_the_light_theme() {
        let c = VoiceGlow::new()
            .theme(crate::widgets::voice_glow::VoiceGlowTheme::Light)
            .config(ResolvedTheme::Dark);
        assert_eq!(c.theme, ResolvedTheme::Light);
        assert_eq!((c.hue_range, c.hue_duration, c.strength), (40.0, 8.5, 0.8));
        assert_eq!(c.style.brightness, 0.95);
        assert_eq!(c.style.saturation, 1.6);
        assert_eq!(c.style.hue_base, 5.0);
        assert_eq!(c.reach, 1.8);
        let core = c.style.core_light.expect("light has an epicentre");
        // boost 0.8: b1 0.8, grow 1.24.
        assert_eq!(core.solid_percent, 36.0);
        assert_eq!(core.mid_percent, 60.0);
        assert_eq!(core.end_percent, 83.2);
        assert_eq!(core.opacity_gain, 2.72);
    }

    #[test]
    fn auto_theme_follows_the_system() {
        let glow = VoiceGlow::new().theme(crate::widgets::voice_glow::VoiceGlowTheme::Auto);
        assert_eq!(
            glow.config(ResolvedTheme::Light).theme,
            ResolvedTheme::Light
        );
        assert_eq!(glow.config(ResolvedTheme::Dark).theme, ResolvedTheme::Dark);
    }

    #[test]
    fn explicit_props_beat_presets_and_scale_multiplies() {
        let c = VoiceGlow::new()
            .kind(VoiceGlowType::Mobile)
            .scale(2.0)
            .bend(10.0)
            .strength(7.0)
            .hue_range(Degrees::new(-5.0))
            .brightness(0.5)
            .config(ResolvedTheme::Dark);
        assert_eq!(c.scale, 2.0);
        assert_eq!(c.bend, 20.0);
        assert_eq!(c.flow, 120.0);
        assert_eq!(c.strength, 1.0);
        assert_eq!(c.hue_range, 0.0);
        assert_eq!(c.style.brightness, 0.5);
        // The type's own saturation still applies.
        assert_eq!(c.style.saturation, 1.5);
    }

    #[test]
    fn clamps_match_the_driver_config() {
        let c = VoiceGlow::new()
            .band_skew(5.0)
            .band_curve(0.0)
            .band_tail(9.0)
            .threshold(2.0)
            .scale(0.0)
            .lobe_spacing(0.0)
            .config(ResolvedTheme::Dark);
        assert_eq!(c.band_skew, 0.9);
        assert_eq!(c.band_curve, 0.3);
        assert_eq!(c.band_tail, 1.5);
        assert_eq!(c.threshold, 0.95);
        assert_eq!(c.scale, 0.05);
        assert_eq!(c.lobe_spacing, 0.1);
    }

    #[test]
    fn mono_is_static_and_dims_layers() {
        let c = VoiceGlow::new()
            .color_variant(ColorVariant::Mono)
            .config(ResolvedTheme::Dark);
        assert!(c.static_colors);
        assert_eq!(c.style.stroke_opacity, 0.7);
        assert_eq!(c.style.inner_opacity, 0.28);
    }

    #[test]
    fn custom_colours_fill_named_slots_only() {
        let red = Rgb::new(255, 0, 0);
        let c = VoiceGlow::new()
            .colors_css(["#f00", "nope"])
            .config(ResolvedTheme::Dark);
        let base = palette(ColorVariant::Colorful, ResolvedTheme::Dark);
        assert_eq!(c.style.colors[0], red);
        assert_eq!(c.style.colors[1], base[1]);
        let d = VoiceGlow::new()
            .color_slot(6, red)
            .config(ResolvedTheme::Dark);
        assert_eq!(d.style.colors[6], red);
        assert_eq!(d.style.colors[0], base[0]);
    }
}
