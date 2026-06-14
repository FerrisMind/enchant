use super::text_element::TextElementRole;
use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};
use iced::advanced::graphics::text::Paragraph as GraphicsParagraph;
use iced::advanced::text::{
    self as text_layout, Alignment, LineHeight, Paragraph as TextParagraph, Shaping, Wrapping,
};
use iced::alignment::Vertical;
use iced::font::{Style, Weight};
use iced::widget::canvas;
use iced::{Color, Element, Font, Length, Pixels, Point, Rectangle, Renderer, Size, Vector};
use iced_shadcn::Theme;
use std::cell::RefCell;
use std::time::Duration;

const LINE_HEIGHT: LineHeight = LineHeight::Relative(1.0);
/// `after:top-[0.04em] after:left-[0.04em]`
const SHADOW_OFFSET_EM: f32 = 0.04;
/// `after:bg-size-[0.06em_0.06em]`
const CELL_SIZE_EM: f32 = 0.06;
/// `transparent 45%, color 45%, color 55%, transparent`
const SHADOW_BAND_START: f32 = 0.45;
const SHADOW_BAND_END: f32 = 0.55;
const INK_PAD_EM: f32 = 0.1;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineShadowTextProps {
    pub shadow_color: Option<Color>,
    pub text_color: Option<Color>,
    pub prefix: Option<&'static str>,
    pub role: TextElementRole,
    pub font_size: Option<u16>,
    pub speed: f32,
    pub italic: bool,
    pub semibold: bool,
}

impl Default for LineShadowTextProps {
    fn default() -> Self {
        Self {
            shadow_color: None,
            text_color: None,
            prefix: None,
            role: TextElementRole::Span,
            font_size: None,
            speed: 15.0,
            italic: false,
            semibold: false,
        }
    }
}

impl LineShadowTextProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn prefix(mut self, prefix: &'static str) -> Self {
        self.prefix = Some(prefix);
        self
    }

    pub fn italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }

    pub fn semibold(mut self, semibold: bool) -> Self {
        self.semibold = semibold;
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    pub fn shadow_color(mut self, color: Color) -> Self {
        self.shadow_color = Some(color);
        self
    }
}

/// Shadow stripe color for Magic UI line-shadow-text demos (black in light, white in dark).
pub fn line_shadow_color_for_mode(dark_mode: bool) -> Color {
    if dark_mode {
        Color::WHITE
    } else {
        Color::BLACK
    }
}

pub fn line_shadow_text<'a, Message: 'a>(
    content: &'a str,
    elapsed: Duration,
    props: LineShadowTextProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let font_size = props
        .font_size
        .unwrap_or(props.role.default_font_size())
        .max(1) as f32;
    let prefix = props.prefix.unwrap_or("");
    let prefix_font = line_shadow_prefix_font(props.semibold);
    let content_font = line_shadow_content_font(props.italic, props.semibold);
    let layout = measure_line_shadow_layout(prefix, content, font_size, prefix_font, content_font);
    let text_color = props.text_color.unwrap_or(theme.palette.foreground);
    let shadow_color = props
        .shadow_color
        .unwrap_or_else(|| line_shadow_color_for_mode(theme.variant().is_dark()));

    effect_canvas::<_, Message>(
        LineShadowTextProgram {
            prefix: prefix.to_owned(),
            content: content.to_owned(),
            font_size,
            prefix_font,
            content_font,
            elapsed,
            speed: props.speed,
            text_color,
            shadow_color,
            y_offset: layout.y_offset,
            content_x: layout.content_x,
        },
        EffectCanvasProps::new()
            .width(Length::Fixed(layout.width.ceil().max(1.0)))
            .height(Length::Fixed(layout.height.ceil().max(1.0)))
            .cache_revision(line_shadow_cache_revision(elapsed)),
    )
    .into()
}

struct LineShadowTextState {
    shadow_cache_key: RefCell<String>,
    shadow_paths: RefCell<Vec<canvas::Path>>,
}

impl Default for LineShadowTextState {
    fn default() -> Self {
        Self {
            shadow_cache_key: RefCell::new(String::new()),
            shadow_paths: RefCell::new(Vec::new()),
        }
    }
}

