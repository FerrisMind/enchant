//! CSS `box-shadow` parsing, ported from liquid-gooey's `shadow.ts`.
//!
//! The reference parser turns a CSS `box-shadow` value into a list of layers
//! and then builds one SVG filter chain per layer. This module ports the
//! parsing half:
//!
//! - [`parse`] mirrors `parseShadow`: it splits the value on top-level commas
//!   (parentheses aware), splits each layer on whitespace, accepts up to four
//!   length tokens (`px` optional, signs and decimals allowed), recognizes
//!   `inset` anywhere in the layer, and treats every remaining token as the
//!   color.
//! - [`parse_color`] resolves the color syntaxes the renderer needs into an
//!   [`iced::Color`]. The reference keeps the raw CSS string and lets the
//!   browser resolve it; iced has no CSS color cascade, so the supported subset
//!   is resolved here.
//!
//! Differences from the reference are limited to what the SVG-based renderer
//! cannot represent:
//!
//! - `blur` and `spread` are clamped to `>= 0`. CSS forbids negative values for
//!   both and `feGaussianBlur` rejects a negative `stdDeviation`. `x` and `y`
//!   stay signed.
//! - A color that cannot be resolved (including `var(...)` and
//!   `currentcolor`) falls back to the upstream default
//!   `rgba(0, 0, 0, 0.35)` inside [`parse`]. Callers that need to resolve
//!   custom properties themselves can call [`parse_color`] directly: it returns
//!   `None` for those cases instead of guessing.
//!
//! `none` (exactly, after trimming) and empty input produce no layers. Invalid
//! tokens that appear in the color position keep the layer alive, matching
//! upstream, which also always pushed a layer with its raw color string.

use iced::Color;

/// The upstream fallback color: `rgba(0, 0, 0, 0.35)`.
const DEFAULT_COLOR: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.35);

/// One parsed CSS `box-shadow` layer, in CSS order (first = topmost).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowLayer {
    /// Horizontal offset, in logical pixels. Signed.
    pub x: f32,
    /// Vertical offset, in logical pixels. Signed.
    pub y: f32,
    /// Gaussian blur radius, in logical pixels. Never negative.
    pub blur: f32,
    /// Spread radius, in logical pixels. Never negative.
    pub spread: f32,
    /// Layer color. Resolved to sRGB; defaults to `rgba(0, 0, 0, 0.35)`.
    pub color: Color,
    /// Inner shadow/highlight: painted inside the liquid edge.
    pub inset: bool,
}

impl Default for ShadowLayer {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            blur: 0.0,
            spread: 0.0,
            color: DEFAULT_COLOR,
            inset: false,
        }
    }
}

/// Parses a CSS `box-shadow` list.
///
/// `none` and empty input return an empty vector. Layers are separated by
/// top-level commas; whitespace inside `rgb(...)`/`hsl(...)`/`color(...)` does
/// not split a layer. Per layer, up to four length tokens are consumed (in
/// order `x`, `y`, `blur`, `spread`), `inset` may appear anywhere, and all
/// remaining tokens form the color. Empty layers are skipped.
///
/// Invalid tokens are ignored in the sense that they never become lengths; a
/// missing or unresolvable color falls back to `rgba(0, 0, 0, 0.35)`.
/// `blur` and `spread` are clamped to `>= 0`; `x` and `y` keep their sign.
#[must_use]
pub fn parse(input: &str) -> Vec<ShadowLayer> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed == "none" {
        return Vec::new();
    }

    let mut layers = Vec::new();
    for layer in split_top_level(trimmed, |ch| ch == ',') {
        let tokens = split_top_level(layer, char::is_whitespace);
        if tokens.is_empty() {
            continue;
        }

        let mut inset = false;
        let mut lengths = [0.0_f32; 4];
        let mut length_count = 0;
        let mut color_parts: Vec<&str> = Vec::new();

        for token in tokens {
            let length = if length_count < 4 {
                parse_length(token)
            } else {
                None
            };

            if token == "inset" {
                inset = true;
            } else if let Some(value) = length {
                lengths[length_count] = value;
                length_count += 1;
            } else {
                color_parts.push(token);
            }
        }

        let color = if color_parts.is_empty() {
            DEFAULT_COLOR
        } else {
            parse_color(&color_parts.join(" ")).unwrap_or(DEFAULT_COLOR)
        };

        layers.push(ShadowLayer {
            x: lengths[0],
            y: lengths[1],
            blur: lengths[2].max(0.0),
            spread: lengths[3].max(0.0),
            color,
            inset,
        });
    }

    layers
}

