use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};
use iced::advanced::graphics::text::Paragraph as GraphicsParagraph;
use iced::advanced::text::{
    self as text_layout, Alignment, LineHeight, Paragraph as TextParagraph, Shaping, Wrapping,
};
use iced::alignment::Vertical;
use iced::widget::canvas;
use iced::{Color, Element, Font, Length, Pixels, Point, Rectangle, Renderer, Size};
use std::time::Duration;

const AURORA_LINE_HEIGHT: LineHeight = LineHeight::Relative(1.0);
const AURORA_GRADIENT_SIZE: f32 = 2.0;

#[derive(Clone, Debug, PartialEq)]
pub struct AuroraTextProps {
    pub colors: Vec<Color>,
    pub speed: f32,
    pub font_size: Option<u16>,
    pub prefix: Option<&'static str>,
    pub text_color: Option<Color>,
}

impl Default for AuroraTextProps {
    fn default() -> Self {
        Self {
            colors: vec![
                Color::from_rgb8(0xFF, 0x00, 0x80),
                Color::from_rgb8(0x79, 0x28, 0xCA),
                Color::from_rgb8(0x00, 0x70, 0xF3),
                Color::from_rgb8(0x38, 0xBD, 0xF8),
            ],
            speed: 1.0,
            font_size: None,
            prefix: None,
            text_color: None,
        }
    }
}

impl AuroraTextProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn prefix(mut self, prefix: &'static str) -> Self {
        self.prefix = Some(prefix);
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    pub fn font_size(mut self, size: u16) -> Self {
        self.font_size = Some(size);
        self
    }
}

pub fn aurora_text<'a, Message: 'a>(
    content: &'a str,
    elapsed: Duration,
    props: &AuroraTextProps,
) -> Element<'a, Message> {
    let font_size = props.font_size.unwrap_or(18).max(1) as f32;
    let prefix = props.prefix.unwrap_or("");
    let layout = measure_aurora_layout(prefix, content, font_size);
    let text_color = props
        .text_color
        .unwrap_or(Color::from_rgb8(0x17, 0x17, 0x17));

    effect_canvas::<_, Message>(
        AuroraTextProgram {
            prefix: prefix.to_owned(),
            content: content.to_owned(),
            font_size,
            elapsed,
            colors: props.colors.clone(),
            speed: props.speed,
            text_color,
            text_size: layout.content_size,
            prefix_width: layout.prefix_width,
            y_offset: layout.y_offset,
        },
        EffectCanvasProps::new()
            .width(Length::Fixed(layout.width.ceil().max(1.0)))
            .height(Length::Fixed(layout.height.ceil().max(font_size))),
    )
    .into()
}

struct AuroraLayoutMetrics {
    width: f32,
    height: f32,
    y_offset: f32,
    prefix_width: f32,
    content_size: Size,
}

struct AuroraTextProgram {
    prefix: String,
    content: String,
    font_size: f32,
    elapsed: Duration,
    colors: Vec<Color>,
    speed: f32,
    text_color: Color,
    text_size: Size,
    prefix_width: f32,
    y_offset: f32,
}

impl<Message> EffectProgram<Message> for AuroraTextProgram {
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
        let origin = Point::new(0.0, self.y_offset);

        if !self.prefix.is_empty() {
            let prefix_text = aurora_canvas_text(&self.prefix, origin, self.font_size);
            prefix_text.draw_with(|path, _| {
                frame.fill(&path, self.text_color);
            });
        }

        let content_origin = Point::new(self.prefix_width, self.y_offset);
        let canvas_text = aurora_canvas_text(&self.content, content_origin, self.font_size);
        let mut glyph_paths = Vec::new();

        canvas_text.draw_with(|path, _| glyph_paths.push(path));

        if glyph_paths.is_empty() {
            return vec![frame.into_geometry()];
        }

        let stops = aurora_stops(&self.colors);
        let phase = aurora_phase(self.elapsed, self.speed);
        let text_width = self.text_size.width.max(1.0);
        let text_height = self.text_size.height.max(1.0);
        let column_count = text_width.ceil() as i32;

