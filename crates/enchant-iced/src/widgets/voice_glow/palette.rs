//! Colours: the eight palettes (dark and light), custom-colour parsing
//! (`color.ts`), the CSS filter chain the glow runs its colours through
//! (hue-rotate, brightness, saturate) and the mood palette of the Studio
//! (`sites/home/src/studio/voice-mood.ts`).

use std::fmt;
use std::str::FromStr;

use super::options::ResolvedTheme;

/// An 8-bit sRGB colour.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::Rgb;
///
/// let c: Rgb = "#f80".parse()?;
/// assert_eq!(c, Rgb::new(255, 136, 0));
/// assert_eq!("rgb(10, 20.4, 30)".parse::<Rgb>()?, Rgb::new(10, 20, 30));
/// assert!("not a colour".parse::<Rgb>().is_err());
/// # Ok::<(), enchant_iced::widgets::voice_glow::ParseRgbError>(())
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rgb {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
}

impl Rgb {
    /// A colour from its channels.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// The channels as `0.0..=1.0` floats.
    #[must_use]
    pub fn to_unit(self) -> [f64; 3] {
        [
            f64::from(self.r) / 255.0,
            f64::from(self.g) / 255.0,
            f64::from(self.b) / 255.0,
        ]
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "rgb({}, {}, {})", self.r, self.g, self.b)
    }
}

impl From<[u8; 3]> for Rgb {
    fn from([r, g, b]: [u8; 3]) -> Self {
        Self { r, g, b }
    }
}

impl From<Rgb> for iced::Color {
    fn from(c: Rgb) -> Self {
        iced::Color::from_rgb8(c.r, c.g, c.b)
    }
}

/// A colour string was not `#rgb`, `#rrggbb`, `rgb(r, g, b)` or `rgba(r, g, b, a)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseRgbError {
    input: String,
}

impl fmt::Display for ParseRgbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`{}` is not a #rgb, #rrggbb, rgb() or rgba() colour",
            self.input
        )
    }
}

impl std::error::Error for ParseRgbError {}

/// JS `Math.round` for the non-negative values a colour channel takes.
fn js_round(v: f64) -> f64 {
    (v + 0.5).floor()
}

fn clamp_channel(v: f64) -> u8 {
    v.clamp(0.0, 255.0) as u8
}

fn leading_number(s: &str) -> Option<(f64, &str)> {
    let end = s
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(s.len());
    if end == 0 {
        return None;
    }
    Some((s[..end].parse().ok()?, &s[end..]))
}

impl FromStr for Rgb {
    type Err = ParseRgbError;

    /// Port of `parseRgb` (`color.ts`): `#rgb`, `#rrggbb`, `rgb(r, g, b)`
    /// and `rgba(r, g, b, a)`. Channels above 255 saturate.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ParseRgbError {
            input: s.to_owned(),
        };
        let c = s.trim();
        if let Some(hex) = c.strip_prefix('#') {
            if !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
                return Err(err());
            }
            let expanded: String = match hex.len() {
                3 => hex.chars().flat_map(|ch| [ch, ch]).collect(),
                6 => hex.to_owned(),
                _ => return Err(err()),
            };
            let channel = |i: usize| u8::from_str_radix(&expanded[i..i + 2], 16).map_err(|_| err());
            return Ok(Self::new(channel(0)?, channel(2)?, channel(4)?));
        }
        let lower = c.to_ascii_lowercase();
        let rest = lower
            .strip_prefix("rgba(")
            .or_else(|| lower.strip_prefix("rgb("))
            .ok_or_else(err)?;
        let mut channels = [0.0_f64; 3];
        let mut rest = rest.trim_start();
        for (i, slot) in channels.iter_mut().enumerate() {
            let (value, tail) = leading_number(rest).ok_or_else(err)?;
            *slot = js_round(value);
            rest = tail.trim_start();
            if i < 2 {
                rest = rest.strip_prefix(',').ok_or_else(err)?.trim_start();
            }
        }
        Ok(Self::new(
            clamp_channel(channels[0]),
            clamp_channel(channels[1]),
            clamp_channel(channels[2]),
        ))
    }
}

