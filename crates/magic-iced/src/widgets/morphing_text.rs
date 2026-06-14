use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};
use iced::advanced::graphics::text::Paragraph as GraphicsParagraph;
use iced::advanced::text::{
    self as text_layout, Alignment, LineHeight, Paragraph as TextParagraph, Shaping, Wrapping,
};
use iced::alignment::Vertical;
use iced::widget::{canvas, container, image as iced_image};
use iced::{Color, Element, Font, Length, Pixels, Point, Rectangle, Renderer, Size};
use iced_shadcn::Theme;
use ::image::{Rgba, RgbaImage};
use lyon_path::PathEvent;
use resvg::tiny_skia::{self, FillRule, Pixmap, Transform};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

const MORPH_LINE_HEIGHT: LineHeight = LineHeight::Relative(1.0);
/// Magic UI `feColorMatrix` alpha row offset (`0 0 0 255 -140`).
const THRESHOLD_BIAS: f32 = 140.0;
/// Padding so per-layer blur does not clip at the edges.
const BLUR_PADDING: f32 = 40.0;
/// Magic UI container filter: `blur(0.6px)`.
const CONTAINER_BLUR_RADIUS: f32 = 0.6;
const MORPH_FONT: Font = Font {
    weight: iced::font::Weight::Bold,
    ..Font::DEFAULT
};

static MASK_CACHE: LazyLock<Mutex<HashMap<u64, Arc<Vec<RgbaImage>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone, Debug, PartialEq)]
pub struct MorphingTextProps {
    pub texts: Vec<String>,
    pub width: Length,
    pub height: Length,
    pub font_size: u16,
    pub morph_time_secs: f32,
    pub cooldown_time_secs: f32,
    pub color: Option<Color>,
}

impl Default for MorphingTextProps {
    fn default() -> Self {
        Self {
            texts: Vec::new(),
            width: Length::Fill,
            height: Length::Fixed(112.0),
            font_size: 64,
            morph_time_secs: 1.5,
            cooldown_time_secs: 0.5,
            color: None,
        }
    }
}

impl MorphingTextProps {
    pub fn new<I, S>(texts: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            texts: texts.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }
}

pub fn morphing_text<'a, Message: 'a>(
    elapsed: Duration,
    props: &MorphingTextProps,
    theme: &Theme,
) -> Element<'a, Message> {
    if props.texts.is_empty() {
        return container(iced::widget::Space::new())
            .width(props.width)
            .height(props.height)
            .into();
    }

    let (index, next, fraction, in_cooldown) = morph_state(elapsed, props);
    let color = props.color.unwrap_or(theme.palette.foreground);
    let font_size = props.font_size.max(1) as f32;
    let layout = measure_layout(&props.texts, font_size);
    let canvas_w = (layout.width + BLUR_PADDING * 2.0).ceil().max(1.0);
    let canvas_h = (layout.height + BLUR_PADDING * 2.0).ceil().max(1.0);
    let buffer = buffer_dimensions(canvas_w, canvas_h);
    let masks = masks_for_texts(&props.texts, font_size, buffer);

    let canvas = effect_canvas::<_, Message>(
        MorphingTextProgram {
            masks,
            index,
            next,
            fraction,
            in_cooldown,
            color,
            buffer_w: buffer.0,
            buffer_h: buffer.1,
        },
        EffectCanvasProps::new()
            .width(Length::Fixed(canvas_w))
            .height(Length::Fixed(canvas_h)),
    );

    container(canvas)
        .width(props.width)
        .height(props.height)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

struct MorphingTextProgram {
    masks: Arc<Vec<RgbaImage>>,
    index: usize,
    next: usize,
    fraction: f32,
    in_cooldown: bool,
    color: Color,
    buffer_w: u32,
    buffer_h: u32,
}

