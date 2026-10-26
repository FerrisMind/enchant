//! The `VoiceGlow` builder: every prop of the upstream `VoiceBeam`
//! component (`types.ts`), as a fluent, validated setter.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use super::audio::LevelSource;
use super::palette::{ColorVariant, Rgb};
use super::presets::{GeometryPatch, geometry_fields};
use super::processing::{MotionSource, ProcessingTuning, VoiceGlowMotion};

/// Host preset: retunes the glow's geometry for the element it wraps.
///
/// A preset only supplies defaults; every geometry setter still overrides it.
/// Upstream: `VoiceBeamType`.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VoiceGlowType {
    /// A chat input or card, ~350px wide.
    #[default]
    Default,
    /// A small recording pill, ~150x44: glow pulled in, shallower bend.
    Pill,
    /// The bottom of a phone screen: wider range, taller rise.
    Mobile,
}

impl VoiceGlowType {
    /// The upstream string name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Pill => "pill",
            Self::Mobile => "mobile",
        }
    }
}

/// Theme mode: adapts the beam's colours to the background; `Auto` follows
/// the system. Upstream: `VoiceBeamTheme`.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VoiceGlowTheme {
    /// For dark backgrounds (default).
    #[default]
    Dark,
    /// For light backgrounds.
    Light,
    /// Follows the host's system theme.
    Auto,
}

impl VoiceGlowTheme {
    /// Resolves `Auto` against the system's theme.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use enchant_iced::widgets::voice_glow::{ResolvedTheme, VoiceGlowTheme};
    ///
    /// assert_eq!(VoiceGlowTheme::Auto.resolve(ResolvedTheme::Light), ResolvedTheme::Light);
    /// assert_eq!(VoiceGlowTheme::Dark.resolve(ResolvedTheme::Light), ResolvedTheme::Dark);
    /// ```
    #[must_use]
    pub const fn resolve(self, system: ResolvedTheme) -> ResolvedTheme {
        match self {
            Self::Dark => ResolvedTheme::Dark,
            Self::Light => ResolvedTheme::Light,
            Self::Auto => system,
        }
    }
}

/// A theme with `Auto` resolved away.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ResolvedTheme {
    /// Dark background.
    #[default]
    Dark,
    /// Light background.
    Light,
}

impl ResolvedTheme {
    /// `true` for [`ResolvedTheme::Dark`].
    #[must_use]
    pub const fn is_dark(self) -> bool {
        matches!(self, Self::Dark)
    }
}

/// An angle in degrees (hue drift range).
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::Degrees;
///
/// assert_eq!(Degrees::new(24.0).get(), 24.0);
/// assert_eq!(Degrees::from(40.0), Degrees::new(40.0));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Degrees(f64);

impl Degrees {
    /// An angle; non-finite input is 0.
    #[must_use]
    pub fn new(degrees: f64) -> Self {
        Self(if degrees.is_finite() { degrees } else { 0.0 })
    }

    /// The angle in degrees.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl From<f64> for Degrees {
    fn from(degrees: f64) -> Self {
        Self::new(degrees)
    }
}

/// A shareable callback (`onLevel`, `onActivate`, `onDeactivate`). Two
/// callbacks are equal when they are the same closure.
pub struct Callback<T>(Arc<dyn Fn(T) + Send + Sync>);

impl<T> Callback<T> {
    /// Wraps a closure.
    #[must_use]
    pub fn new(f: impl Fn(T) + Send + Sync + 'static) -> Self {
        Self(Arc::new(f))
    }

    pub(crate) fn call(&self, value: T) {
        (self.0)(value);
    }
}

impl<T> Clone for Callback<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T> fmt::Debug for Callback<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Callback(..)")
    }
}

impl<T> PartialEq for Callback<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// Your own band colours: unset ones keep the theme's defaults.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::{BandColors, Rgb};
///
/// let band = BandColors::new().core(Rgb::new(255, 240, 200));
/// assert_eq!(band.core, Some(Rgb::new(255, 240, 200)));
/// assert_eq!(band.above, None);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BandColors {
    /// The core ridge.
    pub core: Option<Rgb>,
    /// The fringe above.
    pub above: Option<Rgb>,
    /// The fringe between.
    pub mid: Option<Rgb>,
    /// The fringe below.
    pub below: Option<Rgb>,
}

impl BandColors {
    /// No overrides.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Overrides the core colour.
    #[must_use]
    pub fn core(mut self, c: Rgb) -> Self {
        self.core = Some(c);
        self
    }

