//! Software reproduction of the liquid-gooey SVG filter graph.
//!
//! Browsers run the group's `feGaussianBlur → feColorMatrix` goo chain, the
//! shadow morphology/composite passes, the waviness displacement, and the
//! dissolve/melt turbulence filters on the GPU (or CPU on WebKit). iced has no
//! filter graph, so this module evaluates the same math on premultiplied float
//! layers:
//!
//! | upstream SVG/filter | here |
//! |---|---|
//! | `<feGaussianBlur>` | [`blur`](crate::widgets::voice_glow::blur) |
//! | `feColorMatrix` alpha rows | plane math in [`render`] |
//! | `feMorphology` spread | [`morphology`] (square kernel, like SVG) |
//! | `feOffset` / `feFlood` / `feComposite` | [`Layer::composite_over`] |
//! | `feTurbulence` + `feDisplacementMap` | [`Noise`] + warped sampling |
//! | `mask-image` edge holes | [`HoleMask`] |
//!
//! The output is three straight-alpha RGBA layers: `behind` (shadows + liquid
//! fill), `content` (image items, which this crate must own to apply pixel
//! effects), and `above` (the warped dissolve/melt copies).

use std::sync::{Arc, Mutex, OnceLock};

use iced::advanced::image::Handle;
use iced::{Color, Point, Rectangle, Size};

use crate::widgets::voice_glow::blur::blur;
use crate::widgets::voice_glow::noise::Turbulence;

use super::geometry::{self, CornerRadii, RectPx};
use super::options::ItemBox;
use super::options::{DissolveOptions, ImageMeltOptions, ShadowLayer};

/// Alpha-binarize matrix used before spread morphology (upstream `BINARIZE`).
const BINARIZE_SLOPE: f32 = 60.0;
const BINARIZE_INTERCEPT: f32 = -29.5;

/// One premultiplied float layer.
#[derive(Clone)]
pub(crate) struct Layer {
    pub width: usize,
    pub height: usize,
    pub data: Vec<[f32; 4]>,
}

impl Layer {
    pub(crate) fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![[0.0; 4]; width * height],
        }
    }

    fn over_pixel(&mut self, x: usize, y: usize, src: [f32; 4]) {
        if x >= self.width || y >= self.height {
            return;
        }
        let dst = &mut self.data[y * self.width + x];
        let inv = 1.0 - src[3];
        dst[0] = src[0] + dst[0] * inv;
        dst[1] = src[1] + dst[1] * inv;
        dst[2] = src[2] + dst[2] * inv;
        dst[3] = src[3] + dst[3] * inv;
    }

    pub(crate) fn composite_over(&mut self, src: &Layer, offset: (i32, i32)) {
        for y in 0..src.height {
            let dy = y as i32 + offset.1;
            if dy < 0 || dy as usize >= self.height {
                continue;
            }
            for x in 0..src.width {
                let dx = x as i32 + offset.0;
                if dx < 0 || dx as usize >= self.width {
                    continue;
                }
                let p = src.data[y * src.width + x];
                if p[3] > 0.0 {
                    self.over_pixel(dx as usize, dy as usize, p);
                }
            }
        }
    }

    pub(crate) fn to_straight_rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.width * self.height * 4);
        for p in &self.data {
            let a = p[3].clamp(0.0, 1.0);
            if a <= 1.0 / 512.0 {
                out.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                out.push((p[0] / a * 255.0).clamp(0.0, 255.0) as u8);
                out.push((p[1] / a * 255.0).clamp(0.0, 255.0) as u8);
                out.push((p[2] / a * 255.0).clamp(0.0, 255.0) as u8);
                out.push((a * 255.0).round() as u8);
            }
        }
        out
    }

    /// Fills the layer with a solid color where `mask` is non-zero.
    fn fill_mask(&mut self, mask: &[f32], color: Color) {
        debug_assert_eq!(mask.len(), self.data.len());
        for (dst, m) in self.data.iter_mut().zip(mask.iter()) {
            let a = (m.clamp(0.0, 1.0)) * color.a;
            if a <= 0.0 {
                continue;
            }
            let src = [color.r * a, color.g * a, color.b * a, a];
            let inv = 1.0 - a;
            dst[0] = src[0] + dst[0] * inv;
            dst[1] = src[1] + dst[1] * inv;
            dst[2] = src[2] + dst[2] * inv;
            dst[3] = a + dst[3] * inv;
        }
    }
}

/// The three raster layers a [`Liquid`](super::Liquid) needs.
#[derive(Debug, Clone)]
pub struct LiquidRaster {
    /// Raster width in pixels.
    pub width: u32,
    /// Raster height in pixels.
    pub height: u32,
    /// Pixels per logical point.
    pub density: f32,
    /// Shadows and the liquid fill, drawn below content.
    pub behind: Vec<u8>,
    /// Image items with rounded masks and pixel effects, drawn above the
    /// liquid and below widget content.
    pub content: Vec<u8>,
    /// Warped dissolve/melt imagery, drawn above content.
    pub above: Vec<u8>,
}

impl LiquidRaster {
    /// An empty raster.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            width: 1,
            height: 1,
            density: 1.0,
            behind: vec![0; 4],
            content: vec![0; 4],
            above: vec![0; 4],
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn dump(&self, dir: &std::path::Path, tag: &str) {
        let _ = std::fs::create_dir_all(dir);
        for (name, layer) in [
            ("behind", &self.behind),
            ("content", &self.content),
            ("above", &self.above),
        ] {
            let _ = image::save_buffer(
                dir.join(format!("{tag}-{name}.png")),
                layer,
                self.width,
                self.height,
                image::ColorType::Rgba8,
            );
        }
    }
}

/// Group-level rendering options (resolved from [`Liquid`](super::Liquid)).
#[derive(Debug, Clone)]
pub(crate) struct GroupStyle {
    pub blur: f32,
    pub contrast: f32,
    pub fill: Color,
    pub shadows: Vec<ShadowLayer>,
    pub waviness: f32,
    pub waviness_freq: f32,
    pub filter_padding: f32,
}

impl Default for GroupStyle {
    fn default() -> Self {
        Self {
            blur: super::options::DEFAULT_BLUR,
            contrast: super::options::DEFAULT_CONTRAST,
            fill: super::options::DEFAULT_FILL,
            shadows: Vec::new(),
            waviness: 0.0,
            waviness_freq: super::options::DEFAULT_WAVINESS_FREQ,
            filter_padding: super::options::DEFAULT_FILTER_PADDING,
        }
    }
}

/// Three trailing droplets of a `move` item (body, mid A, mid B).
#[derive(Debug, Clone, Copy)]
pub(crate) struct TailCircles {
    pub tail: (Point, f32),
    pub mid_a: (Point, f32),
    pub mid_b: (Point, f32),
}

