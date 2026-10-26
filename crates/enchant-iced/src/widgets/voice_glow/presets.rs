//! Host presets and theme presets, ported value for value from
//! `packages/voice-glow/src/presets.ts` and the `themePresets` table of
//! `styles.ts`.
//!
//! A preset only supplies defaults; every geometry prop on
//! [`VoiceGlow`](super::VoiceGlow) still overrides it.

use super::options::{ResolvedTheme, VoiceGlowType};

/// The single list of geometry knobs. A callback macro receives
/// `name, "doc", default;` triples so the resolved struct, the sparse
/// override struct and the builder setters can never drift apart.
macro_rules! geometry_fields {
    ($callback:ident) => {
        $callback! {
            scale, "Size of the whole effect: multiplies every pixel dimension at once (lobes, spacing, range, bend, band, core, bloom blur, flow speed).", 1.0;
            glow_size, "Bloom blur multiplier: below 1 a tighter halo, above 1 a wider, softer one.", 1.0;
            stroke_opacity, "Opacity multiplier of the crisp edge stroke, on the theme's own.", 1.0;
            inner_opacity, "Opacity multiplier of the soft inner light, on the theme's own.", 1.0;
            bloom_opacity, "Opacity multiplier of the blurred bloom, on the theme's own.", 1.0;
            idle, "Resting presence, 0-1: how much the beam breathes while silent. 0 hides it between sounds.", 0.18;
            reach, "How tall the glow grows at full level, as a multiple of its resting height.", 1.2;
            spread, "How far the glow widens at full level, as a fraction of its resting width.", 1.05;
            flow, "How fast the spectrum travels sideways, in px per second at full level. Negative flows right to left; 0 holds the lobes in place.", 48.0;
            bend, "Bend: px the glow's top contour humps up at the centre at full level. 0 keeps the plain ellipse.", 60.0;
            band_strength, "Opacity of the band, 0 hides it.", 1.55;
            band_width, "Thickness of the band.", 2.15;
            band_position, "Height of the band's peak as a fraction of the glow's ceiling, 0.1-1.3.", 0.35;
            band_curve, "Bell exponent: below 2 an exponential, cusp-like rise; 2 a gaussian; above, a flatter top.", 1.75;
            band_spread, "Bell width as a fraction of the glow's half-range.", 0.87;
            band_skew, "Asymmetry, -0.6 to 0.6: positive widens the right side and steepens the left.", 0.12;
            band_offset, "Vertical shift of the whole band line in px; negative sinks it toward the edge.", -27.0;
            band_tail, "How far the band's ends rise again toward the corners, as a fraction of its peak, 0-1.", 0.59;
            band_tail_position, "Where the tail's rise starts, as a share of the distance from the beam's centre to the element's edge.", 0.67;
            band_tail_curve, "Exponent of the tail's rise: 1 a straight ramp, 2 a parabola, higher a hook.", 2.4;
            band_tail_overflow, "Px the band runs past each side of the element, so the hook is cropped by the host's edge.", 15.0;
            band_aberration, "Chromatic aberration: how far the red and blue fringes split from the core, 0-1.", 0.89;
            distortion, "How much the glow under the band warps sideways, 0-1. 0 turns the filter off entirely.", 0.62;
            distortion_detail, "Grain of the distortion's noise: below 1 broad slow waves, above 1 finer ripples.", 2.3;
            glow_width, "Width of every colour lobe, all layers.", 0.65;
            glow_height, "Height of every colour lobe, all layers.", 1.25;
            lobe_spacing, "Distance between the lobes (and the ring the flow travels).", 0.85;
            range_width, "Width of the visible ellipse the glow is masked to.", 0.75;
            range_height, "Height of the visible ellipse the glow is masked to.", 1.0;
            softness, "Where each lobe fades out: below 1 crisper, above 1 softer.", 1.07;
            core_size, "The white hot spot at the centre of the edge.", 1.0;
            core_light, "The epicentre: a soft white wash under the band line at the centre of the edge, 0-3.", 0.0;
            core_light_width, "Width of the epicentre wash.", 1.0;
            core_light_height, "Height of the epicentre wash.", 1.0;
            stroke_scale, "Size of the colours painted into the 1px edge stroke.", 1.0;
            inner_scale, "Size of the soft light inside the element.", 1.0;
            inner_height, "Extra height of the inner light: how far it reaches into the element.", 1.0;
            bloom_scale, "Size of the blurred halo.", 1.0;
            bloom_height, "Extra height of the halo: how far the soft light climbs.", 1.0;
        }
    };
}
pub(super) use geometry_fields;

