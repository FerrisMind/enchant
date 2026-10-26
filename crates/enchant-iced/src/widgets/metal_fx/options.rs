//! Builder options of a [`MetalFx`](super::MetalFx), usable without a widget
//! tree for headless rendering. They carry every public prop of the reference
//! `MetalFxProps` that has an iced meaning.

use iced::Color;

use super::geometry::MetalFxVariant;
use super::preset::{MetalFxPreset, MetalFxResolvedTheme, MetalFxTheme, MetalPresetMode};
use super::rim::MetalRimOptions;

/// The wrapper's background under the child (`.metal-fx-root` background).
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::MetalFxSurface;
///
/// assert_eq!(MetalFxSurface::default(), MetalFxSurface::Themed);
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum MetalFxSurface {
    /// `#272727` on dark, `#ffffff` on light (the reference default).
    #[default]
    Themed,
    /// A custom surface colour.
    Solid(Color),
    /// Nothing under the child.
    Transparent,
}

/// Where the metal layers sit relative to the child.
///
/// The reference hoists the child above every overlay and strips its own
/// chrome (`normalizeHostStyles`). iced cannot restyle an arbitrary child, so
/// the default draws the metal *over* it: every overlay is confined to the
/// ring band and the halo outside the box, so the visible result is the same
/// unless the child paints over its own edge.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::MetalFxLayer;
///
/// assert_eq!(MetalFxLayer::default(), MetalFxLayer::OverChild);
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MetalFxLayer {
    /// Metal drawn above the child (safe with opaque children).
    #[default]
    OverChild,
    /// Metal drawn below the child, as on the web; the child must not paint
    /// over its own border band.
    BehindChild,
}

/// Options of the metal effect.
///
/// Numeric setters clamp to their documented ranges; non-finite values are
/// ignored and keep the current value.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::{
///     MetalFxOptions, MetalFxPreset, MetalFxResolvedTheme, MetalFxTheme, MetalFxVariant,
/// };
///
/// let options = MetalFxOptions::new()
///     .variant(MetalFxVariant::Circle)
///     .preset(MetalFxPreset::Gold)
///     .theme(MetalFxTheme::Dark)
///     .strength(0.8)
///     .inner_shadow(true);
/// assert_eq!(options.resolved_ring_px(), 2.0);
/// assert_eq!(options.resolved_shader_scale(), 1.3);
/// assert_eq!(options.resolved_theme(true), MetalFxResolvedTheme::Dark);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[must_use = "options do nothing unless used by a MetalFx or rendered"]
pub struct MetalFxOptions {
    pub(crate) variant: MetalFxVariant,
    pub(crate) preset: MetalFxPreset,
    pub(crate) theme: MetalFxTheme,
    pub(crate) strength: f32,
    pub(crate) glow_gain: f32,
    pub(crate) paused: bool,
    pub(crate) border_radius: Option<f32>,
    pub(crate) glow: bool,
    pub(crate) inner_shadow: Option<MetalRimOptions>,
    pub(crate) shader_scale: Option<f32>,
    pub(crate) ring_px: Option<f32>,
    pub(crate) scale: f32,
    pub(crate) surface: MetalFxSurface,
    pub(crate) layer: MetalFxLayer,
    pub(crate) mode: Option<MetalPresetMode>,
}

impl Default for MetalFxOptions {
    fn default() -> Self {
        Self {
            variant: MetalFxVariant::Button,
            preset: MetalFxPreset::Chromatic,
            theme: MetalFxTheme::Auto,
            strength: 1.0,
            glow_gain: 1.0,
            paused: false,
            border_radius: None,
            glow: true,
            inner_shadow: None,
            shader_scale: None,
            ring_px: None,
            scale: 1.0,
            surface: MetalFxSurface::Themed,
            layer: MetalFxLayer::OverChild,
            mode: None,
        }
    }
}

