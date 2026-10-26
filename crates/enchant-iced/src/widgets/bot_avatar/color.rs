//! Colour parsing and interpolation ported from `color.ts` in the
//! Libraries.dev `bot-avatars` package.
//!
//! The web package works in CSS strings. This port keeps the same arithmetic
//! and the same HSL/OKLab conventions, but stores the result as 8-bit sRGB
//! channels so the renderer can use it directly.

/// An sRGB colour with an 8-bit alpha channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgba {
    /// Red, 0–255.
    pub r: u8,
    /// Green, 0–255.
    pub g: u8,
    /// Blue, 0–255.
    pub b: u8,
    /// Alpha, 0–255.
    pub a: u8,
}

impl Rgba {
    /// Build a colour from its four 8-bit channels.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Build an opaque colour from its sRGB channels.
    #[must_use]
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 255)
    }

    /// Parse `#rgb`, `#rrggbb`, `rgb()` / `rgba()` or `hsl()` / `hsla()`.
    ///
    /// Commas and whitespace are both accepted between the components, as is
    /// an optional `deg` suffix on the HSL hue and an optional `/` before the
    /// alpha component. Anything unrecognised returns [`None`].
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if let Some(hex) = s.strip_prefix('#') {
            return parse_hex(hex);
        }

        let open = s.find('(')?;
        if !s.ends_with(')') {
            return None;
        }
        let name = s.get(..open)?.trim();
        let body = s.get(open + 1..s.len() - 1)?;

        if name.eq_ignore_ascii_case("rgb") || name.eq_ignore_ascii_case("rgba") {
            parse_rgb_function(body)
        } else if name.eq_ignore_ascii_case("hsl") || name.eq_ignore_ascii_case("hsla") {
            parse_hsl_function(body)
        } else {
            None
        }
    }

    /// The colour as the CSS string the web package would write.
    #[must_use]
    pub fn to_css(self) -> String {
        if self.a == 255 {
            format!("rgb({}, {}, {})", self.r, self.g, self.b)
        } else {
            let alpha = format_alpha(self.a);
            format!("rgba({}, {}, {}, {alpha})", self.r, self.g, self.b)
        }
    }

    /// Convert to HSL, with hue, saturation and lightness each in `0..=1`.
    #[must_use]
    pub fn to_hsl(self) -> (f64, f64, f64) {
        rgb_to_hsl(self.r, self.g, self.b)
    }

    /// Convert from HSL. The hue wraps, while saturation and lightness are
    /// clamped to `0..=1`.
    #[must_use]
    pub fn from_hsl(h: f64, s: f64, l: f64) -> Self {
        let h = if h.is_finite() {
            h.rem_euclid(1.0)
        } else {
            0.0
        };
        let s = if s.is_finite() {
            s.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let l = if l.is_finite() {
            l.clamp(0.0, 1.0)
        } else {
            0.0
        };

        if s == 0.0 {
            let v = channel(l * 255.0);
            return Self::from_rgb(v, v, v);
        }

        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;
        let f = |t: f64| {
            let t = t.rem_euclid(1.0);
            if t < 1.0 / 6.0 {
                p + (q - p) * 6.0 * t
            } else if t < 1.0 / 2.0 {
                q
            } else if t < 2.0 / 3.0 {
                p + (q - p) * (2.0 / 3.0 - t) * 6.0
            } else {
                p
            }
        };

        Self::from_rgb(
            channel(f(h + 1.0 / 3.0) * 255.0),
            channel(f(h) * 255.0),
            channel(f(h - 1.0 / 3.0) * 255.0),
        )
    }

    /// Replace the alpha channel with `a` (clamped to `0..=1`).
    #[must_use]
    pub fn with_alpha(self, a: f32) -> Self {
        let alpha = if a.is_nan() {
            0.0
        } else {
            f64::from(a).clamp(0.0, 1.0)
        };
        Self {
            a: channel(alpha * 255.0),
            ..self
        }
    }

    /// Interpolate the HSL components (the web package's `mixCss`).
    ///
    /// The hue is interpolated linearly, not along the shorter arc, matching
    /// `mixCss`.
    #[must_use]
    pub fn mix(self, other: Self, t: f64) -> Self {
        mix_hsl(self, other, t)
    }

    /// The sRGB channels converted to linear-light values in `0..=1`.
    #[must_use]
    pub fn linear(self) -> [f64; 3] {
        [
            srgb_to_linear(f64::from(self.r) / 255.0),
            srgb_to_linear(f64::from(self.g) / 255.0),
            srgb_to_linear(f64::from(self.b) / 255.0),
        ]
    }

    /// The four channels normalised to `0..=1`, ready for a GPU uniform.
    #[must_use]
    pub fn to_f32(self) -> [f32; 4] {
        [
            f32::from(self.r) / 255.0,
            f32::from(self.g) / 255.0,
            f32::from(self.b) / 255.0,
            f32::from(self.a) / 255.0,
        ]
    }
}

