use crate::primitives::{AnimatedValue, EffectCanvasProps, EffectProgram, MotionSpec, effect_canvas};
use iced::advanced::graphics::text::Paragraph as GraphicsParagraph;
use iced::advanced::text::{
    self as text_layout, Alignment, LineHeight, Paragraph as TextParagraph, Shaping, Wrapping,
};
use iced::alignment::Vertical;
use iced::font::Weight;
use iced::mouse;
use iced::time::Instant;
use iced::widget::canvas;
use iced::{Color, Element, Font, Length, Pixels, Point, Rectangle, Renderer, Size, Vector};
use iced_shadcn::Theme;
use twill::prelude::{Easing, TransitionDuration};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KineticTextRole {
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    P,
    Span,
}

impl KineticTextRole {
    fn default_font_size(self) -> u16 {
        match self {
            Self::H1 => 96,
            Self::H2 => 72,
            Self::H3 => 48,
            Self::H4 => 36,
            Self::H5 => 24,
            Self::H6 => 20,
            Self::P | Self::Span => 16,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KineticTextProps {
    pub role: KineticTextRole,
    pub font_size: Option<u16>,
    pub color: Option<Color>,
}

impl Default for KineticTextProps {
    fn default() -> Self {
        Self {
            role: KineticTextRole::H1,
            font_size: None,
            color: None,
        }
    }
}

impl KineticTextProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn role(mut self, role: KineticTextRole) -> Self {
        self.role = role;
        self
    }

    pub fn font_size(mut self, font_size: u16) -> Self {
        self.font_size = Some(font_size.max(1));
        self
    }
}

/// Retained for API compatibility; hover + animation live in the canvas state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KineticTextState {
    _private: (),
}

impl KineticTextState {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(clippy::unused_self)]
    pub fn set_hovered(&mut self, _index: Option<usize>) {}
}

const KINETIC_LINE_HEIGHT: LineHeight = LineHeight::Relative(1.0);
const TRANSITION_MS: u16 = 400;

pub fn kinetic_text<'a, Message: 'a>(
    content: &'a str,
    _state: &KineticTextState,
    now: Instant,
    props: &KineticTextProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let font_size = props
        .font_size
        .unwrap_or(props.role.default_font_size())
        .max(1) as f32;
    let color = props.color.unwrap_or(theme.palette.foreground);
    let metrics = measure_kinetic_layout(content, font_size);

    effect_canvas::<_, Message>(
        KineticTextProgram {
            content: content.to_owned(),
            font_size,
            color,
            now,
            y_offset: metrics.y_offset,
        },
        EffectCanvasProps::new()
            .width(Length::Fixed(metrics.width.ceil().max(1.0)))
            .height(Length::Fixed(metrics.height.ceil().max(1.0))),
    )
    .into()
}

struct KineticTextProgram {
    content: String,
    font_size: f32,
    color: Color,
    now: Instant,
    y_offset: f32,
}

struct KineticLayoutMetrics {
    width: f32,
    height: f32,
    y_offset: f32,
}

#[derive(Clone, Debug)]
struct LetterAnimation {
    padding: AnimatedValue,
    stroke: AnimatedValue,
}

impl LetterAnimation {
    fn new() -> Self {
        let motion = kinetic_motion();
        Self {
            padding: AnimatedValue::with_spec(0.0, motion),
            stroke: AnimatedValue::with_spec(0.0, motion),
        }
    }

    fn set_targets(&mut self, padding: f32, stroke: f32, now: Instant) {
        self.padding.transition(padding, now);
        self.stroke.transition(stroke, now);
    }
}

#[derive(Clone, Debug, Default)]
struct KineticCanvasState {
    letters: Vec<LetterAnimation>,
    hovered_index: Option<usize>,
    content_key: String,
}

impl KineticCanvasState {
    fn ensure_content(&mut self, content: &str) {
        if self.content_key == content {
            return;
        }

        self.content_key = content.to_owned();
        self.letters = content.chars().map(|_| LetterAnimation::new()).collect();
        self.hovered_index = None;
    }

    fn set_hovered(&mut self, index: Option<usize>, font_size: f32, now: Instant) {
        if self.hovered_index == index {
            return;
        }

        self.hovered_index = index;
        let hover_padding = font_size / 12.0;

        for (letter_index, animation) in self.letters.iter_mut().enumerate() {
            let padding = kinetic_padding_target(letter_index, index, hover_padding);
            let stroke = if index == Some(letter_index) { 1.0 } else { 0.0 };
            animation.set_targets(padding, stroke, now);
        }
    }
}

