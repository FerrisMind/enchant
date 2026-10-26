//! Hex parsing and HSV helpers, from `engine/color.ts`.

/// Parses `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa` (leading `#` optional)
/// into normalised `[r, g, b, a]`; alpha is `1` when the hex omits it.
///
/// Paper's shader takes colours as `vec4`, and for the tint the alpha is a
/// colour-burn *amount*, not an opacity, so the 8-digit form is the normal
/// way to write one.
///
/// Returns `None` for any other length or non-hex digits.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::hex_to_rgba;
///
/// let tint = hex_to_rgba("#88ccff2e").unwrap();
/// assert!((tint[3] - 46.0 / 255.0).abs() < 1e-6);
/// assert_eq!(hex_to_rgba("#fff"), Some([1.0, 1.0, 1.0, 1.0]));
/// assert_eq!(hex_to_rgba("nope"), None);
/// ```
#[must_use]
pub fn hex_to_rgba(hex: &str) -> Option<[f32; 4]> {
    let digits = hex.strip_prefix('#').unwrap_or(hex);
    if !digits.is_ascii() {
        return None;
    }
    let expanded: String = match digits.len() {
        3 | 4 => digits.chars().flat_map(|c| [c, c]).collect(),
        6 | 8 => digits.to_owned(),
        _ => return None,
    };
    let byte = |i: usize| u8::from_str_radix(&expanded[i..i + 2], 16).ok();
    let alpha = if expanded.len() == 8 {
        f32::from(byte(6)?) / 255.0
    } else {
        1.0
    };
    Some([
        f32::from(byte(0)?) / 255.0,
        f32::from(byte(2)?) / 255.0,
        f32::from(byte(4)?) / 255.0,
        alpha,
    ])
}

/// Parses `#rgb` / `#rrggbb` into normalised `[r, g, b]`.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::hex_to_rgb;
///
/// assert_eq!(hex_to_rgb("#ff0000"), Some([1.0, 0.0, 0.0]));
/// ```
#[must_use]
pub fn hex_to_rgb(hex: &str) -> Option<[f32; 3]> {
    hex_to_rgba(hex).map(|[r, g, b, _]| [r, g, b])
}

/// `#rrggbbaa` literal as normalised RGBA, usable in `const` context.
pub(crate) const fn rgba8(value: u32) -> [f32; 4] {
    [
        ((value >> 24) & 0xff) as f32 / 255.0,
        ((value >> 16) & 0xff) as f32 / 255.0,
        ((value >> 8) & 0xff) as f32 / 255.0,
        (value & 0xff) as f32 / 255.0,
    ]
}

/// RGB (`0..=255`) to HSV (`h`, `s`, `v` all `0..=1`).
pub(crate) fn rgb_to_hsv(r: f32, g: f32, b: f32) -> [f32; 3] {
    let (r, g, b) = (r / 255.0, g / 255.0, b / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let mut h = 0.0;
    let s = if max == 0.0 { 0.0 } else { d / max };
    if d != 0.0 {
        h = if max == r {
            ((g - b) / d + 6.0) % 6.0
        } else if max == g {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };
        h /= 6.0;
    }
    [h, s, max]
}

/// HSV to RGB, rounded to `0..=255` like the reference.
pub(crate) fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let i = (h * 6.0).floor();
    let f = h * 6.0 - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    let (r, g, b) = match (i as i32).rem_euclid(6) {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [
        (r * 255.0).round(),
        (g * 255.0).round(),
        (b * 255.0).round(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_forms_match_the_reference() {
        assert_eq!(hex_to_rgba("#ffffff66").map(|c| c[3]), Some(102.0 / 255.0));
        assert_eq!(
            hex_to_rgba("#abc"),
            Some([170.0 / 255.0, 187.0 / 255.0, 204.0 / 255.0, 1.0])
        );
        assert_eq!(hex_to_rgba("#abcd").map(|c| c[3]), Some(221.0 / 255.0));
        assert_eq!(hex_to_rgba("00000000"), Some([0.0; 4]));
        assert_eq!(hex_to_rgba("#12345"), None);
        assert_eq!(hex_to_rgba("#gggggg"), None);
        assert_eq!(hex_to_rgba("#ééé"), None);
    }

    #[test]
    fn rgba8_const_matches_the_parser() {
        assert_eq!(rgba8(0x88ccff2e), hex_to_rgba("#88ccff2e").unwrap());
    }

    #[test]
    fn hsv_round_trips() {
        for rgb in [
            [255.0, 0.0, 0.0],
            [10.0, 200.0, 90.0],
            [128.0, 128.0, 255.0],
            [0.0, 0.0, 0.0],
        ] {
            let [h, s, v] = rgb_to_hsv(rgb[0], rgb[1], rgb[2]);
            assert_eq!(hsv_to_rgb(h, s, v), rgb);
        }
    }
}
