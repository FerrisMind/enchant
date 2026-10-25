//! Ink colors and the browser-matching dot weight.
//!
//! The pixels themselves come from [`super::soft`].

use iced::Color;

/// Maps one ink value to a color.
///
/// Grayscale by default. With a tint the ink ramp is preserved as a ramp on
/// the tint itself: on dark substrates the tint fades toward black with depth
/// (as the grays do), on light ones toward white, so depth reads identically
/// in color.
pub(crate) fn ink_color(white: f64, alpha: f64, dark: bool, tint: Option<Color>) -> Color {
    let w = white.clamp(0.0, 1.0);
    let a = alpha as f32;
    match tint {
        None => {
            let g = ((if dark { 1.0 - w } else { w }) * 255.0).round() as u8;
            Color::from_rgba8(g, g, g, a)
        }
        Some(tint) => {
            let ramp = |c: f32| {
                let c = f64::from((c * 255.0).round());
                let v = if dark {
                    c * (1.0 - w)
                } else {
                    c + (255.0 - c) * w
                };
                v.round() as u8
            };
            Color::from_rgba8(ramp(tint.r), ramp(tint.g), ramp(tint.b), a)
        }
    }
}

/// [`ink_color`] split into sRGB channels (`0..=1`) and alpha.
pub(crate) fn ink_rgb(white: f64, alpha: f64, dark: bool, tint: Option<Color>) -> ([f32; 3], f32) {
    let c = ink_color(white, alpha, dark, tint);
    ([c.r, c.g, c.b], c.a.clamp(0.0, 1.0))
}

/// Radius, in device pixels, whose exact area equals what a browser canvas
/// actually inks for a circle of radius `r`.
///
/// Browsers over-cover tiny arcs: measured in Chrome, a 0.36 px circle inks
/// about twice its geometric area, a 0.8 px one 1.24 times, a 1.5 px one 1.04
/// times. Matching that keeps thin dots at the reference weight.
pub(crate) fn widen(r: f32) -> f32 {
    // Measured extra area / pi: about 0.14 below 1 px, 0.08 from 1 to 2 px,
    // fading out for larger dots.
    let k = match r {
        r if r < 1.0 => 0.14,
        r => 0.08 * (2.0 / r.max(2.0)).powi(4),
    };
    (r * r + k).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grayscale_mirrors_on_dark() {
        let light = ink_color(0.0, 1.0, false, None);
        let dark = ink_color(0.0, 1.0, true, None);
        assert_eq!(light, Color::from_rgba8(0, 0, 0, 1.0));
        assert_eq!(dark, Color::from_rgba8(255, 255, 255, 1.0));
    }

    #[test]
    fn white_is_clamped() {
        assert_eq!(
            ink_color(-3.0, 1.0, false, None),
            ink_color(0.0, 1.0, false, None)
        );
        assert_eq!(
            ink_color(9.0, 1.0, false, None),
            ink_color(1.0, 1.0, false, None)
        );
    }

    #[test]
    fn tint_ramps_toward_black_on_dark_and_white_on_light() {
        let tint = Color::from_rgb8(200, 100, 50);
        let near_dark = ink_color(0.0, 1.0, true, Some(tint));
        assert_eq!(near_dark, Color::from_rgba8(200, 100, 50, 1.0));
        let far_dark = ink_color(1.0, 1.0, true, Some(tint));
        assert_eq!(far_dark, Color::from_rgba8(0, 0, 0, 1.0));
        let far_light = ink_color(1.0, 1.0, false, Some(tint));
        assert_eq!(far_light, Color::from_rgba8(255, 255, 255, 1.0));
    }

    #[test]
    fn alpha_passes_through() {
        assert!((ink_color(0.5, 0.25, true, None).a - 0.25).abs() < 1e-6);
    }

    #[test]
    fn small_dots_widen_and_big_ones_do_not() {
        assert!(widen(0.36) > 0.5);
        assert!((widen(10.0) - 10.0).abs() < 0.01);
    }
}