struct LineShadowLayoutMetrics {
    width: f32,
    height: f32,
    y_offset: f32,
    content_x: f32,
}

struct LineShadowTextProgram {
    prefix: String,
    content: String,
    font_size: f32,
    prefix_font: Font,
    content_font: Font,
    elapsed: Duration,
    speed: f32,
    text_color: Color,
    shadow_color: Color,
    y_offset: f32,
    content_x: f32,
}

impl<Message> EffectProgram<Message> for LineShadowTextProgram {
    type State = LineShadowTextState;

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let origin = Point::new(0.0, self.y_offset);
        let content_origin = Point::new(self.content_x, self.y_offset);
        let shadow_offset = self.font_size * SHADOW_OFFSET_EM;
        let shadow_origin = content_origin + Vector::new(shadow_offset, shadow_offset);
        let cell = (self.font_size * CELL_SIZE_EM).max(1.0);

        if !self.prefix.is_empty() {
            line_shadow_canvas_text(&self.prefix, origin, self.font_size, self.prefix_font)
                .draw_with(|path, _| {
                    frame.fill(&path, self.text_color);
                });
        }

        if !self.content.is_empty() {
            let shadow_cache_key = shadow_paths_cache_key(
                &self.content,
                self.font_size,
                self.content_font,
                content_origin,
            );
            if *state.shadow_cache_key.borrow() != shadow_cache_key {
                // Same glyph paths as foreground; ::after shift applied when drawing.
                *state.shadow_paths.borrow_mut() = collect_canvas_text_paths(
                    &self.content,
                    content_origin,
                    self.font_size,
                    self.content_font,
                );
                *state.shadow_cache_key.borrow_mut() = shadow_cache_key;
            }

            let shadow_paths = state.shadow_paths.borrow();
            if !shadow_paths.is_empty() {
                let (cmin_x, cmin_y, cmax_x, cmax_y) =
                    glyph_layout_bounds(&self.content, self.font_size, self.content_font);
                // ::after padding box: origin at +0.04em, size matches the span content box.
                let layer_origin = shadow_origin;
                let layer_w = (cmax_x - cmin_x).max(1.0);
                let layer_h = (cmax_y - cmin_y).max(1.0);
                let stripe_bounds = Rectangle::new(
                    Point::new(layer_origin.x + cmin_x, layer_origin.y + cmin_y),
                    Size::new(layer_w + shadow_offset, layer_h + shadow_offset),
                );
                let progress = (self.elapsed.as_secs_f32() / self.speed.max(0.05)).rem_euclid(1.0);
                let (bg_x, bg_y) =
                    line_shadow_background_position(progress, layer_w, layer_h, cell);
                draw_striped_shadow_paths(
                    &mut frame,
                    &shadow_paths,
                    stripe_bounds,
                    layer_origin,
                    Vector::new(shadow_offset, shadow_offset),
                    cell,
                    bg_x,
                    bg_y,
                    self.shadow_color,
                );
            }

            line_shadow_canvas_text(
                &self.content,
                content_origin,
                self.font_size,
                self.content_font,
            )
            .draw_with(|path, _| {
                frame.fill(&path, self.text_color);
            });
        }

        vec![frame.into_geometry()]
    }
}

fn line_shadow_canvas_text(
    content: &str,
    position: Point,
    font_size: f32,
    font: Font,
) -> canvas::Text {
    canvas::Text {
        content: content.to_owned(),
        position,
        max_width: f32::INFINITY,
        color: Color::TRANSPARENT,
        size: Pixels(font_size),
        line_height: LINE_HEIGHT,
        font,
        align_x: Alignment::Left,
        align_y: Vertical::Top,
        ..canvas::Text::default()
    }
}