/// Colour variant of the beam: the same eight palettes as border-beam.
///
/// Upstream: `VoiceBeamColorVariant`.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColorVariant {
    /// Full spectrum, Siri-like (default).
    #[default]
    Colorful,
    /// Monochromatic grayscale.
    Mono,
    /// Blue and purple tones.
    Ocean,
    /// Warm orange, yellow and red tones.
    Sunset,
    /// Green and teal tones.
    Forest,
    /// Pink and magenta tones.
    Candy,
    /// Cyan and pale blue tones.
    Ice,
    /// Amber and yellow tones.
    Gold,
}

impl ColorVariant {
    /// Every variant, in upstream declaration order.
    pub const ALL: [ColorVariant; 8] = [
        Self::Colorful,
        Self::Mono,
        Self::Ocean,
        Self::Sunset,
        Self::Forest,
        Self::Candy,
        Self::Ice,
        Self::Gold,
    ];

    /// The upstream string name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Colorful => "colorful",
            Self::Mono => "mono",
            Self::Ocean => "ocean",
            Self::Sunset => "sunset",
            Self::Forest => "forest",
            Self::Candy => "candy",
            Self::Ice => "ice",
            Self::Gold => "gold",
        }
    }
}

impl fmt::Display for ColorVariant {
    /// The upstream string name (`"ocean"`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// One colour per lobe: centre first, then pairs outward.
pub type LobePalette = [Rgb; 7];

const fn p(rows: [[u8; 3]; 7]) -> LobePalette {
    let mut out = [Rgb::new(0, 0, 0); 7];
    let mut i = 0;
    while i < 7 {
        out[i] = Rgb::new(rows[i][0], rows[i][1], rows[i][2]);
        i += 1;
    }
    out
}

/// The seven lobe colours of a variant on a theme (`voicePalettes`
/// upstream). Light palettes use deeper values because they sit on a white
/// surface at far lower opacity.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::{palette, ColorVariant, ResolvedTheme, Rgb};
///
/// let dark = palette(ColorVariant::Colorful, ResolvedTheme::Dark);
/// assert_eq!(dark[0], Rgb::new(255, 70, 120));
/// ```
#[must_use]
pub const fn palette(variant: ColorVariant, theme: ResolvedTheme) -> LobePalette {
    match (variant, theme) {
        (ColorVariant::Colorful, ResolvedTheme::Dark) => p([
            [255, 70, 120],
            [60, 190, 255],
            [175, 70, 255],
            [60, 220, 130],
            [255, 150, 40],
            [90, 100, 255],
            [40, 200, 190],
        ]),
        (ColorVariant::Colorful, ResolvedTheme::Light) => p([
            [255, 201, 21],
            [126, 196, 255],
            [180, 40, 230],
            [235, 100, 160],
            [255, 176, 122],
            [154, 160, 255],
            [127, 217, 238],
        ]),
        (ColorVariant::Mono, ResolvedTheme::Dark) => p([
            [215, 215, 215],
            [180, 180, 180],
            [190, 190, 190],
            [160, 160, 160],
            [170, 170, 170],
            [150, 150, 150],
            [155, 155, 155],
        ]),
        (ColorVariant::Mono, ResolvedTheme::Light) => p([
            [60, 60, 60],
            [90, 90, 90],
            [85, 85, 85],
            [110, 110, 110],
            [105, 105, 105],
            [125, 125, 125],
            [120, 120, 120],
        ]),
        (ColorVariant::Ocean, ResolvedTheme::Dark) => p([
            [80, 140, 255],
            [40, 200, 230],
            [120, 90, 255],
            [30, 170, 210],
            [160, 80, 240],
            [60, 110, 255],
            [40, 190, 180],
        ]),
        (ColorVariant::Ocean, ResolvedTheme::Light) => p([
            [40, 100, 240],
            [20, 160, 200],
            [90, 60, 230],
            [20, 130, 180],
            [130, 50, 220],
            [40, 80, 230],
            [20, 150, 150],
        ]),
        (ColorVariant::Sunset, ResolvedTheme::Dark) => p([
            [255, 110, 60],
            [255, 180, 40],
            [255, 60, 90],
            [255, 210, 80],
            [240, 70, 140],
            [255, 140, 50],
            [230, 50, 110],
        ]),
        (ColorVariant::Sunset, ResolvedTheme::Light) => p([
            [235, 80, 30],
            [230, 150, 10],
            [230, 30, 70],
            [225, 175, 30],
            [215, 40, 110],
            [235, 110, 20],
            [205, 30, 90],
        ]),
        (ColorVariant::Forest, ResolvedTheme::Dark) => p([
            [70, 220, 120],
            [40, 200, 180],
            [140, 230, 80],
            [30, 170, 140],
            [190, 235, 70],
            [50, 190, 110],
            [30, 150, 120],
        ]),
        (ColorVariant::Forest, ResolvedTheme::Light) => p([
            [30, 170, 80],
            [20, 150, 130],
            [90, 180, 30],
            [20, 130, 100],
            [130, 180, 20],
            [30, 150, 80],
            [20, 120, 90],
        ]),
        (ColorVariant::Candy, ResolvedTheme::Dark) => p([
            [255, 90, 170],
            [255, 120, 220],
            [210, 80, 255],
            [255, 150, 190],
            [180, 110, 255],
            [255, 70, 140],
            [230, 100, 240],
        ]),
        (ColorVariant::Candy, ResolvedTheme::Light) => p([
            [235, 40, 140],
            [230, 70, 190],
            [180, 40, 230],
            [235, 100, 160],
            [150, 70, 230],
            [230, 30, 110],
            [200, 60, 210],
        ]),
        (ColorVariant::Ice, ResolvedTheme::Dark) => p([
            [150, 230, 255],
            [90, 200, 255],
            [190, 240, 255],
            [120, 190, 255],
            [160, 220, 250],
            [80, 170, 255],
            [200, 235, 255],
        ]),
        (ColorVariant::Ice, ResolvedTheme::Light) => p([
            [30, 160, 220],
            [20, 130, 210],
            [60, 180, 230],
            [40, 120, 220],
            [50, 160, 220],
            [20, 110, 220],
            [70, 170, 230],
        ]),
        (ColorVariant::Gold, ResolvedTheme::Dark) => p([
            [255, 200, 70],
            [255, 170, 40],
            [255, 220, 110],
            [240, 150, 30],
            [255, 235, 140],
            [230, 160, 40],
            [250, 210, 90],
        ]),
        (ColorVariant::Gold, ResolvedTheme::Light) => p([
            [200, 140, 10],
            [190, 120, 0],
            [210, 160, 30],
            [180, 110, 0],
            [205, 170, 40],
            [175, 115, 5],
            [195, 150, 20],
        ]),
    }
}

/// The band's colours: the core ridge and its chromatic fringes above,
/// between and below.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::{BandPalette, ResolvedTheme, Rgb};
///
/// let dark = BandPalette::theme_default(ResolvedTheme::Dark);
/// assert_eq!(dark.above, Rgb::new(255, 70, 80));
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BandPalette {
    /// The ridge itself.
    pub core: Rgb,
    /// The fringe above the ridge (red on dark).
    pub above: Rgb,
    /// The faint fringe between.
    pub mid: Rgb,
    /// The fringe below the ridge (blue on dark).
    pub below: Rgb,
}

impl BandPalette {
    /// A band palette from its four colours.
    #[must_use]
    pub const fn new(core: Rgb, above: Rgb, mid: Rgb, below: Rgb) -> Self {
        Self {
            core,
            above,
            mid,
            below,
        }
    }

