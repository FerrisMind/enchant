//! Glyph alpha masks — port of `engine/textMask.ts` + MetalText's rim bake.
//!
//! Uses the crate's bundled Geist variable font so headless tests and demos
//! share the same letterforms without depending on the host OS.

use std::sync::OnceLock;

use swash::scale::{Render, ScaleContext, Source};
use swash::shape::ShapeContext;
use swash::zeno::{Format, Vector};
use swash::{FontRef, GlyphId};

use super::bake::gauss_blur;
use super::mask::AlphaMask;

static GEIST: OnceLock<Vec<u8>> = OnceLock::new();

fn geist_bytes() -> &'static [u8] {
    GEIST
        .get_or_init(|| {
            include_bytes!("../../../assets/Geist/Geist-VariableFont_wght.ttf").to_vec()
        })
        .as_slice()
}

/// Figma text inner-shadow defaults (`MetalText.tsx`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextInnerShadow {
    /// Downward shift of the subtracted glyph, CSS px.
    pub offset_y: f32,
    /// Blur sigma of the rim, CSS px.
    pub blur: f32,
    /// Strength of the white rim (`0..=1`).
    pub alpha: f32,
}

impl Default for TextInnerShadow {
    fn default() -> Self {
        Self {
            offset_y: 1.0,
            blur: 0.5,
            alpha: 0.9,
        }
    }
}

/// Tuned defaults from `METAL_TEXT_DEFAULTS`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MetalTextDefaults {
    /// Metal opacity over the base colour.
    pub metal_opacity: f32,
    /// Shader sampling scale inside the glyphs.
    pub shader_scale: f32,
    /// Glow multiplier when glow is on.
    pub glow_gain: f32,
    /// Glyph top-edge rim.
    pub inner_shadow: TextInnerShadow,
}

impl Default for MetalTextDefaults {
    fn default() -> Self {
        Self {
            metal_opacity: 0.62,
            shader_scale: 2.8,
            glow_gain: 2.5,
            inner_shadow: TextInnerShadow::default(),
        }
    }
}

/// Shared defaults (`METAL_TEXT_DEFAULTS`).
pub const METAL_TEXT_DEFAULTS: MetalTextDefaults = MetalTextDefaults {
    metal_opacity: 0.62,
    shader_scale: 2.8,
    glow_gain: 2.5,
    inner_shadow: TextInnerShadow {
        offset_y: 1.0,
        blur: 0.5,
        alpha: 0.9,
    },
};

/// Font description for glyph masks.
#[derive(Clone, Debug, PartialEq)]
pub struct MetalTextFont {
    /// Size in CSS px (the em height).
    pub size: f32,
    /// Variable-font weight (`100..=900`).
    pub weight: f32,
}

impl Default for MetalTextFont {
    fn default() -> Self {
        Self {
            size: 24.0,
            weight: 500.0,
        }
    }
}