impl<Message> EffectProgram<Message> for KineticTextProgram {
    type State = KineticCanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Option<canvas::Action<Message>> {
        state.ensure_content(&self.content);

        match event {
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let position = cursor.position_in(bounds)?;
                let next = hit_test_letter(&self.content, self.font_size, state, self.now, position.x);
                if next != state.hovered_index {
                    state.set_hovered(next, self.font_size, self.now);
                    return Some(canvas::Action::request_redraw());
                }
            }
            canvas::Event::Mouse(mouse::Event::CursorLeft) => {
                if state.hovered_index.is_some() {
                    state.set_hovered(None, self.font_size, self.now);
                    return Some(canvas::Action::request_redraw());
                }
            }
            _ => {}
        }

        if state.letters.iter().any(|letter| {
            letter.padding.is_animating(self.now) || letter.stroke.is_animating(self.now)
        }) {
            return Some(canvas::Action::request_redraw());
        }

        None
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> mouse::Interaction {
        mouse::Interaction::Pointer
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let stroke_width = self.font_size * 125.0 / 6_000.0;
        let padding = kinetic_padding_values(state, self.now, self.content.len());
        let stroke_amounts: Vec<f32> = (0..self.content.len())
            .map(|index| {
                state
                    .letters
                    .get(index)
                    .map(|animation| animation.stroke.value_at(self.now))
                    .unwrap_or(0.0)
            })
            .collect();
        let paragraph = build_layout_paragraph(&self.content, state.hovered_index, self.font_size);

        draw_layout_glyphs(
            &mut frame,
            &paragraph,
            &padding,
            &stroke_amounts,
            self.color,
            stroke_width,
            self.y_offset,
        );

        vec![frame.into_geometry()]
    }
}

fn kinetic_motion() -> MotionSpec {
    MotionSpec::new()
        .duration(TransitionDuration::CustomMs(TRANSITION_MS))
        .easing(Easing::InOut)
}

fn draw_layout_glyphs(
    frame: &mut canvas::Frame,
    paragraph: &GraphicsParagraph,
    padding: &[f32],
    stroke_amounts: &[f32],
    color: Color,
    stroke_width: f32,
    y_offset: f32,
) {
    use iced::advanced::graphics::text::{self as gfx_text, cosmic_text};

    let buffer = paragraph.buffer();
    let mut swash_cache = cosmic_text::SwashCache::new();
    let mut font_system = gfx_text::font_system()
        .write()
        .expect("Write font system");

    for run in buffer.layout_runs() {
        for glyph in run.glyphs.iter() {
            let letter_index = glyph.metadata;
            let pad_before = padding_before(letter_index, padding);
            let pad = padding.get(letter_index).copied().unwrap_or(0.0);
            let stroke_amount = stroke_amounts
                .get(letter_index)
                .copied()
                .unwrap_or(0.0);
            let physical_glyph = glyph.physical((0.0, 0.0), 1.0);
            let start_x = glyph.x + glyph.x_offset + pad_before + pad;
            let start_y = glyph.y_offset + run.line_y + y_offset;
            let offset = Vector::new(start_x, start_y);
            let active_stroke_width = stroke_width * (1.0 + stroke_amount);
            let stroke_color = Color {
                a: color.a * stroke_amount,
                ..color
            };

            if let Some(commands) = swash_cache.get_outline_commands(
                font_system.raw(),
                physical_glyph.cache_key,
            ) {
                let path = canvas::Path::new(|path| {
                    for command in commands {
                        match command {
                            cosmic_text::Command::MoveTo(point) => {
                                path.move_to(Point::new(point.x, -point.y) + offset);
                            }
                            cosmic_text::Command::LineTo(point) => {
                                path.line_to(Point::new(point.x, -point.y) + offset);
                            }
                            cosmic_text::Command::CurveTo(control_a, control_b, to) => {
                                path.bezier_curve_to(
                                    Point::new(control_a.x, -control_a.y) + offset,
                                    Point::new(control_b.x, -control_b.y) + offset,
                                    Point::new(to.x, -to.y) + offset,
                                );
                            }
                            cosmic_text::Command::QuadTo(control, to) => {
                                path.quadratic_curve_to(
                                    Point::new(control.x, -control.y) + offset,
                                    Point::new(to.x, -to.y) + offset,
                                );
                            }
                            cosmic_text::Command::Close => {
                                path.close();
                            }
                        }
                    }
                });

                frame.fill(&path, color);
                if stroke_amount > f32::EPSILON {
                    frame.stroke(
                        &path,
                        canvas::Stroke::default()
                            .with_width(active_stroke_width)
                            .with_color(stroke_color),
                    );
                }
            } else {
                let [r, g, b, a] = color.into_rgba8();

                swash_cache.with_pixels(
                    font_system.raw(),
                    physical_glyph.cache_key,
                    cosmic_text::Color::rgba(r, g, b, a),
                    |x, y, pixel| {
                        frame.fill(
                            &canvas::Path::rectangle(
                                Point::new(x as f32, y as f32) + offset,
                                Size::new(1.0, 1.0),
                            ),
                            Color::from_rgba8(
                                pixel.r(),
                                pixel.g(),
                                pixel.b(),
                                pixel.a() as f32 / 255.0,
                            ),
                        );
                    },
                );
            }
        }
    }
}