impl<Message> EffectProgram<Message> for MorphingTextProgram {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let image = render_frame(self);
        let handle =
            iced_image::Handle::from_rgba(self.buffer_w, self.buffer_h, image.into_raw());
        frame.draw_image(Rectangle::with_size(bounds.size()), &handle);
        vec![frame.into_geometry()]
    }
}

/// Mirrors Magic UI `setStyles` + container `filter: url(#threshold) blur(0.6px)`.
///
/// CSS applies filters left-to-right: threshold first, then `blur(0.6px)`.
/// `feColorMatrix` only changes alpha (`A' = A - 140`), RGB stays intact until tint.
fn render_frame(program: &MorphingTextProgram) -> RgbaImage {
    let len = (program.buffer_w * program.buffer_h) as usize;
    let mut composite_alpha = vec![0u8; len];

    if program.in_cooldown {
        if let Some(mask) = program.masks.get(program.index) {
            add_alpha_layer(&mut composite_alpha, mask, 1.0, 0.0);
        }
    } else {
        let outgoing_opacity = morph_opacity(1.0 - program.fraction);
        let incoming_opacity = morph_opacity(program.fraction);
        let outgoing_blur = morph_blur_radius(1.0 - program.fraction);
        let incoming_blur = morph_blur_radius(program.fraction);

        if let Some(mask) = program.masks.get(program.index) {
            add_alpha_layer(&mut composite_alpha, mask, outgoing_opacity, outgoing_blur);
        }
        if let Some(mask) = program.masks.get(program.next) {
            add_alpha_layer(&mut composite_alpha, mask, incoming_opacity, incoming_blur);
        }
    }

    // `filter: url(#threshold) blur(0.6px)` — threshold before container blur.
    apply_threshold_alpha(&mut composite_alpha);
    apply_container_blur_alpha(&mut composite_alpha, program.buffer_w, program.buffer_h);
    alpha_to_tinted_rgba(
        &composite_alpha,
        program.buffer_w,
        program.buffer_h,
        program.color,
    )
}

fn masks_for_texts(
    texts: &[String],
    font_size: f32,
    buffer: (u32, u32),
) -> Arc<Vec<RgbaImage>> {
    let key = texts_cache_key(texts, font_size, buffer);
    if let Some(cached) = MASK_CACHE.lock().expect("mask cache").get(&key) {
        return Arc::clone(cached);
    }

    let (buffer_w, buffer_h) = buffer;
    let center = Point::new(buffer_w as f32 * 0.5, buffer_h as f32 * 0.5);
    let masks = texts
        .iter()
        .map(|text| rasterize_text_mask(text, center, font_size, buffer_w, buffer_h))
        .collect();

    let arc = Arc::new(masks);
    MASK_CACHE
        .lock()
        .expect("mask cache")
        .insert(key, Arc::clone(&arc));
    arc
}

fn texts_cache_key(texts: &[String], font_size: f32, buffer: (u32, u32)) -> u64 {
    let mut hasher = DefaultHasher::new();
    for text in texts {
        text.hash(&mut hasher);
    }
    font_size.to_bits().hash(&mut hasher);
    buffer.0.hash(&mut hasher);
    buffer.1.hash(&mut hasher);
    hasher.finish()
}

fn buffer_dimensions(canvas_w: f32, canvas_h: f32) -> (u32, u32) {
    (canvas_w.ceil().max(1.0) as u32, canvas_h.ceil().max(1.0) as u32)
}

fn rasterize_text_mask(
    content: &str,
    center: Point,
    font_size: f32,
    buffer_w: u32,
    buffer_h: u32,
) -> RgbaImage {
    let mut pixmap =
        Pixmap::new(buffer_w, buffer_h).unwrap_or_else(|| Pixmap::new(1, 1).unwrap());
    let paths = collect_text_paths(content, center, font_size);

    let mut paint = tiny_skia::Paint::default();
    paint.set_color(tiny_skia::Color::WHITE);
    paint.anti_alias = true;

    for path in paths {
        let Some(sk_path) = skia_path_from_canvas(&path) else {
            continue;
        };
        pixmap.fill_path(&sk_path, &paint, FillRule::Winding, Transform::identity(), None);
    }

    let mut image = pixmap_to_straight_rgba(&pixmap);
    for pixel in image.pixels_mut() {
        let [r, g, b, a] = pixel.0;
        if a > 0 {
            pixel.0 = [255, 255, 255, a.max(r.max(g).max(b))];
        }
    }
    image
}

