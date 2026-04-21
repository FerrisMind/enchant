use iced::widget::canvas::{self, Fill};
use iced::{Color, Element, Length, Point, Rectangle, Renderer};
use std::time::Duration;

use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas, stable_noise};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlickeringGridProps {
    pub width: Length,
    pub height: Length,
    pub square_size: f32,
    pub grid_gap: f32,
    pub flicker_chance: f32,
    pub color: Color,
    pub max_opacity: f32,
    pub seed: u64,
}

impl Default for FlickeringGridProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            height: Length::Fixed(320.0),
            square_size: 4.0,
            grid_gap: 6.0,
            flicker_chance: 0.3,
            color: Color::from_rgb8(0, 0, 0),
            max_opacity: 0.2,
            seed: 17,
        }
    }
}

impl FlickeringGridProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn flickering_grid<'a, Message: 'a>(
    elapsed: Duration,
    props: FlickeringGridProps,
) -> Element<'a, Message> {
    let width = props.width;
    let height = props.height;
    effect_canvas::<_, Message>(
        FlickeringGridProgram { elapsed, props },
        EffectCanvasProps::new().width(width).height(height),
    )
    .into()
}

struct FlickeringGridProgram {
    elapsed: Duration,
    props: FlickeringGridProps,
}

impl<Message> EffectProgram<Message> for FlickeringGridProgram {
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
        let step = (self.props.square_size + self.props.grid_gap).max(1.0);
        let columns = (bounds.width / step).ceil().max(1.0) as usize;
        let rows = (bounds.height / step).ceil().max(1.0) as usize;
        let tick = (self.elapsed.as_secs_f32() * 12.0).floor() as u64;

        for column in 0..columns {
            for row in 0..rows {
                let index = (column * rows + row) as u64;
                let chance = stable_noise(self.props.seed + tick, index);
                if chance > self.props.flicker_chance.clamp(0.0, 1.0) {
                    continue;
                }

                let opacity = stable_noise(self.props.seed ^ 0x9E37_79B9, index + tick * 13)
                    * self.props.max_opacity.clamp(0.0, 1.0);
                frame.fill(
                    &canvas::Path::rectangle(
                        Point::new(column as f32 * step, row as f32 * step),
                        iced::Size::new(self.props.square_size, self.props.square_size),
                    ),
                    Fill::from(Color {
                        a: opacity,
                        ..self.props.color
                    }),
                );
            }
        }

        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flickering_grid_defaults_match_docs() {
        let props = FlickeringGridProps::new();

        assert_eq!(props.square_size, 4.0);
        assert_eq!(props.grid_gap, 6.0);
        assert_eq!(props.flicker_chance, 0.3);
        assert_eq!(props.max_opacity, 0.2);
    }
}