fn hit_test_letter(
    content: &str,
    font_size: f32,
    state: &KineticCanvasState,
    now: Instant,
    x: f32,
) -> Option<usize> {
    let padding = kinetic_padding_values(state, now, content.len());
    let paragraph = build_layout_paragraph(content, state.hovered_index, font_size);

    for index in 0..content.chars().count() {
        let (start, end) = letter_hit_bounds(index, &paragraph, &padding)?;
        if (start..end).contains(&x) {
            return Some(index);
        }
    }

    None
}

fn kinetic_weight(index: usize, hovered: Option<usize>) -> Weight {
    match hovered {
        None => Weight::Light,
        Some(active) => {
            let distance = index.abs_diff(active);
            match distance {
                0 => Weight::Black,
                1 => Weight::Semibold,
                2 => Weight::Normal,
                _ => Weight::Light,
            }
        }
    }
}

fn kinetic_padding_target(index: usize, hovered: Option<usize>, hover_padding: f32) -> f32 {
    match hovered {
        Some(active) if index.abs_diff(active) <= 1 => hover_padding,
        _ => 0.0,
    }
}

fn letter_string(letter: char) -> String {
    if letter == ' ' {
        "\u{00A0}".to_string()
    } else {
        letter.to_string()
    }
}

fn build_layout_paragraph(
    content: &str,
    hovered: Option<usize>,
    font_size: f32,
) -> GraphicsParagraph {
    let pieces: Vec<String> = content.chars().map(letter_string).collect();
    let spans: Vec<text_layout::Span<'_>> = pieces
        .iter()
        .enumerate()
        .map(|(index, piece)| {
            text_layout::Span::new(piece.as_str()).font(Font {
                weight: kinetic_weight(index, hovered),
                ..Font::DEFAULT
            })
        })
        .collect();

    GraphicsParagraph::with_spans(text_layout::Text {
        content: &spans,
        bounds: Size::new(f32::INFINITY, f32::INFINITY),
        size: Pixels(font_size),
        line_height: KINETIC_LINE_HEIGHT,
        font: Font::DEFAULT,
        align_x: Alignment::Left,
        align_y: Vertical::Top,
        shaping: Shaping::Advanced,
        wrapping: Wrapping::None,
    })
}

fn kinetic_padding_values(state: &KineticCanvasState, now: Instant, len: usize) -> Vec<f32> {
    (0..len)
        .map(|index| {
            state
                .letters
                .get(index)
                .map(|animation| animation.padding.value_at(now))
                .unwrap_or(0.0)
        })
        .collect()
}

fn padding_before(index: usize, padding: &[f32]) -> f32 {
    padding.iter().take(index).map(|value| value * 2.0).sum()
}

fn letter_hit_bounds(
    index: usize,
    paragraph: &GraphicsParagraph,
    padding: &[f32],
) -> Option<(f32, f32)> {
    let bounds = paragraph.span_bounds(index).first().copied()?;
    let pad = padding.get(index).copied().unwrap_or(0.0);
    let start = bounds.x + padding_before(index, padding);
    Some((start, start + bounds.width + pad * 2.0))
}

fn glyph_extent(paragraph: &GraphicsParagraph) -> (f32, f32) {
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
        (0.0, 0.0)
    } else {
        (min_y, max_y)
    }
}