/// Face ink used on light bodies.
pub const DARK_INK: Rgba = Rgba::new(0x1E, 0x1A, 0x33, 255);

/// Face ink used on dark bodies.
pub const LIGHT_INK: Rgba = Rgba::new(0xF7, 0xF5, 0xF2, 255);

/// Parse a CSS colour. See [`Rgba::parse`].
#[must_use]
pub fn parse_color(s: &str) -> Option<Rgba> {
    Rgba::parse(s)
}

/// WCAG relative luminance of an sRGB colour, `0..=1`.
#[must_use]
pub fn luminance(color: Rgba) -> f64 {
    let lin = |v: u8| {
        let x = f64::from(v) / 255.0;
        if x <= 0.03928 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(color.r) + 0.7152 * lin(color.g) + 0.0722 * lin(color.b)
}

/// Face ink for a body colour: dark ink, or light ink on a dark body.
#[must_use]
pub fn auto_ink(color: Rgba) -> Rgba {
    if luminance(color) < 0.13 {
        LIGHT_INK
    } else {
        DARK_INK
    }
}

/// A shade of a colour: `dl` moves the lightness (`-1..=1`), `ds` the
/// saturation.
///
/// Darker shades get a touch more saturation so they stay rich instead of
/// going grey, the way a painted surface falls into shadow. The resulting HSL
/// is rounded to a tenth, matching the `hsl()` string the web package writes.
#[must_use]
pub fn shade(color: Rgba, dl: f64, ds: f64) -> Rgba {
    let (h, s, l) = color.to_hsl();
    let saturation = clamp_unit(s + ds + if dl < 0.0 { -dl * 0.25 } else { 0.0 });
    let lightness = clamp_unit(l + dl);

    let mut out = Rgba::from_hsl(
        round_tenth(h * 360.0) / 360.0,
        round_tenth(saturation * 100.0) / 100.0,
        round_tenth(lightness * 100.0) / 100.0,
    );
    out.a = color.a;
    out
}

/// A more vivid version of a colour, `t` in `0..=1`.
///
/// The chroma is raised along the colour's own hue in OKLab by up to half
/// again. Where the screen cannot show that at the same lightness, the colour
/// deepens by at most 15% of its lightness toward the hue's most vivid shade,
/// and the chroma stops at whatever the gamut can show.
#[must_use]
pub fn richer(color: Rgba, t: f64) -> Rgba {
    if !(t > 0.0) {
        return color;
    }

    let [r, g, b] = color.linear();
    let [l, a, b] = oklab(r, g, b);
    let chroma = a.hypot(b);
    if chroma < 1e-4 {
        return color;
    }

    let ua = a / chroma;
    let ub = b / chroma;
    let want = chroma * (1.0 + 0.5 * t);
    let mut best: Option<[f64; 3]> = None;
    let mut best_chroma = chroma;

    // The lightness from the colour's own down to 15% deeper, in steps. At
    // each, find the most chroma the screen can show up to `want`.
    for k in 0..=6 {
        let lightness = l * (1.0 - (0.15 * t * f64::from(k)) / 6.0);
        let mut lo = 0.0;
        let mut hi = want;
        for _ in 0..18 {
            let mid = (lo + hi) / 2.0;
            if in_gamut(from_oklab(lightness, ua * mid, ub * mid)) {
                lo = mid;
            } else {
                hi = mid;
            }
        }

        // Deeper only for what it gains: each step must buy real chroma.
        if lo > best_chroma + 0.004 * f64::from(k) {
            best_chroma = lo;
            best = Some(from_oklab(lightness, ua * lo, ub * lo));
        }
        if lo >= want - 1e-4 {
            break;
        }
    }

    let Some(best) = best else {
        return color;
    };

    let mut out = Rgba::from_rgb(
        channel(255.0 * to_srgb(best[0].clamp(0.0, 1.0))),
        channel(255.0 * to_srgb(best[1].clamp(0.0, 1.0))),
        channel(255.0 * to_srgb(best[2].clamp(0.0, 1.0))),
    );
    out.a = color.a;
    out
}

/// Interpolate two colours through HSL, matching the web package's `mixCss`.
#[must_use]
pub fn mix_hsl(a: Rgba, b: Rgba, t: f64) -> Rgba {
    if t <= 0.0 {
        return a;
    }
    if t >= 1.0 {
        return b;
    }

    let (ah, as_, al) = a.to_hsl();
    let (bh, bs, bl) = b.to_hsl();
    let mut out = Rgba::from_hsl(ah + (bh - ah) * t, as_ + (bs - as_) * t, al + (bl - al) * t);
    out.a = channel(f64::from(a.a) + (f64::from(b.a) - f64::from(a.a)) * t);
    out
}

fn parse_hex(hex: &str) -> Option<Rgba> {
    let bytes = hex.as_bytes();
    match bytes.len() {
        3 => {
            let r = hex_nibble(bytes[0])?;
            let g = hex_nibble(bytes[1])?;
            let b = hex_nibble(bytes[2])?;
            Some(Rgba::from_rgb(r * 17, g * 17, b * 17))
        }
        6 => {
            let r = hex_nibble(bytes[0])? * 16 + hex_nibble(bytes[1])?;
            let g = hex_nibble(bytes[2])? * 16 + hex_nibble(bytes[3])?;
            let b = hex_nibble(bytes[4])? * 16 + hex_nibble(bytes[5])?;
            Some(Rgba::from_rgb(r, g, b))
        }
        _ => None,
    }
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn parse_rgb_function(body: &str) -> Option<Rgba> {
    let args = split_args(body);
    if args.len() != 3 && args.len() != 4 {
        return None;
    }

    let r = parse_rgb_channel(&args[0])?;
    let g = parse_rgb_channel(&args[1])?;
    let b = parse_rgb_channel(&args[2])?;
    let a = parse_alpha(args.get(3).map(String::as_str))?;

    Some(Rgba::new(r, g, b, a))
}

fn parse_hsl_function(body: &str) -> Option<Rgba> {
    let args = split_args(body);
    if args.len() != 3 && args.len() != 4 {
        return None;
    }

    let hue = parse_hue(&args[0])?;
    let saturation = parse_percent(&args[1])?;
    let lightness = parse_percent(&args[2])?;
    let alpha = parse_alpha(args.get(3).map(String::as_str))?;

    let mut color = Rgba::from_hsl(hue, saturation, lightness);
    color.a = alpha;
    Some(color)
}

fn split_args(body: &str) -> Vec<String> {
    // Comma-, whitespace- and slash-separated components all become simple
    // whitespace-separated components. `split_whitespace` then drops empties.
    let normalized = body.replace(['/', ','], " ");
    normalized.split_whitespace().map(str::to_owned).collect()
}

fn parse_rgb_channel(token: &str) -> Option<u8> {
    let token = token.trim();
    if let Some(percent) = token.strip_suffix('%') {
        let value = parse_number(percent)?;
        Some(channel(value * 255.0 / 100.0))
    } else {
        Some(channel(parse_number(token)?))
    }
}

fn parse_hue(token: &str) -> Option<f64> {
    let token = token.trim();
    let lower = token.to_ascii_lowercase();
    let value = match lower.strip_suffix("deg") {
        Some(degrees) => parse_number(degrees)?,
        None => parse_number(token)?,
    };
    Some(value / 360.0)
}

fn parse_percent(token: &str) -> Option<f64> {
    let token = token.trim();
    let value = token.strip_suffix('%')?;
    Some(parse_number(value)? / 100.0)
}

fn parse_alpha(token: Option<&str>) -> Option<u8> {
    let Some(token) = token else {
        return Some(255);
    };
    let token = token.trim();
    if let Some(percent) = token.strip_suffix('%') {
        let value = parse_number(percent)?;
        Some(channel(value * 255.0 / 100.0))
    } else {
        Some(channel(parse_number(token)? * 255.0))
    }
}

fn parse_number(token: &str) -> Option<f64> {
    let value = token.trim().parse::<f64>().ok()?;
    value.is_finite().then_some(value)
}

fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let r = f64::from(r) / 255.0;
    let g = f64::from(g) / 255.0;
    let b = f64::from(b) / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if max == min {
        return (0.0, 0.0, l);
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };

    (h / 6.0, s, l)
}

fn channel(value: f64) -> u8 {
    if value.is_nan() {
        0
    } else {
        value.clamp(0.0, 255.0).round() as u8
    }
}

fn clamp_unit(value: f64) -> f64 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
}

