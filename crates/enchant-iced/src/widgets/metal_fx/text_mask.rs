//! Glyph alpha masks — port of `engine/textMask.ts` + MetalText's rim bake.
//!
//! Uses the crate's bundled Geist variable font so headless tests and demos
//! share the same letterforms without depending on the host OS.

use std::borrow::Cow;
use std::sync::OnceLock;

use swash::scale::{Render, ScaleContext, Source};
use swash::shape::ShapeContext;
use swash::zeno::{Format, Vector};
use swash::{FontRef, GlyphId};

use super::bake::gauss_blur;
use super::mask::AlphaMask;

static GEIST: OnceLock<Vec<u8>> = OnceLock::new();
static GEIST_LOADED: OnceLock<()> = OnceLock::new();

fn geist_bytes() -> &'static [u8] {
    GEIST
        .get_or_init(|| {
            include_bytes!("../../../assets/Geist/Geist-VariableFont_wght.ttf").to_vec()
        })
        .as_slice()
}

/// Registers the bundled Geist variable font with iced's cosmic-text system so
/// live labels share metrics with [`rasterize_text_mask`].
pub fn ensure_geist_font_loaded() {
    use iced::advanced::graphics::text as gfx_text;
    GEIST_LOADED.get_or_init(|| {
        let mut font_system = gfx_text::font_system()
            .write()
            .expect("write iced font system");
        font_system.load_font(Cow::Borrowed(geist_bytes()));
    });
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
/// Sized from the **rendered** glyph placements (not typographic ascent /
/// descent alone) so rounded bottoms and AA fringe are never clipped.
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

    // Render every glyph once in pen space (baseline Y = 0, Y-up for `top`)
    // so we can size the mask to the true ink box instead of typographic
    // ascent/descent — Geist's bowls often spill a few px past descent.
    struct Placed {
        left: i32,
        top: i32,
        w: u32,
        h: u32,
        data: Vec<u8>,
    }
    let mut scale_ctx = ScaleContext::new();
    let mut scaler = scale_ctx
        .builder(font_ref)
        .size(size)
        .variations([("wght", weight)])
        .hint(true)
        .build();
    let mut placed: Vec<Placed> = Vec::with_capacity(glyphs.len());
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    for &(id, x) in &glyphs {
        let offset = Vector::new(x.fract(), 0.0);
        let Some(img) = Render::new(&[Source::Outline])
            .format(Format::Alpha)
            .offset(offset)
            .render(&mut scaler, id)
        else {
            continue;
        };
        // Baseline at y=0, Y-up `top` → Y-down bitmap top edge.
        let left = img.placement.left + x.floor() as i32;
        let top = -img.placement.top;
        let right = left + img.placement.width as i32;
        let bottom = top + img.placement.height as i32;
        min_x = min_x.min(left);
        min_y = min_y.min(top);
        max_x = max_x.max(right);
        max_y = max_y.max(bottom);
        placed.push(Placed {
            left,
            top,
            w: img.placement.width,
            h: img.placement.height,
            data: img.data,
        });
    }

    let pad_px = (pad * density).ceil().max(1.0) as i32;
    let (width, height, origin_x, origin_y) = if placed.is_empty() {
        let metrics = font_ref.metrics(&[]);
        let scale = size / f32::from(metrics.units_per_em);
        let ascent = (metrics.ascent * scale).ceil() as i32;
        let descent = ((-metrics.descent) * scale).ceil() as i32;
        let w = (pen_x.ceil() as i32 + 2 * pad_px).max(1) as u32;
        let h = (ascent + descent + 2 * pad_px).max(1) as u32;
        (w, h, pad_px, pad_px + ascent)
    } else {
        let w = (max_x - min_x + 2 * pad_px).max(1) as u32;
        let h = (max_y - min_y + 2 * pad_px).max(1) as u32;
        (w, h, pad_px - min_x, pad_px - min_y)
    };

    let mut alpha = vec![0u8; (width * height) as usize];
    for g in &placed {
        blit_alpha(
            &mut alpha,
            width,
            height,
            g.left + origin_x,
            g.top + origin_y,
            g.w,
            g.h,
            &g.data,
        );
    }
    AlphaMask::from_alpha(width, height, density, alpha)
}

#[allow(clippy::too_many_arguments)]
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

/// Default CSS pad around the tight glyph box (`MetalText` / neighbour labels).
pub const METAL_TEXT_PAD_CSS: f32 = 3.0;

/// Tints `mask` into a straight-RGBA iced image at the mask's CSS size.
///
/// Use this for live labels that must share the same pixel grid as a
/// [`MetalFxMask::Custom`](super::mask::MetalFxMask::Custom) / reflection
/// glyph mask — iced `text` metrics will not line up.
#[must_use]
pub fn glyph_mask_image<'a, Message: 'a>(
    mask: &AlphaMask,
    color: iced::Color,
) -> iced::Element<'a, Message> {
    use iced::widget::image;
    use iced::{ContentFit, Length};

    let (w, h) = mask.pixel_size();
    let density = mask.density().max(1e-3);
    let css_w = w as f32 / density;
    let css_h = h as f32 / density;
    let n = (w * h) as usize;
    let mut rgba = vec![0u8; n * 4];
    let r = (color.r.clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (color.g.clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (color.b.clamp(0.0, 1.0) * 255.0).round() as u8;
    let ca = color.a.clamp(0.0, 1.0);
    for (i, &a) in mask.alpha().iter().enumerate() {
        let j = i * 4;
        rgba[j] = r;
        rgba[j + 1] = g;
        rgba[j + 2] = b;
        rgba[j + 3] = ((f32::from(a) / 255.0) * ca * 255.0).round() as u8;
    }
    let handle = image::Handle::from_rgba(w, h, rgba);
    image(handle)
        .width(Length::Fixed(css_w))
        .height(Length::Fixed(css_h))
        .content_fit(ContentFit::Fill)
        .into()
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
    fn rasterize_pro_keeps_pad_below_ink() {
        let pad = 2.0;
        let density = 2.0;
        let mask = rasterize_text_mask(
            "Pro",
            &MetalTextFont {
                size: 24.0,
                weight: 500.0,
            },
            density,
            pad,
        );
        let (w, h) = mask.pixel_size();
        let a = mask.alpha();
        let mut max_y = 0u32;
        for y in 0..h {
            for x in 0..w {
                if a[(y * w + x) as usize] > 8 {
                    max_y = max_y.max(y);
                }
            }
        }
        let bottom_pad = h.saturating_sub(1).saturating_sub(max_y);
        let need = (pad * density).floor() as u32;
        assert!(
            bottom_pad >= need,
            "bottom pad {bottom_pad} < {need} (ink max_y={max_y}, h={h})"
        );
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
