//! The builder options, the resolved per-theme spec and headless rendering
//! entry points.

use std::time::Duration;

use iced::Color;

use super::data::THEME_PRESETS;
use super::frame::{self, BorderBeamFrame};
use super::palette::{
    BeamHost, BorderBeamPalette, BorderBeamSize, BorderBeamTheme, ResolvedBeamTheme, Rgba,
};
use super::raster::{BeamLayers, BorderBeamRaster};

/// Defaults of the reference implementation.
pub(crate) const FADE_IN: Duration = Duration::from_millis(600);
/// Default fade-out duration.
pub(crate) const FADE_OUT: Duration = Duration::from_millis(500);
const MIN_DURATION: Duration = Duration::from_millis(50);

/// Builder options for a [`BorderBeam`](super::BorderBeam), usable without a
/// widget tree for headless rendering.
///
/// Numeric setters clamp to their documented ranges; non-finite values are
/// ignored and keep the current/default value.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::{
///     BeamHost, BorderBeamOptions, BorderBeamPalette, BorderBeamSize, BeamRenderOptions,
///     ResolvedBeamTheme,
/// };
/// use std::time::Duration;
///
/// let options = BorderBeamOptions::new()
///     .size(BorderBeamSize::Line)
///     .palette(BorderBeamPalette::Ocean)
///     .strength(0.8);
/// let layers = options.render_at(
///     Duration::from_secs_f32(1.55),
///     BeamHost::new(348.0, 122.0),
///     ResolvedBeamTheme::Dark,
///     &BeamRenderOptions::new().density(2.0),
/// );
/// assert!(layers.above().is_some());
/// ```
#[derive(Clone, Debug, PartialEq)]
#[must_use = "options do nothing unless used by a BorderBeam or rendered"]
pub struct BorderBeamOptions {
    pub(crate) size: BorderBeamSize,
    pub(crate) palette: BorderBeamPalette,
    pub(crate) theme: BorderBeamTheme,
    pub(crate) static_colors: bool,
    pub(crate) duration: Option<Duration>,
    pub(crate) active: bool,
    pub(crate) radius: Option<f32>,
    pub(crate) border_width: Option<f32>,
    pub(crate) brightness: Option<f32>,
    pub(crate) saturation: Option<f32>,
    pub(crate) hue_range: f32,
    pub(crate) glow_size: f32,
    pub(crate) strength: f32,
    pub(crate) reduced_motion: bool,
}

impl Default for BorderBeamOptions {
    fn default() -> Self {
        Self {
            size: BorderBeamSize::Md,
            palette: BorderBeamPalette::Colorful,
            theme: BorderBeamTheme::Dark,
            static_colors: false,
            duration: None,
            active: true,
            radius: None,
            border_width: None,
            brightness: None,
            saturation: None,
            hue_range: 30.0,
            glow_size: 1.0,
            strength: 1.0,
            reduced_motion: false,
        }
    }
}

impl BorderBeamOptions {
    /// Reference defaults: `md`, colorful, dark, active.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the size preset (default [`BorderBeamSize::Md`]).
    pub fn size(mut self, size: BorderBeamSize) -> Self {
        self.size = size;
        self
    }

    /// Sets the colour palette (default [`BorderBeamPalette::Colorful`]).
    pub fn palette(mut self, palette: BorderBeamPalette) -> Self {
        self.palette = palette;
        self
    }

    /// Alias for [`Self::palette`], matching the reference `colorVariant` prop.
    pub fn color_variant(self, palette: BorderBeamPalette) -> Self {
        self.palette(palette)
    }

