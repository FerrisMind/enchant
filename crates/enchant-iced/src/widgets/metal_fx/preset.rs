//! Bundled presets and the theme model, from `engine/presets.ts` and
//! `types.ts`.

use super::color::rgba8;

/// Theme mode of a [`MetalFx`](super::MetalFx).
///
/// - [`Auto`](Self::Auto) (default) follows the iced theme: dark when its
///   palette is dark.
/// - [`Dark`](Self::Dark) / [`Light`](Self::Light) pin the tunings.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::{MetalFxResolvedTheme, MetalFxTheme};
///
/// assert_eq!(MetalFxTheme::Auto.resolve(false), MetalFxResolvedTheme::Light);
/// assert_eq!(MetalFxTheme::Dark.resolve(false), MetalFxResolvedTheme::Dark);
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MetalFxTheme {
    /// Pin the dark-mode tunings.
    Dark,
    /// Pin the light-mode tunings.
    Light,
    /// Follow the ambient theme.
    #[default]
    Auto,
}

impl MetalFxTheme {
    /// Resolves `Auto` against whether the ambient theme is dark.
    #[must_use]
    pub fn resolve(self, ambient_is_dark: bool) -> MetalFxResolvedTheme {
        match self {
            Self::Dark => MetalFxResolvedTheme::Dark,
            Self::Light => MetalFxResolvedTheme::Light,
            Self::Auto if ambient_is_dark => MetalFxResolvedTheme::Dark,
            Self::Auto => MetalFxResolvedTheme::Light,
        }
    }
}

/// A [`MetalFxTheme`] after `Auto` has been resolved.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MetalFxResolvedTheme {
    /// Dark-mode tunings.
    Dark,
    /// Light-mode tunings.
    Light,
}

impl MetalFxResolvedTheme {
    /// Whether this is the light theme.
    #[must_use]
    pub fn is_light(self) -> bool {
        matches!(self, Self::Light)
    }

    /// The wrapper's default surface (`.metal-fx-root` background): `#272727`
    /// on dark, `#ffffff` on light.
    #[must_use]
    pub fn surface(self) -> iced::Color {
        match self {
            Self::Dark => iced::Color::from_rgb8(0x27, 0x27, 0x27),
            Self::Light => iced::Color::WHITE,
        }
    }

    /// The wrapper's default foreground (`color` of `.metal-fx-root`):
    /// `#f8f8f8` on dark, `#1d1d1d` on light. Use it for the child's text.
    #[must_use]
    pub fn text(self) -> iced::Color {
        match self {
            Self::Dark => iced::Color::from_rgb8(0xf8, 0xf8, 0xf8),
            Self::Light => iced::Color::from_rgb8(0x1d, 0x1d, 0x1d),
        }
    }
}

/// Bundled preset names. Each ships a dark and a light block.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::{MetalFxPreset, MetalFxResolvedTheme};
///
/// let gold = MetalFxPreset::Gold.mode(MetalFxResolvedTheme::Dark);
/// assert_eq!(gold.speed, 0.85);
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MetalFxPreset {
    /// Cool blue burn with strong R/B dispersion.
    #[default]
    Chromatic,
    /// Paper's material essentially untouched.
    Silver,
    /// Warm amber burn.
    Gold,
}

impl MetalFxPreset {
    /// Every bundled preset.
    pub const ALL: [Self; 3] = [Self::Chromatic, Self::Silver, Self::Gold];