/// The hole mask applied to an item's original image during a dissolve.
#[derive(Debug, Clone, Copy)]
pub(crate) enum HoleMask {
    /// Above a neighbour: a radial hole around the contact point.
    Radial {
        /// Centre in the item's local logical coordinates.
        center: Point,
        /// Alpha inside the core (`0` = fully erased, `1` = intact).
        alpha: f32,
        /// Fully-erased core radius.
        core: f32,
        /// Mid stop radius.
        mid: f32,
        /// Radius where the image is intact again.
        outer: f32,
    },
    /// Overlapping bodies: a linear feather centred on the neighbour's centre.
    Linear {
        /// Neighbour centre in the item's local logical coordinates.
        center: Point,
        /// Alpha at the core.
        alpha: f32,
        /// Radius of the fully erased core.
        core: f32,
        /// Radius where the image is intact again.
        outer: f32,
    },
}

/// Per-item dissolve state handed to the rasterizer.
#[derive(Debug, Clone)]
pub(crate) struct DissolvePaint {
    pub options: DissolveOptions,
    /// Structural strength (`meltFade`, before the opacity fade).
    pub strength: f32,
    /// Overlay opacity (`meltOp`).
    pub opacity: f32,
    /// Contact point in group logical coordinates.
    pub seam: Point,
    /// Neighbour centre in group logical coordinates.
    pub neighbor_center: Point,
    /// Gap between the boxes (0 when overlapping).
    pub gap: f32,
    /// Overlap depth normalised by the smaller body.
    pub embed: f32,
    /// Melt zone diameter.
    pub d: f32,
    /// Churn phase.
    pub phase: f32,
    /// Dominant flow axis (`true` = horizontal).
    pub axis_x: bool,
    /// The hole cut into the original image, if the neck reaches it.
    pub hole: Option<HoleMask>,
}

/// Per-item image-melt state handed to the rasterizer.
#[derive(Debug, Clone)]
pub(crate) struct MeltPaint {
    pub options: ImageMeltOptions,
    /// Proximity-eased reveal strength.
    pub strength: f32,
}

/// One item as the rasterizer sees it (all logical coordinates).
#[derive(Debug, Clone)]
pub(crate) struct RasterItem {
    /// Spring-lagged liquid body (after blob inset / bridge growth).
    pub bounds: ItemBox,
    /// The item's own content box, before the liquid body is inflated or
    /// shrunk. Images and melt seams use this so the picture never stretches
    /// with the goo.
    pub content: ItemBox,
    pub radii: CornerRadii,
    pub image: Option<Arc<DecodedImage>>,
    pub content_blur: f32,
    /// Smoothed bend: `(vertical bow, cap deformation)`.
    pub bend: Option<(f32, f32)>,
    /// Velocity stretch of the liquid body along the motion axis.
    pub stretch: Option<Stretch>,
    pub tail: Option<TailCircles>,
    pub dissolve: Option<DissolvePaint>,
    pub melt: Option<MeltPaint>,
    /// Whether the body joins the merged liquid silhouette.
    pub silhouette: bool,
}

/// Anisotropic scale of the liquid body along its direction of travel.
///
/// Mirrors upstream's `rotate(a) scale(1 + st, 1 / (1 + st * 0.65)) rotate(-a)`:
/// the leading half grows, the trailing half squeezes, and the body turns into
/// a soft drop instead of an ellipse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Stretch {
    /// Direction of travel, in radians.
    pub angle: f32,
    /// Scale along the motion axis (`>= 1`).
    pub along: f32,
    /// Scale across the motion axis (`<= 1`).
    pub across: f32,
}

/// A decoded image in straight RGBA, f32 per channel.
#[derive(Debug)]
pub(crate) struct DecodedImage {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<[f32; 4]>,
}

impl DecodedImage {
    fn from_rgba(width: u32, height: u32, bytes: &[u8]) -> Option<Self> {
        let width = width as usize;
        let height = height as usize;
        if width == 0 || height == 0 || bytes.len() < width * height * 4 {
            return None;
        }
        let mut pixels = Vec::with_capacity(width * height);
        for chunk in bytes[..width * height * 4].chunks_exact(4) {
            pixels.push([
                f32::from(chunk[0]) / 255.0,
                f32::from(chunk[1]) / 255.0,
                f32::from(chunk[2]) / 255.0,
                f32::from(chunk[3]) / 255.0,
            ]);
        }
        Some(Self {
            width,
            height,
            pixels,
        })
    }

    fn sample(&self, x: f32, y: f32) -> [f32; 4] {
        if x < 0.0 || y < 0.0 || x > self.width as f32 - 1.0 || y > self.height as f32 - 1.0 {
            return [0.0; 4];
        }
        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);
        let fx = x - x0 as f32;
        let fy = y - y0 as f32;
        let p00 = self.pixels[y0 * self.width + x0];
        let p10 = self.pixels[y0 * self.width + x1];
        let p01 = self.pixels[y1 * self.width + x0];
        let p11 = self.pixels[y1 * self.width + x1];
        std::array::from_fn(|c| {
            let a = p00[c] + (p10[c] - p00[c]) * fx;
            let b = p01[c] + (p11[c] - p01[c]) * fx;
            a + (b - a) * fy
        })
    }

    /// Samples the image with `object-fit: cover` inside `box` (logical).
    /// `local` is relative to the box origin.
    fn sample_cover(&self, local: Point, box_size: Size) -> [f32; 4] {
        if box_size.width <= 0.0 || box_size.height <= 0.0 {
            return [0.0; 4];
        }
        let scale = (self.width as f32 / box_size.width).max(self.height as f32 / box_size.height);
        if scale <= 0.0 || !scale.is_finite() {
            return [0.0; 4];
        }
        let drawn_w = self.width as f32 / scale;
        let drawn_h = self.height as f32 / scale;
        let ox = (box_size.width - drawn_w) / 2.0;
        let oy = (box_size.height - drawn_h) / 2.0;
        self.sample((local.x - ox) * scale, (local.y - oy) * scale)
    }
}

static DECODE_CACHE: OnceLock<Mutex<Vec<DecodeEntry>>> = OnceLock::new();

type DecodeEntry = (Handle, Option<Arc<DecodedImage>>);

/// Decodes (and caches) an iced image handle.
pub(crate) fn decode(handle: &Handle) -> Option<Arc<DecodedImage>> {
    let cache = DECODE_CACHE.get_or_init(|| Mutex::new(Vec::new()));
    if let Some((_, hit)) = cache.lock().ok()?.iter().find(|(key, _)| key == handle) {
        return hit.clone();
    }
    let decoded = match handle {
        Handle::Rgba {
            width,
            height,
            pixels,
            ..
        } => DecodedImage::from_rgba(*width, *height, pixels).map(Arc::new),
        Handle::Bytes(_, bytes) => image::load_from_memory(bytes)
            .ok()
            .map(|image| image.to_rgba8())
            .and_then(|rgba| {
                DecodedImage::from_rgba(rgba.width(), rgba.height(), rgba.as_raw()).map(Arc::new)
            }),
        Handle::Path(_, path) => image::open(path)
            .ok()
            .map(|image| image.to_rgba8())
            .and_then(|rgba| {
                DecodedImage::from_rgba(rgba.width(), rgba.height(), rgba.as_raw()).map(Arc::new)
            }),
    };
    if let Ok(mut cache) = cache.lock() {
        cache.push((handle.clone(), decoded.clone()));
    }
    decoded
}

struct Noise;