        for column in 0..column_count {
            let x = column as f32 + 0.5;
            let color = aurora_color_at(
                x,
                text_height * 0.5,
                text_width,
                text_height,
                phase,
                &stops,
            );

            let strip = Rectangle::new(
                Point::new(self.prefix_width + x - 0.5, 0.0),
                Size::new(1.0, bounds.height),
            );

            frame.with_clip(strip, |frame| {
                for path in &glyph_paths {
                    frame.fill(path, color);
                }
            });
        }

        vec![frame.into_geometry()]
    }
}

fn aurora_canvas_text(content: &str, position: Point, font_size: f32) -> canvas::Text {
    canvas::Text {
        content: content.to_owned(),
        position,
        max_width: f32::INFINITY,
        color: Color::TRANSPARENT,
        size: Pixels(font_size),
        line_height: AURORA_LINE_HEIGHT,
        font: Font::DEFAULT,
        align_x: Alignment::Left,
        align_y: Vertical::Top,
        ..canvas::Text::default()
    }
}

fn measure_text(content: &str, font_size: f32) -> Size {
    if content.is_empty() {
        return Size::ZERO;
    }

    aurora_paragraph(content, font_size).min_bounds()
}

fn aurora_paragraph(content: &str, font_size: f32) -> GraphicsParagraph {
    GraphicsParagraph::with_text(text_layout::Text {
        content,
        bounds: Size::new(f32::INFINITY, f32::INFINITY),
        size: Pixels(font_size),
        line_height: AURORA_LINE_HEIGHT,
        font: Font::DEFAULT,
        align_x: Alignment::Left,
        align_y: Vertical::Top,
        shaping: Shaping::Advanced,
        wrapping: Wrapping::None,
    })
}

fn glyph_extent(content: &str, font_size: f32) -> (f32, f32) {
    let paragraph = aurora_paragraph(content, font_size);
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for run in paragraph.buffer().layout_runs() {
        for glyph in run.glyphs.iter() {
            let top = run.line_y + glyph.y_offset;
            let bottom = top + glyph.line_height_opt.unwrap_or(run.line_height);
            min_y = min_y.min(top);
            max_y = max_y.max(bottom);
        }
    }

    if min_y.is_infinite() {
        (0.0, font_size)
    } else {
        (min_y, max_y)
    }
}

fn measure_aurora_layout(prefix: &str, content: &str, font_size: f32) -> AuroraLayoutMetrics {
    let prefix_size = measure_text(prefix, font_size);
    let content_size = measure_text(content, font_size);
    let mut min_y = 0.0_f32;
    let mut max_y = font_size;

    if !prefix.is_empty() {
        let (top, bottom) = glyph_extent(prefix, font_size);
        min_y = min_y.min(top);
        max_y = max_y.max(bottom);
    }

    if !content.is_empty() {
        let (top, bottom) = glyph_extent(content, font_size);
        min_y = min_y.min(top);
        max_y = max_y.max(bottom);
    }

    AuroraLayoutMetrics {
        width: prefix_size.width + content_size.width,
        height: (max_y - min_y).max(1.0),
        y_offset: -min_y,
        prefix_width: prefix_size.width,
        content_size,
    }
}

fn aurora_stops(colors: &[Color]) -> Vec<(f32, Color)> {
    let fallback = [
        Color::from_rgb8(0xFF, 0x00, 0x80),
        Color::from_rgb8(0x79, 0x28, 0xCA),
        Color::from_rgb8(0x00, 0x70, 0xF3),
        Color::from_rgb8(0x38, 0xBD, 0xF8),
    ];
    let colors = if colors.len() >= 2 {
        colors
    } else {
        &fallback[..]
    };

    let mut stops = Vec::with_capacity(colors.len() + 1);
    let last = colors.len().max(1) as f32;

    for (index, color) in colors.iter().enumerate() {
        stops.push((index as f32 / last, *color));
    }

    if let Some(first) = colors.first() {
        stops.push((1.0, *first));
    }

    stops
}

/// Matches `linear-gradient(135deg, ...)` with `background-size: 200% auto`.
fn aurora_color_at(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    phase: f32,
    stops: &[(f32, Color)],
) -> Color {
    let nx = (x / width.max(1.0)).clamp(0.0, 1.0);
    let ny = (y / height.max(1.0)).clamp(0.0, 1.0);
    let diagonal = (nx + ny) * 0.5;
    let gradient_t = (diagonal * AURORA_GRADIENT_SIZE + phase).rem_euclid(1.0);

    sample_stops(gradient_t, stops)
}