    /// The theme's own band colours (`BAND_COLORS` in `VoiceBeam.tsx`): a
    /// white core with red / green / blue fringes on dark, a pink-violet core
    /// with rose / sky / mint on light.
    #[must_use]
    pub const fn theme_default(theme: ResolvedTheme) -> Self {
        match theme {
            ResolvedTheme::Dark => Self {
                core: Rgb::new(255, 255, 255),
                above: Rgb::new(255, 70, 80),
                mid: Rgb::new(90, 255, 150),
                below: Rgb::new(80, 140, 255),
            },
            ResolvedTheme::Light => Self {
                core: Rgb::new(197, 139, 255),
                above: Rgb::new(255, 122, 182),
                mid: Rgb::new(126, 196, 255),
                below: Rgb::new(45, 255, 171),
            },
        }
    }
}

/// The CSS filter chain every glow layer runs through:
/// `hue-rotate(base + drift) brightness(b) saturate(s)`.
///
/// The matrices are the Filter Effects Module Level 1 ones; each stage is
/// clamped to `0..=1` like a browser does between filter functions.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::{ColorFilter, Rgb};
///
/// let identity = ColorFilter::new(0.0, 1.0, 1.0);
/// let c = identity.apply(Rgb::new(255, 70, 120));
/// assert!((c[0] - 1.0).abs() < 1e-9);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorFilter {
    /// Total hue rotation in degrees.
    pub hue_degrees: f64,
    /// Brightness multiplier.
    pub brightness: f64,
    /// Saturation multiplier.
    pub saturation: f64,
}