fn measure_kinetic_layout(content: &str, font_size: f32) -> KineticLayoutMetrics {
    let chars: Vec<char> = content.chars().collect();
    let hover_padding = font_size / 12.0;
    let mut max_width: f32 = 0.0;
    let mut min_y = 0.0_f32;
    let mut max_y = font_size;

    let hover_states = if chars.is_empty() {
        vec![None]
    } else {
        std::iter::once(None)
            .chain((0..chars.len()).map(Some))
            .collect()
    };

    for hovered in hover_states {
        let paragraph = build_layout_paragraph(content, hovered, font_size);
        let padding: Vec<f32> = (0..chars.len())
            .map(|index| {
                kinetic_padding_target(index, hovered, hover_padding)
            })
            .collect();

        if let Some(last) = chars.len().checked_sub(1)
            && let Some((_, end)) = letter_hit_bounds(last, &paragraph, &padding)
        {
            max_width = max_width.max(end);
        } else if chars.is_empty() {
            max_width = 0.0;
        } else if max_width == 0.0 {
            max_width = paragraph.min_bounds().width;
        }

        let (top, bottom) = glyph_extent(&paragraph);
        min_y = min_y.min(top);
        max_y = max_y.max(bottom);
    }

    KineticLayoutMetrics {
        width: max_width,
        height: (max_y - min_y).max(1.0),
        y_offset: -min_y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinetic_weight_matches_reference_hierarchy() {
        assert_eq!(kinetic_weight(4, Some(4)), Weight::Black);
        assert_eq!(kinetic_weight(3, Some(4)), Weight::Semibold);
        assert_eq!(kinetic_weight(2, Some(4)), Weight::Normal);
        assert_eq!(kinetic_weight(0, Some(4)), Weight::Light);
        assert_eq!(kinetic_weight(0, None), Weight::Light);
    }

    #[test]
    fn kinetic_padding_applies_to_neighbors_only() {
        assert_eq!(kinetic_padding_target(3, Some(4), 8.0), 8.0);
        assert_eq!(kinetic_padding_target(4, Some(4), 8.0), 8.0);
        assert_eq!(kinetic_padding_target(2, Some(4), 8.0), 0.0);
    }

    #[test]
    fn line_y_stays_stable_when_hover_changes() {
        let content = "Nostalgia";
        let idle = build_layout_paragraph(content, None, 96.0);
        let hovered = build_layout_paragraph(content, Some(4), 96.0);
        let idle_line_y = idle.buffer().layout_runs().next().unwrap().line_y;
        let hovered_line_y = hovered.buffer().layout_runs().next().unwrap().line_y;

        assert!(
            (idle_line_y - hovered_line_y).abs() < 0.01,
            "line_y shifted ({idle_line_y} vs {hovered_line_y})"
        );
    }

    #[test]
    fn shared_layout_keeps_glyph_baselines_aligned_across_weights() {
        let content = "Nostalgia";
        let idle = build_layout_paragraph(content, None, 96.0);
        let hovered = build_layout_paragraph(content, Some(4), 96.0);
        let idle_baselines = glyph_baselines(&idle);
        let hovered_baselines = glyph_baselines(&hovered);

        assert_eq!(idle_baselines.len(), hovered_baselines.len());
        for (index, (idle_y, hovered_y)) in idle_baselines
            .into_iter()
            .zip(hovered_baselines)
            .enumerate()
        {
            assert!(
                (idle_y - hovered_y).abs() < 0.01,
                "glyph {index} baseline shifted ({idle_y} vs {hovered_y})"
            );
        }
    }

    #[test]
    fn hovered_black_glyphs_fit_inside_measured_canvas_height() {
        let content = "Nostalgia";
        let font_size = 96.0;
        let metrics = measure_kinetic_layout(content, font_size);
        let paragraph = build_layout_paragraph(content, Some(4), font_size);
        let (min_y, max_y) = glyph_extent(&paragraph);
        let shifted_min = min_y + metrics.y_offset;
        let shifted_max = max_y + metrics.y_offset;

        assert!(
            shifted_min >= -0.5,
            "glyphs extend above canvas origin ({shifted_min})"
        );
        assert!(
            shifted_max <= metrics.height + 0.5,
            "glyphs extend below canvas height ({shifted_max} > {})",
            metrics.height
        );
    }

    fn glyph_baselines(paragraph: &GraphicsParagraph) -> Vec<f32> {
        paragraph
            .buffer()
            .layout_runs()
            .flat_map(|run| {
                let line_y = run.line_y;
                run.glyphs
                    .iter()
                    .map(move |glyph| line_y + glyph.y_offset)
            })
            .collect()
    }
}