    /// Overrides the fringe above.
    #[must_use]
    pub fn above(mut self, c: Rgb) -> Self {
        self.above = Some(c);
        self
    }

    /// Overrides the fringe between.
    #[must_use]
    pub fn mid(mut self, c: Rgb) -> Self {
        self.mid = Some(c);
        self
    }

    /// Overrides the fringe below.
    #[must_use]
    pub fn below(mut self, c: Rgb) -> Self {
        self.below = Some(c);
        self
    }
}

/// Builder for a voice glow: the model of the upstream `VoiceBeam`.
///
/// Nothing is drawn here. Resolve the props with [`VoiceGlow::config`], drive
/// them with a [`VoiceGlowEngine`](super::VoiceGlowEngine), or ask for one
/// deterministic instant with [`VoiceGlow::frame_at`].
///
/// Defaults match the component: `type` default, `theme` dark, `sensitivity`
/// 3.1, `threshold` 0.015, `attack` 0.325 s, `release` 0.86 s, breathing every
/// 5.2 s, `bands` on, `colorVariant` colourful, `active` on, a 16px corner
/// radius, level 0.
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use enchant_iced::widgets::voice_glow::{
///     ColorVariant, HostSize, ResolvedTheme, VoiceGlow, VoiceGlowType,
/// };
///
/// let glow = VoiceGlow::new()
///     .kind(VoiceGlowType::Pill)
///     .color_variant(ColorVariant::Ocean)
///     .level(0.7)
///     .bend(30.0);
/// let frame = glow.frame_at(Duration::from_secs(1), HostSize::new(150.0, 44.0), ResolvedTheme::Dark);
/// assert!(frame.level > 0.3);
/// ```
#[must_use = "builders do nothing unless resolved into a config, engine or frame"]
#[derive(Clone, Debug, PartialEq)]
pub struct VoiceGlow {
    pub(super) kind: VoiceGlowType,
    pub(super) geometry: GeometryPatch,
    pub(super) level: LevelSource,
    pub(super) sensitivity: f64,
    pub(super) threshold: f64,
    pub(super) attack: Duration,
    pub(super) release: Duration,
    pub(super) breathe_duration: Duration,
    pub(super) bands: bool,
    pub(super) motion: MotionSource,
    pub(super) processing: bool,
    pub(super) processing_tuning: Option<ProcessingTuning>,
    pub(super) color_variant: ColorVariant,
    pub(super) colors: [Option<Rgb>; 7],
    pub(super) band_colors: BandColors,
    pub(super) theme: VoiceGlowTheme,
    pub(super) static_colors: bool,
    pub(super) hue_range: Option<Degrees>,
    pub(super) hue_duration: Option<Duration>,
    pub(super) active: bool,
    pub(super) paused: bool,
    pub(super) reduced_motion: bool,
    pub(super) border_radius: Option<f64>,
    pub(super) brightness: Option<f64>,
    pub(super) saturation: Option<f64>,
    pub(super) strength: Option<f64>,
    pub(super) band_opacity: f64,
    pub(super) core_light_opacity: f64,
    pub(super) hue_base: Option<Degrees>,
    pub(super) on_level: Option<Callback<f64>>,
    pub(super) on_activate: Option<Callback<()>>,
    pub(super) on_deactivate: Option<Callback<()>>,
}

impl Default for VoiceGlow {
    fn default() -> Self {
        Self {
            kind: VoiceGlowType::Default,
            geometry: GeometryPatch::default(),
            level: LevelSource::default(),
            sensitivity: 3.1,
            threshold: 0.015,
            attack: Duration::from_secs_f64(0.325),
            release: Duration::from_secs_f64(0.86),
            breathe_duration: Duration::from_secs_f64(5.2),
            bands: true,
            motion: MotionSource::Rest,
            processing: false,
            processing_tuning: None,
            color_variant: ColorVariant::Colorful,
            colors: [None; 7],
            band_colors: BandColors::default(),
            theme: VoiceGlowTheme::Dark,
            static_colors: false,
            hue_range: None,
            hue_duration: None,
            active: true,
            paused: false,
            reduced_motion: false,
            border_radius: None,
            brightness: None,
            saturation: None,
            strength: None,
            band_opacity: 1.0,
            core_light_opacity: 1.0,
            hue_base: None,
            on_level: None,
            on_activate: None,
            on_deactivate: None,
        }
    }
}

macro_rules! define_geometry_setters {
    ($($name:ident, $doc:literal, $default:expr;)+) => {
        impl VoiceGlow {
            $(
                #[doc = $doc]
                ///
                /// Non-finite values are ignored; the preset's value stays.
                pub fn $name(mut self, value: f64) -> Self {
                    if value.is_finite() {
                        self.geometry.$name = Some(value);
                    }
                    self
                }
            )+
        }
    };
}
geometry_fields!(define_geometry_setters);

impl VoiceGlow {
    /// A glow with the component's defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Host preset (`type`): sets the defaults of the geometry setters for
    /// that host; any setter you call still wins.
    pub fn kind(mut self, kind: VoiceGlowType) -> Self {
        self.kind = kind;
        self
    }

    /// A manual level, 0-1 (`level` as a number).
    pub fn level(mut self, level: f64) -> Self {
        self.level = LevelSource::Constant(level);
        self
    }

    /// Any level source: a getter sampled every frame, or an analysed PCM
    /// stream (`level` as a getter, or `stream`).
    pub fn level_source(mut self, source: impl Into<LevelSource>) -> Self {
        self.level = source.into();
        self
    }

    /// Input gain on the analysed audio. The default 3.1 suits a laptop
    /// microphone at conversational distance. Negative values clamp to 0.
    pub fn sensitivity(mut self, sensitivity: f64) -> Self {
        self.sensitivity = if sensitivity.is_finite() {
            sensitivity.max(0.0)
        } else {
            self.sensitivity
        };
        self
    }

    /// Noise gate, 0-1 (clamped to 0-0.95): levels below it read as silence.
    pub fn threshold(mut self, threshold: f64) -> Self {
        self.threshold = if threshold.is_finite() {
            threshold.clamp(0.0, 0.95)
        } else {
            self.threshold
        };
        self
    }

    /// Time the glow takes to rise toward a louder level.
    pub fn attack(mut self, attack: Duration) -> Self {
        self.attack = attack;
        self
    }

    /// Time the glow takes to settle after the sound drops.
    pub fn release(mut self, release: Duration) -> Self {
        self.release = release;
        self
    }

    /// Period of the idle breathing (minimum 0.2 s).
    pub fn breathe_duration(mut self, period: Duration) -> Self {
        self.breathe_duration = period;
        self
    }

    /// Let the low / mid / high bands move the lobes independently. With a
    /// manual level the bands are synthesised from the level.
    pub fn bands(mut self, bands: bool) -> Self {
        self.bands = bands;
        self
    }

    /// The glow's large-scale motion, sampled every frame.
    pub fn motion(mut self, motion: MotionSource) -> Self {
        self.motion = motion;
        self
    }

    /// A fixed motion.
    pub fn motion_fixed(mut self, motion: VoiceGlowMotion) -> Self {
        self.motion = MotionSource::Fixed(motion);
        self
    }

    /// Turns the processing state on or off: the lobes gather into one beam
    /// that sweeps the range (`useVoiceProcessing`). An explicit
    /// [`motion`](Self::motion) that supplies a value wins.
    pub fn processing(mut self, processing: bool) -> Self {
        self.processing = processing;
        self
    }

    /// Overrides the processing sweep's tuning (defaults per host type).
    pub fn processing_tuning(mut self, tuning: ProcessingTuning) -> Self {
        self.processing_tuning = Some(tuning);
        self
    }

    /// Colour variant of the beam.
    pub fn color_variant(mut self, variant: ColorVariant) -> Self {
        self.color_variant = variant;
        self
    }

    /// Your own lobe colours, centre first then the pairs outward: up to
    /// seven. Slots you leave out keep the variant's colour.
    pub fn colors(mut self, colors: impl IntoIterator<Item = Rgb>) -> Self {
        self.colors = [None; 7];
        for (slot, c) in self.colors.iter_mut().zip(colors) {
            *slot = Some(c);
        }
        self
    }

    /// Your own lobe colours as CSS strings (`#rgb`, `#rrggbb`, `rgb()`,
    /// `rgba()`). A string that fails to parse keeps the variant's colour.
    pub fn colors_css<S: AsRef<str>>(mut self, colors: impl IntoIterator<Item = S>) -> Self {
        self.colors = [None; 7];
        for (slot, c) in self.colors.iter_mut().zip(colors) {
            *slot = c.as_ref().parse().ok();
        }
        self
    }

    /// Overrides one lobe colour slot (0 = centre). Slots past 6 are ignored.
    pub fn color_slot(mut self, slot: usize, color: Rgb) -> Self {
        if let Some(s) = self.colors.get_mut(slot) {
            *s = Some(color);
        }
        self
    }

    /// The band's colours: the core ridge and its chromatic fringes.
    pub fn band_colors(mut self, colors: BandColors) -> Self {
        self.band_colors = colors;
        self
    }

    /// Theme mode.
    pub fn theme(mut self, theme: VoiceGlowTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Disables the slow hue drift. (Always on for the mono variant.)
    pub fn static_colors(mut self, static_colors: bool) -> Self {
        self.static_colors = static_colors;
        self
    }

    /// Hue drift range: the palette wanders +/- this far. Default 24 degrees
    /// (dark) / 40 (light).
    pub fn hue_range(mut self, range: Degrees) -> Self {
        self.hue_range = Some(range);
        self
    }

    /// Period of the hue drift (minimum 0.5 s). Default 12 s (dark) / 8.5 s
    /// (light).
    pub fn hue_duration(mut self, period: Duration) -> Self {
        self.hue_duration = Some(period);
        self
    }

    /// Whether the effect is on. Off fades the beam out and stops the audio
    /// analysis.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Freezes the effect where it is: the glow, the band and the audio
    /// analysis all hold their last frame, without fading out. The clocks
    /// resume where they stopped.
    pub fn paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        self
    }

    /// Reduced motion: no flow, no hue drift, no distortion, no beam travel,
    /// a still breath. iced exposes no OS setting, so this is the host's
    /// switch (upstream reads `prefers-reduced-motion`).
    pub fn reduced_motion(mut self, reduced: bool) -> Self {
        self.reduced_motion = reduced;
        self
    }

    /// The host's corner radius in px. Default 16 (upstream auto-detects it
    /// from the first child; an iced host has to say).
    pub fn border_radius(mut self, radius: f64) -> Self {
        if radius.is_finite() {
            self.border_radius = Some(radius.max(0.0));
        }
        self
    }

    /// Brightness multiplier. Default per theme and type (see
    /// [`resolve_style`](super::resolve_style)).
    pub fn brightness(mut self, brightness: f64) -> Self {
        if brightness.is_finite() {
            self.brightness = Some(brightness);
        }
        self
    }

    /// Saturation multiplier. Default per theme and type.
    pub fn saturation(mut self, saturation: f64) -> Self {
        if saturation.is_finite() {
            self.saturation = Some(saturation);
        }
        self
    }

    /// Opacity multiplier of the band layer, on top of [`strength`](Self::strength)
    /// (the `--voice-band-opacity` CSS hook). Default 1; negative values clamp
    /// to 0.
    pub fn band_opacity(mut self, opacity: f64) -> Self {
        if opacity.is_finite() {
            self.band_opacity = opacity.max(0.0);
        }
        self
    }

    /// Opacity multiplier of the epicentre wash (the
    /// `--voice-core-light-opacity` CSS hook). Default 1; negative values
    /// clamp to 0.
    pub fn core_light_opacity(mut self, opacity: f64) -> Self {
        if opacity.is_finite() {
            self.core_light_opacity = opacity.max(0.0);
        }
        self
    }

    /// A fixed hue shift added to the drift (the `--voice-hue-base` CSS hook);
    /// replaces the theme's own resting shift (5 degrees on light).
    pub fn hue_base(mut self, base: Degrees) -> Self {
        self.hue_base = Some(base);
        self
    }

    /// Audio to react to (`stream`): an analysed PCM stream. Wins over a
    /// manual `level`, like upstream; shorthand for
    /// `level_source(LevelSource::pcm(stream))`.
    pub fn stream(mut self, stream: super::PcmStream) -> Self {
        self.level = LevelSource::pcm(stream);
        self
    }

    /// Overall strength/opacity of the effect, 0-1 (clamped). Default 1 dark,
    /// 0.8 light; 1 on both for `Mobile`.
    pub fn strength(mut self, strength: f64) -> Self {
        if strength.is_finite() {
            self.strength = Some(strength);
        }
        self
    }

    /// Called every step with the smoothed level (0-1) the glow is showing
    /// (`onLevel`).
    pub fn on_level(mut self, f: impl Fn(f64) + Send + Sync + 'static) -> Self {
        self.on_level = Some(Callback::new(f));
        self
    }

    /// Called when the fade-in completes (`onActivate`).
    pub fn on_activate(mut self, f: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_activate = Some(Callback::new(move |()| f()));
        self
    }

    /// Called when the fade-out completes (`onDeactivate`).
    pub fn on_deactivate(mut self, f: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_deactivate = Some(Callback::new(move |()| f()));
        self
    }
}