impl Noise {
    fn turbulence() -> &'static Turbulence {
        static FIELD: OnceLock<Turbulence> = OnceLock::new();
        FIELD.get_or_init(|| Turbulence::new(7))
    }

    fn at(x: f32, y: f32, freq: (f32, f32), octaves: u32, phase: f32) -> f32 {
        Self::turbulence().fractal_red(
            f64::from(x + phase * 13.0),
            f64::from(y - phase * 7.0),
            (f64::from(freq.0), f64::from(freq.1)),
            octaves.clamp(1, 4),
        )
    }
}

/// Renders the three layers for a group of items.
pub(crate) fn render(
    size: Size,
    density: f32,
    style: &GroupStyle,
    items: &[RasterItem],
) -> LiquidRaster {
    let density = if density.is_finite() {
        density.clamp(0.5, 3.0)
    } else {
        1.0
    };
    let (width, height) = geometry::resize(size, density);
    let mut behind = Layer::new(width, height);
    let mut content = Layer::new(width, height);
    let mut above = Layer::new(width, height);

    if width < 2 || height < 2 {
        return LiquidRaster {
            width: width as u32,
            height: height as u32,
            density,
            behind: behind.to_straight_rgba(),
            content: content.to_straight_rgba(),
            above: above.to_straight_rgba(),
        };
    }

    // 1. Silhouette: union of every item body, trailing droplets, and bent
    //    bodies (a bowed body is a polygon, not a rect).
    let mut shape = vec![0.0_f32; width * height];
    for item in items.iter().filter(|item| item.silhouette) {
        let rect = RectPx::from_rect(
            Rectangle {
                x: item.bounds.origin.x,
                y: item.bounds.origin.y,
                width: item.bounds.size.width,
                height: item.bounds.size.height,
            },
            density,
        );
        let radii = item.radii.scaled(density);
        let (bend_y, bend_x) = item.bend.unwrap_or((0.0, 0.0));
        let bowed = bend_y.abs() > 0.35 || bend_x.abs() > 0.35;
        if bowed || item.stretch.is_some() {
            let mut points =
                geometry::bend_polygon(rect, radii, bend_y * density, bend_x * density);
            if let Some(stretch) = item.stretch {
                let center = Point::new(
                    item.bounds.origin.x * density + rect.w / 2.0,
                    item.bounds.origin.y * density + rect.h / 2.0,
                );
                geometry::stretch_points(
                    &mut points,
                    center,
                    stretch.angle,
                    stretch.along,
                    stretch.across,
                );
            }
            geometry::fill_polygon(&mut shape, width, height, &points);
            #[cfg(debug_assertions)]
            if std::env::var_os("GOOEY_BEND_TRACE").is_some() {
                let covered = shape.iter().filter(|a| **a > 0.0).count();
                eprintln!(
                    "[bend] rect={rect:?} radii={radii:?} bend=({bend_y},{bend_x}) points={} covered={covered}",
                    points.len()
                );
            }
        } else {
            geometry::fill_rrect(&mut shape, width, height, rect, radii);
        }
        if let Some(tail) = item.tail {
            for (center, radius) in [tail.tail, tail.mid_a, tail.mid_b] {
                geometry::fill_circle(
                    &mut shape,
                    width,
                    height,
                    center.x * density,
                    center.y * density,
                    radius * density,
                );
            }
        }
    }

    // 2. Waviness: one shared turbulence displacement field.
    if style.waviness > 0.0 {
        shape = displace(
            &shape,
            width,
            height,
            density,
            style.waviness * 2.0,
            style.waviness_freq,
            2,
        );
    }

    // 3. Goo: blur, alpha contrast, then `SourceGraphic atop goo`.
    let shape_raw = shape.clone();
    let sigma = (style.blur * density).max(0.0);
    let mut gooed = shape_raw.clone();
    if sigma > 0.0 {
        blur_plane(&mut gooed, width, height, sigma, sigma);
    }
    let contrast = style.contrast;
    // Upstream: intercept = round((0.5 - contrast*(5/12))*100)/100.
    let intercept = ((0.5 - contrast * (5.0 / 12.0)) * 100.0).round() / 100.0;
    for (s, g) in shape.iter_mut().zip(gooed.iter()) {
        let a = (contrast * g + intercept).clamp(0.0, 1.0);
        *s *= a;
    }

    // 4. Binarized silhouette shared by every spread/inset pass.
    let bin = if style
        .shadows
        .iter()
        .any(|shadow| shadow.inset || shadow.spread != 0.0)
    {
        Some(
            shape_raw
                .iter()
                .map(|a| (BINARIZE_SLOPE * a + BINARIZE_INTERCEPT).clamp(0.0, 1.0))
                .collect::<Vec<_>>(),
        )
    } else {
        None
    };

    // 5. Outer shadows below the shape, in reverse CSS order.
    for shadow in style.shadows.iter().rev().filter(|s| !s.inset) {
        let mut mask = if shadow.spread == 0.0 {
            shape_raw.clone()
        } else {
            let mut m = bin.clone().unwrap_or_else(|| shape_raw.clone());
            morphology(
                &mut m,
                width,
                height,
                (shadow.spread.abs() * density).round() as usize,
                shadow.spread > 0.0,
            );
            m
        };
        shadow_blur(&mut mask, width, height, shadow.blur * 0.5 * density);
        let mask = offset_mask(&mask, width, height, shadow.x * density, shadow.y * density);
        behind.fill_mask(&mask, shadow.color);
    }

    // 6. The shape itself.
    behind.fill_mask(&shape, style.fill);

    // 7. Inset shadows above the shape.
    for shadow in style.shadows.iter().filter(|s| s.inset) {
        let bin = bin.as_deref().unwrap_or(&shape_raw);
        let mut shrunk = bin.to_vec();
        if shadow.spread != 0.0 {
            morphology(
                &mut shrunk,
                width,
                height,
                (shadow.spread.abs() * density).round() as usize,
                shadow.spread < 0.0,
            );
        }
        if shadow.x != 0.0 || shadow.y != 0.0 {
            shrunk = offset_mask(
                &shrunk,
                width,
                height,
                shadow.x * density,
                shadow.y * density,
            );
        }
        shadow_blur(&mut shrunk, width, height, shadow.blur * 0.5 * density);
        let band: Vec<f32> = bin
            .iter()
            .zip(shrunk.iter())
            .map(|(b, s)| (b - s).clamp(0.0, 1.0) * b)
            .collect();
        behind.fill_mask(&band, shadow.color);
    }

    // 8. Image content and the dissolve overlays. Image-melt items paint
    //    nothing individually: the pair compositor owns their pixels.
    for item in items.iter().filter(|item| item.melt.is_none()) {
        let Some(image) = &item.image else {
            continue;
        };
        paint_image_item(item, image, density, &mut content, &mut above);
    }
    paint_image_melt(items, density, width, height, &mut content);

    #[cfg(debug_assertions)]
    if std::env::var_os("GOOEY_AUDIT").is_some() {
        audit_layer("content", &content);
        audit_layer("above", &above);
    }

    LiquidRaster {
        width: width as u32,
        height: height as u32,
        density,
        behind: behind.to_straight_rgba(),
        content: content.to_straight_rgba(),
        above: above.to_straight_rgba(),
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_image_item(
    item: &RasterItem,
    image: &Arc<DecodedImage>,
    density: f32,
    content: &mut Layer,
    above: &mut Layer,
) {
    let card = item.content;
    let rect = RectPx::from_rect(
        Rectangle {
            x: card.origin.x,
            y: card.origin.y,
            width: card.size.width,
            height: card.size.height,
        },
        density,
    );
    let radii = item.radii.scaled(density);
    let pad = ((image_blur_pad(item) * density).ceil() as usize).min(96);
    let x0 = (rect.x.floor() as i32 - pad as i32).max(0);
    let y0 = (rect.y.floor() as i32 - pad as i32).max(0);
    let x1 = (rect.right().ceil() as i32 + pad as i32).min(content.width as i32);
    let y1 = (rect.bottom().ceil() as i32 + pad as i32).min(content.height as i32);
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let w = (x1 - x0) as usize;
    let h = (y1 - y0) as usize;
    let mut layer = Layer::new(w, h);
    for y in 0..h {
        let py = y as f32 + y0 as f32 + 0.5;
        for x in 0..w {
            let px = x as f32 + x0 as f32 + 0.5;
            let alpha = geometry::coverage(px, py, rect, radii);
            if alpha <= 0.0 {
                continue;
            }
            let local = Point::new(px / density - card.origin.x, py / density - card.origin.y);
            let c = image.sample_cover(local, card.size);
            let a = c[3] * alpha;
            layer.data[y * w + x] = [c[0] * a, c[1] * a, c[2] * a, a];
        }
    }

    if item.content_blur > 0.3 {
        let sigma = item.content_blur * density;
        let mut tmp = Vec::new();
        blur(&mut layer.data, w, h, sigma, sigma, &mut tmp);
    }

    // Edge hole / crisp erase masks on the original image.
    if let Some(hole) = item.dissolve.as_ref().and_then(|paint| paint.hole.as_ref()) {
        for y in 0..h {
            let local_y = (y as f32 + y0 as f32 + 0.5) / density - card.origin.y;
            for x in 0..w {
                let local_x = (x as f32 + x0 as f32 + 0.5) / density - card.origin.x;
                let keep = hole_alpha(hole, Point::new(local_x, local_y));
                if keep < 1.0 {
                    let p = &mut layer.data[y * w + x];
                    p[0] *= keep;
                    p[1] *= keep;
                    p[2] *= keep;
                    p[3] *= keep;
                }
            }
        }
    }
    content.composite_over(&layer, (x0, y0));

    if let Some(paint) = item.dissolve.as_ref().filter(|d| d.strength > 0.001) {
        paint_dissolve(item, image, paint, density, above);
    }
}

#[cfg(debug_assertions)]
fn audit_layer(name: &str, layer: &Layer) {
    let mut bad = 0_usize;
    let mut first = None;
    for (index, p) in layer.data.iter().enumerate() {
        let invalid = p.iter().any(|v| !v.is_finite())
            || p[3] < -1e-4
            || p[3] > 1.001
            || p[0] > p[3] + 1e-3
            || p[1] > p[3] + 1e-3
            || p[2] > p[3] + 1e-3;
        if invalid {
            bad += 1;
            if first.is_none() {
                first = Some((index % layer.width, index / layer.width, *p));
            }
        }
    }
    if bad > 0 {
        eprintln!("gooey audit {name}: {bad} invalid px, first {first:?}");
    }
}

fn image_blur_pad(item: &RasterItem) -> f32 {
    let mut pad = item.content_blur * 3.0;
    if let Some(paint) = &item.dissolve {
        pad = pad
            .max((paint.options.blur * 3.0) + paint.options.warp * 1.5)
            .max(paint.options.gravity * 0.6);
    }
    if let Some(melt) = &item.melt {
        pad = pad.max(melt.options.blur * 3.0 + melt.options.waviness * 1.5);
    }
    pad.max(1.0)
}

fn paint_dissolve(
    item: &RasterItem,
    image: &Arc<DecodedImage>,
    paint: &DissolvePaint,
    density: f32,
    above: &mut Layer,
) {
    let s = paint.strength.clamp(0.0, 1.0);
    let e = s * s * (3.0 - 2.0 * s);
    if e <= 0.001 {
        return;
    }
    let opts = &paint.options;
    let strength = paint.opacity.clamp(0.0, 1.0);
    if strength <= 0.001 {
        return;
    }
    let ncx = paint.neighbor_center.x;
    let ncy = paint.neighbor_center.y;
    let dx = ncx - paint.seam.x;
    let dy = ncy - paint.seam.y;
    let len = (dx * dx + dy * dy).sqrt();
    let (gux, guy) = if len > 1e-4 {
        (dx / len, dy / len)
    } else {
        (1.0, 0.0)
    };
    let g_deg = guy.atan2(gux);
    let g_amt = opts.gravity.max(0.0) * e;
    let d = paint.d * density;
    let bx = (paint.seam.x + gux * paint.d * 0.05) * density;
    let by = (paint.seam.y + guy * paint.d * 0.05) * density;
    let taper = opts.taper.clamp(0.0, 1.0);
    let settle = 1.0 - 0.45 * smoothstep((paint.embed * 2.2).min(1.0));
    let k_flow = ((g_amt + paint.gap.max(0.0)) / (8.0_f32.max(2.0 * paint.d))).min(2.2)
        * (0.5 + taper)
        * settle;
    let anchor_x = paint.seam.x - gux * paint.d;
    let anchor_y = paint.seam.y - guy * paint.d;
    let elong = 1.0 + (paint.gap / 8.0_f32.max(2.0 * paint.d)).min(1.0) * 1.8;

    // Three graded layers, outermost to innermost.
    for (index, t) in [0.0_f32, 0.5, 1.0].into_iter().enumerate() {
        let blur_k = 0.15 + 0.85 * t.powf(1.4);
        let warp_k = 0.45 + 0.55 * t;
        let mask_r = d * (0.95 - 0.55 * t);
        let layer_opacity = e * (0.75 + 0.25 * t);
        let phase = paint.phase * (0.7 + 0.45 * t) + index as f32 * 0.7;
        let (mut layer, offset) = warp_image(
            item,
            image,
            density,
            WarpSpec {
                center: Point::new(bx / density, by / density),
                mask_radius: mask_r / density,
                blur: opts.blur * blur_k * e,
                warp: opts.warp * warp_k * e,
                mix: opts.mix * e,
                flow: k_flow * (0.4 + 0.6 * t),
                angle: g_deg,
                anchor: Point::new(anchor_x, anchor_y),
                axis_x: paint.axis_x,
                phase,
                octaves: opts.detail,
                elong,
                opacity: layer_opacity,
            },
        );
        if blur_k > 0.0 {
            let sigma = opts.blur * blur_k * e * density;
            let mut tmp = Vec::new();
            blur(
                &mut layer.data,
                layer.width,
                layer.height,
                sigma,
                sigma,
                &mut tmp,
            );
        }
        erode(
            &mut layer,
            opts.mix * e,
            density,
            opts.detail,
            paint.phase,
            t,
            offset,
        );
        above.composite_over(&layer, offset);
    }

    // Seam-blend wash: plain heavy blur, half opacity, both sides paint one.
    let seam_blur = opts.seam_blur.unwrap_or(opts.blur * 1.6);
    if seam_blur > 0.0 {
        let (mut layer, offset) = warp_image(
            item,
            image,
            density,
            WarpSpec {
                center: paint.seam,
                mask_radius: d * 1.25 / density,
                blur: 0.0,
                warp: 0.0,
                mix: 0.0,
                flow: 0.0,
                angle: g_deg,
                anchor: paint.seam,
                axis_x: paint.axis_x,
                phase: paint.phase,
                octaves: opts.detail,
                elong,
                opacity: (e * 1.2).min(1.0) * 0.55,
            },
        );
        let sigma = seam_blur * e * density;
        let mut tmp = Vec::new();
        blur(
            &mut layer.data,
            layer.width,
            layer.height,
            sigma,
            sigma,
            &mut tmp,
        );
        above.composite_over(&layer, offset);
    }
}

/// Paints the first two image-melt items as one pair, mirroring upstream
/// `imageMelt.tsx`: a molten goo silhouette whose colours come from a wider
/// blur clipped to the goo alpha, a marbled pass over the touch zone, and the
/// two crisp faces erased back from the seam.
fn paint_image_melt(
    items: &[RasterItem],
    density: f32,
    width: usize,
    height: usize,
    content: &mut Layer,
) {
    let mut melt_items = items
        .iter()
        .filter(|item| item.melt.is_some() && item.image.is_some());
    let (Some(a), Some(b)) = (melt_items.next(), melt_items.next()) else {
        return;
    };
    let (Some(image_a), Some(image_b)) = (a.image.as_ref(), b.image.as_ref()) else {
        return;
    };
    let (Some(melt_a), Some(melt_b)) = (a.melt.as_ref(), b.melt.as_ref()) else {
        return;
    };
    let opts = melt_a.options;
    let prox = melt_a.strength.min(melt_b.strength).clamp(0.0, 1.0);

    let mut molten = Layer::new(width, height);
    draw_melt_card(&mut molten, a, image_a, density);
    draw_melt_card(&mut molten, b, image_b, density);
    if prox <= 0.001 {
        content.composite_over(&molten, (0, 0));
        return;
    }

    let ca = a.content.center();
    let cb = b.content.center();
    let (aw, ah) = (a.content.size.width, a.content.size.height);
    let (bw, bh) = (b.content.size.width, b.content.size.height);
    let reach = opts.reach.max(0.0);
    let r_a = (aw.hypot(ah) / 2.0) * reach * prox;
    let r_b = (bw.hypot(bh) / 2.0) * reach * prox;
    let seam = Point::new((ca.x + cb.x) / 2.0, (ca.y + cb.y) / 2.0);
    let dxc = cb.x - ca.x;
    let dyc = cb.y - ca.y;
    let dc = dxc.hypot(dyc).max(1e-3);
    let (tx, ty) = (-dyc / dc, dxc / dc);
    let ovx = ((aw + bw) / 2.0 - dxc.abs()).max(0.0);
    let ovy = ((ah + bh) / 2.0 - dyc.abs()).max(0.0);
    let tan_half = 0.5 * (ovx * tx.abs() + ovy * ty.abs()) * prox;
    let seam_angle = ty.atan2(tx);
    let mix_amt = (opts.mix * prox).clamp(0.0, 1.0);
    let blur_eff = (2.0 + (opts.blur - 2.0) * prox).max(0.0);
    let warp_eff = opts.warp * prox;
    let color_blur = (blur_eff * 2.5).max(0.0);
    let edge_soft = (0.4 + (2.0 + opts.blur * 0.8) * prox).max(0.0);
    let intercept = ((0.5 - opts.contrast * (5.0 / 12.0)) * 100.0).round() / 100.0;
    let goo_alpha = |layer: &Layer| -> Vec<f32> {
        layer
            .data
            .iter()
            .map(|p| (opts.contrast * p[3] + intercept).clamp(0.0, 1.0))
            .collect()
    };

    // Molten: blur mixes the colours, a wider blur supplies the colour field,
    // and the goo alpha clips it back into a liquid shape.
    let mut goo = molten.clone();
    let mut tmp = Vec::new();
    blur(
        &mut goo.data,
        width,
        height,
        blur_eff * density,
        blur_eff * density,
        &mut tmp,
    );
    let mut mix = molten.clone();
    blur(
        &mut mix.data,
        width,
        height,
        color_blur * density,
        color_blur * density,
        &mut tmp,
    );
    let goo_shape = goo_alpha(&goo);
    for (p, a_goo) in mix.data.iter_mut().zip(goo_shape.iter()) {
        p[0] *= a_goo;
        p[1] *= a_goo;
        p[2] *= a_goo;
        p[3] *= a_goo;
    }
    let mut melt_layer = if warp_eff > 0.01 {
        displace_rgba(
            &mix,
            density,
            warp_eff,
            (
                (2.0 + opts.waviness * 1.2) / 1000.0,
                (2.0 + opts.waviness * 1.2) / 1000.0,
            ),
            MeltNoise::warp(),
        )
    } else {
        mix
    };
    solidify_rgba(&mut melt_layer.data, 3);
    blur(
        &mut melt_layer.data,
        width,
        height,
        0.6 * density,
        0.6 * density,
        &mut tmp,
    );
    content.composite_over(&melt_layer, (0, 0));

    // Marble: nested large-scale displacements fold both palettes into
    // streaks inside the touch zone, clipped to the goo alpha.
    if mix_amt > 0.01 {
        let mut marble = molten.clone();
        blur(
            &mut marble.data,
            width,
            height,
            opts.mix_blur * density,
            opts.mix_blur * density,
            &mut tmp,
        );
        marble = displace_rgba(
            &marble,
            density,
            mix_amt * 90.0,
            (0.011, 0.011),
            MeltNoise::marble_a(),
        );
        marble = displace_rgba(
            &marble,
            density,
            mix_amt * 50.0,
            (0.019, 0.019),
            MeltNoise::marble_b(),
        );
        solidify_rgba(&mut marble.data, 2);
        blur(
            &mut marble.data,
            width,
            height,
            0.6 * density,
            0.6 * density,
            &mut tmp,
        );
        let zone = ellipse_mask(
            width,
            height,
            density,
            Ellipse {
                center: seam,
                radius_x: (r_a + r_b) / 2.0 + tan_half,
                radius_y: ((r_a + r_b) / 2.0) * opts.gravity.max(0.0),
                angle: seam_angle,
                blur: opts.fade.max(0.0),
            },
        );
        for (p, (zone, a_goo)) in marble
            .data
            .iter_mut()
            .zip(zone.iter().zip(goo_shape.iter()))
        {
            let keep = zone * a_goo;
            p[0] *= keep;
            p[1] *= keep;
            p[2] *= keep;
            p[3] *= keep;
        }
        content.composite_over(&marble, (0, 0));
    }

    // Crisp faces: each card minus a blurred seam-spanning erase ellipse.
    for (item, image, other_r) in [(a, image_a, r_b), (b, image_b, r_a)] {
        let mut face = Layer::new(width, height);
        draw_melt_card(&mut face, item, image, density);
        let mask = crisp_mask(
            width,
            height,
            density,
            item,
            seam,
            other_r + tan_half,
            other_r,
            seam_angle,
            edge_soft,
            opts.fade.max(0.0),
        );
        for (p, m) in face.data.iter_mut().zip(mask.iter()) {
            p[0] *= m;
            p[1] *= m;
            p[2] *= m;
            p[3] *= m;
        }
        content.composite_over(&face, (0, 0));
    }
}

fn draw_melt_card(layer: &mut Layer, item: &RasterItem, image: &DecodedImage, density: f32) {
    let card = item.content;
    let rect = RectPx::from_rect(
        Rectangle {
            x: card.origin.x,
            y: card.origin.y,
            width: card.size.width,
            height: card.size.height,
        },
        density,
    );
    let radii = item.radii.scaled(density);
    let x0 = rect.x.floor().max(0.0) as usize;
    let y0 = rect.y.floor().max(0.0) as usize;
    let x1 = (rect.right().ceil().max(0.0) as usize).min(layer.width);
    let y1 = (rect.bottom().ceil().max(0.0) as usize).min(layer.height);
    for y in y0..y1 {
        let py = y as f32 + 0.5;
        for x in x0..x1 {
            let px = x as f32 + 0.5;
            let alpha = geometry::coverage(px, py, rect, radii);
            if alpha <= 0.0 {
                continue;
            }
            let local = Point::new(px / density - card.origin.x, py / density - card.origin.y);
            let c = image.sample_cover(local, card.size);
            let a = c[3] * alpha;
            layer.over_pixel(x, y, [c[0] * a, c[1] * a, c[2] * a, a]);
        }
    }
}

fn solidify_rgba(data: &mut [[f32; 4]], passes: usize) {
    for _ in 0..passes {
        for p in data.iter_mut() {
            let inv = 1.0 - p[3];
            p[0] += p[0] * inv;
            p[1] += p[1] * inv;
            p[2] += p[2] * inv;
            p[3] += p[3] * inv;
        }
    }
}

fn displace_rgba(
    layer: &Layer,
    density: f32,
    scale: f32,
    freq: (f32, f32),
    noise: &Turbulence,
) -> Layer {
    let mut out = Layer::new(layer.width, layer.height);
    if scale.abs() < 0.01 {
        out.data.copy_from_slice(&layer.data);
        return out;
    }
    let amount = scale * density;
    for y in 0..layer.height {
        for x in 0..layer.width {
            let lx = (x as f32 + 0.5) / density;
            let ly = (y as f32 + 0.5) / density;
            let n1 = melt_noise(noise, lx, ly, freq, 2);
            let n2 = melt_noise(noise, lx + 137.0, ly - 71.0, freq, 2);
            let sx = (x as f32 + (n1 - 0.5) * amount).round() as i32;
            let sy = (y as f32 + (n2 - 0.5) * amount).round() as i32;
            if sx < 0 || sy < 0 || sx as usize >= layer.width || sy as usize >= layer.height {
                continue;
            }
            out.data[y * layer.width + x] = layer.data[sy as usize * layer.width + sx as usize];
        }
    }
    out
}

fn melt_noise(noise: &Turbulence, x: f32, y: f32, freq: (f32, f32), octaves: u32) -> f32 {
    noise.fractal_red(
        f64::from(x),
        f64::from(y),
        (f64::from(freq.0), f64::from(freq.1)),
        octaves.clamp(1, 4),
    )
}

/// A rotated ellipse in the seam's coordinate space.
struct Ellipse {
    center: Point,
    radius_x: f32,
    radius_y: f32,
    angle: f32,
    /// Softening of the edge, in logical pixels.
    blur: f32,
}

/// A blurred, rotated ellipse coverage plane (the seam erase / marble zone).
fn ellipse_mask(width: usize, height: usize, density: f32, ellipse: Ellipse) -> Vec<f32> {
    let mut mask = vec![0.0_f32; width * height];
    let rx = ellipse.radius_x.max(0.01);
    let ry = ellipse.radius_y.max(0.01);
    let (sin, cos) = ellipse.angle.sin_cos();
    for y in 0..height {
        let py = (y as f32 + 0.5) / density - ellipse.center.y;
        for x in 0..width {
            let px = (x as f32 + 0.5) / density - ellipse.center.x;
            let u = px * cos + py * sin;
            let v = -px * sin + py * cos;
            if (u / rx).powi(2) + (v / ry).powi(2) <= 1.0 {
                mask[y * width + x] = 1.0;
            }
        }
    }
    blur_plane(
        &mut mask,
        width,
        height,
        ellipse.blur * density,
        ellipse.blur * density,
    );
    mask
}

/// The crisp-face mask: the card's rounded rect (softened) minus the seam
/// erase ellipse.
#[allow(clippy::too_many_arguments)]
fn crisp_mask(
    width: usize,
    height: usize,
    density: f32,
    item: &RasterItem,
    seam: Point,
    radius_x: f32,
    radius_y: f32,
    angle: f32,
    edge_soft: f32,
    fade: f32,
) -> Vec<f32> {
    let card = item.content;
    let rect = RectPx::from_rect(
        Rectangle {
            x: card.origin.x,
            y: card.origin.y,
            width: card.size.width,
            height: card.size.height,
        },
        density,
    );
    let radii = item.radii.scaled(density);
    let mut mask = vec![0.0_f32; width * height];
    for y in 0..height {
        let py = y as f32 + 0.5;
        for x in 0..width {
            let px = x as f32 + 0.5;
            mask[y * width + x] = geometry::coverage(px, py, rect, radii);
        }
    }
    blur_plane(
        &mut mask,
        width,
        height,
        edge_soft * density,
        edge_soft * density,
    );
    let erase = ellipse_mask(
        width,
        height,
        density,
        Ellipse {
            center: seam,
            radius_x,
            radius_y,
            angle,
            blur: fade,
        },
    );
    for (m, e) in mask.iter_mut().zip(erase.iter()) {
        *m *= 1.0 - e;
    }
    mask
}

/// The seeded turbulence fields used by the image-melt filter graph.
struct MeltNoise;

impl MeltNoise {
    fn warp() -> &'static Turbulence {
        static FIELD: OnceLock<Turbulence> = OnceLock::new();
        FIELD.get_or_init(|| Turbulence::new(4))
    }

    fn marble_a() -> &'static Turbulence {
        static FIELD: OnceLock<Turbulence> = OnceLock::new();
        FIELD.get_or_init(|| Turbulence::new(5))
    }

    fn marble_b() -> &'static Turbulence {
        static FIELD: OnceLock<Turbulence> = OnceLock::new();
        FIELD.get_or_init(|| Turbulence::new(11))
    }
}

