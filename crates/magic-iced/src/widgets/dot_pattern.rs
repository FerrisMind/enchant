use crate::primitives::{
    EffectCanvasProps, EffectProgram, GridPatternSpec, effect_canvas, grid_dimensions, stable_noise,
};
use iced::widget::canvas;
use iced::{Color, Element, Length, Rectangle, Renderer};
use iced_shadcn::Theme;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DotPatternProps {
    pub width: Length,
    pub height: Length,
    pub pattern: GridPatternSpec,
    pub cx: f32,
    pub cy: f32,
    pub cr: f32,
    pub glow: bool,
    pub seed: u64,
    pub color: Option<Color>,
}

impl Default for DotPatternProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            height: Length::Fixed(220.0),
            pattern: GridPatternSpec::new()
                .cell_width(16.0)
                .cell_height(16.0)
                .offset(0.0, 0.0),
            cx: 1.0,
            cy: 1.0,
            cr: 1.0,
            glow: false,
            seed: 1,
            color: None,
        }
    }
}

impl DotPatternProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn dot_pattern<'a, Message: 'a>(
    elapsed: Duration,
    props: DotPatternProps,
    theme: &Theme,
) -> Element<'a, Message> {
    effect_canvas::<_, Message>(
        DotPatternProgram {
            elapsed,
            props,
            color: props
                .color
                .or_else(|| theme.registry.color("magic.pattern.dot"))
                .unwrap_or(Color::from_rgba(1.0, 1.0, 1.0, 0.36)),
        },
        EffectCanvasProps::new()
            .width(props.width)
            .height(props.height),
    )
    .into()
}

struct DotPatternProgram {
    elapsed: Duration,
    props: DotPatternProps,
    color: Color,
}

impl<Message> EffectProgram<Message> for DotPatternProgram {
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
        let (columns, rows) = grid_dimensions(bounds.size(), self.props.pattern);
        let spec = self.props.pattern;
        let base_radius = self.props.cr.max(0.5);
        let time = self.elapsed.as_secs_f32();

        for row in 0..rows {
            for column in 0..columns {
                let x = spec.offset_x + column as f32 * spec.cell_width + self.props.cx;
                let y = spec.offset_y + row as f32 * spec.cell_height + self.props.cy;
                let index = (row * columns + column) as u64;

                let (radius, alpha) = if self.props.glow {
                    let phase = stable_noise(self.props.seed, index * 17);
                    let speed = 0.8 + stable_noise(self.props.seed + 1, index * 31) * 1.6;
                    let pulse = ((time * speed + phase * std::f32::consts::TAU).sin() + 1.0) * 0.5;
                    (base_radius * (1.0 + pulse * 0.8), 0.25 + pulse * 0.75)
                } else {
                    (base_radius, 1.0)
                };

                frame.fill(
                    &canvas::Path::circle(iced::Point::new(x, y), radius),
                    Color {
                        a: self.color.a * alpha,
                        ..self.color
                    },
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
    fn dot_pattern_defaults_match_docs() {
        let props = DotPatternProps::new();

        assert_eq!(props.pattern.cell_width, 16.0);
        assert_eq!(props.pattern.cell_height, 16.0);
        assert!(!props.glow);
    }
}
