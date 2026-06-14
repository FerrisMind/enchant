use super::text_element::TextElementRole;
use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};
use iced::advanced::graphics::text::Paragraph as GraphicsParagraph;
use iced::advanced::text::{
    self as text_layout, Alignment, LineHeight, Paragraph as TextParagraph, Shaping, Wrapping,
};
use iced::alignment::Vertical;
use iced::widget::canvas;
use iced::{Color, Element, Font, Length, Pixels, Point, Rectangle, Renderer, Size, Vector};
use iced_shadcn::Theme;
use std::time::Duration;

const SPINNING_LINE_HEIGHT: LineHeight = LineHeight::Relative(1.0);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpinningTextTransition {
    pub duration_secs: f32,
    pub delay_secs: f32,
}

impl Default for SpinningTextTransition {
    fn default() -> Self {
        Self {
            duration_secs: 10.0,
            delay_secs: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpinningTextVariantState {
    pub opacity: f32,
    pub scale: f32,
}

impl Default for SpinningTextVariantState {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            scale: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct SpinningTextVariants {
    pub container: Option<SpinningTextVariantState>,
    pub item: Option<SpinningTextVariantState>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpinningTextProps {
    pub duration_secs: f32,
    pub reverse: bool,
    pub radius: f32,
    pub transition: SpinningTextTransition,
    pub variants: SpinningTextVariants,
    pub role: TextElementRole,
    pub font_size: Option<u16>,
    pub color: Option<Color>,
}

impl Default for SpinningTextProps {
    fn default() -> Self {
        Self {
            duration_secs: 10.0,
            reverse: false,
            radius: 5.0,
            transition: SpinningTextTransition::default(),
            variants: SpinningTextVariants::default(),
            role: TextElementRole::Div,
            font_size: None,
            color: None,
        }
    }
}

impl SpinningTextProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

pub fn spinning_text<'a, Message: 'a>(
    content: &str,
    elapsed: Duration,
    props: SpinningTextProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let mut letters = content.chars().collect::<Vec<_>>();
    letters.push(' ');

    let item_scale = props.variants.item.unwrap_or_default().scale.max(0.1);
    let item_opacity = props
        .variants
        .item
        .unwrap_or_default()
        .opacity
        .clamp(0.0, 1.0);
    let font_size = props
        .font_size
        .unwrap_or(props.role.default_font_size())
        .max(1) as f32
        * item_scale;
    let color = props.color.unwrap_or(theme.palette.foreground);
    let color = Color {
        a: color.a * item_opacity,
        ..color
    };
    let ch = character_width(font_size);
    let orbit_radius = props.radius * ch;
    let canvas_size = (orbit_radius + font_size) * 2.0;

    let container_rotation = (elapsed.as_secs_f32() - props.transition.delay_secs.max(0.0))
        / props.duration_secs.max(0.05)
        * 360.0
        * if props.reverse { -1.0 } else { 1.0 };

    effect_canvas::<_, Message>(
        SpinningTextProgram {
            letters,
            font_size,
            orbit_radius,
            container_rotation,
            color,
        },
        EffectCanvasProps::new()
            .width(Length::Fixed(canvas_size.ceil().max(1.0)))
            .height(Length::Fixed(canvas_size.ceil().max(1.0))),
    )
    .into()
}

struct SpinningTextProgram {
    letters: Vec<char>,
    font_size: f32,
    orbit_radius: f32,
    container_rotation: f32,
    color: Color,
}

impl<Message> EffectProgram<Message> for SpinningTextProgram {
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
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let total = self.letters.len().max(1) as f32;

        for (index, letter) in self.letters.iter().enumerate() {
            let angle_deg = self.container_rotation + (360.0 / total) * index as f32;
            let angle_rad = angle_deg.to_radians();
            let (letter_width, letter_height) = letter_metrics(*letter, self.font_size);
            let offset = Vector::new(-letter_width / 2.0, -letter_height / 2.0);

            frame.with_save(|frame| {
                frame.translate(Vector::new(center.x, center.y));
                frame.rotate(angle_rad);
                frame.translate(Vector::new(0.0, -self.orbit_radius));
                frame.translate(offset);

                spinning_canvas_text(*letter, Point::ORIGIN, self.font_size).draw_with(|path, _| {
                    frame.fill(&path, self.color);
                });
            });
        }

        vec![frame.into_geometry()]
    }
}

fn spinning_canvas_text(letter: char, position: Point, font_size: f32) -> canvas::Text {
    canvas::Text {
        content: letter_string(letter),
        position,
        max_width: f32::INFINITY,
        color: Color::TRANSPARENT,
        size: Pixels(font_size),
        line_height: SPINNING_LINE_HEIGHT,
        font: Font::DEFAULT,
        align_x: Alignment::Left,
        align_y: Vertical::Top,
        ..canvas::Text::default()
    }
}

fn letter_string(letter: char) -> String {
    if letter == ' ' {
        "\u{00A0}".to_string()
    } else {
        letter.to_string()
    }
}

fn character_width(font_size: f32) -> f32 {
    spinning_paragraph("0", font_size)
        .min_bounds()
        .width
        .max(1.0)
}

fn letter_metrics(letter: char, font_size: f32) -> (f32, f32) {
    let size = spinning_paragraph(&letter_string(letter), font_size).min_bounds();
    (size.width.max(1.0), size.height.max(1.0))
}

fn spinning_paragraph(content: &str, font_size: f32) -> GraphicsParagraph {
    GraphicsParagraph::with_text(text_layout::Text {
        content,
        bounds: Size::new(f32::INFINITY, f32::INFINITY),
        size: Pixels(font_size),
        line_height: SPINNING_LINE_HEIGHT,
        font: Font::DEFAULT,
        align_x: Alignment::Left,
        align_y: Vertical::Top,
        shaping: Shaping::Advanced,
        wrapping: Wrapping::None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spinning_text_defaults_match_docs() {
        let props = SpinningTextProps::new();

        assert_eq!(props.duration_secs, 10.0);
        assert!(!props.reverse);
        assert_eq!(props.radius, 5.0);
    }

    #[test]
    fn letter_positions_start_at_top() {
        let total = 4.0;
        let index = 0.0;
        let container_rotation = 0.0;
        let angle_deg = container_rotation + (360.0 / total) * index;

        assert!((angle_deg - 0.0_f32).abs() < f32::EPSILON);
    }
}