/// Rasterises `text` into a white-on-transparent alpha mask covering the
/// tight glyph box padded by `pad` CSS px on every side.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::{MetalTextFont, rasterize_text_mask};
///
/// let mask = rasterize_text_mask("Pro", &MetalTextFont::default(), 2.0, 2.0);
/// assert!(mask.pixel_size().0 > 8 && mask.interior_points(2.0).len() > 4);
/// ```
#[must_use]
pub fn rasterize_text_mask(text: &str, font: &MetalTextFont, density: f32, pad: f32) -> AlphaMask {
    let density = if density.is_finite() && density > 0.0 {
        density
    } else {
        1.0
    };
    let font_bytes = geist_bytes();
    let font_ref = FontRef::from_index(font_bytes, 0).expect("bundled Geist font");
    let size = font.size.max(1.0) * density;
    let weight = font.weight.clamp(100.0, 900.0);
    let mut shape_ctx = ShapeContext::new();
    let mut shaper = shape_ctx
        .builder(font_ref)
        .size(size)
        .variations([("wght", weight)])
        .build();
    shaper.add_str(text);
    let mut glyphs: Vec<(GlyphId, f32)> = Vec::new();
    let mut pen_x = 0.0_f32;
    shaper.shape_with(|cluster| {
        for g in cluster.glyphs {
            glyphs.push((g.id, pen_x + g.x));
            pen_x += g.advance;
        }
    });
    let metrics = font_ref.metrics(&[]);
    let scale = size / f32::from(metrics.units_per_em);
    let ascent = metrics.ascent * scale;
    let descent = (-metrics.descent) * scale;
    let pad_px = (pad * density).ceil().max(1.0);
    let width = (pen_x + 2.0 * pad_px).ceil().max(1.0) as u32;
    let height = (ascent + descent + 2.0 * pad_px).ceil().max(1.0) as u32;
    let mut alpha = vec![0u8; (width * height) as usize];
    let mut scale_ctx = ScaleContext::new();
    let mut scaler = scale_ctx
        .builder(font_ref)
        .size(size)
        .variations([("wght", weight)])
        .hint(true)
        .build();
    for &(id, x) in &glyphs {
        let px = x + pad_px;
        let py = pad_px + ascent;
        let offset = Vector::new(px.fract(), py.fract());
        let Some(img) = Render::new(&[Source::Outline])
            .format(Format::Alpha)
            .offset(offset)
            .render(&mut scaler, id)
        else {
            continue;
        };
        // Swash/zeno placements are origin=baseline with Y-up: `top` is the
        // offset of the bitmap's top edge above the baseline, so flip into the
        // Y-down mask buffer.
        blit_alpha(
            &mut alpha,
            width,
            height,
            img.placement.left + px.floor() as i32,
            py.floor() as i32 - img.placement.top,
            img.placement.width,
            img.placement.height,
            &img.data,
        );
    }
    AlphaMask::from_alpha(width, height, density, alpha)
}

fn blit_alpha(
    dst: &mut [u8],
    dst_w: u32,
    dst_h: u32,
    left: i32,
    top: i32,
    w: u32,
    h: u32,
    src: &[u8],
) {
    for y in 0..h {
        let dy = top + y as i32;
        if dy < 0 || dy as u32 >= dst_h {
            continue;
        }
        for x in 0..w {
            let dx = left + x as i32;
            if dx < 0 || dx as u32 >= dst_w {
                continue;
            }
            let s = src[(y * w + x) as usize];
            let i = (dy as u32 * dst_w + dx as u32) as usize;
            dst[i] = dst[i].max(s);
        }
    }
}

/// Builds the white top-edge rim overlay from a glyph mask (`drawInnerShadow`).
#[must_use]
pub fn text_inner_shadow_mask(mask: &AlphaMask, shadow: TextInnerShadow) -> AlphaMask {
    let (w, h) = mask.pixel_size();
    let n = (w * h) as usize;
    let mut a = vec![0.0_f32; n];
    for (i, &v) in mask.alpha().iter().enumerate() {
        a[i] = f32::from(v) / 255.0;
    }
    let shift = (shadow.offset_y * mask.density()).round().max(1.0) as usize * w as usize;
    let mut rim = vec![0.0_f32; n];
    for i in 0..n {
        let above = if i >= shift { a[i - shift] } else { 0.0 };
        rim[i] = (a[i] - above).max(0.0);
    }
    let blurred = gauss_blur_f32(&rim, w, h, shadow.blur * mask.density());
    let mut out = vec![0u8; n];
    for i in 0..n {
        let v = (blurred[i] * a[i] * shadow.alpha).clamp(0.0, 1.0);
        out[i] = (v * 255.0).round() as u8;
    }
    AlphaMask::from_alpha(w, h, mask.density(), out)
}

fn gauss_blur_f32(src: &[f32], w: u32, h: u32, sigma: f32) -> Vec<f32> {
    let mut buf = src.to_vec();
    gauss_blur(&mut buf, w as usize, h as usize, sigma);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rasterize_pro_has_ink() {
        let mask = rasterize_text_mask("Pro", &MetalTextFont::default(), 2.0, 2.0);
        assert!(mask.interior_points(2.0).len() > 8);
    }

    #[test]
    fn inner_shadow_is_subset_of_glyphs() {
        let mask = rasterize_text_mask("A", &MetalTextFont::default(), 2.0, 2.0);
        let rim = text_inner_shadow_mask(&mask, TextInnerShadow::default());
        let rim_outside = mask
            .alpha()
            .iter()
            .zip(rim.alpha().iter())
            .any(|(&g, &r)| r > 0 && g == 0);
        assert!(!rim_outside, "rim must be clipped to the glyph");
    }
}