struct WarpSpec {
    center: Point,
    mask_radius: f32,
    blur: f32,
    warp: f32,
    mix: f32,
    flow: f32,
    angle: f32,
    anchor: Point,
    axis_x: bool,
    phase: f32,
    octaves: u32,
    elong: f32,
    opacity: f32,
}

/// Builds the warped, masked image copy used by dissolve and melt.
fn warp_image(
    item: &RasterItem,
    image: &Arc<DecodedImage>,
    density: f32,
    spec: WarpSpec,
) -> (Layer, (i32, i32)) {
    let rect = RectPx::from_rect(
        Rectangle {
            x: item.content.origin.x,
            y: item.content.origin.y,
            width: item.content.size.width,
            height: item.content.size.height,
        },
        density,
    );
    let radii = item.radii.scaled(density);
    let margin = (spec.mask_radius + spec.blur * 2.0 + spec.warp * 2.0 + 24.0).max(16.0);
    let x0 = ((spec.center.x - margin) * density).floor() as i32;
    let y0 = ((spec.center.y - margin) * density).floor() as i32;
    let x1 = ((spec.center.x + margin) * density).ceil() as i32;
    let y1 = ((spec.center.y + margin) * density).ceil() as i32;
    let w = (x1 - x0).max(1) as usize;
    let h = (y1 - y0).max(1) as usize;
    let mut layer = Layer::new(w, h);

    let cos = spec.angle.cos();
    let sin = spec.angle.sin();
    let (bf_along, bf_across) = {
        let zone = spec.mask_radius.max(8.0);
        let base = (1.7 / (zone * 1.1)).clamp(0.01, 0.3);
        (base * 0.35, base * 1.6)
    };
    let freq = if spec.axis_x {
        (bf_along, bf_across)
    } else {
        (bf_across, bf_along)
    };
    let sx = 1.0 + spec.flow * 0.6;
    let sy = 1.0 / (1.0 + spec.flow * 0.35);
    let mix_amt = spec.mix.clamp(0.0, 1.0);

    for y in 0..h {
        let py = y as f32 + y0 as f32 + 0.5;
        for x in 0..w {
            let px = x as f32 + x0 as f32 + 0.5;
            let world = Point::new(px / density, py / density);
            let dcx = world.x - spec.center.x;
            let dcy = world.y - spec.center.y;
            // Elongated mask along the flow axis, circular across.
            let ca = spec.angle.cos();
            let sa = spec.angle.sin();
            let along = dcx * ca + dcy * sa;
            let across = -dcx * sa + dcy * ca;
            let dist = ((along / spec.elong).powi(2) + across.powi(2)).sqrt();
            let feather = (spec.mask_radius * 0.18).max(1.0);
            let mask = smoothstep((spec.mask_radius - dist) / feather);
            if mask <= 0.0 {
                continue;
            }
            let inside = geometry::coverage(px, py, rect, radii);
            if inside <= 0.0 {
                continue;
            }

            // Inverse gravity stretch anchored at the trailing edge, in
            // world coordinates.
            let ax = world.x - spec.anchor.x;
            let ay = world.y - spec.anchor.y;
            let rx = ax * cos + ay * sin;
            let ry = -ax * sin + ay * cos;
            let ux = rx / sx;
            let uy = ry / sy;
            let mut src_x = spec.anchor.x + ux * cos - uy * sin;
            let mut src_y = spec.anchor.y + ux * sin + uy * cos;

            if spec.warp > 0.0 {
                let n1 = Noise::at(src_x, src_y, freq, spec.octaves, spec.phase);
                let n2 = Noise::at(
                    src_x + 137.0,
                    src_y - 71.0,
                    freq,
                    spec.octaves,
                    spec.phase + 0.37,
                );
                src_x += (n1 - 0.5) * 2.0 * spec.warp;
                src_y += (n2 - 0.5) * 2.0 * spec.warp;
            }
            let local_src =
                Point::new(src_x - item.content.origin.x, src_y - item.content.origin.y);
            let c = image.sample_cover(local_src, item.content.size);
            let a = c[3] * inside * mask * spec.opacity;
            if a <= 0.0 {
                continue;
            }
            let mut p = [c[0] * a, c[1] * a, c[2] * a, a];
            if mix_amt > 0.0 {
                let n = Noise::at(world.x, world.y, freq, spec.octaves, spec.phase + 1.1);
                let k = 1.0 + 4.0 * mix_amt;
                let c0 = 1.0 - k * (0.38 + 0.12 * mix_amt);
                let alpha = (k * n + c0).clamp(0.0, 1.0);
                p[0] *= alpha;
                p[1] *= alpha;
                p[2] *= alpha;
                p[3] *= alpha;
            }
            layer.data[y * w + x] = p;
        }
    }
    (layer, (x0, y0))
}