/// Parses one CSS color.
///
/// Supported syntaxes:
///
/// - `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`
/// - `rgb()` / `rgba()` with commas or spaces, numbers (`0..255`) or
///   percentages, and an optional `/ alpha` (number or percentage)
/// - `hsl()` / `hsla()` with an optional angle unit for the hue and
///   percentages (or bare `0..100` values) for saturation/lightness
/// - `color(display-p3 r g b / a)` with an approximate conversion to sRGB;
///   `color(srgb ...)` is accepted as a pass-through
/// - `transparent`, the 16 CSS named colors, plus `grey`, `orange`
///
/// `currentcolor` and CSS variables cannot be resolved here:
/// `var(...)` and `currentcolor` return `None` so callers can fall back.
/// Unsupported or malformed colors also return `None`.
#[must_use]
pub fn parse_color(token: &str) -> Option<Color> {
    let token = token.trim();
    if token.is_empty() {
        return None;
    }

    let lower = token.to_ascii_lowercase();
    if lower == "transparent" {
        return Some(Color::from_rgba(0.0, 0.0, 0.0, 0.0));
    }
    if lower == "currentcolor" {
        return None;
    }
    if let Some(hex) = lower.strip_prefix('#') {
        return parse_hex(hex);
    }

    if let Some((name, args)) = split_function(token) {
        return match name.to_ascii_lowercase().as_str() {
            "rgb" | "rgba" => parse_rgb(args),
            "hsl" | "hsla" => parse_hsl(args),
            "color" => parse_color_function(args),
            // `var(...)` and any unknown function need external resolution.
            _ => None,
        };
    }

    named_color(&lower)
}

/// Splits `input` on top-level separators, ignoring separators inside
/// parentheses. Empty parts are dropped and every part is trimmed.
fn split_top_level(input: &str, is_separator: impl Fn(char) -> bool) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth: i32 = 0;
    let mut start = 0;

    for (index, ch) in input.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }

        if depth == 0 && is_separator(ch) {
            let part = input[start..index].trim();
            if !part.is_empty() {
                parts.push(part);
            }
            start = index + ch.len_utf8();
        }
    }

    let tail = input[start..].trim();
    if !tail.is_empty() {
        parts.push(tail);
    }

    parts
}

/// Matches upstream's `LENGTH` regex: `^[+-]?(\d+\.?\d*|\.\d+)(px)?$`.
///
/// The `px` suffix is optional and case-sensitive, matching the reference.
fn parse_length(token: &str) -> Option<f32> {
    let bytes = token.as_bytes();
    let mut index = 0;

    if matches!(bytes.first(), Some(b'+' | b'-')) {
        index += 1;
    }

    let integer_start = index;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        index += 1;
    }
    let has_integer = index > integer_start;

    let mut has_fraction = false;
    if index < bytes.len() && bytes[index] == b'.' {
        index += 1;
        let fraction_start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        has_fraction = index > fraction_start;
    }

    if !has_integer && !has_fraction {
        return None;
    }

    let suffix = &token[index..];
    if !suffix.is_empty() && suffix != "px" {
        return None;
    }

    token[..index].parse::<f32>().ok()
}

fn split_function(token: &str) -> Option<(&str, &str)> {
    if !token.ends_with(')') {
        return None;
    }
    let open = token.find('(')?;
    let name = token[..open].trim();
    if name.is_empty() {
        return None;
    }
    Some((name, token[open + 1..token.len() - 1].trim()))
}