/// Paint shadow glyphs where the tiled `linear-gradient(45deg, …)` is opaque.
#[allow(clippy::too_many_arguments)]
fn draw_striped_shadow_paths(
    frame: &mut canvas::Frame,
    paths: &[canvas::Path],
    bounds: Rectangle,
    layer_origin: Point,
    shadow_offset: Vector,
    cell: f32,
    bg_x: f32,
    bg_y: f32,
    color: Color,
) {
    let x_start = bounds.x.floor() as i32;
    let y_start = bounds.y.floor().max(0.0) as i32;
    let x_end = (bounds.x + bounds.width).ceil() as i32;
    let y_end = (bounds.y + bounds.height).ceil() as i32;

    for y in y_start..y_end {
        let mut x = x_start;
        while x < x_end {
            let layer_x = x as f32 + 0.5 - layer_origin.x;
            let layer_y = y as f32 + 0.5 - layer_origin.y;
            if !line_shadow_sample_visible(layer_x, layer_y, bg_x, bg_y, cell) {
                x += 1;
                continue;
            }

            let run_start = x;
            x += 1;
            while x < x_end {
                let lx = x as f32 + 0.5 - layer_origin.x;
                let ly = y as f32 + 0.5 - layer_origin.y;
                if !line_shadow_sample_visible(lx, ly, bg_x, bg_y, cell) {
                    break;
                }
                x += 1;
            }

            let run = Rectangle::new(
                Point::new(run_start as f32, y as f32),
                Size::new((x - run_start) as f32, 1.0),
            );
            frame.with_clip(run, |frame| {
                frame.translate(shadow_offset);
                for path in paths {
                    frame.fill(path, color);
                }
            });
        }
    }
}

fn shadow_paths_cache_key(content: &str, font_size: f32, font: Font, origin: Point) -> String {
    format!(
        "{content}|{font_size:.3}|{:?}|{:?}|{:.3},{:.3}",
        font.weight, font.style, origin.x, origin.y
    )
}

fn collect_canvas_text_paths(
    content: &str,
    position: Point,
    font_size: f32,
    font: Font,
) -> Vec<canvas::Path> {
    let mut paths = Vec::new();
    line_shadow_canvas_text(content, position, font_size, font)
        .draw_with(|path, _| paths.push(path));
    paths
}

fn line_shadow_prefix_font(semibold: bool) -> Font {
    Font {
        weight: if semibold {
            Weight::Semibold
        } else {
            Weight::Normal
        },
        style: Style::Normal,
        ..Font::DEFAULT
    }
}

fn line_shadow_content_font(italic: bool, semibold: bool) -> Font {
    Font {
        weight: if semibold {
            Weight::Semibold
        } else {
            Weight::Normal
        },
        style: if italic { Style::Italic } else { Style::Normal },
        ..Font::DEFAULT
    }
}

fn line_shadow_paragraph(content: &str, font_size: f32, font: Font) -> GraphicsParagraph {
    GraphicsParagraph::with_text(text_layout::Text {
        content,
        bounds: Size::new(f32::INFINITY, f32::INFINITY),
        size: Pixels(font_size),
        line_height: LINE_HEIGHT,
        font,
        align_x: Alignment::Left,
        align_y: Vertical::Top,
        shaping: Shaping::Advanced,
        wrapping: Wrapping::None,
    })
}

fn glyph_layout_bounds(content: &str, font_size: f32, font: Font) -> (f32, f32, f32, f32) {
    let outline = outline_bounds(content, font_size, font);
    if let Some(bounds) = outline {
        return bounds;
    }

    let paragraph = line_shadow_paragraph(content, font_size, font);
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for run in paragraph.buffer().layout_runs() {
        for glyph in run.glyphs.iter() {
            let left = glyph.x + glyph.x_offset;
            let right = left + glyph.w;
            let top = run.line_y + glyph.y_offset;
            let bottom = top + glyph.line_height_opt.unwrap_or(run.line_height);
            min_x = min_x.min(left);
            max_x = max_x.max(right);
            min_y = min_y.min(top);
            max_y = max_y.max(bottom);
        }
    }

    if min_x.is_infinite() {
        (0.0, 0.0, font_size, font_size)
    } else {
        (min_x, min_y, max_x, max_y)
    }
}