/// Erosion pass (upstream's alpha `feColorMatrix` after blur).
fn erode(
    layer: &mut Layer,
    mix: f32,
    density: f32,
    octaves: u32,
    phase: f32,
    t: f32,
    origin: (i32, i32),
) {
    let amt = mix.clamp(0.0, 1.0) * (0.15 + 0.85 * t);
    if amt < 0.002 {
        return;
    }
    let k = 1.0 + 4.0 * amt;
    let c = 1.0 - k * (0.38 + 0.12 * amt);
    for y in 0..layer.height {
        for x in 0..layer.width {
            let n = Noise::at(
                (x as i32 + origin.0) as f32 / density,
                (y as i32 + origin.1) as f32 / density,
                (0.02, 0.02),
                octaves,
                phase,
            );
            let target = (k * n + c).clamp(0.0, 1.0);
            let p = &mut layer.data[y * layer.width + x];
            // `feComposite operator="in"`: the noise threshold scales the
            // clipped copy's alpha (and its premultiplied colour with it).
            for channel in p.iter_mut() {
                *channel *= target;
            }
        }
    }
}

fn hole_alpha(hole: &HoleMask, local: Point) -> f32 {
    match *hole {
        HoleMask::Radial {
            center,
            alpha,
            core,
            mid,
            outer,
        } => {
            let r = distance(local, center);
            if r <= core {
                alpha
            } else if r <= mid {
                lerp(
                    alpha,
                    (1.0 + 2.0 * alpha) / 3.0,
                    (r - core) / (mid - core).max(1e-4),
                )
            } else if r < outer {
                lerp(
                    (1.0 + 2.0 * alpha) / 3.0,
                    1.0,
                    (r - mid) / (outer - mid).max(1e-4),
                )
            } else {
                1.0
            }
        }
        HoleMask::Linear {
            center,
            alpha,
            core,
            outer,
        } => {
            let r = distance(local, center);
            if r <= core {
                alpha
            } else if r >= outer {
                1.0
            } else {
                lerp(alpha, 1.0, (r - core) / (outer - core).max(1e-4))
            }
        }
    }
}