impl MetalFxOptions {
    /// Reference defaults: button, chromatic, auto theme, strength 1, glow on.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the variant (ring width and shader scale baseline).
    pub fn variant(mut self, variant: MetalFxVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Sets the colour preset (default [`MetalFxPreset::Chromatic`]).
    pub fn preset(mut self, preset: MetalFxPreset) -> Self {
        self.preset = preset;
        self
    }

    /// Sets the theme mode (default [`MetalFxTheme::Auto`]).
    pub fn theme(mut self, theme: MetalFxTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Effect strength in `0..=1`: multiplies the shader opacity and the glow
    /// alpha. The shader keeps animating at full intensity; only the rendered
    /// alpha is scaled.
    pub fn strength(mut self, strength: f32) -> Self {
        if strength.is_finite() {
            self.strength = strength.clamp(0.0, 1.0);
        }
        self
    }

    /// Extra multiplier on the glow only (halo + catch-light), on top of
    /// [`strength`](Self::strength); clamped to `>= 0` and to `0..=1` after
    /// multiplying (`glowGain`).
    pub fn glow_gain(mut self, gain: f32) -> Self {
        if gain.is_finite() {
            self.glow_gain = gain.max(0.0);
        }
        self
    }

    /// Pauses the animation. The ring keeps its last painted frame.
    pub fn paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        self
    }

    /// Overrides the corner radius, logical px (clamped to half the shorter
    /// side). Without it the ring is fully rounded: iced cannot read the
    /// child's own radius the way the reference reads its computed style.
    pub fn border_radius(mut self, radius: f32) -> Self {
        if radius.is_finite() {
            self.border_radius = Some(radius.max(0.0));
        }
        self
    }

    /// Restores the inferred (fully rounded) radius.
    pub fn reset_border_radius(mut self) -> Self {
        self.border_radius = None;
        self
    }

    /// Turns the wandering halo on or off (`disableGlow` inverted; default on).
    /// The ring still renders without it.
    pub fn glow(mut self, glow: bool) -> Self {
        self.glow = glow;
        self
    }

    /// Light rim along the top inside edge of the ring; `true` uses the design
    /// defaults (white 90 %, offset 1 px, blur 0.5 px).
    pub fn inner_shadow(mut self, enabled: bool) -> Self {
        self.inner_shadow = enabled.then(MetalRimOptions::new);
        self
    }

    /// Enables the inner shadow with explicit tuning.
    pub fn inner_shadow_with(mut self, rim: MetalRimOptions) -> Self {
        self.inner_shadow = Some(rim);
        self
    }

    /// Overrides the shader sampling scale (default: the variant baseline,
    /// 1.6 / 1.3, times [`scale`](Self::scale)). Clamped to `>= 0.01`.
    pub fn shader_scale(mut self, scale: f32) -> Self {
        if scale.is_finite() {
            self.shader_scale = Some(scale.max(0.01));
        }
        self
    }

    /// Overrides the ring thickness, logical px (default: 1 / 2 times
    /// [`scale`](Self::scale)). Clamped to `>= 0`.
    pub fn ring_px(mut self, px: f32) -> Self {
        if px.is_finite() {
            self.ring_px = Some(px.max(0.0));
        }
        self
    }

    /// Master multiplier on every absolute-pixel constant of the engine
    /// (shader sampling, ring thickness, glow strokes and blurs). Clamped to
    /// `0.1..=8`.
    pub fn scale(mut self, scale: f32) -> Self {
        if scale.is_finite() {
            self.scale = scale.clamp(0.1, 8.0);
        }
        self
    }

    /// The wrapper's background under the child.
    pub fn surface(mut self, surface: MetalFxSurface) -> Self {
        self.surface = surface;
        self
    }

    /// Where the metal layers sit relative to the child.
    pub fn layer(mut self, layer: MetalFxLayer) -> Self {
        self.layer = layer;
        self
    }

    /// Replaces the preset with raw Paper `liquidMetal` parameters
    /// (`setSharedPresetMode`, per instance here).
    pub fn preset_mode(mut self, mode: MetalPresetMode) -> Self {
        self.mode = Some(mode);
        self
    }

    /// Hands control back to [`preset`](Self::preset) / [`theme`](Self::theme).
    pub fn reset_preset_mode(mut self) -> Self {
        self.mode = None;
        self
    }

    /// The variant.
    #[must_use]
    pub fn variant_value(&self) -> MetalFxVariant {
        self.variant
    }

    /// The preset.
    #[must_use]
    pub fn preset_value(&self) -> MetalFxPreset {
        self.preset
    }

    /// The theme mode.
    #[must_use]
    pub fn theme_value(&self) -> MetalFxTheme {
        self.theme
    }

    /// The strength.
    #[must_use]
    pub fn strength_value(&self) -> f32 {
        self.strength
    }

    /// Whether the animation is paused.
    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Whether the halo is on.
    #[must_use]
    pub fn has_glow(&self) -> bool {
        self.glow
    }

    /// The inner-shadow tuning, if enabled.
    #[must_use]
    pub fn inner_shadow_options(&self) -> Option<&MetalRimOptions> {
        self.inner_shadow.as_ref()
    }

    /// Resolves the theme against whether the ambient theme is dark.
    #[must_use]
    pub fn resolved_theme(&self, ambient_is_dark: bool) -> MetalFxResolvedTheme {
        self.theme.resolve(ambient_is_dark)
    }

    /// The mode block drawn for `theme` (the raw override wins).
    #[must_use]
    pub fn resolved_mode(&self, theme: MetalFxResolvedTheme) -> MetalPresetMode {
        self.mode.unwrap_or_else(|| self.preset.mode(theme))
    }

    /// Ring thickness in logical px.
    #[must_use]
    pub fn resolved_ring_px(&self) -> f32 {
        self.ring_px.unwrap_or(self.variant.ring_px() * self.scale)
    }

    /// Shader sampling scale.
    #[must_use]
    pub fn resolved_shader_scale(&self) -> f32 {
        self.shader_scale
            .unwrap_or(self.variant.shader_scale() * self.scale)
    }

    /// Corner radius for a `width × height` box: a circle for
    /// [`MetalFxVariant::Circle`], else the override (or fully rounded) clamped
    /// to half the shorter side.
    #[must_use]
    pub fn resolved_radius(&self, width: f32, height: f32) -> f32 {
        let half = width.min(height).max(0.0) / 2.0;
        if self.variant == MetalFxVariant::Circle {
            return half;
        }
        self.border_radius.unwrap_or(half).min(half)
    }

    /// The surface colour for `theme` (`None` when transparent).
    #[must_use]
    pub fn resolved_surface(&self, theme: MetalFxResolvedTheme) -> Option<Color> {
        match self.surface {
            MetalFxSurface::Themed => Some(theme.surface()),
            MetalFxSurface::Solid(color) => Some(color),
            MetalFxSurface::Transparent => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_follow_the_reference_props() {
        let o = MetalFxOptions::new();
        assert_eq!(o.variant_value(), MetalFxVariant::Button);
        assert_eq!(o.preset_value(), MetalFxPreset::Chromatic);
        assert_eq!(o.theme_value(), MetalFxTheme::Auto);
        assert_eq!((o.strength_value(), o.glow_gain), (1.0, 1.0));
        assert!(!o.is_paused() && o.has_glow());
        assert!(o.inner_shadow_options().is_none());
        assert_eq!(o.resolved_ring_px(), 1.0);
        assert_eq!(o.resolved_shader_scale(), 1.6);
    }

    #[test]
    fn scale_multiplies_baselines_but_overrides_win() {
        let o = MetalFxOptions::new().scale(2.0);
        assert_eq!(o.resolved_ring_px(), 2.0);
        assert!((o.resolved_shader_scale() - 3.2).abs() < 1e-6);
        let o = o.ring_px(5.0).shader_scale(0.7);
        assert_eq!(o.resolved_ring_px(), 5.0);
        assert_eq!(o.resolved_shader_scale(), 0.7);
        let c = MetalFxOptions::new()
            .variant(MetalFxVariant::Circle)
            .scale(2.0);
        assert_eq!(c.resolved_ring_px(), 4.0);
        assert!((c.resolved_shader_scale() - 2.6).abs() < 1e-6);
    }

    #[test]
    fn radius_follows_resolve_radius() {
        let pill = MetalFxOptions::new();
        assert_eq!(pill.resolved_radius(134.0, 40.0), 20.0);
        assert_eq!(
            pill.clone().border_radius(8.0).resolved_radius(134.0, 40.0),
            8.0
        );
        assert_eq!(
            pill.clone()
                .border_radius(999.0)
                .resolved_radius(134.0, 40.0),
            20.0
        );
        let circle = MetalFxOptions::new()
            .variant(MetalFxVariant::Circle)
            .border_radius(2.0);
        assert_eq!(
            circle.resolved_radius(32.0, 32.0),
            16.0,
            "circle ignores the override"
        );
        assert_eq!(pill.reset_border_radius().resolved_radius(10.0, 40.0), 5.0);
    }

    #[test]
    fn setters_validate_and_ignore_non_finite_values() {
        let o = MetalFxOptions::new()
            .strength(5.0)
            .glow_gain(-1.0)
            .scale(0.0);
        assert_eq!(o.strength_value(), 1.0);
        assert_eq!(o.glow_gain, 0.0);
        assert_eq!(o.scale, 0.1);
        let o = o
            .strength(f32::NAN)
            .border_radius(f32::INFINITY)
            .shader_scale(f32::NAN);
        assert_eq!(o.strength_value(), 1.0);
        assert_eq!(o.border_radius, None);
        assert_eq!(o.shader_scale, None);
    }

    #[test]
    fn inner_shadow_true_uses_the_design_defaults() {
        let o = MetalFxOptions::new().inner_shadow(true);
        assert_eq!(o.inner_shadow_options(), Some(&MetalRimOptions::DEFAULTS));
        let o = o.inner_shadow(false);
        assert!(o.inner_shadow_options().is_none());
        let o = o.inner_shadow_with(MetalRimOptions::new().alpha(0.4));
        assert_eq!(o.inner_shadow_options().unwrap().alpha_value(), 0.4);
    }

    #[test]
    fn mode_override_wins_over_the_preset() {
        let mode = MetalPresetMode {
            repetition: 5.0,
            ..MetalPresetMode::BASE
        };
        let o = MetalFxOptions::new()
            .preset(MetalFxPreset::Gold)
            .preset_mode(mode);
        assert_eq!(o.resolved_mode(MetalFxResolvedTheme::Dark), mode);
        assert_eq!(
            o.reset_preset_mode()
                .resolved_mode(MetalFxResolvedTheme::Dark),
            MetalFxPreset::Gold.mode(MetalFxResolvedTheme::Dark)
        );
    }

    #[test]
    fn surface_resolves_per_theme() {
        let o = MetalFxOptions::new();
        assert_eq!(
            o.resolved_surface(MetalFxResolvedTheme::Dark),
            Some(Color::from_rgb8(0x27, 0x27, 0x27))
        );
        assert_eq!(
            o.resolved_surface(MetalFxResolvedTheme::Light),
            Some(Color::WHITE)
        );
        let red = o
            .clone()
            .surface(MetalFxSurface::Solid(Color::from_rgb(1.0, 0.0, 0.0)));
        assert_eq!(
            red.resolved_surface(MetalFxResolvedTheme::Dark),
            Some(Color::from_rgb(1.0, 0.0, 0.0))
        );
        assert_eq!(
            o.surface(MetalFxSurface::Transparent)
                .resolved_surface(MetalFxResolvedTheme::Dark),
            None
        );
    }
}