    /// Sets the theme preset (default [`BorderBeamTheme::Dark`]).
    pub fn theme(mut self, theme: BorderBeamTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Freezes hue rotation (`staticColors`). The mono palette always freezes.
    pub fn static_colors(mut self, static_colors: bool) -> Self {
        self.static_colors = static_colors;
        self
    }

    /// Loops duration (default `1.96s`, line `3.1s`, pulse `2.3s`).
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration.max(MIN_DURATION));
        self
    }

    /// Restores the size-dependent default duration.
    pub fn reset_duration(mut self) -> Self {
        self.duration = None;
        self
    }

    /// Whether the effect is logically on (default `true`).
    ///
    /// The widget fades in/out over 0.6s/0.5s. [`BorderBeamCanvas`] treats
    /// `false` as opacity `0` (drive its [`opacity`] for a fade), and the
    /// headless renderer ignores this flag and takes an explicit opacity in
    /// [`BeamRenderOptions`].
    ///
    /// [`BorderBeamCanvas`]: super::BorderBeamCanvas
    /// [`opacity`]: super::BorderBeamCanvas::opacity
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Corner radius (default 32 for `sm`, 16 otherwise).
    pub fn radius(mut self, radius: f32) -> Self {
        if radius.is_finite() {
            self.radius = Some(radius.max(0.0));
        }
        self
    }

    /// Restores the size-dependent default radius.
    pub fn reset_radius(mut self) -> Self {
        self.radius = None;
        self
    }

    /// Ring thickness in logical pixels (default 1).
    ///
    /// `pulse-outside` always strokes a 1px ring, like the reference.
    pub fn border_width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.border_width = Some(width.max(0.0));
        }
        self
    }

    /// Restores the default 1px ring.
    pub fn reset_border_width(mut self) -> Self {
        self.border_width = None;
        self
    }

    /// Overrides the preset brightness (default: theme preset, or 1.3).
    pub fn brightness(mut self, brightness: f32) -> Self {
        if let Some(value) = clamp_finite(brightness, 0.0, 8.0) {
            self.brightness = Some(value);
        }
        self
    }

    /// Restores the theme preset brightness.
    pub fn reset_brightness(mut self) -> Self {
        self.brightness = None;
        self
    }

    /// Overrides the preset saturation.
    pub fn saturation(mut self, saturation: f32) -> Self {
        if let Some(value) = clamp_finite(saturation, 0.0, 16.0) {
            self.saturation = Some(value);
        }
        self
    }

    /// Restores the theme preset saturation.
    pub fn reset_saturation(mut self) -> Self {
        self.saturation = None;
        self
    }

    /// Hue ping-pong range in degrees (default 30; line is capped at 13).
    pub fn hue_range(mut self, degrees: f32) -> Self {
        if let Some(value) = clamp_finite(degrees, 0.0, 360.0) {
            self.hue_range = value;
        }
        self
    }

    /// Multiplies every glow blur radius (default 1).
    pub fn glow_size(mut self, scale: f32) -> Self {
        if let Some(value) = clamp_finite(scale, 0.0, 8.0) {
            self.glow_size = value;
        }
        self
    }

    /// Layer opacity multiplier in `0..=1` (default 1).
    pub fn strength(mut self, strength: f32) -> Self {
        if let Some(value) = clamp_finite(strength, 0.0, 1.0) {
            self.strength = value;
        }
        self
    }

    /// Freezes the animation clock and shows the effect at a representative
    /// static phase (accessibility; the reference only handles pulse).
    pub fn reduced_motion(mut self, reduced: bool) -> Self {
        self.reduced_motion = reduced;
        self
    }

    /// Whether the effect is logically active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// The animation clock is frozen for reduced motion.
    pub fn is_reduced_motion(&self) -> bool {
        self.reduced_motion
    }

    /// Resolves the full per-theme style for `system`.
    pub fn spec(&self, system: ResolvedBeamTheme) -> BorderBeamSpec {
        let theme = self.theme.resolve(system);
        let index = self.size.index() * 2 + usize::from(theme == ResolvedBeamTheme::Light);
        let preset = THEME_PRESETS[index];
        let mono = self.palette.is_mono();
        // The reference halves sm/md/pulse layer opacities for mono but
        // leaves `line` untouched (its palette is already attenuated).
        let mono_mul = if mono && self.size != BorderBeamSize::Line {
            0.5
        } else {
            1.0
        };
        let duration = self
            .duration
            .unwrap_or_else(|| match self.size {
                BorderBeamSize::Line => Duration::from_secs_f32(3.1),
                BorderBeamSize::PulseInner | BorderBeamSize::PulseOutside => {
                    Duration::from_secs_f32(2.3)
                }
                BorderBeamSize::Sm | BorderBeamSize::Md => Duration::from_secs_f32(1.96),
            })
            .max(MIN_DURATION);
        let radius = self.radius.unwrap_or(match self.size {
            BorderBeamSize::Sm => 32.0,
            _ => 16.0,
        });
        BorderBeamSpec {
            size: self.size,
            palette: self.palette,
            theme,
            static_colors: self.static_colors || mono,
            reduced_motion: self.reduced_motion,
            duration,
            radius,
            border_width: if self.size == BorderBeamSize::PulseOutside {
                1.0
            } else {
                self.border_width.unwrap_or(1.0)
            },
            stroke_opacity: (preset.stroke * mono_mul).clamp(0.0, 1.0),
            inner_opacity: (preset.inner * mono_mul).clamp(0.0, 1.0),
            bloom_opacity: (preset.bloom * mono_mul).clamp(0.0, 1.0),
            inner_shadow: preset.inner_shadow,
            brightness: self.brightness.unwrap_or(preset.brightness),
            saturation: self.saturation.unwrap_or(preset.saturation),
            hue_range: if self.size == BorderBeamSize::Line {
                self.hue_range.min(13.0)
            } else {
                self.hue_range
            },
            glow_size: self.glow_size,
            strength: self.strength,
            hairline_opacity: if self.size == BorderBeamSize::PulseOutside {
                preset.hairline.clamp(0.0, 1.0)
            } else {
                0.0
            },
        }
    }

    /// Computes the continuous animation frame for the given clock time.
    ///
    /// This does not apply the active fade; pass it to
    /// [`BorderBeamRaster::render`] or use
    /// [`BeamRenderOptions::opacity`] with [`Self::render_at`].
    pub fn frame_at(
        &self,
        elapsed: Duration,
        host: BeamHost,
        system: ResolvedBeamTheme,
    ) -> BorderBeamFrame {
        frame::frame_at(&self.spec(system), host, elapsed)
    }

    /// Rasterizes one frame without a widget tree.
    ///
    /// A fresh scratch rasterizer is allocated per call; widgets keep their own
    /// [`BorderBeamRaster`] and call [`BorderBeamRaster::render`] instead.
    pub fn render_at(
        &self,
        elapsed: Duration,
        host: BeamHost,
        system: ResolvedBeamTheme,
        options: &BeamRenderOptions,
    ) -> BeamLayers {
        let frame = self.frame_at(elapsed, host, system);
        let mut raster = BorderBeamRaster::new();
        raster.render(&frame, options.opacity, options.density)
    }
}