fn add_alpha_layer(composite: &mut [u8], mask: &RgbaImage, opacity: f32, blur_px: f32) {
    if opacity <= f32::EPSILON {
        return;
    }

    let mut layer: Vec<u8> = mask
        .pixels()
        .map(|pixel| (pixel.0[3] as f32 * opacity).round().clamp(0.0, 255.0) as u8)
        .collect();

    if blur_px > 0.5 {
        let radius = blur_px.round().clamp(1.0, 100.0) as i32;
        blur_alpha_in_place(&mut layer, mask.width(), mask.height(), radius);
    }

    composite_alpha_over(composite, &layer);
}

fn composite_alpha_over(base: &mut [u8], layer: &[u8]) {
    for (dst, src) in base.iter_mut().zip(layer.iter()) {
        if *src == 0 {
            continue;
        }
        let src_a = *src as u16;
        let dst_a = *dst as u16;
        *dst = (src_a + dst_a * (255 - src_a) / 255).min(255) as u8;
    }
}

/// Magic UI `feColorMatrix` alpha row: `A' = clamp(255 * A - 140, 0, 1)`.
/// Browsers treat any positive value as fully opaque (gooey threshold).
fn apply_threshold_alpha(alpha: &mut [u8]) {
    for value in alpha.iter_mut() {
        let filtered = 255.0 * (*value as f32 / 255.0) - THRESHOLD_BIAS;
        *value = if filtered > 0.0 { 255 } else { 0 };
    }
}

fn apply_container_blur_alpha(alpha: &mut [u8], width: u32, height: u32) {
    if CONTAINER_BLUR_RADIUS <= f32::EPSILON {
        return;
    }

    if CONTAINER_BLUR_RADIUS < 1.0 {
        let mut blurred = alpha.to_vec();
        blur_alpha_in_place(&mut blurred, width, height, 1);
        for (sharp, soft) in alpha.iter_mut().zip(blurred.iter()) {
            let blended =
                (*sharp as f32 * (1.0 - CONTAINER_BLUR_RADIUS) + *soft as f32 * CONTAINER_BLUR_RADIUS)
                    .round()
                    .clamp(0.0, 255.0) as u8;
            *sharp = blended;
        }
        return;
    }

    let radius = CONTAINER_BLUR_RADIUS.round().clamp(1.0, 100.0) as i32;
    blur_alpha_in_place(alpha, width, height, radius);
}

fn alpha_to_tinted_rgba(alpha: &[u8], width: u32, height: u32, color: Color) -> RgbaImage {
    let r = (color.r * 255.0).round().clamp(0.0, 255.0) as u8;
    let g = (color.g * 255.0).round().clamp(0.0, 255.0) as u8;
    let b = (color.b * 255.0).round().clamp(0.0, 255.0) as u8;
    let mut image = RgbaImage::new(width, height);

    for (pixel, value) in image.pixels_mut().zip(alpha.iter()) {
        let a = (*value as f32 * color.a).round().clamp(0.0, 255.0) as u8;
        if a == 0 {
            *pixel = Rgba([0, 0, 0, 0]);
        } else {
            pixel.0 = [r, g, b, a];
        }
    }

    image
}

fn blur_alpha_in_place(alpha: &mut [u8], width: u32, height: u32, radius: i32) {
    if radius <= 0 || width == 0 || height == 0 {
        return;
    }

    let mut scratch = alpha.to_vec();
    for _ in 0..3 {
        box_blur_alpha_pass(&scratch, alpha, width, height, radius, true);
        box_blur_alpha_pass(alpha, &mut scratch, width, height, radius, false);
    }
}

