use iced::Color;
use iced::advanced::image::Id;
use iced::widget::image::Handle;
use image::{ImageBuffer, Rgba, RgbaImage};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{LazyLock, Mutex};

/// Layout + texture padding so blur fades before bounds (≈2.5× SVG stdDeviation).
pub fn glow_spread(blur: f32) -> f32 {
    (blur * 2.5).max(8.0)
}

/// Supersample factor for SVG → glow rasterization (display stays vector `svg()`).
const SVG_GLOW_SUPERSAMPLE: f32 = 3.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct GlowCacheKey {
    source_id: Id,
    blur: u16,
    spread_milli: u16,
    saturation_milli: u16,
    radius_milli: u16,
    width: u16,
    height: u16,
}

static GLOW_CACHE: LazyLock<Mutex<HashMap<GlowCacheKey, Handle>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct SvgGlowCacheKey {
    svg_hash: u64,
    blur: u16,
    spread_milli: u16,
    saturation_milli: u16,
    radius_milli: u16,
    width: u16,
    height: u16,
    tint: (u8, u8, u8, u8),
}

static SVG_GLOW_CACHE: LazyLock<Mutex<HashMap<SvgGlowCacheKey, Handle>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Rasterize SVG bytes to an image handle (contain-fit, transparent background).
///
/// When `tint` is set, opaque pixels are recolored (for mono / symbolic icons).
pub fn rasterize_svg_bytes(
    bytes: &[u8],
    width: u32,
    height: u32,
    tint: Option<Color>,
) -> Option<Handle> {
    use resvg::tiny_skia::{Color as SkColor, Pixmap, Transform};

    let width = width.max(1);
    let height = height.max(1);
    let tree = resvg::usvg::Tree::from_data(bytes, &resvg::usvg::Options::default()).ok()?;
    let mut pixmap = Pixmap::new(width, height)?;
    pixmap.fill(SkColor::TRANSPARENT);

    let svg_size = tree.size();
    let scale = (width as f32 / svg_size.width()).min(height as f32 / svg_size.height());
    let tx = (width as f32 - svg_size.width() * scale) * 0.5;
    let ty = (height as f32 - svg_size.height() * scale) * 0.5;
    let transform = Transform::from_translate(tx, ty).pre_scale(scale, scale);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let mut pixels = pixmap_to_rgba(&pixmap);
    if let Some(color) = tint {
        apply_color_tint(&mut pixels, color);
    }

    Some(Handle::from_rgba(width, height, pixels))
}

/// Build a cached glow layer from SVG bytes (supersampled raster, optional theme tint).
pub fn glow_from_svg_bytes(
    bytes: &[u8],
    blur: f32,
    saturation: f32,
    corner_radius: f32,
    display_width: f32,
    display_height: f32,
    tint: Option<Color>,
) -> Option<Handle> {
    let width_key = display_width.round().max(1.0) as u16;
    let height_key = display_height.round().max(1.0) as u16;
    let key = SvgGlowCacheKey {
        svg_hash: hash_bytes(bytes),
        blur: blur.round().max(0.0) as u16,
        spread_milli: (glow_spread(blur) * 1_000.0).round() as u16,
        saturation_milli: (saturation * 1_000.0).round() as u16,
        radius_milli: (corner_radius * 1_000.0).round() as u16,
        width: width_key,
        height: height_key,
        tint: tint_key(tint),
    };

    if let Ok(cache) = SVG_GLOW_CACHE.lock()
        && let Some(handle) = cache.get(&key)
    {
        return Some(handle.clone());
    }

    let raster_w = (display_width * SVG_GLOW_SUPERSAMPLE).round().max(1.0) as u32;
    let raster_h = (display_height * SVG_GLOW_SUPERSAMPLE).round().max(1.0) as u32;
    let raster = rasterize_svg_bytes(bytes, raster_w, raster_h, tint)?;
    let glow = build_glow(
        &decode_handle(&raster)?,
        blur,
        saturation,
        corner_radius,
        width_key as u32,
        height_key as u32,
    )?;

    if let Ok(mut cache) = SVG_GLOW_CACHE.lock() {
        cache.insert(key, glow.clone());
    }

    Some(glow)
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

fn tint_key(tint: Option<Color>) -> (u8, u8, u8, u8) {
    tint.map(|color| {
        (
            (color.r * 255.0).round() as u8,
            (color.g * 255.0).round() as u8,
            (color.b * 255.0).round() as u8,
            (color.a * 255.0).round() as u8,
        )
    })
    .unwrap_or((0, 0, 0, 0))
}

fn apply_color_tint(pixels: &mut [u8], color: Color) {
    let r = (color.r * 255.0).round() as u8;
    let g = (color.g * 255.0).round() as u8;
    let b = (color.b * 255.0).round() as u8;

    for pixel in pixels.chunks_mut(4) {
        if pixel[3] == 0 {
            continue;
        }

        pixel[0] = r;
        pixel[1] = g;
        pixel[2] = b;
    }
}

fn pixmap_to_rgba(pixmap: &resvg::tiny_skia::Pixmap) -> Vec<u8> {
    pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let a = pixel.alpha();
            if a == 0 {
                [0, 0, 0, 0]
            } else if a == 255 {
                [pixel.red(), pixel.green(), pixel.blue(), 255]
            } else {
                let af = a as f32 / 255.0;
                [
                    (pixel.red() as f32 / af).round().min(255.0) as u8,
                    (pixel.green() as f32 / af).round().min(255.0) as u8,
                    (pixel.blue() as f32 / af).round().min(255.0) as u8,
                    a,
                ]
            }
        })
        .collect()
}