fn clamp_finite(value: f32, min: f32, max: f32) -> Option<f32> {
    value.is_finite().then(|| value.clamp(min, max))
}

/// The fully resolved style of one size, palette and theme.
///
/// Opacities are already clamped to `0..=1` (CSS clamps `opacity`), mono has
/// halved them for every size except `line` (which the reference leaves as
/// authored), and the line palette's hue range is already capped at 13°.
/// # Examples
///
/// ```rust
/// use enchant_iced::{BorderBeamOptions, BorderBeamSize, ResolvedBeamTheme};
///
/// let spec = BorderBeamOptions::new()
///     .size(BorderBeamSize::Line)
///     .hue_range(90.0)
///     .spec(ResolvedBeamTheme::Dark);
/// assert_eq!(spec.hue_range(), 13.0);
/// ```
#[must_use = "a spec does nothing unless rendered"]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderBeamSpec {
    pub(crate) size: BorderBeamSize,
    pub(crate) palette: BorderBeamPalette,
    pub(crate) theme: ResolvedBeamTheme,
    pub(crate) static_colors: bool,
    pub(crate) reduced_motion: bool,
    pub(crate) duration: Duration,
    pub(crate) radius: f32,
    pub(crate) border_width: f32,
    pub(crate) stroke_opacity: f32,
    pub(crate) inner_opacity: f32,
    pub(crate) bloom_opacity: f32,
    pub(crate) inner_shadow: Rgba,
    pub(crate) brightness: f32,
    pub(crate) saturation: f32,
    pub(crate) hue_range: f32,
    pub(crate) glow_size: f32,
    pub(crate) strength: f32,
    pub(crate) hairline_opacity: f32,
}

impl BorderBeamSpec {
    /// The resolved size preset.
    pub fn size(&self) -> BorderBeamSize {
        self.size
    }

    /// The resolved palette.
    pub fn palette(&self) -> BorderBeamPalette {
        self.palette
    }

    /// The resolved light/dark preset.
    pub fn theme(&self) -> ResolvedBeamTheme {
        self.theme
    }

    /// Palette colours are static (explicit or mono); reduced motion freezes
    /// the frame time but does not zero the hue.
    pub fn static_colors(&self) -> bool {
        self.static_colors
    }

    /// The animation is frozen at a representative phase.
    pub fn reduced_motion(&self) -> bool {
        self.reduced_motion
    }

    /// Loop duration.
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Corner radius in logical pixels.
    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// Ring thickness in logical pixels. Always `1` for
    /// [`PulseOutside`](BorderBeamSize::PulseOutside).
    pub fn border_width(&self) -> f32 {
        self.border_width
    }

    /// Stroke-layer opacity after theme, mono and clamping.
    pub fn stroke_opacity(&self) -> f32 {
        self.stroke_opacity
    }

    /// Inner-glow opacity after theme, mono and clamping.
    pub fn inner_opacity(&self) -> f32 {
        self.inner_opacity
    }