fn outline_bounds(content: &str, font_size: f32, font: Font) -> Option<(f32, f32, f32, f32)> {
    use iced::advanced::graphics::text::{self as gfx_text, cosmic_text};

    if content.is_empty() {
        return None;
    }

    let paragraph = line_shadow_paragraph(content, font_size, font);
    let buffer = paragraph.buffer();
    let mut swash_cache = cosmic_text::SwashCache::new();
    let mut font_system = gfx_text::font_system().write().expect("Write font system");
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    let mut found = false;

    for run in buffer.layout_runs() {
        for glyph in run.glyphs.iter() {
            let physical_glyph = glyph.physical((0.0, 0.0), 1.0);
            let offset = Vector::new(glyph.x + glyph.x_offset, glyph.y_offset + run.line_y);
            let Some(commands) =
                swash_cache.get_outline_commands(font_system.raw(), physical_glyph.cache_key)
            else {
                continue;
            };

            let (left, top, right, bottom) = outline_command_bounds(commands, offset);
            min_x = min_x.min(left);
            min_y = min_y.min(top);
            max_x = max_x.max(right);
            max_y = max_y.max(bottom);
            found = true;
        }
    }

    if found {
        Some((min_x, min_y, max_x, max_y))
    } else {
        None
    }
}

fn outline_command_bounds(
    commands: &[iced::advanced::graphics::text::cosmic_text::Command],
    offset: Vector,
) -> (f32, f32, f32, f32) {
    use iced::advanced::graphics::text::cosmic_text;

    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for command in commands {
        let points = match command {
            cosmic_text::Command::MoveTo(point) | cosmic_text::Command::LineTo(point) => {
                vec![*point]
            }
            cosmic_text::Command::CurveTo(control_a, control_b, to) => {
                vec![*control_a, *control_b, *to]
            }
            cosmic_text::Command::QuadTo(control, to) => vec![*control, *to],
            cosmic_text::Command::Close => Vec::new(),
        };

        for point in points {
            let x = point.x + offset.x;
            let y = -point.y + offset.y;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }

    (min_x, min_y, max_x, max_y)
}

/// `@keyframes line-shadow`: `background-position: 0 0` → `100% -100%`.
fn line_shadow_background_position(
    progress: f32,
    element_width: f32,
    element_height: f32,
    cell: f32,
) -> (f32, f32) {
    let align_x = progress * (element_width - cell);
    let align_y = progress * (-(element_height - cell));
    (align_x, align_y)
}

/// `linear-gradient(45deg, …)` with horizontal mirror (matches browser rendering).
fn line_shadow_gradient_visible(local_x: f32, local_y: f32, cell: f32) -> bool {
    let u = (local_x / cell).rem_euclid(1.0);
    let v = (local_y / cell).rem_euclid(1.0);
    let t = ((1.0 - u) + v) * 0.5;
    (SHADOW_BAND_START..SHADOW_BAND_END).contains(&t)
}

/// Repeating `background-size: 0.06em` field shifted by animated `background-position`.
fn line_shadow_sample_visible(layer_x: f32, layer_y: f32, bg_x: f32, bg_y: f32, cell: f32) -> bool {
    let sample_x = (layer_x - bg_x).rem_euclid(cell);
    let sample_y = (layer_y - bg_y).rem_euclid(cell);
    line_shadow_gradient_visible(sample_x, sample_y, cell)
}

fn line_shadow_cache_revision(elapsed: Duration) -> u64 {
    elapsed.as_micros() as u64
}

fn measure_line_shadow_layout(
    prefix: &str,
    content: &str,
    font_size: f32,
    prefix_font: Font,
    content_font: Font,
) -> LineShadowLayoutMetrics {
    let shadow_pad = font_size * SHADOW_OFFSET_EM;
    let ink_pad = font_size * INK_PAD_EM;

    let (prefix_min_x, prefix_min_y, prefix_max_x, prefix_max_y) =
        glyph_layout_bounds(prefix, font_size, prefix_font);
    let (_content_min_x, content_min_y, content_max_x, content_max_y) =
        glyph_layout_bounds(content, font_size, content_font);

    let prefix_width = if prefix.is_empty() {
        0.0
    } else {
        (prefix_max_x - prefix_min_x).max(0.0)
    };

    // Demo: `Ship` immediately followed by `<LineShadowText>` (no extra space node).
    let content_x = prefix_width;

    let mut min_y = 0.0_f32;
    let mut max_y = font_size;

    if !prefix.is_empty() {
        min_y = min_y.min(prefix_min_y);
        max_y = max_y.max(prefix_max_y);
    }

    if !content.is_empty() {
        min_y = min_y.min(content_min_y);
        max_y = max_y.max(content_max_y);
    }

    let foreground_right = content_x + content_max_x;
    let shadow_right = content_x + shadow_pad + content_max_x + shadow_pad;
    let width = foreground_right.max(shadow_right) + ink_pad;

    LineShadowLayoutMetrics {
        width,
        height: (max_y - min_y).max(1.0) + shadow_pad + ink_pad * 2.0,
        y_offset: -min_y + ink_pad,
        content_x,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_shadow_defaults_match_docs() {
        let props = LineShadowTextProps::new();

        assert_eq!(props.role, TextElementRole::Span);
        assert_eq!(props.speed, 15.0);
    }

    #[test]
    fn line_shadow_color_for_mode_matches_magic_ui_demo() {
        assert_eq!(line_shadow_color_for_mode(false), Color::BLACK);
        assert_eq!(line_shadow_color_for_mode(true), Color::WHITE);
    }

    #[test]
    fn css_constants_match_magic_ui_source() {
        assert_eq!(SHADOW_OFFSET_EM, 0.04);
        assert_eq!(CELL_SIZE_EM, 0.06);
        assert_eq!(SHADOW_BAND_START, 0.45);
        assert_eq!(SHADOW_BAND_END, 0.55);
    }

    #[test]
    fn gradient_band_matches_horizontally_mirrored_45deg_stops() {
        let cell = 6.0;
        assert!(line_shadow_gradient_visible(cell * 0.5, cell * 0.5, cell));
        assert!(!line_shadow_gradient_visible(cell * 0.9, cell * 0.1, cell));
    }

    #[test]
    fn background_position_matches_keyframes() {
        let cell = 4.0;
        let w = 100.0;
        let h = 50.0;
        assert_eq!(line_shadow_background_position(0.0, w, h, cell), (0.0, 0.0));
        assert_eq!(
            line_shadow_background_position(1.0, w, h, cell),
            (w - cell, -(h - cell))
        );
    }

    #[test]
    fn animated_field_moves_with_background_position() {
        let cell = 10.0;
        let layer_w = 120.0;
        let layer_h = 60.0;
        let mut saw_change = false;
        for step in 0..8 {
            let progress = step as f32 / 8.0;
            let (bg_x, bg_y) = line_shadow_background_position(progress, layer_w, layer_h, cell);
            let on = line_shadow_sample_visible(cell * 0.5, cell * 0.5, bg_x, bg_y, cell);
            let off = line_shadow_sample_visible(cell * 0.9, cell * 0.1, bg_x, bg_y, cell);
            saw_change |= on != off;
        }
        assert!(saw_change);
    }

    #[test]
    fn demo_words_are_adjacent() {
        let font_size = 72.0;
        let prefix_font = line_shadow_prefix_font(true);
        let content_font = line_shadow_content_font(true, true);
        let layout =
            measure_line_shadow_layout("Ship", "Fast", font_size, prefix_font, content_font);
        let (prefix_min_x, _, prefix_max_x, _) =
            glyph_layout_bounds("Ship", font_size, prefix_font);

        assert_eq!(layout.content_x, prefix_max_x - prefix_min_x);
    }

    #[test]
    fn fast_italic_fits_inside_layout_width() {
        let font_size = 72.0;
        let prefix_font = line_shadow_prefix_font(true);
        let content_font = line_shadow_content_font(true, true);
        let layout =
            measure_line_shadow_layout("Ship", "Fast", font_size, prefix_font, content_font);
        let (_, _, content_max_x, _) = glyph_layout_bounds("Fast", font_size, content_font);

        assert!(
            layout.content_x + content_max_x <= layout.width,
            "italic Fast right edge {} must fit in canvas width {}",
            layout.content_x + content_max_x,
            layout.width
        );
    }

    #[test]
    fn layout_keeps_glyphs_inside_canvas() {
        let font_size = 72.0;
        let prefix_font = line_shadow_prefix_font(true);
        let content_font = line_shadow_content_font(true, true);
        let layout =
            measure_line_shadow_layout("Ship", "Fast", font_size, prefix_font, content_font);

        assert!(layout.y_offset >= 0.0);
        assert!(layout.height >= font_size);
        assert!(layout.width > 100.0);
    }
}