/// Cached glow layer: crop to display size → rounded mask → pad → blur → saturate.
pub fn glow_from_handle(
    source: &Handle,
    blur: f32,
    saturation: f32,
    corner_radius: f32,
    display_width: f32,
    display_height: f32,
) -> Option<Handle> {
    let width_key = display_width.round().max(1.0) as u16;
    let height_key = display_height.round().max(1.0) as u16;
    let key = GlowCacheKey {
        source_id: source.id(),
        blur: blur.round().max(0.0) as u16,
        spread_milli: (glow_spread(blur) * 1_000.0).round() as u16,
        saturation_milli: (saturation * 1_000.0).round() as u16,
        radius_milli: (corner_radius * 1_000.0).round() as u16,
        width: width_key,
        height: height_key,
    };

    if let Ok(cache) = GLOW_CACHE.lock()
        && let Some(handle) = cache.get(&key)
    {
        return Some(handle.clone());
    }

    let source = decode_handle(source)?;
    let glow = build_glow(
        &source,
        blur,
        saturation,
        corner_radius,
        width_key as u32,
        height_key as u32,
    )?;

    if let Ok(mut cache) = GLOW_CACHE.lock() {
        cache.insert(key, glow.clone());
    }

    Some(glow)
}

fn build_glow(
    source: &RgbaImage,
    blur: f32,
    saturation: f32,
    corner_radius: f32,
    display_width: u32,
    display_height: u32,
) -> Option<Handle> {
    let mut rgba = crop_cover(source, display_width, display_height);
    apply_round_rect_mask(&mut rgba, corner_radius);

    let spread = glow_spread(blur).round().max(1.0) as u32;
    rgba = pad_image(&rgba, spread);

    let radius_px = blur.round().max(1.0) as i32;
    premultiply_in_place(&mut rgba);
    box_blur_in_place(&mut rgba, radius_px);
    unpremultiply_in_place(&mut rgba);
    saturate_in_place(&mut rgba, saturation.max(0.0));

    Some(Handle::from_rgba(
        rgba.width(),
        rgba.height(),
        rgba.into_raw(),
    ))
}

fn decode_handle(source: &Handle) -> Option<RgbaImage> {
    match source {
        Handle::Bytes(_, bytes) => Some(image::load_from_memory(bytes).ok()?.into_rgba8()),
        Handle::Rgba {
            width,
            height,
            pixels,
            ..
        } => ImageBuffer::from_raw(*width, *height, pixels.as_ref().to_vec()),
        Handle::Path(_, path) => Some(image::open(path).ok()?.into_rgba8()),
    }
}