impl ColorFilter {
    /// A filter chain.
    #[must_use]
    pub const fn new(hue_degrees: f64, brightness: f64, saturation: f64) -> Self {
        Self {
            hue_degrees,
            brightness,
            saturation,
        }
    }

    /// The filtered colour as `0.0..=1.0` linear-in-sRGB-encoded channels.
    #[must_use]
    pub fn apply(&self, color: Rgb) -> [f64; 3] {
        let [r, g, b] = color.to_unit();
        let rad = self.hue_degrees.to_radians();
        let (sin, cos) = rad.sin_cos();
        let m = [
            [
                0.213 + cos * 0.787 - sin * 0.213,
                0.715 - cos * 0.715 - sin * 0.715,
                0.072 - cos * 0.072 + sin * 0.928,
            ],
            [
                0.213 - cos * 0.213 + sin * 0.143,
                0.715 + cos * 0.285 + sin * 0.140,
                0.072 - cos * 0.072 - sin * 0.283,
            ],
            [
                0.213 - cos * 0.213 - sin * 0.787,
                0.715 - cos * 0.715 + sin * 0.715,
                0.072 + cos * 0.928 + sin * 0.072,
            ],
        ];
        let rotated = mat3(&m, [r, g, b]).map(|v| v.clamp(0.0, 1.0));
        let lit = rotated.map(|v| (v * self.brightness).clamp(0.0, 1.0));
        let s = self.saturation;
        let sat = [
            [0.213 + 0.787 * s, 0.715 - 0.715 * s, 0.072 - 0.072 * s],
            [0.213 - 0.213 * s, 0.715 + 0.285 * s, 0.072 - 0.072 * s],
            [0.213 - 0.213 * s, 0.715 - 0.715 * s, 0.072 + 0.928 * s],
        ];
        mat3(&sat, lit).map(|v| v.clamp(0.0, 1.0))
    }
}

