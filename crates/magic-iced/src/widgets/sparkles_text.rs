use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas, stable_noise};
use iced::advanced::graphics::text::Paragraph as GraphicsParagraph;
use iced::advanced::text::{
    self as text_layout, Alignment, LineHeight, Paragraph as TextParagraph, Shaping, Wrapping,
};
use iced::alignment::Vertical;
use iced::widget::{container, stack, text};
use iced::{Color, Element, Font, Length, Pixels, Point, Rectangle, Renderer, Size, Vector};
use iced_shadcn::Theme;
use std::time::Duration;

const SPARKLE_CYCLE_SECS: f32 = 0.8;
/// Magic UI sparkle SVG viewBox is 21×21.
const SPARKLE_VIEWBOX: f32 = 21.0;
const SPARKLE_HALF: f32 = SPARKLE_VIEWBOX / 2.0;
/// Reference scale range is `0.3 + random()` → max `1.3`.
const SPARKLE_MAX_SCALE: f32 = 1.3;
/// Room for sparkles to spill outside the text box (reference does not clip overflow).
const SPARKLE_FIELD_PADDING: f32 = SPARKLE_HALF * SPARKLE_MAX_SCALE;
const LINE_HEIGHT: LineHeight = LineHeight::Relative(1.0);
const BOLD_FONT: Font = Font {
    weight: iced::font::Weight::Bold,
    ..Font::DEFAULT
};

const DEFAULT_SPARKLE_FIRST: Color = Color::from_rgb8(0x9E, 0x7A, 0xFF);
const DEFAULT_SPARKLE_SECOND: Color = Color::from_rgb8(0xFE, 0x8B, 0xBB);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SparklesColors {
    pub first: Color,
    pub second: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SparklesTextProps {
    pub sparkles_count: usize,
    pub colors: Option<SparklesColors>,
    pub font_size: u16,
    pub seed: u64,
    /// Override foreground color (for example blended theme transitions).
    pub text_color: Option<Color>,
}

impl Default for SparklesTextProps {
    fn default() -> Self {
        Self {
            sparkles_count: 10,
            colors: None,
            font_size: 24,
            seed: 23,
            text_color: None,
        }
    }
}

impl SparklesTextProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }
}

pub fn sparkles_text<'a, Message: 'a>(
    content: &'a str,
    elapsed: Duration,
    props: SparklesTextProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let font_size = props.font_size.max(1) as f32;
    let layout = measure_sparkles_text(content, font_size);
    let text_width = layout.width.ceil().max(1.0);
    let text_height = layout.height.ceil().max(font_size);
    let padding = SPARKLE_FIELD_PADDING;
    let field_width = text_width + padding * 2.0;
    let field_height = text_height + padding * 2.0;

    let colors = props.colors.unwrap_or(SparklesColors {
        first: DEFAULT_SPARKLE_FIRST,
        second: DEFAULT_SPARKLE_SECOND,
    });
    let text_color = props.text_color.unwrap_or(theme.palette.foreground);

    container(
        stack(vec![
            container(
                text(content)
                    .font(BOLD_FONT)
                    .size(u32::from(props.font_size.max(1)))
                    .style({
                        let color = text_color;
                        move |_| iced::widget::text::Style { color: Some(color) }
                    }),
            )
            .padding(padding)
            .width(Length::Shrink)
            .height(Length::Shrink)
            .into(),
            effect_canvas::<_, Message>(
                SparklesProgram {
                    elapsed,
                    count: props.sparkles_count,
                    colors,
                    seed: props.seed,
                    text_width,
                    text_height,
                    field_padding: padding,
                },
                EffectCanvasProps::new()
                    .width(Length::Fixed(field_width))
                    .height(Length::Fixed(field_height)),
            )
            .into(),
        ])
        .width(Length::Fixed(field_width))
        .height(Length::Fixed(field_height)),
    )
    .width(Length::Fixed(field_width))
    .height(Length::Fixed(field_height))
    .into()
}