    /// Bloom opacity after theme, mono and clamping.
    pub fn bloom_opacity(&self) -> f32 {
        self.bloom_opacity
    }

    /// The `box-shadow: inset` colour.
    pub fn inner_shadow(&self) -> Color {
        self.inner_shadow.to_color()
    }

    /// Current `brightness()` factor.
    pub fn brightness(&self) -> f32 {
        self.brightness
    }

    /// Current `saturate()` factor.
    pub fn saturation(&self) -> f32 {
        self.saturation
    }

    /// Effective hue range in degrees.
    pub fn hue_range(&self) -> f32 {
        self.hue_range
    }

    /// Blur-radius multiplier.
    pub fn glow_size(&self) -> f32 {
        self.glow_size
    }

    /// Layer opacity multiplier.
    pub fn strength(&self) -> f32 {
        self.strength
    }

    /// Opacity of the optional pulse-outside hairline (`0` by default).
    pub fn hairline_opacity(&self) -> f32 {
        self.hairline_opacity
    }
}

/// Headless raster settings.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::BeamRenderOptions;
///
/// let options = BeamRenderOptions::new().density(2.0).opacity(1.0);
/// let _ = options;
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
#[must_use = "render options do nothing on their own"]
pub struct BeamRenderOptions {
    pub(crate) density: f32,
    pub(crate) opacity: f32,
}

impl Default for BeamRenderOptions {
    fn default() -> Self {
        Self {
            density: 1.0,
            opacity: 1.0,
        }
    }
}

impl BeamRenderOptions {
    /// 1x density and fully opaque.
    pub fn new() -> Self {
        Self::default()
    }

    /// Device pixels per logical pixel, clamped to `0.5..=4`. Headless analog
    /// of the widget/canvas `pixel_density`.
    pub fn density(mut self, density: f32) -> Self {
        self.density = if density.is_finite() {
            density.clamp(0.5, 4.0)
        } else {
            1.0
        };
        self
    }

    /// Fade opacity applied on top of the resolved layers. The resolved
    /// strength is applied separately by the rasterizer.
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = clamp_finite(opacity, 0.0, 1.0).unwrap_or(0.0);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_halves_every_size_except_line() {
        let plain = BorderBeamOptions::new()
            .size(BorderBeamSize::Md)
            .spec(ResolvedBeamTheme::Dark);
        let mono = BorderBeamOptions::new()
            .size(BorderBeamSize::Md)
            .palette(BorderBeamPalette::Mono)
            .spec(ResolvedBeamTheme::Dark);
        assert_eq!(mono.stroke_opacity(), plain.stroke_opacity() * 0.5);

        let line = BorderBeamOptions::new()
            .size(BorderBeamSize::Line)
            .spec(ResolvedBeamTheme::Dark);
        let line_mono = BorderBeamOptions::new()
            .size(BorderBeamSize::Line)
            .palette(BorderBeamPalette::Mono)
            .spec(ResolvedBeamTheme::Dark);
        assert_eq!(line_mono.stroke_opacity(), line.stroke_opacity());
        assert_eq!(line_mono.inner_opacity(), line.inner_opacity());
        assert_eq!(line_mono.bloom_opacity(), line.bloom_opacity());
    }

    #[test]
    fn pulse_outside_forces_a_one_pixel_ring() {
        let spec = BorderBeamOptions::new()
            .size(BorderBeamSize::PulseOutside)
            .border_width(6.0)
            .spec(ResolvedBeamTheme::Dark);
        assert_eq!(spec.border_width(), 1.0);
        assert_eq!(spec.hairline_opacity(), 0.0);
    }

    #[test]
    fn non_finite_overrides_keep_the_previous_value() {
        let spec = BorderBeamOptions::new()
            .brightness(f32::NAN)
            .saturation(f32::INFINITY)
            .hue_range(f32::NAN)
            .glow_size(f32::NAN)
            .strength(f32::NAN)
            .radius(f32::NAN)
            .border_width(f32::NAN)
            .spec(ResolvedBeamTheme::Dark);
        let default = BorderBeamOptions::new().spec(ResolvedBeamTheme::Dark);
        assert_eq!(spec.brightness(), default.brightness());
        assert_eq!(spec.saturation(), default.saturation());
        assert_eq!(spec.hue_range(), default.hue_range());
        assert_eq!(spec.glow_size(), default.glow_size());
        assert_eq!(spec.strength(), default.strength());
        assert_eq!(spec.radius(), default.radius());
        assert_eq!(spec.border_width(), default.border_width());
    }
}
