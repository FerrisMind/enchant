//! The slice palette from `draw.ts`.
//!
//! The body's far side sits in shade, the near side is lit, and the two are
//! joined by seventeen slice colours. Crisp and flat use the crisp ramp, which
//! keeps the far slices dark and hands the near ones to a lit gradient; smooth
//! ramps all the way to the body colour so no edge shows at the cap.

use super::super::color::{Rgba, mix_hsl, shade};

pub(crate) const SLICES: usize = 17;

#[derive(Clone, Debug)]
pub(crate) struct Palette {
    pub base: Rgba,
    pub far: Rgba,
    pub near: Rgba,
    pub light: Rgba,
    pub dark: Rgba,
    pub cap_top: Rgba,
    pub cap_bottom: Rgba,
    pub crisp_mix: [Rgba; SLICES],
    pub smooth_mix: [Rgba; SLICES],
}

pub(crate) fn palette(color: Rgba, shadow: f32, highlight: f32) -> Palette {
    let shadow = f64::from(shadow);
    let highlight = f64::from(highlight);
    let far = shade(color, -0.3 * shadow, 0.05 * shadow);
    let near = shade(color, -0.12 * shadow, 0.03 * shadow);
    let mut crisp_mix = [color; SLICES];
    let mut smooth_mix = [color; SLICES];
    for j in 0..SLICES {
        let t = j as f64 / (SLICES - 1) as f64;
        crisp_mix[j] = if t > 0.6 {
            near
        } else {
            mix_hsl(far, near, t / 0.6)
        };
        smooth_mix[j] = if t >= 0.5 {
            color
        } else {
            mix_hsl(far, color, t / 0.5)
        };
    }
    Palette {
        base: color,
        far,
        near,
        light: shade(color, 0.04 * highlight, 0.0),
        dark: shade(color, -0.3 * shadow, 0.05 * shadow),
        cap_top: shade(color, 0.035 * highlight, 0.0),
        cap_bottom: shade(color, -0.035 * shadow, 0.0),
        crisp_mix,
        smooth_mix,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ramps_start_dark_and_end_at_the_body() {
        let p = palette(Rgba::from_rgb(65, 196, 255), 0.35, 1.3);
        assert_eq!(p.crisp_mix[0], p.far);
        assert_eq!(p.smooth_mix[SLICES - 1], p.base);
        let l0 = p.crisp_mix[0].to_hsl().2;
        let l1 = p.crisp_mix[10].to_hsl().2;
        assert!(l1 > l0, "{l0} {l1}");
    }
}
