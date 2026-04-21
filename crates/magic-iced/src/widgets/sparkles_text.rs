use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas, stable_noise};
use iced::widget::{container, stack, text};
use iced::{Color, Element, Length, Rectangle, Renderer};
use iced_shadcn::Theme;
use std::time::Duration;

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
}

impl Default for SparklesTextProps {
    fn default() -> Self {
        Self {
            sparkles_count: 10,
            colors: None,
            font_size: 24,
            seed: 23,
        }
    }
}

impl SparklesTextProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn sparkles_text<'a, Message: 'a>(
    content: &'a str,
    elapsed: Duration,
    props: SparklesTextProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let colors = props.colors.unwrap_or(SparklesColors {
        first: theme
            .registry
            .color("magic.sparkle.first")
            .unwrap_or(Color::from_rgb8(0x9E, 0x7A, 0xFF)),
        second: theme
            .registry
            .color("magic.sparkle.second")
            .unwrap_or(Color::from_rgb8(0xFE, 0x8B, 0xBB)),
    });

    stack(vec![
        container(text(content).size(u32::from(props.font_size)).style({
            let color = theme.palette.foreground;
            move |_| iced::widget::text::Style { color: Some(color) }
        }))
        .width(Length::Shrink)
        .height(Length::Shrink)
        .into(),
        effect_canvas::<_, Message>(
            SparklesProgram {
                elapsed,
                count: props.sparkles_count,
                colors,
                seed: props.seed,
            },
            EffectCanvasProps::new()
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .into(),
    ])
    .into()
}

struct SparklesProgram {
    elapsed: Duration,
    count: usize,
    colors: SparklesColors,
    seed: u64,
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
            let x = stable_noise(self.seed, index as u64 * 3) * bounds.width;
            let y = stable_noise(self.seed + 1, index as u64 * 7) * bounds.height;
            let delay = stable_noise(self.seed + 2, index as u64 * 13) * 2.0;
            let scale = 0.3 + stable_noise(self.seed + 3, index as u64 * 17);
            let color = if stable_noise(self.seed + 4, index as u64 * 19) > 0.5 {
                self.colors.first
            } else {
                self.colors.second
            };
            let pulse = (((time + delay) / 0.8) * std::f32::consts::TAU).sin().abs();
            let radius = 2.0 + scale * 5.0 * pulse;

            if pulse <= 0.02 {
                continue;
            }

            draw_sparkle(
                &mut frame,
                iced::Point::new(x, y),
                radius,
                Color { a: pulse, ..color },
            );
        }

        vec![frame.into_geometry()]
    }
}

fn draw_sparkle(
    frame: &mut iced::widget::canvas::Frame,
    center: iced::Point,
    radius: f32,
    color: Color,
) {
    use iced::widget::canvas;

    frame.stroke(
        &canvas::Path::line(
            iced::Point::new(center.x - radius, center.y),
            iced::Point::new(center.x + radius, center.y),
        ),
        canvas::Stroke::default().with_width(1.0).with_color(color),
    );
    frame.stroke(
        &canvas::Path::line(
            iced::Point::new(center.x, center.y - radius),
            iced::Point::new(center.x, center.y + radius),
        ),
        canvas::Stroke::default().with_width(1.0).with_color(color),
    );
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
}