struct SparklesProgram {
    elapsed: Duration,
    count: usize,
    colors: SparklesColors,
    seed: u64,
    text_width: f32,
    text_height: f32,
    field_padding: f32,
}

impl<Message> EffectProgram<Message> for SparklesProgram {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<iced::widget::canvas::Geometry> {
        use iced::widget::canvas;

        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let time = self.elapsed.as_secs_f32();

        for index in 0..self.count {
            let index = index as u64;
            let lifetime = 5.0 + stable_noise(self.seed, index.wrapping_mul(29)) * 10.0;
            let birth_offset = stable_noise(self.seed, index.wrapping_mul(31)) * lifetime;
            let generation = ((time + birth_offset) / lifetime).floor() as u64;
            let x_pct = stable_noise(
                self.seed,
                index
                    .wrapping_mul(3)
                    .wrapping_add(generation.wrapping_mul(97)),
            );
            let y_pct = stable_noise(
                self.seed.wrapping_add(1),
                index
                    .wrapping_mul(7)
                    .wrapping_add(generation.wrapping_mul(97)),
            );
            // Percentages are relative to the text box; padding lets sparkles spill freely.
            let center_x = self.field_padding + x_pct * self.text_width + SPARKLE_HALF;
            let center_y = self.field_padding + y_pct * self.text_height + SPARKLE_HALF;
            let delay = stable_noise(self.seed.wrapping_add(2), index.wrapping_mul(13)) * 2.0;
            let scale = 0.3 + stable_noise(self.seed.wrapping_add(3), index.wrapping_mul(17));
            let color = if stable_noise(self.seed.wrapping_add(4), index.wrapping_mul(19)) > 0.5 {
                self.colors.first
            } else {
                self.colors.second
            };

            let local = ((time + delay).rem_euclid(SPARKLE_CYCLE_SECS)) / SPARKLE_CYCLE_SECS;
            let opacity = tri_keyframe([0.0, 1.0, 0.0], local);
            if opacity <= 0.01 {
                continue;
            }

            let animated_scale = tri_keyframe([0.0, scale, 0.0], local);
            let rotation = tri_keyframe([75.0, 120.0, 150.0], local).to_radians();

            draw_sparkle(
                &mut frame,
                Point::new(center_x, center_y),
                animated_scale,
                rotation,
                Color {
                    a: opacity,
                    ..color
                },
            );
        }

        vec![frame.into_geometry()]
    }
}

fn draw_sparkle(
    frame: &mut iced::widget::canvas::Frame,
    center: Point,
    scale: f32,
    rotation: f32,
    color: Color,
) {
    if scale <= 0.01 {
        return;
    }

    frame.with_save(|frame| {
        frame.translate(Vector::new(center.x, center.y));
        frame.rotate(rotation);
        frame.scale(scale);
        frame.fill(sparkle_star_path(), color);
    });
}

#[inline]
fn sparkle_point(x: f32, y: f32) -> Point {
    Point::new(x - SPARKLE_HALF, y - SPARKLE_HALF)
}