fn distance(a: Point, b: Point) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

#[allow(clippy::cast_precision_loss)]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

fn smoothstep(t: f32) -> f32 {
    let c = t.clamp(0.0, 1.0);
    c * c * (3.0 - 2.0 * c)
}

/// Blurs a mask plane.
fn blur_plane(plane: &mut [f32], width: usize, height: usize, sx: f32, sy: f32) {
    if width == 0 || height == 0 {
        return;
    }
    let mut packed: Vec<[f32; 1]> = plane.iter().map(|value| [*value]).collect();
    let mut tmp = Vec::new();
    blur(&mut packed, width, height, sx, sy, &mut tmp);
    for (dst, src) in plane.iter_mut().zip(packed.iter()) {
        *dst = src[0];
    }
}

fn shadow_blur(mask: &mut [f32], width: usize, height: usize, sigma: f32) {
    if sigma <= 0.0 {
        return;
    }
    blur_plane(mask, width, height, sigma, sigma);
}

/// Applies a translational offset to a mask (positive = right/down).
fn offset_mask(mask: &[f32], w: usize, h: usize, dx: f32, dy: f32) -> Vec<f32> {
    if dx == 0.0 && dy == 0.0 {
        return mask.to_vec();
    }
    let mut out = vec![0.0; mask.len()];
    let ix = dx.round() as i32;
    let iy = dy.round() as i32;
    for y in 0..h {
        let sy = y as i32 - iy;
        if sy < 0 || sy >= h as i32 {
            continue;
        }
        for x in 0..w {
            let sx = x as i32 - ix;
            if sx < 0 || sx >= w as i32 {
                continue;
            }
            out[y * w + x] = mask[sy as usize * w + sx as usize];
        }
    }
    out
}

