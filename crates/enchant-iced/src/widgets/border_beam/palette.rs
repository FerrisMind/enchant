//! Public option enums and the internal palette data model.
//!
//! The tables themselves live in [`super::data`]; this module only defines the
//! types they are made of and the names exposed to users.

use iced::Color;
use std::fmt;

/// A straight (non-premultiplied) RGBA colour with components in `0..=1`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// Builds an [`Rgba`] from 8-bit channels and a unit alpha.
pub(crate) const fn rgba(r: u8, g: u8, b: u8, a: f32) -> Rgba {
    Rgba {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a,
    }
}

impl Rgba {
    pub(crate) const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    pub(crate) fn with_alpha(self, a: f32) -> Self {
        Self {
            a: a.clamp(0.0, 1.0),
            ..self
        }
    }

    pub(crate) fn to_color(self) -> Color {
        Color::from_rgba(self.r, self.g, self.b, self.a)
    }
}

/// One `radial-gradient(ellipse <w> <h> at <cx> <cy>, color, transparent)`.
///
/// `cx`/`cy` are fractions of the host box; `w`/`h` are logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GradientDef {
    pub color: Rgba,
    pub cx: f32,
    pub cy: f32,
    pub w: f32,
    pub h: f32,
}

/// One line-family sprite: position, authored size and offset from the beam
/// centre (`dx`/`dy` in px).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LineSprite {
    pub color: Rgba,
    pub w: f32,
    pub h: f32,
    pub dx: f32,
    pub dy: f32,
}

/// A line-bloom spike pair (`color1` at the core, `color2` further out).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LineBloomSpike {
    pub c1: Rgba,
    pub c2: Rgba,
}

/// One palette's full set of gradient tables.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PaletteColors {
    pub border: [GradientDef; 9],
    pub small_border: [GradientDef; 8],
    pub small_inner: [GradientDef; 8],
    pub line_dark: [LineSprite; 9],
    pub line_light: [LineSprite; 9],
    pub line_inner: [LineSprite; 9],
    pub bloom_dark: [LineBloomSpike; 5],
    pub bloom_light: [LineBloomSpike; 5],
    pub spike_dark: [Rgba; 2],
    pub spike_light: [Rgba; 2],
}

/// The theme-dependent opacities and filter values of one size preset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ThemePreset {
    pub stroke: f32,
    pub inner: f32,
    pub bloom: f32,
    pub inner_shadow: Rgba,
    pub saturation: f32,
    pub brightness: f32,
    pub hairline: f32,
}

/// One entry of the pulse geometry tables.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PulseGradientDef {
    pub palette_index: u8,
    pub w: f32,
    pub h: f32,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub region: u8,
    pub quad: Quadrant,
}

/// The quadrant whose opacity oscillator scales a pulse gradient.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Quadrant {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Quadrant {
    pub(crate) fn index(self) -> usize {
        match self {
            Self::TopLeft => 0,
            Self::TopRight => 1,
            Self::BottomLeft => 2,
            Self::BottomRight => 3,
        }
    }
}

/// Visual size presets, mirroring Libraries.dev `BorderBeamSize`.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::BorderBeamSize;
///
/// assert_eq!(BorderBeamSize::default(), BorderBeamSize::Md);
/// assert_eq!(BorderBeamSize::PulseOutside.to_string(), "pulse-outside");
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BorderBeamSize {
    /// Compact beam (70x36-ish targets), default radius 32.
    Sm,
    /// General-purpose rotating border glow (default).
    #[default]
    Md,
    /// A travelling glow that hugs the bottom edge.
    Line,
    /// A contained, breathing perimeter glow.
    PulseInner,
    /// A breathing glow that spills outward behind the child.
    PulseOutside,
}

impl BorderBeamSize {
    /// All presets, in reference order (`pulse-outside` before `pulse-inner`).
    pub const ALL: [Self; 5] = [
        Self::Sm,
        Self::Md,
        Self::Line,
        Self::PulseOutside,
        Self::PulseInner,
    ];

    pub(crate) fn index(self) -> usize {
        match self {
            Self::Sm => 0,
            Self::Md => 1,
            Self::Line => 2,
            Self::PulseOutside => 3,
            Self::PulseInner => 4,
        }
    }

    pub(crate) fn is_pulse(self) -> bool {
        matches!(self, Self::PulseInner | Self::PulseOutside)
    }
}

impl fmt::Display for BorderBeamSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Sm => "sm",
            Self::Md => "md",
            Self::Line => "line",
            Self::PulseInner => "pulse-inner",
            Self::PulseOutside => "pulse-outside",
        })
    }
}

