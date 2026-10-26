//! Voice glow: the model of Libraries.dev's sound-reactive `VoiceBeam`.
//!
//! A centred, colourful beam along the bottom edge of a host element that
//! rises and blooms with the level of a voice. This module is the headless
//! half of the port: every prop, preset, palette and the driver's maths, as a
//! deterministic state machine that yields a [`VoiceGlowFrame`] describing
//! per-lobe geometry, colours, the band curve and every layer's opacity. A
//! renderer only has to paint that frame.
//!
//! The port follows `packages/voice-glow` of the reference repository:
//!
//! | here | upstream |
//! |------|----------|
//! | [`VoiceGlow`] (options) | `VoiceBeamProps` (`types.ts`) |
//! | [`resolve_defaults`], [`resolve_style`], [`theme_preset`] | `presets.ts`, `themePresets` |
//! | [`palette`], [`BandPalette`], [`ColorFilter`], [`mood_colors`] | `voicePalettes`, `color.ts`, `voice-mood.ts` |
//! | [`VoiceGlowEngine`], [`VoiceGlowFrame`] | `voiceDriver.ts` |
//! | [`LevelSource`], [`PcmStream`], [`SpectrumAnalyser`] | `audio.ts`, `readAnalyser` |
//! | [`VoiceProcessing`], [`VoiceGlowMotion`] | `voice-processing.ts`, `VoiceBeamMotion` |
//! | [`VoiceGlow::stroke_opacity`], `inner_opacity`, `bloom_opacity`, [`VoiceGlow::band_opacity`], [`VoiceGlow::core_light_opacity`], [`VoiceGlow::hue_base`], [`VoiceGlow::strength`] | the `--voice-*` CSS hooks |
//! | [`VoiceGlowEngine::is_active`], `is_fading`, `is_paused`, `is_warp_off` | `data-active`, `data-fading`, `data-paused`, `data-voice-warp` |
//! | [`VoiceGlowRaster`], [`VoiceBeam`] | the generated stylesheet + SVG filter + band canvas |
//!
//! # Not ported
//!
//! `look="dots"` and `look="lines"` (and the `dot*`, `line*`, `surface*`,
//! `texture`, `gravity`, `seeThrough` props) are unreleased upstream: the
//! default build compiles them out (`LOOKS.md`), so only `look="glow"` exists
//! here. The React-only plumbing (`className`, `style`, `css`, the SVG
//! filter's WebKit area gate, the `IntersectionObserver` pause) has no
//! equivalent: drive visibility by simply not stepping the engine.
//!
//! # Examples
//!
//! ```rust
//! use std::time::Duration;
//! use enchant_iced::widgets::voice_glow::{
//!     ColorVariant, HostSize, ResolvedTheme, VoiceGlow, VoiceGlowType,
//! };
//!
//! let glow = VoiceGlow::new()
//!     .kind(VoiceGlowType::Mobile)
//!     .color_variant(ColorVariant::Sunset)
//!     .level(0.6);
//! let frame = glow.frame_at(
//!     Duration::from_millis(1500),
//!     HostSize::new(400.0, 300.0),
//!     ResolvedTheme::Dark,
//! );
//! assert_eq!(frame.lobes.len(), 7);
//! assert!(frame.layers.stroke > 0.0);
//! ```

mod audio;
pub(crate) mod blur;
mod config;
mod driver;
mod frame;
mod geometry;
mod headless;
#[cfg(feature = "microphone")]
mod microphone;
pub(crate) mod noise;
mod options;
mod paint;
mod palette;
mod presets;
mod processing;
mod raster;
mod widget;

pub use audio::{
    BAND_GAIN, BANDS_HZ, BASE_GAIN, FFT_SIZE, LevelSource, PcmStream, Reading, SMOOTHING,
    SpectrumAnalyser,
};
pub use config::{
    BORDER_WIDTH, CoreLightStyle, DEFAULT_RADIUS, HotSpot, LobeLayer, VoiceGlowConfig,
    VoiceGlowStyle,
};
pub use driver::{FRAME_STEP, MAX_STEP, VoiceGlowEngine};
pub use frame::{
    BandPaint, BandRidge, BandStroke, DistortionFrame, HostSize, LayerOpacity, LobeFrame,
    VoiceGlowFrame,
};
pub use geometry::{BAND_SAMPLES, LOBE_SPACING, LOBE_SPAN, LOBES, VoiceLobe};
pub use headless::{GlowLayer, RenderOptions};
#[cfg(feature = "microphone")]
pub use microphone::{Microphone, MicrophoneConstraints, MicrophoneState};
pub use options::{
    BandColors, Callback, Degrees, ResolvedTheme, VoiceGlow, VoiceGlowTheme, VoiceGlowType,
};
pub use palette::{
    BandPalette, ColorFilter, ColorVariant, LobePalette, MoodPoint, MoodPreset, ParseRgbError, Rgb,
    mood_band, mood_colors, palette,
};
pub use presets::{
    GeometryPatch, ThemePreset, VoiceGeometry, VoiceTypeStyle, resolve_defaults, resolve_style,
    theme_preset, type_preset,
};
pub use processing::{MotionSource, ProcessingTuning, VoiceGlowMotion, VoiceProcessing};
pub use raster::{GlowBitmap, VoiceGlowRaster};
pub use widget::{GlowLayering, VoiceBeam};

/// Whether this build carries the surface looks (`look="dots"` / `"lines"`),
/// `VOICE_SURFACE_LOOKS` upstream. They are not released upstream, so this
/// is always `false`: the glow is the only look.
///
/// # Examples
///
/// ```rust
/// assert!(!enchant_iced::widgets::voice_glow::VOICE_SURFACE_LOOKS);
/// ```
pub const VOICE_SURFACE_LOOKS: bool = false;