fn crop_cover(image: &RgbaImage, target_w: u32, target_h: u32) -> RgbaImage {
    let target_w = target_w.max(1);
    let target_h = target_h.max(1);
    let src_w = image.width() as f32;
    let src_h = image.height() as f32;
    let scale = (target_w as f32 / src_w).max(target_h as f32 / src_h);
    let scaled_w = (src_w * scale).ceil().max(1.0) as u32;
    let scaled_h = (src_h * scale).ceil().max(1.0) as u32;
    let scaled = image::imageops::resize(
        image,
        scaled_w,
        scaled_h,
        image::imageops::FilterType::Triangle,
    );
    let x = (scaled_w.saturating_sub(target_w)) / 2;
    let y = (scaled_h.saturating_sub(target_h)) / 2;
    image::imageops::crop_imm(&scaled, x, y, target_w, target_h).to_image()
}

fn pad_image(image: &RgbaImage, spread: u32) -> RgbaImage {
    let (width, height) = image.dimensions();
    let out_w = width + spread * 2;
    let out_h = height + spread * 2;
    let mut padded = ImageBuffer::from_fn(out_w, out_h, |_, _| Rgba([0, 0, 0, 0]));

    for y in 0..height {
        for x in 0..width {
            padded.put_pixel(x + spread, y + spread, *image.get_pixel(x, y));
        }
    }

    padded
}

fn apply_round_rect_mask(image: &mut RgbaImage, radius: f32) {
    let (width, height) = image.dimensions();
    let w = width as f32;
    let h = height as f32;
    let r = radius.min(w / 2.0).min(h / 2.0);

    for y in 0..height {
        for x in 0..width {
            let coverage = rounded_rect_coverage(x as f32 + 0.5, y as f32 + 0.5, w, h, r);
            let pixel = image.get_pixel_mut(x, y);
            pixel.0[3] = ((pixel.0[3] as f32) * coverage).round() as u8;
        }
    }
}

fn rounded_rect_coverage(x: f32, y: f32, width: f32, height: f32, radius: f32) -> f32 {
    if radius <= 0.0 {
        return 1.0;
    }

    let left = radius;
    let top = radius;
    let right = width - radius;
    let bottom = height - radius;

    let (cx, cy) = if x < left && y < top {
        (left, top)
    } else if x > right && y < top {
        (right, top)
    } else if x < left && y > bottom {
        (left, bottom)
    } else if x > right && y > bottom {
        (right, bottom)
    } else {
        return 1.0;
    };

    let dx = x - cx;
    let dy = y - cy;
    let distance = (dx * dx + dy * dy).sqrt();
    let inner = radius - 1.0;
    let outer = radius + 1.0;

    if distance <= inner {
        1.0
    } else if distance >= outer {
        0.0
    } else {
        1.0 - (distance - inner) / (outer - inner)
    }
}

fn premultiply_in_place(image: &mut RgbaImage) {
    for pixel in image.pixels_mut() {
        let a = pixel.0[3] as f32 / 255.0;
        pixel.0[0] = (pixel.0[0] as f32 * a).round() as u8;
        pixel.0[1] = (pixel.0[1] as f32 * a).round() as u8;
        pixel.0[2] = (pixel.0[2] as f32 * a).round() as u8;
    }
}

fn unpremultiply_in_place(image: &mut RgbaImage) {
    for pixel in image.pixels_mut() {
        let a = pixel.0[3] as f32 / 255.0;
        if a <= 0.0 {
            pixel.0 = [0, 0, 0, 0];
            continue;
        }

        pixel.0[0] = ((pixel.0[0] as f32 / a).min(255.0)).round() as u8;
        pixel.0[1] = ((pixel.0[1] as f32 / a).min(255.0)).round() as u8;
        pixel.0[2] = ((pixel.0[2] as f32 / a).min(255.0)).round() as u8;
    }
}