fn mat3(m: &[[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

// ── Mood palette (voice-mood.ts) ─────────────────────────────────────────

/// A point on the valence / arousal mood plane.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::{mood_colors, MoodPoint, MoodPreset};
///
/// let happy = mood_colors(MoodPreset::Happy.point(), true);
/// assert_eq!(happy.len(), 7);
/// let custom = MoodPoint::new(0.0, 0.5);
/// assert_ne!(mood_colors(custom, true), happy);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MoodPoint {
    /// Negative (red) to positive (green), -1..1.
    pub valence: f64,
    /// Calm to excited, 0..1.
    pub arousal: f64,
}

impl MoodPoint {
    /// A mood point.
    #[must_use]
    pub const fn new(valence: f64, arousal: f64) -> Self {
        Self { valence, arousal }
    }
}

/// The four corners of the mood plane, as the SwiftUI presets place them
/// (`MOOD_PRESETS` upstream).
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MoodPreset {
    /// Positive, excited.
    Happy,
    /// Positive, calm.
    Calm,
    /// Negative, excited.
    Angry,
    /// Negative, calm.
    Sad,
}

impl MoodPreset {
    /// The preset's place on the plane.
    #[must_use]
    pub const fn point(self) -> MoodPoint {
        match self {
            Self::Happy => MoodPoint::new(0.8, 0.75),
            Self::Calm => MoodPoint::new(0.6, 0.15),
            Self::Angry => MoodPoint::new(-0.75, 0.85),
            Self::Sad => MoodPoint::new(-0.7, 0.15),
        }
    }
}

struct Corners {
    happy: LobePalette,
    angry: LobePalette,
    sad: LobePalette,
    calm: LobePalette,
}

const MOOD_DARK: Corners = Corners {
    happy: p([
        [70, 230, 120],
        [150, 235, 70],
        [40, 215, 165],
        [190, 240, 80],
        [60, 220, 100],
        [30, 195, 140],
        [120, 230, 90],
    ]),
    angry: p([
        [255, 50, 55],
        [255, 85, 60],
        [235, 30, 80],
        [255, 65, 45],
        [240, 40, 100],
        [255, 100, 75],
        [215, 30, 50],
    ]),
    sad: p([
        [200, 30, 60],
        [225, 45, 75],
        [180, 25, 70],
        [210, 40, 55],
        [190, 30, 90],
        [230, 60, 80],
        [170, 20, 50],
    ]),
    calm: p([
        [60, 210, 200],
        [90, 200, 255],
        [80, 230, 170],
        [40, 180, 215],
        [120, 220, 235],
        [50, 200, 160],
        [100, 190, 240],
    ]),
};

const MOOD_LIGHT: Corners = Corners {
    happy: p([
        [30, 175, 75],
        [100, 185, 25],
        [20, 160, 120],
        [140, 190, 30],
        [25, 165, 60],
        [15, 145, 100],
        [80, 175, 45],
    ]),
    angry: p([
        [220, 30, 40],
        [230, 60, 35],
        [205, 20, 65],
        [225, 45, 30],
        [210, 25, 85],
        [230, 75, 50],
        [185, 20, 40],
    ]),
    sad: p([
        [180, 25, 50],
        [200, 40, 60],
        [160, 20, 60],
        [190, 35, 45],
        [170, 25, 75],
        [205, 50, 65],
        [145, 15, 40],
    ]),
    calm: p([
        [20, 165, 160],
        [40, 150, 220],
        [30, 175, 130],
        [20, 135, 175],
        [60, 165, 200],
        [25, 155, 125],
        [50, 145, 205],
    ]),
};

type Lab = [f64; 3];

fn to_lab(c: Rgb) -> Lab {
    let lin = |v: u8| {
        let x = f64::from(v) / 255.0;
        if x <= 0.04045 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = (lin(c.r), lin(c.g), lin(c.b));
    let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
    let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
    let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
    [
        0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
    ]
}

fn from_lab([ll, a, b]: Lab) -> Rgb {
    let l = ll + 0.3963377774 * a + 0.2158037573 * b;
    let m = ll - 0.1055613458 * a - 0.0638541728 * b;
    let s = ll - 0.0894841775 * a - 1.291485548 * b;
    let (l3, m3, s3) = (l * l * l, m * m * m, s * s * s);
    let srgb = |x: f64| {
        let v = x.clamp(0.0, 1.0);
        let e = if v <= 0.0031308 {
            12.92 * v
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        };
        js_round(e * 255.0) as u8
    };
    Rgb::new(
        srgb(4.0767416621 * l3 - 3.3077115913 * m3 + 0.2309699292 * s3),
        srgb(-1.2684380046 * l3 + 2.6097574011 * m3 - 0.3413193965 * s3),
        srgb(-0.0041960863 * l3 - 0.7034186147 * m3 + 1.707614701 * s3),
    )
}

fn mix(a: Lab, b: Lab, t: f64) -> Lab {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// The mood palette at a point (`moodColors` upstream): below -0.35 valence
/// fully the negative (red) side, above +0.35 fully the positive side, a
/// smooth crossover in between; arousal blends the calm row into the excited
/// one. Blended in OKLab like the SwiftUI port.
///
/// Feed the result to [`VoiceGlow::colors`](super::VoiceGlow::colors).
#[must_use]
pub fn mood_colors(point: MoodPoint, dark: bool) -> LobePalette {
    let c = if dark { &MOOD_DARK } else { &MOOD_LIGHT };
    let t = ((point.valence + 0.35) / 0.7).clamp(0.0, 1.0);
    let u = t * t * (3.0 - 2.0 * t);
    let a = point.arousal.clamp(0.0, 1.0);
    let mut out = [Rgb::default(); 7];
    for (i, slot) in out.iter_mut().enumerate() {
        let top = mix(to_lab(c.angry[i]), to_lab(c.happy[i]), u);
        let bottom = mix(to_lab(c.sad[i]), to_lab(c.calm[i]), u);
        *slot = from_lab(mix(bottom, top, a));
    }
    out
}

/// The band's fringes take the mood too, 75% of the way as on SwiftUI
/// (`moodBand` upstream); its core stays.
#[must_use]
pub fn mood_band(band: &BandPalette, colors: &LobePalette) -> BandPalette {
    let tint = |c: Rgb, m: Rgb| from_lab(mix(to_lab(c), to_lab(m), 0.75));
    BandPalette {
        above: tint(band.above, colors[1]),
        mid: tint(band.mid, colors[0]),
        below: tint(band.below, colors[2]),
        ..*band
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_like_color_ts() {
        assert_eq!("#abc".parse(), Ok(Rgb::new(170, 187, 204)));
        assert_eq!(" #FF8000 ".parse(), Ok(Rgb::new(255, 128, 0)));
        assert_eq!("rgb(255, 70.5, 120)".parse(), Ok(Rgb::new(255, 71, 120)));
        assert_eq!("RGBA(1,2,3,0.5)".parse(), Ok(Rgb::new(1, 2, 3)));
        assert!("#abcd".parse::<Rgb>().is_err());
        assert!("hsl(1,2,3)".parse::<Rgb>().is_err());
        assert!("rgb(1,2)".parse::<Rgb>().is_err());
    }

    #[test]
    fn every_palette_has_seven_distinct_dark_and_light_entries() {
        for v in ColorVariant::ALL {
            let d = palette(v, ResolvedTheme::Dark);
            let l = palette(v, ResolvedTheme::Light);
            assert_ne!(d, l, "{}", v.name());
        }
    }

    #[test]
    fn identity_filter_is_identity_and_hue_rotation_preserves_gray() {
        let gray = Rgb::new(128, 128, 128);
        let out = ColorFilter::new(137.0, 1.0, 1.0).apply(gray);
        for v in out {
            assert!((v - 128.0 / 255.0).abs() < 1e-3);
        }
        let c = Rgb::new(255, 70, 120);
        let id = ColorFilter::new(0.0, 1.0, 1.0).apply(c);
        let want = c.to_unit();
        for (a, b) in id.iter().zip(want) {
            assert!((a - b).abs() < 1e-12);
        }
    }

    #[test]
    fn mood_extremes_hit_the_corner_colours() {
        // Valence far negative, arousal 1: the `angry` row exactly.
        let angry = mood_colors(MoodPoint::new(-1.0, 1.0), true);
        assert_eq!(angry, MOOD_DARK.angry);
        let calm = mood_colors(MoodPoint::new(1.0, 0.0), false);
        assert_eq!(calm, MOOD_LIGHT.calm);
    }

    #[test]
    fn mood_band_keeps_the_core() {
        let band = BandPalette::theme_default(ResolvedTheme::Dark);
        let tinted = mood_band(&band, &mood_colors(MoodPreset::Calm.point(), true));
        assert_eq!(tinted.core, band.core);
        assert_ne!(tinted.above, band.above);
    }
}