/// Separable square morphology: dilate (`dilate = true`) or erode.
pub(crate) fn morphology(
    plane: &mut [f32],
    width: usize,
    height: usize,
    radius: usize,
    dilate: bool,
) {
    if radius == 0 || width == 0 || height == 0 {
        return;
    }
    let mut tmp = vec![0.0; width * height];
    // Horizontal.
    for y in 0..height {
        let row = &plane[y * width..(y + 1) * width];
        morph_line(row, &mut tmp[y * width..(y + 1) * width], radius, dilate);
    }
    // Vertical.
    for x in 0..width {
        let mut col = vec![0.0; height];
        for y in 0..height {
            col[y] = tmp[y * width + x];
        }
        let mut out = vec![0.0; height];
        morph_line(&col, &mut out, radius, dilate);
        for y in 0..height {
            plane[y * width + x] = out[y];
        }
    }
}

fn morph_line(src: &[f32], dst: &mut [f32], radius: usize, dilate: bool) {
    use std::collections::VecDeque;
    let n = src.len();
    if n == 0 {
        return;
    }
    let mut deque: VecDeque<usize> = VecDeque::new();
    let mut next = 0_usize;
    for (i, out) in dst.iter_mut().enumerate() {
        let hi = (i + radius).min(n - 1);
        while next <= hi {
            while let Some(&j) = deque.back() {
                let better = if dilate {
                    src[j] <= src[next]
                } else {
                    src[j] >= src[next]
                };
                if better {
                    deque.pop_back();
                } else {
                    break;
                }
            }
            deque.push_back(next);
            next += 1;
        }
        while let Some(&j) = deque.front() {
            if j + radius < i {
                deque.pop_front();
            } else {
                break;
            }
        }
        *out = deque.front().map_or(0.0, |&j| src[j]);
    }
}