fn saturate_in_place(image: &mut RgbaImage, saturation: f32) {
    for pixel in image.pixels_mut() {
        let [r, g, b, a] = pixel.0;
        if a == 0 {
            continue;
        }

        let rf = r as f32 / 255.0;
        let gf = g as f32 / 255.0;
        let bf = b as f32 / 255.0;

        let lum = 0.213 * rf + 0.715 * gf + 0.072 * bf;
        let nr = lum + saturation * (rf - lum);
        let ng = lum + saturation * (gf - lum);
        let nb = lum + saturation * (bf - lum);

        *pixel = Rgba([
            (nr.clamp(0.0, 1.0) * 255.0).round() as u8,
            (ng.clamp(0.0, 1.0) * 255.0).round() as u8,
            (nb.clamp(0.0, 1.0) * 255.0).round() as u8,
            a,
        ]);
    }
}

fn box_blur_in_place(image: &mut RgbaImage, radius: i32) {
    if radius <= 0 {
        return;
    }

    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return;
    }

    let mut scratch = image.clone();
    for _ in 0..2 {
        box_blur_pass(&scratch, image, width, height, radius, true);
        box_blur_pass(image, &mut scratch, width, height, radius, false);
    }
}

fn box_blur_pass(
    source: &RgbaImage,
    dest: &mut RgbaImage,
    width: u32,
    height: u32,
    radius: i32,
    horizontal: bool,
) {
    if horizontal {
        for y in 0..height {
            for x in 0..width {
                dest.put_pixel(
                    x,
                    y,
                    average_window(source, width, height, x, y, radius, true),
                );
            }
        }
    } else {
        for y in 0..height {
            for x in 0..width {
                dest.put_pixel(
                    x,
                    y,
                    average_window(source, width, height, x, y, radius, false),
                );
            }
        }
    }
}

fn average_window(
    source: &RgbaImage,
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    radius: i32,
    horizontal: bool,
) -> Rgba<u8> {
    let mut r = 0u32;
    let mut g = 0u32;
    let mut b = 0u32;
    let mut a = 0u32;
    let mut count = 0u32;

    for offset in -radius..=radius {
        let (sx, sy) = if horizontal {
            (x as i32 + offset, y as i32)
        } else {
            (x as i32, y as i32 + offset)
        };

        if sx < 0 || sy < 0 || sx >= width as i32 || sy >= height as i32 {
            continue;
        }

        let pixel = source.get_pixel(sx as u32, sy as u32).0;
        r += pixel[0] as u32;
        g += pixel[1] as u32;
        b += pixel[2] as u32;
        a += pixel[3] as u32;
        count += 1;
    }

    if count == 0 {
        return Rgba([0, 0, 0, 0]);
    }

    Rgba([
        (r / count) as u8,
        (g / count) as u8,
        (b / count) as u8,
        (a / count) as u8,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glow_from_rgba_handle_preserves_padded_dimensions() {
        let pixels = vec![
            255u8, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ];
        let source = Handle::from_rgba(2, 2, pixels);
        let spread = glow_spread(2.0).round() as u32;
        let glow = glow_from_handle(&source, 2.0, 4.0, 0.0, 2.0, 2.0).expect("glow");

        match glow {
            Handle::Rgba { width, height, .. } => {
                assert_eq!(width, 2 + spread * 2);
                assert_eq!(height, 2 + spread * 2);
            }
            other => panic!("expected rgba handle, got {other:?}"),
        }
    }

    #[test]
    fn glow_cache_returns_same_handle() {
        let pixels = vec![255u8; 16];
        let source = Handle::from_rgba(2, 2, pixels);
        let first = glow_from_handle(&source, 1.0, 4.0, 0.0, 2.0, 2.0).expect("first");
        let second = glow_from_handle(&source, 1.0, 4.0, 0.0, 2.0, 2.0).expect("second");
        assert_eq!(first.id(), second.id());
    }

    #[test]
    fn rounded_mask_zeroes_corners() {
        let pixels = vec![255u8; 16 * 16 * 4];
        let mut image = ImageBuffer::from_raw(16, 16, pixels).expect("image");
        apply_round_rect_mask(&mut image, 4.0);
        assert!(image.get_pixel(0, 0).0[3] < 32);
        assert_eq!(image.get_pixel(8, 8).0[3], 255);
    }
}