/// Sweep phase for the aurora animation (`10s / speed`, ease-in-out, alternate).
fn aurora_phase(elapsed: Duration, speed: f32) -> f32 {
    let duration = 10.0 / speed.max(0.01);
    let cycle = elapsed.as_secs_f32() / duration;
    let ping_pong = if cycle.rem_euclid(2.0) > 1.0 {
        2.0 - cycle.rem_euclid(2.0)
    } else {
        cycle.rem_euclid(2.0)
    };

    ease_in_out(ping_pong.clamp(0.0, 1.0))
}

fn ease_in_out(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

fn sample_stops(progress: f32, stops: &[(f32, Color)]) -> Color {
    let progress = progress.clamp(0.0, 1.0);

    for window in stops.windows(2) {
        let (start_offset, start_color) = window[0];
        let (end_offset, end_color) = window[1];
        if progress <= end_offset {
            let local = ((progress - start_offset) / (end_offset - start_offset).max(f32::EPSILON))
                .clamp(0.0, 1.0);
            return mix_color(start_color, end_color, local);
        }
    }

    stops
        .last()
        .map(|(_, color)| *color)
        .unwrap_or(Color::WHITE)
}

fn mix_color(from: Color, to: Color, progress: f32) -> Color {
    let progress = progress.clamp(0.0, 1.0);

    Color {
        r: from.r + (to.r - from.r) * progress,
        g: from.g + (to.g - from.g) * progress,
        b: from.b + (to.b - from.b) * progress,
        a: from.a + (to.a - from.a) * progress,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aurora_defaults_match_docs() {
        let props = AuroraTextProps::new();

        assert_eq!(props.colors.len(), 4);
        assert_eq!(props.speed, 1.0);
    }

    #[test]
    fn aurora_stops_loop_first_color_at_end() {
        let stops = aurora_stops(&AuroraTextProps::new().colors);

        assert_eq!(stops.first().map(|(_, color)| color), stops.last().map(|(_, color)| color));
        assert!(stops.len() >= 5);
    }

    #[test]
    fn aurora_color_changes_smoothly_across_columns() {
        let stops = aurora_stops(&AuroraTextProps::new().colors);
        let width = 240.0;
        let height = 72.0;
        let phase = aurora_phase(Duration::ZERO, 1.0);
        let left = aurora_color_at(10.0, height / 2.0, width, height, phase, &stops);
        let mid = aurora_color_at(120.0, height / 2.0, width, height, phase, &stops);
        let right = aurora_color_at(230.0, height / 2.0, width, height, phase, &stops);

        assert_ne!(left, mid);
        assert_ne!(mid, right);

        let step = aurora_color_at(50.0, height / 2.0, width, height, phase, &stops);
        let next = aurora_color_at(51.0, height / 2.0, width, height, phase, &stops);
        let channel_delta = (step.r - next.r).abs() + (step.g - next.g).abs() + (step.b - next.b).abs();

        assert!(
            channel_delta < 0.2,
            "adjacent columns should blend smoothly (delta {channel_delta})"
        );
    }

    #[test]
    fn aurora_phase_advances_during_entire_forward_cycle() {
        let duration = 10.0;
        let start = aurora_phase(Duration::ZERO, 1.0);
        let early = aurora_phase(Duration::from_secs_f32(duration * 0.12), 1.0);
        let mid = aurora_phase(Duration::from_secs_f32(duration * 0.5), 1.0);
        let late = aurora_phase(Duration::from_secs_f32(duration * 0.88), 1.0);

        assert!((early - start).abs() > 0.02, "phase should move in the first quarter");
        assert!((mid - early).abs() > 0.02, "phase should move in the middle");
        assert!((late - mid).abs() > 0.02, "phase should move in the last quarter");
    }

    #[test]
    fn aurora_phase_alternates() {
        let duration = 10.0;
        let forward_end = aurora_phase(Duration::from_secs_f32(duration), 1.0);
        let reverse = aurora_phase(Duration::from_secs_f32(duration * 1.25), 1.0);

        assert!((forward_end - 1.0).abs() < f32::EPSILON);
        assert!(reverse < forward_end);
    }
}
