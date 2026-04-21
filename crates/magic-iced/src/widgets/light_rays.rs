use crate::primitives::StableRng;
use iced::widget::canvas::{self, Fill};
use iced::{Color, Element, Length, Point, Rectangle, Renderer};
use iced_shadcn::Theme;
use std::time::Duration;

use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightRaysProps {
    pub width: Length,
    pub height: Length,
    pub count: usize,
    pub color: Option<Color>,
    pub blur: f32,
    pub speed: f32,
    pub length: f32,
    pub seed: u64,
}

impl Default for LightRaysProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            height: Length::Fixed(280.0),
            count: 7,
            color: None,
            blur: 36.0,
            speed: 14.0,
            length: 420.0,
            seed: 7,
        }
    }
}

impl LightRaysProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn light_rays<'a, Message: 'a>(
    elapsed: Duration,
    props: LightRaysProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let width = props.width;
    let height = props.height;
    let color = props
        .color
        .or_else(|| theme.registry.color("magic.light_rays.color"))
        .unwrap_or(Color::from_rgba8(160, 210, 255, 0.2));

    effect_canvas::<_, Message>(
        LightRaysProgram {
            elapsed,
            props,
            color,
        },
        EffectCanvasProps::new().width(width).height(height),
    )
    .into()
}

struct LightRaysProgram {
    elapsed: Duration,
    props: LightRaysProps,
    color: Color,
}

impl<Message> EffectProgram<Message> for LightRaysProgram {
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
        let mut rng = StableRng::new(self.props.seed);
        let cycle = self.props.speed.max(0.1);
        let progress = self.elapsed.as_secs_f32() / cycle;

        for index in 0..self.props.count.max(1) {
            let left = 0.08 + rng.next_f32() * 0.84;
            let width = 160.0 + rng.next_f32() * 160.0;
            let swing = 0.8 + rng.next_f32() * 1.8;
            let delay = rng.next_f32() * cycle;
            let duration = cycle * (0.75 + rng.next_f32() * 0.5);
            let rotate = -28.0 + rng.next_f32() * 56.0;
            let intensity = 0.55 + rng.next_f32() * 0.45;

            let local = ((self.elapsed.as_secs_f32() + delay) / duration.max(0.1)).fract();
            let opacity = ((local * std::f32::consts::TAU).sin() * 0.5 + 0.5) * intensity;
            let current_rotate = rotate + (progress * swing * 2.0 + index as f32).sin() * swing;
            let top = -bounds.height * 0.12;
            let center_x = bounds.width * left;
            let base_y = top + self.props.length;

            frame.with_save(|frame| {
                frame.translate(iced::Vector::new(center_x, top));
                frame.rotate(current_rotate.to_radians());

                for layer in 0..4 {
                    let softness = 1.0 + layer as f32 * 0.2;
                    let alpha = opacity * (0.18 / softness);
                    let color = Color {
                        a: alpha,
                        ..self.color
                    };
                    let half_width = width * (0.5 + layer as f32 * 0.12);
                    let path = canvas::Path::new(|builder| {
                        builder.move_to(Point::new(0.0, 0.0));
                        builder.line_to(Point::new(-half_width, base_y));
                        builder.line_to(Point::new(half_width, base_y));
                        builder.close();
                    });
                    frame.fill(&path, Fill::from(color));
                }
            });
        }

        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_rays_defaults_match_docs() {
        let props = LightRaysProps::new();

        assert_eq!(props.count, 7);
        assert_eq!(props.blur, 36.0);
        assert_eq!(props.speed, 14.0);
    }
}