macro_rules! define_geometry {
    ($($name:ident, $doc:literal, $default:expr;)+) => {
        /// The geometry and response knobs a host preset may retune
        /// (`VoiceGeometry` upstream), fully resolved.
        ///
        /// Obtain one with [`resolve_defaults`]; individual values are
        /// public because this is a plain data record with no invariants.
        ///
        /// # Examples
        ///
        /// ```rust
        /// use enchant_iced::widgets::voice_glow::{resolve_defaults, ResolvedTheme, VoiceGlowType};
        ///
        /// let pill = resolve_defaults(VoiceGlowType::Pill, ResolvedTheme::Dark);
        /// assert_eq!(pill.scale, 0.45);
        /// assert_eq!(pill.reach, 1.35);
        /// ```
        #[non_exhaustive]
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub struct VoiceGeometry {
            $(#[doc = $doc] pub $name: f64,)+
        }

        impl Default for VoiceGeometry {
            /// The tuned defaults: the `default` type, a ~350px chat input
            /// (`voiceDefaults` upstream).
            fn default() -> Self {
                Self { $($name: $default,)+ }
            }
        }

        /// A sparse set of [`VoiceGeometry`] values: `Partial<VoiceGeometry>`.
        #[non_exhaustive]
        #[derive(Clone, Copy, Debug, Default, PartialEq)]
        pub struct GeometryPatch {
            $(#[doc = $doc] pub $name: Option<f64>,)+
        }

        impl GeometryPatch {
            /// `base` with every value this patch sets laid over it.
            #[must_use]
            pub fn apply(&self, base: VoiceGeometry) -> VoiceGeometry {
                VoiceGeometry { $($name: self.$name.unwrap_or(base.$name),)+ }
            }
        }
    };
}
geometry_fields!(define_geometry);

/// Per-type overrides on the defaults. The glow is authored in px for the
/// default host, so a smaller or larger host needs its lobes, range and band
/// rescaled to read the same (`voiceTypePresets` upstream).
///
/// - `Pill`: a ~150x44 recording pill, everything pulled in, a lower reach, a
///   shallower bend, a thinner band.
/// - `Mobile`: the bottom of a phone screen (~400px wide, tall): a wider range
///   and lobe ring, a taller rise, a broader band.
#[must_use]
pub fn type_preset(kind: VoiceGlowType) -> GeometryPatch {
    match kind {
        VoiceGlowType::Pill => GeometryPatch {
            scale: Some(0.45),
            glow_size: Some(0.95),
            stroke_opacity: Some(1.2),
            inner_opacity: Some(0.85),
            reach: Some(1.35),
            spread: Some(1.1),
            flow: Some(0.0),
            bend: Some(23.0),
            band_strength: Some(1.55),
            band_width: Some(1.85),
            band_curve: Some(1.95),
            band_spread: Some(0.38),
            band_offset: Some(-16.0),
            band_tail: Some(0.0),
            distortion: Some(0.45),
            distortion_detail: Some(3.0),
            glow_width: Some(0.65),
            glow_height: Some(0.95),
            lobe_spacing: Some(0.45),
            range_width: Some(0.8),
            range_height: Some(0.7),
            softness: Some(0.88),
            core_size: Some(0.25),
            stroke_scale: Some(1.25),
            inner_scale: Some(0.95),
            bloom_scale: Some(1.05),
            bloom_height: Some(2.25),
            ..GeometryPatch::default()
        },
        VoiceGlowType::Mobile => GeometryPatch {
            scale: Some(1.25),
            spread: Some(0.45),
            reach: Some(3.0),
            flow: Some(60.0),
            bend: Some(70.0),
            band_width: Some(2.4),
            band_curve: Some(1.55),
            band_spread: Some(0.9),
            band_offset: Some(-50.0),
            band_tail: Some(0.62),
            band_tail_position: Some(0.42),
            band_tail_curve: Some(2.7),
            band_tail_overflow: Some(22.0),
            band_strength: Some(1.8),
            distortion_detail: Some(2.0),
            glow_width: Some(1.15),
            glow_height: Some(2.1),
            lobe_spacing: Some(1.35),
            range_width: Some(1.25),
            range_height: Some(1.2),
            softness: Some(1.1),
            ..GeometryPatch::default()
        },
        VoiceGlowType::Default => GeometryPatch::default(),
    }
}

/// Colour tuning a type may carry on top of the theme preset (brightness and
/// saturation default per theme, so these are optional overrides).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VoiceTypeStyle {
    /// Brightness multiplier override.
    pub brightness: Option<f64>,
    /// Saturation multiplier override.
    pub saturation: Option<f64>,
    /// Overall strength override.
    pub strength: Option<f64>,
}

/// The colour tuning for a type on a theme (`resolveVoiceStyle` upstream).
///
/// On the light theme a type's entry replaces the dark one outright: the
/// pill's brightness lift exists to carry the glow on a dark chip, so on
/// light it takes the theme's own.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::{resolve_style, ResolvedTheme, VoiceGlowType};
///
/// assert_eq!(resolve_style(VoiceGlowType::Pill, ResolvedTheme::Dark).brightness, Some(1.35));
/// assert_eq!(resolve_style(VoiceGlowType::Pill, ResolvedTheme::Light).brightness, None);
/// ```
#[must_use]
pub fn resolve_style(kind: VoiceGlowType, theme: ResolvedTheme) -> VoiceTypeStyle {
    match (theme, kind) {
        (ResolvedTheme::Light, VoiceGlowType::Mobile) => VoiceTypeStyle {
            strength: Some(1.0),
            ..VoiceTypeStyle::default()
        },
        (ResolvedTheme::Light, _) => VoiceTypeStyle::default(),
        (ResolvedTheme::Dark, VoiceGlowType::Default) => VoiceTypeStyle {
            brightness: Some(1.15),
            ..VoiceTypeStyle::default()
        },
        (ResolvedTheme::Dark, VoiceGlowType::Pill) => VoiceTypeStyle {
            brightness: Some(1.35),
            saturation: Some(1.5),
            strength: None,
        },
        (ResolvedTheme::Dark, VoiceGlowType::Mobile) => VoiceTypeStyle {
            brightness: Some(1.2),
            saturation: Some(1.5),
            strength: Some(1.0),
        },
    }
}

/// The full geometry for a type on a theme (`resolveVoiceDefaults` upstream):
/// the defaults with the type's overrides and, for what the type leaves
/// alone, the theme's own (the light chat input carries a reach and spread of
/// its own, an epicentre, and a band strength per type).
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::{resolve_defaults, ResolvedTheme, VoiceGlowType};
///
/// let light = resolve_defaults(VoiceGlowType::Default, ResolvedTheme::Light);
/// assert_eq!((light.reach, light.spread, light.core_light, light.band_strength), (1.8, 0.8, 1.8, 1.7));
/// ```
#[must_use]
pub fn resolve_defaults(kind: VoiceGlowType, theme: ResolvedTheme) -> VoiceGeometry {
    let type_patch = type_preset(kind);
    let mut theme_patch = GeometryPatch::default();
    if theme == ResolvedTheme::Light {
        // The light card was tuned taller and narrower than the dark one.
        if type_patch.reach.is_none() {
            theme_patch.reach = Some(1.8);
        }
        if type_patch.spread.is_none() {
            theme_patch.spread = Some(0.8);
        }
        // On white the epicentre reads lighter than the band; dark keeps it off.
        if type_patch.core_light.is_none() {
            theme_patch.core_light = Some(1.8);
        }
    }
    let mut out = theme_patch.apply(VoiceGeometry::default());
    out = type_patch.apply(out);
    if theme == ResolvedTheme::Light {
        // The band on white, whatever the dark strengths.
        out.band_strength = match kind {
            VoiceGlowType::Pill => 2.0,
            VoiceGlowType::Default | VoiceGlowType::Mobile => 1.7,
        };
    }
    out
}

/// Per-theme colour preset (`themePresets` upstream, tuned against
/// border-beam's `line` type on the same stages).
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemePreset {
    /// Opacity of the edge stroke.
    pub stroke_opacity: f64,
    /// Opacity of the inner light.
    pub inner_opacity: f64,
    /// Opacity of the bloom.
    pub bloom_opacity: f64,
    /// Inner inset shadow as `(r, g, b, alpha)`.
    pub inner_shadow: (u8, u8, u8, f64),
    /// Saturation multiplier.
    pub saturation: f64,
    /// Brightness multiplier.
    pub brightness: f64,
    /// Hue drift range in degrees, when the theme carries one (else 24).
    pub hue_range: Option<f64>,
    /// Hue drift period in seconds, when the theme carries one (else 12).
    pub hue_duration: Option<f64>,
    /// Resting hue shift in degrees (else 0).
    pub hue_base: Option<f64>,
    /// Overall strength, when the theme carries one (else 1).
    pub strength: Option<f64>,
    /// Per-theme band strength.
    pub band_strength: Option<f64>,
}

/// The preset for a theme.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::{theme_preset, ResolvedTheme};
///
/// assert_eq!(theme_preset(ResolvedTheme::Dark).saturation, 1.2);
/// assert_eq!(theme_preset(ResolvedTheme::Light).hue_base, Some(5.0));
/// ```
#[must_use]
pub fn theme_preset(theme: ResolvedTheme) -> ThemePreset {
    match theme {
        ResolvedTheme::Dark => ThemePreset {
            stroke_opacity: 1.16,
            inner_opacity: 0.47,
            bloom_opacity: 0.89,
            inner_shadow: (255, 255, 255, 0.1),
            saturation: 1.2,
            brightness: 1.1,
            hue_range: None,
            hue_duration: None,
            hue_base: None,
            strength: None,
            band_strength: None,
        },
        ResolvedTheme::Light => ThemePreset {
            stroke_opacity: 1.2,
            inner_opacity: 0.85,
            bloom_opacity: 0.5,
            inner_shadow: (0, 0, 0, 0.08),
            saturation: 1.6,
            brightness: 0.95,
            hue_range: Some(40.0),
            hue_duration: Some(8.5),
            hue_base: Some(5.0),
            strength: Some(0.8),
            band_strength: Some(1.7),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_voice_defaults_table() {
        let d = VoiceGeometry::default();
        assert_eq!(d.idle, 0.18);
        assert_eq!(d.band_offset, -27.0);
        assert_eq!(d.bloom_height, 1.0);
        assert_eq!(d.lobe_spacing, 0.85);
    }

    #[test]
    fn mobile_light_band_strength_is_overridden_after_the_preset() {
        let dark = resolve_defaults(VoiceGlowType::Mobile, ResolvedTheme::Dark);
        let light = resolve_defaults(VoiceGlowType::Mobile, ResolvedTheme::Light);
        assert_eq!(dark.band_strength, 1.8);
        assert_eq!(light.band_strength, 1.7);
        // Mobile carries its own reach/spread, so the light theme keeps them,
        // but it still gets the epicentre.
        assert_eq!(
            (light.reach, light.spread, light.core_light),
            (3.0, 0.45, 1.8)
        );
    }

    #[test]
    fn pill_light_keeps_type_reach_and_takes_band_two() {
        let light = resolve_defaults(VoiceGlowType::Pill, ResolvedTheme::Light);
        assert_eq!(light.reach, 1.35);
        assert_eq!(light.band_strength, 2.0);
        assert_eq!(light.band_tail, 0.0);
    }

    #[test]
    fn light_style_replaces_dark_entry() {
        assert_eq!(
            resolve_style(VoiceGlowType::Default, ResolvedTheme::Light),
            VoiceTypeStyle::default()
        );
        assert_eq!(
            resolve_style(VoiceGlowType::Mobile, ResolvedTheme::Light).strength,
            Some(1.0)
        );
        assert_eq!(
            resolve_style(VoiceGlowType::Mobile, ResolvedTheme::Dark).saturation,
            Some(1.5)
        );
    }
}