fn sparkle_star_path() -> &'static iced::widget::canvas::Path {
    use iced::widget::canvas;
    use std::sync::OnceLock;

    static PATH: OnceLock<canvas::Path> = OnceLock::new();
    PATH.get_or_init(|| {
        canvas::Path::new(|path| {
            // Exact path from magicui/sparkles-text.tsx (viewBox 0 0 21 21).
            path.move_to(sparkle_point(9.825_31, 0.843_845));
            path.bezier_curve_to(
                sparkle_point(10.0553, 0.215_178),
                sparkle_point(10.9446, 0.215_178),
                sparkle_point(11.1746, 0.843_845),
            );
            path.line_to(sparkle_point(11.8618, 2.720_26));
            path.bezier_curve_to(
                sparkle_point(12.4006, 4.192_29),
                sparkle_point(12.3916, 6.391_57),
                sparkle_point(13.5, 7.5),
            );
            path.bezier_curve_to(
                sparkle_point(14.6084, 8.608_43),
                sparkle_point(16.8077, 8.599_35),
                sparkle_point(18.2797, 9.138_22),
            );
            path.line_to(sparkle_point(20.1561, 9.825_34));
            path.bezier_curve_to(
                sparkle_point(20.7858, 10.0553),
                sparkle_point(20.7858, 10.9447),
                sparkle_point(20.1561, 11.1747),
            );
            path.line_to(sparkle_point(18.2797, 11.8618));
            path.bezier_curve_to(
                sparkle_point(16.8077, 12.4007),
                sparkle_point(14.6084, 12.3916),
                sparkle_point(13.5, 13.5),
            );
            path.bezier_curve_to(
                sparkle_point(12.3916, 14.6084),
                sparkle_point(12.4006, 16.8077),
                sparkle_point(11.8618, 18.2798),
            );
            path.line_to(sparkle_point(11.1746, 20.1562));
            path.bezier_curve_to(
                sparkle_point(10.9446, 20.7858),
                sparkle_point(10.0553, 20.7858),
                sparkle_point(9.825_31, 20.1562),
            );
            path.line_to(sparkle_point(9.138_19, 18.2798));
            path.bezier_curve_to(
                sparkle_point(8.599_32, 16.8077),
                sparkle_point(8.608_43, 14.6084),
                sparkle_point(7.5, 13.5),
            );
            path.bezier_curve_to(
                sparkle_point(6.391_57, 12.3916),
                sparkle_point(4.192_25, 12.4007),
                sparkle_point(2.720_23, 11.8618),
            );
            path.line_to(sparkle_point(0.843_814, 11.1747));
            path.bezier_curve_to(
                sparkle_point(0.215_148, 10.9447),
                sparkle_point(0.215_148, 10.0553),
                sparkle_point(0.843_814, 9.825_34),
            );
            path.line_to(sparkle_point(2.720_23, 9.138_22));
            path.bezier_curve_to(
                sparkle_point(4.192_25, 8.599_35),
                sparkle_point(6.391_57, 8.608_43),
                sparkle_point(7.5, 7.5),
            );
            path.bezier_curve_to(
                sparkle_point(8.608_43, 6.391_57),
                sparkle_point(8.599_32, 4.192_29),
                sparkle_point(9.138_19, 2.720_26),
            );
            path.close();
        })
    })
}

fn tri_keyframe(values: [f32; 3], progress: f32) -> f32 {
    let progress = progress.clamp(0.0, 1.0);

    if progress <= 0.5 {
        let t = progress / 0.5;
        values[0] + (values[1] - values[0]) * t
    } else {
        let t = (progress - 0.5) / 0.5;
        values[1] + (values[2] - values[1]) * t
    }
}

fn measure_sparkles_text(content: &str, font_size: f32) -> Size {
    if content.is_empty() {
        return Size::new(1.0, font_size);
    }

    sparkles_paragraph(content, font_size).min_bounds()
}

fn sparkles_paragraph(content: &str, font_size: f32) -> GraphicsParagraph {
    GraphicsParagraph::with_text(text_layout::Text {
        content,
        bounds: Size::new(f32::INFINITY, f32::INFINITY),
        size: Pixels(font_size),
        line_height: LINE_HEIGHT,
        font: BOLD_FONT,
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
    fn sparkles_defaults_match_docs() {
        let props = SparklesTextProps::new();

        assert_eq!(props.sparkles_count, 10);
        assert_eq!(props.font_size, 24);
    }

    #[test]
    fn sparkle_animation_keyframes_match_reference() {
        assert_eq!(tri_keyframe([0.0, 1.0, 0.0], 0.0), 0.0);
        assert_eq!(tri_keyframe([0.0, 1.0, 0.0], 0.5), 1.0);
        assert_eq!(tri_keyframe([0.0, 1.0, 0.0], 1.0), 0.0);
        assert_eq!(tri_keyframe([75.0, 120.0, 150.0], 0.5), 120.0);
    }
}