/// Displaces an alpha plane with a fractal turbulence field.
fn displace(
    plane: &[f32],
    width: usize,
    height: usize,
    density: f32,
    scale: f32,
    freq: f32,
    octaves: u32,
) -> Vec<f32> {
    let mut out = vec![0.0; width * height];
    let freq = (freq, freq);
    for y in 0..height {
        let py = y as f32 + 0.5;
        for x in 0..width {
            let px = x as f32 + 0.5;
            let lx = px / density;
            let ly = py / density;
            let n1 = Noise::at(lx, ly, freq, octaves, 0.0);
            let n2 = Noise::at(lx + 311.0, ly - 97.0, freq, octaves, 0.4);
            let sx = px + (n1 - 0.5) * scale * 2.0 * density;
            let sy = py + (n2 - 0.5) * scale * 2.0 * density;
            out[y * width + x] = sample_plane(plane, width, height, sx, sy);
        }
    }
    out
}

fn sample_plane(plane: &[f32], width: usize, height: usize, x: f32, y: f32) -> f32 {
    if x < 0.0 || y < 0.0 || x >= width as f32 || y >= height as f32 {
        return 0.0;
    }
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let a = plane[y0 * width + x0] + (plane[y0 * width + x1] - plane[y0 * width + x0]) * fx;
    let b = plane[y1 * width + x0] + (plane[y1 * width + x1] - plane[y1 * width + x0]) * fx;
    a + (b - a) * fy
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(stretch: Option<Stretch>) -> RasterItem {
        RasterItem {
            bounds: ItemBox::new(22.0, 22.0, 20.0, 20.0),
            content: ItemBox::new(22.0, 22.0, 20.0, 20.0),
            radii: CornerRadii::uniform(10.0),
            image: None,
            content_blur: 0.0,
            bend: None,
            stretch,
            tail: None,
            dissolve: None,
            melt: None,
            silhouette: true,
        }
    }

    /// Horizontal extent of the painted silhouette, in raster pixels.
    fn extent(raster: &LiquidRaster) -> (usize, usize) {
        let mut min = usize::MAX;
        let mut max = 0;
        for (index, pixel) in raster.behind.chunks_exact(4).enumerate() {
            if pixel[3] > 16 {
                let x = index % raster.width as usize;
                min = min.min(x);
                max = max.max(x + 1);
            }
        }
        (min, max)
    }

    #[test]
    fn empty_group_paints_nothing() {
        let raster = render(Size::new(64.0, 64.0), 1.0, &GroupStyle::default(), &[]);
        assert!(raster.behind.iter().all(|byte| *byte == 0));
    }

    #[test]
    fn stretch_widens_the_body_along_its_motion_axis() {
        let style = GroupStyle {
            // No blur: this test measures geometry, not the goo filter.
            blur: 0.0,
            ..GroupStyle::default()
        };
        let plain = render(Size::new(64.0, 64.0), 1.0, &style, &[item(None)]);
        let (plain_min, plain_max) = extent(&plain);
        let stretched = render(
            Size::new(64.0, 64.0),
            1.0,
            &style,
            &[item(Some(Stretch {
                angle: 0.0,
                along: 1.5,
                across: 1.0,
            }))],
        );
        let (stretch_min, stretch_max) = extent(&stretched);
        let plain_width = plain_max - plain_min;
        let stretch_width = stretch_max - stretch_min;
        assert!(
            stretch_width >= plain_width + 8,
            "plain {plain_width} stretch {stretch_width}"
        );
        // The centre stays put, so both sides grow symmetrically.
        assert!(stretch_min + 4 <= plain_min, "{stretch_min} vs {plain_min}");
        assert!(stretch_max >= plain_max + 4, "{stretch_max} vs {plain_max}");
    }

    #[test]
    fn bend_lowers_the_top_edge_midpoint() {
        let style = GroupStyle {
            blur: 0.0,
            ..GroupStyle::default()
        };
        // A circle has no straight top edge to bow, so use a wide pill: the
        // bow only exists where the edge has length.
        let mut bowed = item(None);
        bowed.bounds = ItemBox::new(12.0, 22.0, 40.0, 20.0);
        bowed.content = bowed.bounds;
        bowed.radii = CornerRadii::uniform(8.0);
        bowed.bend = Some((6.0, 0.0));
        let raster = render(Size::new(64.0, 64.0), 1.0, &style, &[bowed]);
        let column = raster.width as usize / 2;
        let first = (0..raster.height as usize)
            .find(|y| raster.behind[(y * raster.width as usize + column) * 4 + 3] > 16)
            .expect("silhouette painted");
        // Quadratic with control 2b at the midpoint: sag = b = 6 px.
        assert!((27..31).contains(&first), "top edge at {first}");
    }
}