    /// The block for `theme`.
    #[must_use]
    pub fn mode(self, theme: MetalFxResolvedTheme) -> MetalPresetMode {
        let base = MetalPresetMode::BASE;
        match (self, theme) {
            (Self::Chromatic, MetalFxResolvedTheme::Dark) => MetalPresetMode {
                color_tint: rgba8(0x88ccff2e),
                shift_red: 0.75,
                shift_blue: 0.75,
                repetition: 2.0,
                softness: 0.09,
                shader_opacity: 1.0,
                ..base
            },
            (Self::Chromatic, MetalFxResolvedTheme::Light) => MetalPresetMode {
                color_tint: rgba8(0x66b0ff99),
                shift_red: 0.6,
                shift_blue: 0.6,
                shader_opacity: 1.0,
                ..base
            },
            (Self::Silver, MetalFxResolvedTheme::Dark) => MetalPresetMode {
                color_tint: rgba8(0xffffff66),
                shader_opacity: 0.88,
                ..base
            },
            (Self::Silver, MetalFxResolvedTheme::Light) => MetalPresetMode {
                color_tint: rgba8(0xffffff40),
                shader_opacity: 1.0,
                ..base
            },
            (Self::Gold, MetalFxResolvedTheme::Dark) => MetalPresetMode {
                color_tint: rgba8(0xffcc55cc),
                speed: 0.85,
                shader_opacity: 0.92,
                ..base
            },
            (Self::Gold, MetalFxResolvedTheme::Light) => MetalPresetMode {
                color_tint: rgba8(0xf7d488aa),
                shader_opacity: 1.0,
                ..base
            },
        }
    }
}

/// Paper's `ShaderFitOptions`.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ShaderFit {
    /// `FIT_NONE`.
    None,
    /// `FIT_CONTAIN` (the metal-fx default).
    #[default]
    Contain,
    /// `FIT_COVER`.
    Cover,
}

impl ShaderFit {
    pub(crate) fn code(self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::Contain => 1.0,
            Self::Cover => 2.0,
        }
    }
}

/// One mode block of a preset: the Paper `liquidMetal` parameter set plus the
/// opacity metal-fx composites the sheet with (`PresetMode` in `presets.ts`).
///
/// The mask shape is fixed to Paper's `none` (a full sheet, which metal-fx
/// carves into a ring itself), as every bundled preset does; the other four
/// shapes are not exposed.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::MetalPresetMode;
///
/// let mode = MetalPresetMode { repetition: 4.0, ..MetalPresetMode::BASE };
/// assert_eq!(mode.softness, 0.05);
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MetalPresetMode {
    /// Backdrop RGBA composited under the metal. Keep it transparent for rings.
    pub color_back: [f32; 4],
    /// Tint RGBA applied as colour burn; the alpha is the blend amount.
    pub color_tint: [f32; 4],
    /// Time multiplier applied before `u_time` is uploaded.
    pub speed: f32,
    /// Stripe density (1..10).
    pub repetition: f32,
    /// Stripe transition blur (0..1).
    pub softness: f32,
    /// R-channel dispersion (-1..1).
    pub shift_red: f32,
    /// B-channel dispersion (-1..1).
    pub shift_blue: f32,
    /// Simplex-noise warp over the stripe field (0..1).
    pub distortion: f32,
    /// How strongly the pattern follows the shape edge (0..1).
    pub contour: f32,
    /// Pattern drift direction in degrees.
    pub angle: f32,
    /// Overall zoom (0.01..4).
    pub scale: f32,
    /// Overall rotation in degrees.
    pub rotation: f32,
    /// Graphic centre offset (-1..1).
    pub offset_x: f32,
    /// Graphic centre offset (-1..1).
    pub offset_y: f32,
    /// World box reference point (0..1).
    pub origin_x: f32,
    /// World box reference point (0..1).
    pub origin_y: f32,
    /// Virtual size before fitting; `0` uses the sheet.
    pub world_width: f32,
    /// Virtual size before fitting; `0` uses the sheet.
    pub world_height: f32,
    /// Fit of the world box.
    pub fit: ShaderFit,
    /// Global alpha applied when the sheet is composited onto an instance.
    pub shader_opacity: f32,
}

impl MetalPresetMode {
    /// Paper's `fullScreenPreset` minus its opaque backdrop (`BASE` in
    /// `presets.ts`), with a neutral tint.
    pub const BASE: Self = Self {
        color_back: [0.0; 4],
        color_tint: [1.0, 1.0, 1.0, 0.0],
        speed: 1.0,
        repetition: 1.5,
        softness: 0.05,
        shift_red: 0.3,
        shift_blue: 0.3,
        distortion: 0.1,
        contour: 0.4,
        angle: 90.0,
        scale: 1.0,
        rotation: 0.0,
        offset_x: 0.0,
        offset_y: 0.0,
        origin_x: 0.5,
        origin_y: 0.5,
        world_width: 0.0,
        world_height: 0.0,
        fit: ShaderFit::Contain,
        shader_opacity: 1.0,
    };
}

