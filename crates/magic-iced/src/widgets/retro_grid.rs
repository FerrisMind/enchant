use iced::widget::canvas::{self, Fill, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Renderer};
use iced_shadcn::Theme;
use std::time::Duration;

use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetroGridProps {
    pub width: Length,
    pub height: Length,
    pub angle: f32,
    pub cell_size: f32,
    pub opacity: f32,
    pub light_line_color: Option<Color>,
    pub dark_line_color: Option<Color>,
}

impl Default for RetroGridProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            height: Length::Fixed(320.0),
            angle: 65.0,
            cell_size: 60.0,
            opacity: 0.5,
            light_line_color: None,
            dark_line_color: None,
        }
    }
}

impl RetroGridProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn retro_grid<'a, Message: 'a>(
    elapsed: Duration,
    props: RetroGridProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let width = props.width;
    let height = props.height;
    let light = props
        .light_line_color
        .or_else(|| theme.registry.color("magic.retro_grid.light"))
        .unwrap_or(Color::from_rgba8(255, 211, 25, props.opacity));
    let dark = props
        .dark_line_color
        .or_else(|| theme.registry.color("magic.retro_grid.dark"))
        .unwrap_or(Color::from_rgba8(140, 30, 255, props.opacity));

    effect_canvas::<_, Message>(
        RetroGridProgram {
            elapsed,
            props,
            light,
            dark,
        },
        EffectCanvasProps::new().width(width).height(height),
    )
    .into()
}

struct RetroGridProgram {
    elapsed: Duration,
    props: RetroGridProps,
    light: Color,
    dark: Color,
}

impl<Message> EffectProgram<Message> for RetroGridProgram {
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
        let horizon = bounds.height * 0.32;
        let depth = (self.props.angle.clamp(1.0, 89.0) / 89.0).clamp(0.1, 1.0);
        let cell = self.props.cell_size.max(8.0);
        let travel = (self.elapsed.as_secs_f32() * cell * 0.4).rem_euclid(cell);

        frame.fill(
            &canvas::Path::rectangle(Point::ORIGIN, bounds.size()),
            Fill::from(Color {
                a: 0.0,
                ..self.dark
            }),
        );

        let stroke = Stroke {
            width: 1.0,
            style: canvas::stroke::Style::Solid(self.light),
            ..Stroke::default()
        };

        let vanishing = Point::new(bounds.width * 0.5, horizon);
        let columns = (bounds.width / cell).ceil() as i32 + 4;
        for column in -columns..=columns {
            let x = bounds.width * 0.5 + column as f32 * cell;
            frame.stroke(
                &canvas::Path::line(Point::new(x, bounds.height), vanishing),
                stroke,
            );
        }

        let mut row = 0usize;
        loop {
            let perspective = (row as f32 * 0.08 + 1.0).powf(1.35);
            let y = horizon + ((row as f32 * cell - travel) / perspective) * depth * 1.8;
            if y > bounds.height {
                break;
            }
            if y >= horizon {
                let alpha =
                    (1.0 - (y - horizon) / (bounds.height - horizon).max(1.0)).clamp(0.1, 1.0);
                frame.stroke(
                    &canvas::Path::line(Point::new(0.0, y), Point::new(bounds.width, y)),
                    Stroke {
                        width: 1.0,
                        style: canvas::stroke::Style::Solid(Color {
                            a: self.props.opacity * alpha,
                            ..self.dark
                        }),
                        ..Stroke::default()
                    },
                );
            }
            row += 1;
        }

        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retro_grid_defaults_match_docs() {
        let props = RetroGridProps::new();

        assert_eq!(props.angle, 65.0);
        assert_eq!(props.cell_size, 60.0);
        assert_eq!(props.opacity, 0.5);
    }
}