fn parse_hex(hex: &str) -> Option<Color> {
    const LENGTHS: [usize; 4] = [3, 4, 6, 8];
    let bytes = hex.as_bytes();
    if !LENGTHS.contains(&bytes.len()) {
        return None;
    }

    let mut nibbles = [0_u8; 8];
    for (nibble, byte) in nibbles.iter_mut().zip(bytes) {
        *nibble = hex_digit(*byte)?;
    }

    match bytes.len() {
        3 => Some(Color::from_rgba8(
            nibbles[0] * 17,
            nibbles[1] * 17,
            nibbles[2] * 17,
            1.0,
        )),
        4 => Some(Color::from_rgba8(
            nibbles[0] * 17,
            nibbles[1] * 17,
            nibbles[2] * 17,
            f32::from(nibbles[3] * 17) / 255.0,
        )),
        6 => Some(Color::from_rgb8(
            nibbles[0] * 16 + nibbles[1],
            nibbles[2] * 16 + nibbles[3],
            nibbles[4] * 16 + nibbles[5],
        )),
        8 => Some(Color::from_rgba8(
            nibbles[0] * 16 + nibbles[1],
            nibbles[2] * 16 + nibbles[3],
            nibbles[4] * 16 + nibbles[5],
            f32::from(nibbles[6] * 16 + nibbles[7]) / 255.0,
        )),
        _ => None,
    }
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn parse_rgb(args: &str) -> Option<Color> {
    let (channels, alpha) = split_components(args)?;
    let r = parse_rgb_channel(channels[0])?;
    let g = parse_rgb_channel(channels[1])?;
    let b = parse_rgb_channel(channels[2])?;
    let a = match alpha {
        Some(token) => parse_alpha(token)?,
        None => 1.0,
    };
    Some(Color::from_rgba(r, g, b, a))
}

fn parse_hsl(args: &str) -> Option<Color> {
    let (channels, alpha) = split_components(args)?;
    let hue = parse_hue(channels[0])?;
    let saturation = parse_percentage(channels[1])?;
    let lightness = parse_percentage(channels[2])?;
    let a = match alpha {
        Some(token) => parse_alpha(token)?,
        None => 1.0,
    };
    let (r, g, b) = hsl_to_rgb(hue, saturation, lightness);
    Some(Color::from_rgba(r, g, b, a))
}

/// Splits the three color channels from an optional alpha channel.
///
/// Accepts both the legacy comma syntax (`rgb(0, 0, 0, .5)`) and the modern
/// space syntax (`rgb(0 0 0 / .5)`). A fourth space-separated token without a
/// slash is also treated as alpha, which browsers tolerate.
fn split_components(args: &str) -> Option<(Vec<&str>, Option<&str>)> {
    let args = args.trim();

    if let Some(slash) = args.find('/') {
        let alpha = args[slash + 1..].trim();
        if alpha.is_empty() || alpha.contains('/') {
            return None;
        }
        let channels: Vec<&str> = args[..slash].split_whitespace().collect();
        if channels.len() != 3 {
            return None;
        }
        return Some((channels, Some(alpha)));
    }

    if args.contains(',') {
        let parts: Vec<&str> = args.split(',').map(str::trim).collect();
        if parts.iter().any(|part| part.is_empty()) {
            return None;
        }
        return match parts.len() {
            3 => Some((parts, None)),
            4 => Some((parts[..3].to_vec(), Some(parts[3]))),
            _ => None,
        };
    }

    let parts: Vec<&str> = args.split_whitespace().collect();
    match parts.len() {
        3 => Some((parts, None)),
        4 => Some((parts[..3].to_vec(), Some(parts[3]))),
        _ => None,
    }
}

fn parse_rgb_channel(token: &str) -> Option<f32> {
    if let Some(percent) = token.strip_suffix('%') {
        parse_number(percent).map(|value| clamp01(value / 100.0))
    } else {
        parse_number(token).map(|value| clamp01(value / 255.0))
    }
}

fn parse_alpha(token: &str) -> Option<f32> {
    if let Some(percent) = token.strip_suffix('%') {
        parse_number(percent).map(|value| clamp01(value / 100.0))
    } else {
        parse_number(token).map(clamp01)
    }
}

fn parse_percentage(token: &str) -> Option<f32> {
    let value = match token.strip_suffix('%') {
        Some(percent) => parse_number(percent)?,
        None => parse_number(token)?,
    };
    Some(clamp01(value / 100.0))
}

fn parse_hue(token: &str) -> Option<f32> {
    let lower = token.trim().to_ascii_lowercase();

    let (number, scale) = if let Some(value) = lower.strip_suffix("deg") {
        (value, 1.0)
    } else if let Some(value) = lower.strip_suffix("grad") {
        (value, 0.9)
    } else if let Some(value) = lower.strip_suffix("turn") {
        (value, 360.0)
    } else if let Some(value) = lower.strip_suffix("rad") {
        (value, 180.0 / std::f32::consts::PI)
    } else {
        (lower.as_str(), 1.0)
    };

    parse_number(number).map(|value| value * scale)
}

fn parse_number(token: &str) -> Option<f32> {
    let value = token.trim().parse::<f32>().ok()?;
    value.is_finite().then_some(value)
}

fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

/// Standard HSL to RGB conversion. `hue` is in degrees and is wrapped;
/// `saturation` and `lightness` are already in `0..=1`.
fn hsl_to_rgb(hue: f32, saturation: f32, lightness: f32) -> (f32, f32, f32) {
    let hue = hue.rem_euclid(360.0) / 60.0;
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let secondary = chroma * (1.0 - (hue % 2.0 - 1.0).abs());
    let offset = lightness - chroma / 2.0;

    let (r, g, b) = match hue as u32 {
        0 => (chroma, secondary, 0.0),
        1 => (secondary, chroma, 0.0),
        2 => (0.0, chroma, secondary),
        3 => (0.0, secondary, chroma),
        4 => (secondary, 0.0, chroma),
        _ => (chroma, 0.0, secondary),
    };

    (
        clamp01(r + offset),
        clamp01(g + offset),
        clamp01(b + offset),
    )
}

fn parse_color_function(args: &str) -> Option<Color> {
    let args = args.trim();
    let (body, alpha) = match args.find('/') {
        Some(slash) => {
            let alpha = args[slash + 1..].trim();
            if alpha.is_empty() || alpha.contains('/') {
                return None;
            }
            (args[..slash].trim(), Some(alpha))
        }
        None => (args, None),
    };

    let parts: Vec<&str> = body.split_whitespace().collect();
    if parts.len() != 4 {
        return None;
    }

    let space = parts[0].to_ascii_lowercase();
    let r = parse_unit_channel(parts[1])?;
    let g = parse_unit_channel(parts[2])?;
    let b = parse_unit_channel(parts[3])?;
    let a = match alpha {
        Some(token) => parse_alpha(token)?,
        None => 1.0,
    };

    let (r, g, b) = match space.as_str() {
        "display-p3" => display_p3_to_srgb(r, g, b),
        "srgb" => (r, g, b),
        _ => return None,
    };

    Some(Color::from_rgba(r, g, b, a))
}

/// Parses a `color()` channel: a unitless value in `0..=1` or a percentage.
fn parse_unit_channel(token: &str) -> Option<f32> {
    if let Some(percent) = token.strip_suffix('%') {
        parse_number(percent).map(|value| clamp01(value / 100.0))
    } else {
        parse_number(token).map(clamp01)
    }
}

/// Approximate Display-P3 to sRGB conversion.
///
/// Linearizes the P3 channels, applies the standard P3-to-sRGB matrix, then
/// re-encodes with the sRGB transfer function. Out-of-gamut results are
/// clamped per channel, which is the usual approximation when no gamut
/// mapping is available.
fn display_p3_to_srgb(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    fn decode(value: f32) -> f32 {
        if value <= 0.040_45 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    fn encode(value: f32) -> f32 {
        if value <= 0.003_130_8 {
            12.92 * value
        } else {
            1.055 * value.powf(1.0 / 2.4) - 0.055
        }
    }

    let r = decode(r);
    let g = decode(g);
    let b = decode(b);

    let sr = 1.224_940_1 * r - 0.224_940_4 * g;
    let sg = -0.042_056_9 * r + 1.042_057_1 * g;
    let sb = -0.019_637_6 * r - 0.078_636_1 * g + 1.098_273_5 * b;

    (
        clamp01(encode(clamp01(sr))),
        clamp01(encode(clamp01(sg))),
        clamp01(encode(clamp01(sb))),
    )
}

fn named_color(name: &str) -> Option<Color> {
    let rgb = match name {
        "black" => (0, 0, 0),
        "silver" => (192, 192, 192),
        "gray" | "grey" => (128, 128, 128),
        "white" => (255, 255, 255),
        "maroon" => (128, 0, 0),
        "red" => (255, 0, 0),
        "purple" => (128, 0, 128),
        "fuchsia" => (255, 0, 255),
        "green" => (0, 128, 0),
        "lime" => (0, 255, 0),
        "olive" => (128, 128, 0),
        "yellow" => (255, 255, 0),
        "navy" => (0, 0, 128),
        "blue" => (0, 0, 255),
        "teal" => (0, 128, 128),
        "aqua" => (0, 255, 255),
        "orange" => (255, 165, 0),
        _ => return None,
    };
    Some(Color::from_rgb8(rgb.0, rgb.1, rgb.2))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    fn assert_color(actual: Color, expected: Color) {
        assert_close(actual.r, expected.r);
        assert_close(actual.g, expected.g);
        assert_close(actual.b, expected.b);
        assert_close(actual.a, expected.a);
    }

    #[test]
    fn parses_multiple_layers_and_inset() {
        let layers = parse("0 2px 4px rgba(0, 0, 0, 0.4), inset -1px -2px 3px 1px #ff0000");
        assert_eq!(layers.len(), 2);

        assert_eq!(layers[0].x, 0.0);
        assert_eq!(layers[0].y, 2.0);
        assert_eq!(layers[0].blur, 4.0);
        assert_eq!(layers[0].spread, 0.0);
        assert_eq!(layers[0].color, Color::from_rgba(0.0, 0.0, 0.0, 0.4));
        assert!(!layers[0].inset);

        assert_eq!(layers[1].x, -1.0);
        assert_eq!(layers[1].y, -2.0);
        assert_eq!(layers[1].blur, 3.0);
        assert_eq!(layers[1].spread, 1.0);
        assert_eq!(layers[1].color, Color::from_rgb8(255, 0, 0));
        assert!(layers[1].inset);
    }

    #[test]
    fn keeps_commas_inside_color_functions() {
        let layers = parse("0 0 4px rgba(0, 0, 0, 0.25)");
        assert_eq!(layers.len(), 1);
        assert_color(layers[0].color, Color::from_rgba(0.0, 0.0, 0.0, 0.25));
    }

    #[test]
    fn parses_hex_with_alpha() {
        let color = parse_color("#11223380").expect("#rrggbbaa should parse");
        assert_close(color.r, 0x11 as f32 / 255.0);
        assert_close(color.g, 0x22 as f32 / 255.0);
        assert_close(color.b, 0x33 as f32 / 255.0);
        assert_close(color.a, 0x80 as f32 / 255.0);

        assert_eq!(parse_color("#f00"), Some(Color::from_rgb8(255, 0, 0)));
        assert_eq!(
            parse_color("#0000"),
            Some(Color::from_rgba(0.0, 0.0, 0.0, 0.0))
        );
    }

    #[test]
    fn parses_rgb_space_syntax() {
        let color = parse_color("rgb(255 0 0 / 50%)").expect("space syntax");
        assert_color(color, Color::from_rgba(1.0, 0.0, 0.0, 0.5));
    }

    #[test]
    fn parses_hsl_red() {
        let color = parse_color("hsl(0, 100%, 50%)").expect("hsl");
        assert_color(color, Color::from_rgb(1.0, 0.0, 0.0));
    }

    #[test]
    fn parses_display_p3_white() {
        let color = parse_color("color(display-p3 1 1 1)").expect("display-p3");
        assert_color(color, Color::WHITE);
    }

    #[test]
    fn ignores_none_and_empty_input() {
        assert!(parse("").is_empty());
        assert!(parse("   ").is_empty());
        assert!(parse("none").is_empty());
        assert!(parse("  none  ").is_empty());
        assert!(parse(",,").is_empty());
    }

    #[test]
    fn preserves_negative_offsets() {
        let layers = parse("-4px -2.5px 0 1px red");
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].x, -4.0);
        assert_eq!(layers[0].y, -2.5);
        assert_eq!(layers[0].blur, 0.0);
        assert_eq!(layers[0].spread, 1.0);
    }

    #[test]
    fn defaults_to_upstream_color() {
        let layers = parse("1px 2px 3px");
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].color, Color::from_rgba(0.0, 0.0, 0.0, 0.35));
        assert_eq!(layers[0].blur, 3.0);
    }

    #[test]
    fn clamps_negative_blur_and_spread() {
        let layers = parse("0 0 -5px -2px red");
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].blur, 0.0);
        assert_eq!(layers[0].spread, 0.0);
    }

    #[test]
    fn parses_optional_px_and_leading_dot() {
        let layers = parse(".5px 1 2px 3px");
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].x, 0.5);
        assert_eq!(layers[0].y, 1.0);
        assert_eq!(layers[0].blur, 2.0);
        assert_eq!(layers[0].spread, 3.0);
    }

    #[test]
    fn treats_fifth_length_as_color() {
        let layers = parse("1px 2px 3px 4px 5px red");
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].x, 1.0);
        assert_eq!(layers[0].spread, 4.0);
        // "5px red" is not a valid color, so the upstream fallback applies.
        assert_eq!(layers[0].color, DEFAULT_COLOR);
    }

    #[test]
    fn resolves_named_and_keyword_colors() {
        assert_eq!(
            parse_color("transparent"),
            Some(Color::from_rgba(0.0, 0.0, 0.0, 0.0))
        );
        assert_eq!(parse_color("RED"), Some(Color::from_rgb8(255, 0, 0)));
        assert_eq!(parse_color("grey"), Some(Color::from_rgb8(128, 128, 128)));
        assert_eq!(parse_color("orange"), Some(Color::from_rgb8(255, 165, 0)));
    }

    #[test]
    fn unresolved_dynamic_colors_return_none() {
        assert_eq!(parse_color("currentcolor"), None);
        assert_eq!(parse_color("var(--shadow)"), None);
        assert_eq!(parse_color("not-a-color"), None);
    }
}
