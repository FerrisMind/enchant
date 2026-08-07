use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};
use iced::time::Duration;
use iced::widget::{canvas, container};
use iced::{Color, Element, Length, Point, Rectangle, Renderer};
use iced_shadcn::Theme;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RippleProps {
    pub main_circle_size: f32,
    pub main_circle_opacity: f32,
    pub num_circles: usize,
    pub duration_secs: f32,
    pub color: Option<Color>,
}

impl Default for RippleProps {
    fn default() -> Self {
        Self {
            main_circle_size: 210.0,
            main_circle_opacity: 0.24,
            num_circles: 8,
            duration_secs: 1.6,
            color: None,
        }
    }
}

impl RippleProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn ripple<'a, Message: 'a>(
    elapsed: Duration,
    props: RippleProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let color = props
        .color
        .or_else(|| theme.registry.color("magic.ripple.color"))
        .unwrap_or(theme.palette.foreground);

    container(effect_canvas::<_, Message>(
        RippleProgram {
            elapsed,
            props,
            color,
        },
        EffectCanvasProps::new()
            .width(Length::Fill)
            .height(Length::Fill),
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

struct RippleProgram {
    elapsed: Duration,
    props: RippleProps,
    color: Color,
}

impl<Message> EffectProgram<Message> for RippleProgram {
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
        let duration = self.props.duration_secs.max(0.05);
        let phase = self.elapsed.as_secs_f32().rem_euclid(duration) / duration;
        let center = Point::new(bounds.width * 0.5, bounds.height * 0.5);

        for index in 0..self.props.num_circles.max(1) {
            let offset = index as f32 / self.props.num_circles.max(1) as f32;
            let local = (phase + offset).fract();
            let opacity = (self.props.main_circle_opacity - index as f32 * 0.03).max(0.0);
            let size = self.props.main_circle_size + index as f32 * 70.0;
            let scale = 0.75 + local * 0.75;
            let alpha = opacity * (1.0 - local).max(0.0);
            let radius = size * scale * 0.5;

            frame.stroke(
                &canvas::Path::circle(center, radius.max(1.0)),
                canvas::Stroke::default().with_width(1.0).with_color(Color {
                    a: alpha,
                    ..self.color
                }),
            );
        }

        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ripple_defaults_are_stable() {
        let props = RippleProps::new();

        assert_eq!(props.main_circle_size, 210.0);
        assert_eq!(props.num_circles, 8);
    }
}