fn round_tenth(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn format_alpha(alpha: u8) -> String {
    let mut text = format!("{:.3}", f64::from(alpha) / 255.0);
    while text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

fn srgb_to_linear(value: f64) -> f64 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn to_srgb(value: f64) -> f64 {
    if value <= 0.003_130_8 {
        12.92 * value
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

fn oklab(r: f64, g: f64, b: f64) -> [f64; 3] {
    let l = (0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b).cbrt();
    let m = (0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b).cbrt();
    let s = (0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b).cbrt();
    [
        0.210_454_255_3 * l + 0.793_617_785 * m - 0.004_072_046_8 * s,
        1.977_998_495_1 * l - 2.428_592_205 * m + 0.450_593_709_9 * s,
        0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766 * s,
    ]
}

fn from_oklab(lightness: f64, a: f64, b: f64) -> [f64; 3] {
    let l = (lightness + 0.396_337_777_4 * a + 0.215_803_757_3 * b).powi(3);
    let m = (lightness - 0.105_561_345_8 * a - 0.063_854_172_8 * b).powi(3);
    let s = (lightness - 0.089_484_177_5 * a - 1.291_485_548 * b).powi(3);
    [
        4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s,
        -1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s,
        -0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701 * s,
    ]
}

fn in_gamut(color: [f64; 3]) -> bool {
    color
        .iter()
        .all(|value| *value >= -1e-4 && *value <= 1.0 + 1e-4)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRESET_COLORS: [&str; 18] = [
        "#41C4FF", "#2FCB7A", "#DC48FF", "#35B8FF", "#2FCB7A", "#F4F2FA", "#9A62FF", "#1ED3C6",
        "#EFDB8E", "#D5DBEA", "#95A6C4", "#9CD66A", "#EB7575", "#EAA06F", "#CFD5E4", "#ACAAF3",
        "#ABC793", "#EE8BDB",
    ];

    fn rgb(hex: &str) -> Rgba {
        Rgba::parse(hex).expect("test colour should parse")
    }

    fn assert_rgb_near(actual: Rgba, expected: Rgba, tolerance: i32) {
        let diff = |a: u8, b: u8| (i32::from(a) - i32::from(b)).abs();
        assert!(
            diff(actual.r, expected.r) <= tolerance
                && diff(actual.g, expected.g) <= tolerance
                && diff(actual.b, expected.b) <= tolerance,
            "expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn parses_all_preset_colours() {
        for color in PRESET_COLORS {
            let parsed = Rgba::parse(color).expect("preset colour should parse");
            assert_eq!(parsed.a, 255);
            assert_eq!(
                parsed.to_css(),
                format!("rgb({}, {}, {})", parsed.r, parsed.g, parsed.b)
            );
        }
    }

    #[test]
    fn parses_hex_rgb_and_hsl_forms() {
        assert_eq!(Rgba::parse("#abc"), Some(Rgba::from_rgb(170, 187, 204)));
        assert_eq!(Rgba::parse(" #41C4FF "), Some(Rgba::from_rgb(65, 196, 255)));
        assert_eq!(
            Rgba::parse("rgb(255, 128, 0)"),
            Some(Rgba::from_rgb(255, 128, 0))
        );
        assert_eq!(
            Rgba::parse("rgba(255 0 0 / 0.5)"),
            Some(Rgba::new(255, 0, 0, 128))
        );
        assert_eq!(
            Rgba::parse("rgb(100%, 0%, 0%)"),
            Some(Rgba::from_rgb(255, 0, 0))
        );
        assert_eq!(
            Rgba::parse("hsl(120, 100%, 50%)"),
            Some(Rgba::from_rgb(0, 255, 0))
        );
        assert_eq!(
            Rgba::parse("hsl(240deg 100% 50%)"),
            Some(Rgba::from_rgb(0, 0, 255))
        );
        assert_eq!(
            Rgba::parse("hsla(0 100% 50% / 25%)"),
            Some(Rgba::new(255, 0, 0, 64))
        );
        assert_eq!(
            Rgba::parse("hsl(-120 100% 50%)"),
            Some(Rgba::from_rgb(0, 0, 255))
        );
    }

    #[test]
    fn malformed_colours_do_not_panic() {
        for input in [
            "",
            " ",
            "#",
            "#12",
            "#12345",
            "#GGGGGG",
            "rgb(1, 2)",
            "rgb(1, 2, 3",
            "rgb(nope, 2, 3)",
            "hsl(120, 100, 50)",
            "hsl(120%, 100%, 50%)",
            "not-a-colour(1, 2, 3)",
            "rgb(1, 2, 3) trailing",
        ] {
            assert_eq!(Rgba::parse(input), None, "unexpectedly parsed {input:?}");
        }
    }

    #[test]
    fn to_css_keeps_alpha_when_needed() {
        assert_eq!(Rgba::from_rgb(65, 196, 255).to_css(), "rgb(65, 196, 255)");
        assert_eq!(
            Rgba::new(65, 196, 255, 128).to_css(),
            "rgba(65, 196, 255, 0.502)"
        );
        assert_eq!(
            Rgba::new(0, 0, 0, 0).with_alpha(1.0),
            Rgba::new(0, 0, 0, 255)
        );
        assert_eq!(
            Rgba::new(0, 0, 0, 255).with_alpha(0.0),
            Rgba::new(0, 0, 0, 0)
        );
    }

    #[test]
    fn auto_ink_switches_on_dark_bodies() {
        assert_eq!(auto_ink(rgb("#FFFFFF")), DARK_INK);
        assert_eq!(auto_ink(rgb("#F4F2FA")), DARK_INK);
        assert_eq!(auto_ink(rgb("#000000")), LIGHT_INK);
        assert_eq!(auto_ink(rgb("#1E1A33")), LIGHT_INK);
        assert_eq!(auto_ink(rgb("#41C4FF")), DARK_INK);
        assert!(luminance(rgb("#000000")) < 0.13);
        assert!(luminance(rgb("#FFFFFF")) > 0.13);
        assert!((luminance(rgb("#FFFFFF")) - 1.0).abs() < 1e-12);
        assert!(luminance(rgb("#000000")).abs() < 1e-12);
    }

    #[test]
    fn hsl_round_trips_rgb_channels() {
        for color in PRESET_COLORS {
            let original = rgb(color);
            let (h, s, l) = original.to_hsl();
            assert_eq!(Rgba::from_hsl(h, s, l), original);
        }
        assert_eq!(Rgba::from_hsl(0.0, 0.0, 0.5), Rgba::from_rgb(128, 128, 128));
    }

    #[test]
    fn shade_matches_the_web_reference_values() {
        assert_eq!(
            shade(rgb("#41C4FF"), -0.3, 0.05),
            Rgba::from_rgb(0, 115, 167)
        );
        assert_eq!(
            shade(rgb("#2FCB7A"), 0.1, 0.02),
            Rgba::from_rgb(83, 218, 148)
        );
        assert_eq!(
            shade(rgb("#35B8FF"), -0.12, 0.03),
            Rgba::from_rgb(0, 160, 247)
        );
        assert_eq!(
            shade(rgb("#DC48FF"), -0.22, 0.08),
            Rgba::from_rgb(174, 0, 215)
        );
        assert_eq!(
            shade(rgb("#EFDB8E"), 0.2, 0.0),
            Rgba::from_rgb(252, 247, 231)
        );
        assert_eq!(shade(rgb("#1E1A33"), 0.14, 0.0), Rgba::from_rgb(58, 50, 98));
    }

    #[test]
    fn shade_is_a_stable_round_trip() {
        for color in PRESET_COLORS {
            let original = rgb(color);
            let flat = shade(original, 0.0, 0.0);
            assert_rgb_near(flat, original, 1);
            assert_eq!(flat.a, original.a);
        }
    }

    #[test]
    fn richer_matches_the_web_reference_values() {
        assert_eq!(richer(rgb("#41C4FF"), 0.5), Rgba::from_rgb(48, 193, 255));
        assert_eq!(richer(rgb("#41C4FF"), 1.0), Rgba::from_rgb(21, 190, 255));
        assert_eq!(richer(rgb("#DC48FF"), 0.8), Rgba::from_rgb(217, 37, 255));
        assert_eq!(richer(rgb("#2FCB7A"), 0.5), Rgba::from_rgb(0, 205, 117));
        assert_eq!(richer(rgb("#EFDB8E"), 0.5), Rgba::from_rgb(244, 218, 118));
        assert_eq!(richer(rgb("#9A62FF"), 0.75), Rgba::from_rgb(144, 68, 255));
    }

    #[test]
    fn richer_leaves_colours_it_cannot_push() {
        let grey = Rgba::from_rgb(128, 128, 128);
        assert_eq!(richer(grey, 1.0), grey);
        assert_eq!(richer(rgb("#41C4FF"), 0.0), rgb("#41C4FF"));
        assert_eq!(richer(rgb("#41C4FF"), -1.0), rgb("#41C4FF"));
    }

    #[test]
    fn mix_hsl_has_exact_endpoints_and_linear_hue() {
        let red = Rgba::from_rgb(255, 0, 0);
        let blue = Rgba::from_rgb(0, 0, 255);
        assert_eq!(mix_hsl(red, blue, 0.0), red);
        assert_eq!(mix_hsl(red, blue, 1.0), blue);
        // mixCss walks the hue linearly, so red -> blue passes through green,
        // not through purple.
        assert_eq!(mix_hsl(red, blue, 0.5), Rgba::from_rgb(0, 255, 0));

        let clear = Rgba::new(255, 0, 0, 0);
        let opaque = Rgba::new(0, 0, 255, 255);
        assert_eq!(mix_hsl(clear, opaque, 0.5), Rgba::new(0, 255, 0, 128));
        assert_eq!(red.mix(blue, 0.0), red);
        assert_eq!(red.mix(blue, 1.0), blue);
    }

    #[test]
    fn linear_and_float_helpers() {
        assert_eq!(Rgba::from_rgb(255, 255, 255).linear(), [1.0, 1.0, 1.0]);
        assert_eq!(Rgba::from_rgb(0, 0, 0).linear(), [0.0, 0.0, 0.0]);
        let mid = Rgba::from_rgb(128, 128, 128).linear()[0];
        assert!((mid - 0.215_860_5).abs() < 1e-6);
        assert_eq!(
            Rgba::new(255, 128, 0, 64).to_f32(),
            [1.0, 128.0 / 255.0, 0.0, 64.0 / 255.0]
        );
    }
}