fn box_blur_alpha_pass(
    source: &[u8],
    dest: &mut [u8],
    width: u32,
    height: u32,
    radius: i32,
    horizontal: bool,
) {
    let index = |x: u32, y: u32| (y * width + x) as usize;

    if horizontal {
        for y in 0..height {
            for x in 0..width {
                dest[index(x, y)] = average_alpha_window(source, width, height, x, y, radius, true);
            }
        }
    } else {
        for y in 0..height {
            for x in 0..width {
                dest[index(x, y)] = average_alpha_window(source, width, height, x, y, radius, false);
            }
        }
    }
}

fn average_alpha_window(
    source: &[u8],
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    radius: i32,
    horizontal: bool,
) -> u8 {
    let index = |x: u32, y: u32| (y * width + x) as usize;
    let mut sum = 0.0_f32;
    let mut count = 0.0_f32;

    if horizontal {
        let start = (x as i32 - radius).max(0) as u32;
        let end = (x as i32 + radius).min(width as i32 - 1) as u32;
        for sample_x in start..=end {
            sum += source[index(sample_x, y)] as f32;
            count += 1.0;
        }
    } else {
        let start = (y as i32 - radius).max(0) as u32;
        let end = (y as i32 + radius).min(height as i32 - 1) as u32;
        for sample_y in start..=end {
            sum += source[index(x, sample_y)] as f32;
            count += 1.0;
        }
    }

    if count <= 0.0 {
        return 0;
    }

    (sum / count).round().clamp(0.0, 255.0) as u8
}

/// Magic UI timing: morph first (`morphTime = 1.5`), then cooldown (`cooldownTime = 0.5`).
fn morph_state(elapsed: Duration, props: &MorphingTextProps) -> (usize, usize, f32, bool) {
    let morph_time = props.morph_time_secs.max(0.05);
    let cooldown = props.cooldown_time_secs.max(0.0);
    let cycle = morph_time + cooldown;
    let t = elapsed.as_secs_f32();
    let phase = t.rem_euclid(cycle);
    let cycle_num = (t / cycle).floor() as usize;
    let len = props.texts.len().max(1);

    if phase >= morph_time {
        let index = (cycle_num + 1) % len;
        let next = (index + 1) % len;
        return (index, next, 0.0, true);
    }

    let index = cycle_num % len;
    let next = (index + 1) % len;
    let fraction = (phase / morph_time).clamp(0.0, 1.0);
    (index, next, fraction, false)
}

/// `blur(${Math.min(8 / fraction - 8, 100)}px)` — JS `8/0` becomes `Infinity`, capped at 100.
fn morph_blur_radius(fraction: f32) -> f32 {
    if fraction <= f32::EPSILON {
        return 100.0;
    }

    (8.0 / fraction - 8.0).clamp(0.0, 100.0)
}

fn morph_opacity(fraction: f32) -> f32 {
    fraction.clamp(0.0, 1.0).powf(0.4)
}

fn measure_layout(texts: &[String], font_size: f32) -> Size {
    let mut max_width = 0.0_f32;
    let mut max_height = 0.0_f32;

    for text in texts {
        let size = morph_paragraph(text, font_size).min_bounds();
        max_width = max_width.max(size.width);
        max_height = max_height.max(size.height);
    }

    Size::new(max_width.max(1.0), max_height.max(1.0))
}

fn morph_paragraph(content: &str, font_size: f32) -> GraphicsParagraph {
    GraphicsParagraph::with_text(text_layout::Text {
        content,
        bounds: Size::new(f32::INFINITY, f32::INFINITY),
        size: Pixels(font_size),
        line_height: MORPH_LINE_HEIGHT,
        font: MORPH_FONT,
        align_x: Alignment::Left,
        align_y: Vertical::Top,
        shaping: Shaping::Advanced,
        wrapping: Wrapping::None,
    })
}