/// Colour palettes (`colorVariant`).
///
/// [`Mono`](Self::Mono) forces static colours and halves the layer opacities
/// for every size except `line`, exactly like the reference.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::BorderBeamPalette;
///
/// assert_eq!(BorderBeamPalette::default(), BorderBeamPalette::Colorful);
/// assert_eq!(BorderBeamPalette::ALL.len(), 8);
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BorderBeamPalette {
    /// Red / blue / green / violet / amber.
    #[default]
    Colorful,
    /// Greys; static and half opacity.
    Mono,
    /// Deep blues and violets.
    Ocean,
    /// Oranges, ambers and reds.
    Sunset,
    /// Greens and teals.
    Forest,
    /// Pinks, magentas and purples.
    Candy,
    /// Light blues and cyans.
    Ice,
    /// Warm yellows and golds.
    Gold,
}

impl BorderBeamPalette {
    /// All palettes, in reference order.
    pub const ALL: [Self; 8] = [
        Self::Colorful,
        Self::Mono,
        Self::Ocean,
        Self::Sunset,
        Self::Forest,
        Self::Candy,
        Self::Ice,
        Self::Gold,
    ];

    pub(crate) fn index(self) -> usize {
        match self {
            Self::Colorful => 0,
            Self::Mono => 1,
            Self::Ocean => 2,
            Self::Sunset => 3,
            Self::Forest => 4,
            Self::Candy => 5,
            Self::Ice => 6,
            Self::Gold => 7,
        }
    }

    pub(crate) fn is_mono(self) -> bool {
        self == Self::Mono
    }
}

impl fmt::Display for BorderBeamPalette {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Colorful => "colorful",
            Self::Mono => "mono",
            Self::Ocean => "ocean",
            Self::Sunset => "sunset",
            Self::Forest => "forest",
            Self::Candy => "candy",
            Self::Ice => "ice",
            Self::Gold => "gold",
        })
    }
}

/// How the theme preset is chosen.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::{BorderBeamTheme, ResolvedBeamTheme};
///
/// assert_eq!(BorderBeamTheme::Auto.resolve(ResolvedBeamTheme::Light), ResolvedBeamTheme::Light);
/// assert_eq!(BorderBeamTheme::Dark.resolve(ResolvedBeamTheme::Light), ResolvedBeamTheme::Dark);
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BorderBeamTheme {
    /// Use the host application's palette.
    Auto,
    /// Dark card tuning (reference default).
    #[default]
    Dark,
    /// Light card tuning.
    Light,
}

impl BorderBeamTheme {
    /// Resolves [`Auto`](Self::Auto) against the host theme.
    pub fn resolve(self, system: ResolvedBeamTheme) -> ResolvedBeamTheme {
        match self {
            Self::Auto => system,
            Self::Dark => ResolvedBeamTheme::Dark,
            Self::Light => ResolvedBeamTheme::Light,
        }
    }
}

impl fmt::Display for BorderBeamTheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Auto => "auto",
            Self::Dark => "dark",
            Self::Light => "light",
        })
    }
}

/// A theme preset that has been resolved to a concrete light/dark value.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::{BorderBeamTheme, ResolvedBeamTheme};
///
/// let system = ResolvedBeamTheme::Light;
/// assert_eq!(BorderBeamTheme::Auto.resolve(system), system);
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ResolvedBeamTheme {
    /// Dark tuning.
    #[default]
    Dark,
    /// Light tuning.
    Light,
}

impl ResolvedBeamTheme {
    /// Whether this is the dark preset.
    pub fn is_dark(self) -> bool {
        self == Self::Dark
    }
}

impl fmt::Display for ResolvedBeamTheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.is_dark() { "dark" } else { "light" })
    }
}

/// The laid-out size of the wrapped child, in logical pixels.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::BeamHost;
///
/// let host = BeamHost::new(348.0, 122.0);
/// assert_eq!(host.width(), 348.0);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BeamHost {
    width: f32,
    height: f32,
}

impl BeamHost {
    /// Creates a host size. Non-finite and negative values are treated as `0`.
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width: finite_positive(width),
            height: finite_positive(height),
        }
    }

    /// Width in logical pixels.
    pub fn width(self) -> f32 {
        self.width
    }

    /// Height in logical pixels.
    pub fn height(self) -> f32 {
        self.height
    }
}

fn finite_positive(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_matches_the_index_order_used_by_the_presets() {
        for (position, size) in BorderBeamSize::ALL.into_iter().enumerate() {
            assert_eq!(size.index(), position, "{size}");
        }
        for (position, palette) in BorderBeamPalette::ALL.into_iter().enumerate() {
            assert_eq!(palette.index(), position, "{palette}");
        }
    }
}
