use crate::primitives::{
    EffectCanvasProps, EffectProgram, GridPatternSpec, effect_canvas, grid_dimensions,
    stable_grid_cell,
};
use iced::widget::canvas::{self, Stroke};
use iced::{Color, Element, Length, Rectangle, Renderer};
use iced_shadcn::Theme;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimatedGridPatternProps {
    pub width: Length,
    pub height: Length,
    pub pattern: GridPatternSpec,
    pub stroke_dasharray: f32,
    pub num_squares: usize,
    pub max_opacity: f32,
    pub duration_secs: f32,
    pub repeat_delay_secs: f32,
    pub seed: u64,
    pub stroke_color: Option<Color>,
    pub fill_color: Option<Color>,
}

impl Default for AnimatedGridPatternProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            height: Length::Fixed(220.0),
            pattern: GridPatternSpec::new(),
            stroke_dasharray: 0.0,
            num_squares: 200,
            max_opacity: 0.5,
            duration_secs: 1.0,
            repeat_delay_secs: 0.5,
            seed: 7,
            stroke_color: None,
            fill_color: None,
        }
    }
}

impl AnimatedGridPatternProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn animated_grid_pattern<'a, Message: 'a>(
    elapsed: Duration,
    props: AnimatedGridPatternProps,
    theme: &Theme,
) -> Element<'a, Message> {
    effect_canvas::<_, Message>(
        AnimatedGridPatternProgram {
            elapsed,
            props,
            stroke_color: props
                .stroke_color
                .or_else(|| theme.registry.color("magic.pattern.grid"))
                .unwrap_or(Color::from_rgba(1.0, 1.0, 1.0, 0.14)),
            fill_color: props
                .fill_color
                .or_else(|| theme.registry.color("magic.pattern.fill"))
                .unwrap_or(Color {
                    a: 0.3,
                    ..theme.palette.primary
                }),
        },
        EffectCanvasProps::new()
            .width(props.width)
            .height(props.height),
    )
    .into()
}

struct AnimatedGridPatternProgram {
    elapsed: Duration,
    props: AnimatedGridPatternProps,
    stroke_color: Color,
    fill_color: Color,
}

impl<Message> EffectProgram<Message> for AnimatedGridPatternProgram {
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
        let spec = self.props.pattern;
        let (columns, rows) = grid_dimensions(bounds.size(), spec);
        if columns == 0 || rows == 0 {
            return vec![frame.into_geometry()];
        }

        let dash_segments = [self.props.stroke_dasharray, self.props.stroke_dasharray];
        let dash = if self.props.stroke_dasharray > 0.0 {
            canvas::LineDash {
                offset: 0,
                segments: &dash_segments,
            }
        } else {
            canvas::LineDash::default()
        };
        let stroke = Stroke {
            width: 1.0,
            style: canvas::stroke::Style::Solid(self.stroke_color),
            line_dash: dash,
            ..Stroke::default()
        };

        let mut x = spec.offset_x;
        while x <= bounds.width + spec.cell_width {
            frame.stroke(
                &canvas::Path::line(iced::Point::new(x, 0.0), iced::Point::new(x, bounds.height)),
                stroke,
            );
            x += spec.cell_width;
        }

        let mut y = spec.offset_y;
        while y <= bounds.height + spec.cell_height {
            frame.stroke(
                &canvas::Path::line(iced::Point::new(0.0, y), iced::Point::new(bounds.width, y)),
                stroke,
            );
            y += spec.cell_height;
        }

        let duration = self.props.duration_secs.max(0.05);
        let repeat_delay = self.props.repeat_delay_secs.max(0.0);
        let cycle = duration + repeat_delay;
        let elapsed = self.elapsed.as_secs_f32();

        for slot in 0..self.props.num_squares {
            let start_offset = slot as f32 * 0.1;
            let local = (elapsed + start_offset).max(0.0);
            let cycle_index = (local / cycle).floor() as usize;
            let cycle_time = local.rem_euclid(cycle);
            if cycle_time > duration {
                continue;
            }

            let progress = (cycle_time / duration).clamp(0.0, 1.0);
            let opacity = (1.0 - (progress - 0.5).abs() * 2.0).clamp(0.0, 1.0);
            let cell = stable_grid_cell(self.props.seed, slot, cycle_index, columns, rows);
            let x = spec.offset_x + cell.x as f32 * spec.cell_width + 1.0;
            let y = spec.offset_y + cell.y as f32 * spec.cell_height + 1.0;

            frame.fill(
                &canvas::Path::rectangle(
                    iced::Point::new(x, y),
                    iced::Size::new(
                        (spec.cell_width - 1.0).max(1.0),
                        (spec.cell_height - 1.0).max(1.0),
                    ),
                ),
                Color {
                    a: self.props.max_opacity.clamp(0.0, 1.0) * opacity,
                    ..self.fill_color
                },
            );
        }

        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animated_grid_pattern_defaults_match_docs() {
        let props = AnimatedGridPatternProps::new();

        assert_eq!(props.num_squares, 200);
        assert_eq!(props.max_opacity, 0.5);
        assert_eq!(props.duration_secs, 1.0);
        assert_eq!(props.repeat_delay_secs, 0.5);
    }
}