fn morph_canvas_text(content: &str, position: Point, font_size: f32) -> canvas::Text {
    canvas::Text {
        content: content.to_owned(),
        position,
        max_width: f32::INFINITY,
        color: Color::WHITE,
        size: Pixels(font_size),
        line_height: MORPH_LINE_HEIGHT,
        font: MORPH_FONT,
        align_x: Alignment::Center,
        align_y: Vertical::Center,
        ..canvas::Text::default()
    }
}

fn collect_text_paths(content: &str, position: Point, font_size: f32) -> Vec<canvas::Path> {
    let text = morph_canvas_text(content, position, font_size);
    let mut paths = Vec::new();
    text.draw_with(|path, _| paths.push(path));
    paths
}

fn skia_path_from_canvas(path: &canvas::Path) -> Option<tiny_skia::Path> {
    let mut builder = tiny_skia::PathBuilder::new();

    for event in path.raw().iter() {
        match event {
            PathEvent::Begin { at } => {
                builder.move_to(at.x, at.y);
            }
            PathEvent::Line { to, .. } => {
                builder.line_to(to.x, to.y);
            }
            PathEvent::Quadratic { ctrl, to, .. } => {
                builder.quad_to(ctrl.x, ctrl.y, to.x, to.y);
            }
            PathEvent::Cubic { ctrl1, ctrl2, to, .. } => {
                builder.cubic_to(ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y, to.x, to.y);
            }
            PathEvent::End { close, .. } => {
                if close {
                    builder.close();
                }
            }
        }
    }

    builder.finish()
}

fn pixmap_to_straight_rgba(pixmap: &Pixmap) -> RgbaImage {
    let mut image = RgbaImage::from_raw(pixmap.width(), pixmap.height(), pixmap.data().to_vec())
        .unwrap_or_else(|| RgbaImage::new(pixmap.width(), pixmap.height()));
    unpremultiply_in_place(&mut image);
    image
}