impl Default for MetalPresetMode {
    fn default() -> Self {
        Self::BASE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::metal_fx::color::hex_to_rgba;

    fn mode(p: MetalFxPreset, t: MetalFxResolvedTheme) -> MetalPresetMode {
        p.mode(t)
    }

    #[test]
    fn presets_match_presets_ts_exactly() {
        use MetalFxPreset::*;
        use MetalFxResolvedTheme::*;
        let base = MetalPresetMode::BASE;

        let m = mode(Chromatic, Dark);
        assert_eq!(m.color_tint, hex_to_rgba("#88ccff2e").unwrap());
        assert_eq!(
            (m.shift_red, m.shift_blue, m.repetition, m.softness),
            (0.75, 0.75, 2.0, 0.09)
        );
        assert_eq!((m.speed, m.shader_opacity), (1.0, 1.0));

        let m = mode(Chromatic, Light);
        assert_eq!(m.color_tint, hex_to_rgba("#66b0ff99").unwrap());
        assert_eq!(
            (m.shift_red, m.shift_blue, m.repetition, m.softness),
            (0.6, 0.6, base.repetition, base.softness)
        );
        assert_eq!(m.shader_opacity, 1.0);

        let m = mode(Silver, Dark);
        assert_eq!(m.color_tint, hex_to_rgba("#ffffff66").unwrap());
        assert_eq!(m.shader_opacity, 0.88);
        let m = mode(Silver, Light);
        assert_eq!(m.color_tint, hex_to_rgba("#ffffff40").unwrap());
        assert_eq!(m.shader_opacity, 1.0);

        let m = mode(Gold, Dark);
        assert_eq!(m.color_tint, hex_to_rgba("#ffcc55cc").unwrap());
        assert_eq!((m.speed, m.shader_opacity), (0.85, 0.92));
        let m = mode(Gold, Light);
        assert_eq!(m.color_tint, hex_to_rgba("#f7d488aa").unwrap());
        assert_eq!((m.speed, m.shader_opacity), (1.0, 1.0));
    }

    #[test]
    fn every_mode_keeps_paper_baseline_values() {
        for preset in MetalFxPreset::ALL {
            for theme in [MetalFxResolvedTheme::Dark, MetalFxResolvedTheme::Light] {
                let m = preset.mode(theme);
                assert_eq!(m.color_back, [0.0; 4]);
                assert_eq!((m.distortion, m.contour, m.angle), (0.1, 0.4, 90.0));
                assert_eq!(
                    (m.scale, m.rotation, m.offset_x, m.offset_y),
                    (1.0, 0.0, 0.0, 0.0)
                );
                assert_eq!(
                    (m.origin_x, m.origin_y, m.world_width, m.world_height),
                    (0.5, 0.5, 0.0, 0.0)
                );
                assert_eq!(m.fit, ShaderFit::Contain);
            }
        }
    }

    #[test]
    fn auto_theme_follows_ambient() {
        assert_eq!(MetalFxTheme::Auto.resolve(true), MetalFxResolvedTheme::Dark);
        assert_eq!(
            MetalFxTheme::Auto.resolve(false),
            MetalFxResolvedTheme::Light
        );
        assert_eq!(
            MetalFxTheme::Light.resolve(true),
            MetalFxResolvedTheme::Light
        );
        assert_eq!(MetalFxTheme::default(), MetalFxTheme::Auto);
    }

    #[test]
    fn surfaces_match_styles_ts() {
        assert_eq!(
            MetalFxResolvedTheme::Dark.surface(),
            iced::Color::from_rgb8(0x27, 0x27, 0x27)
        );
        assert_eq!(MetalFxResolvedTheme::Light.surface(), iced::Color::WHITE);
        assert_eq!(
            MetalFxResolvedTheme::Dark.text(),
            iced::Color::from_rgb8(0xf8, 0xf8, 0xf8)
        );
        assert_eq!(
            MetalFxResolvedTheme::Light.text(),
            iced::Color::from_rgb8(0x1d, 0x1d, 0x1d)
        );
    }
}