fn unpremultiply_in_place(image: &mut RgbaImage) {
    for pixel in image.pixels_mut() {
        let [r, g, b, a] = pixel.0;
        if a == 0 {
            continue;
        }

        let alpha = a as f32 / 255.0;
        pixel.0 = [
            ((r as f32 / alpha).min(255.0)).round() as u8,
            ((g as f32 / alpha).min(255.0)).round() as u8,
            ((b as f32 / alpha).min(255.0)).round() as u8,
            a,
        ];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn morphing_text_defaults_match_docs() {
        let props = MorphingTextProps::new(["Hello", "World"]);

        assert_eq!(props.font_size, 64);
        assert_eq!(props.morph_time_secs, 1.5);
        assert_eq!(props.cooldown_time_secs, 0.5);
        assert_eq!(props.texts.len(), 2);
    }

    #[test]
    fn morph_blur_matches_reference_curve() {
        assert_eq!(morph_blur_radius(1.0), 0.0);
        assert_eq!(morph_blur_radius(0.5), 8.0);
        assert_eq!(morph_blur_radius(0.0), 100.0);
    }

    #[test]
    fn morph_opacity_uses_reference_exponent() {
        assert!((morph_opacity(0.5) - 0.5_f32.powf(0.4)).abs() < f32::EPSILON);
    }

    #[test]
    fn morphing_text_paths_are_non_empty() {
        let paths = collect_text_paths("Hello", Point::new(100.0, 50.0), 64.0);
        assert!(!paths.is_empty());
    }

    #[test]
    fn morph_state_starts_morph_immediately() {
        let props = MorphingTextProps::new(["A", "B"]);
        let (_, _, fraction, in_cooldown) = morph_state(Duration::from_millis(0), &props);
        assert_eq!(fraction, 0.0);
        assert!(!in_cooldown);

        let (_, _, fraction, in_cooldown) = morph_state(Duration::from_millis(100), &props);
        assert!(fraction > 0.0);
        assert!(!in_cooldown);
    }

    #[test]
    fn morph_state_enters_cooldown_after_morph() {
        let props = MorphingTextProps::new(["A", "B"]);
        let (_, _, _, in_cooldown) = morph_state(Duration::from_secs_f32(1.5), &props);
        assert!(in_cooldown);
    }

    #[test]
    fn mask_cache_builds_non_empty_masks() {
        let layout = measure_layout(&["Hello".to_string()], 64.0);
        let canvas_w = layout.width + BLUR_PADDING * 2.0;
        let canvas_h = layout.height + BLUR_PADDING * 2.0;
        let buffer = buffer_dimensions(canvas_w, canvas_h);
        let masks = masks_for_texts(&["Hello".to_string()], 64.0, buffer);
        assert!(masks[0].pixels().any(|pixel| pixel.0[3] > 0));
        assert_eq!(masks[0].width(), buffer.0);
    }

    #[test]
    fn render_frame_matches_reference_pipeline() {
        let layout = measure_layout(&["Hello".to_string(), "World".to_string()], 64.0);
        let canvas_w = (layout.width + BLUR_PADDING * 2.0).ceil();
        let canvas_h = (layout.height + BLUR_PADDING * 2.0).ceil();
        let buffer = buffer_dimensions(canvas_w, canvas_h);
        let masks = masks_for_texts(
            &["Hello".to_string(), "World".to_string()],
            64.0,
            buffer,
        );
        let program = MorphingTextProgram {
            masks,
            index: 0,
            next: 1,
            fraction: 0.5,
            in_cooldown: false,
            color: Color::BLACK,
            buffer_w: buffer.0,
            buffer_h: buffer.1,
        };

        let frame = render_frame(&program);
        assert!(frame.pixels().any(|pixel| pixel.0[3] > 0));
        assert_eq!(frame.width(), buffer.0);
        assert_eq!(frame.height(), buffer.1);
    }

    #[test]
    fn cooldown_text_stays_opaque_after_threshold() {
        let layout = measure_layout(&["Hello".to_string()], 64.0);
        let canvas_w = (layout.width + BLUR_PADDING * 2.0).ceil();
        let canvas_h = (layout.height + BLUR_PADDING * 2.0).ceil();
        let buffer = buffer_dimensions(canvas_w, canvas_h);
        let masks = masks_for_texts(&["Hello".to_string()], 64.0, buffer);
        let program = MorphingTextProgram {
            masks,
            index: 0,
            next: 1,
            fraction: 0.0,
            in_cooldown: true,
            color: Color::from_rgb(0.0, 0.0, 0.0),
            buffer_w: buffer.0,
            buffer_h: buffer.1,
        };

        let frame = render_frame(&program);
        let opaque = frame.pixels().filter(|pixel| pixel.0[3] >= 200).count();
        assert!(opaque > 100, "expected crisp opaque glyphs, got {opaque} pixels");
    }

    #[test]
    fn cooldown_renders_current_word_only() {
        let layout = measure_layout(&["Hello".to_string(), "World".to_string()], 64.0);
        let canvas_w = (layout.width + BLUR_PADDING * 2.0).ceil();
        let canvas_h = (layout.height + BLUR_PADDING * 2.0).ceil();
        let buffer = buffer_dimensions(canvas_w, canvas_h);
        let masks = masks_for_texts(
            &["Hello".to_string(), "World".to_string()],
            64.0,
            buffer,
        );
        let program = MorphingTextProgram {
            masks,
            index: 1,
            next: 0,
            fraction: 0.0,
            in_cooldown: true,
            color: Color::BLACK,
            buffer_w: buffer.0,
            buffer_h: buffer.1,
        };

        let frame = render_frame(&program);
        assert!(frame.pixels().any(|pixel| pixel.0[3] > 0));
    }
}
