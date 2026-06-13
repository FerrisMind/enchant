use crate::primitives::{
    EffectCanvasProps, EffectProgram, GridPatternSpec, PatternTransform, effect_canvas,
    grid_dimensions, pulse_opacity, stable_grid_cell,
};
use iced::widget::canvas::{self, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size};
use iced_shadcn::Theme;
use std::time::Duration;
use twill::prelude::Easing;

const GRAY_400: Color = Color::from_rgb8(0x9C, 0xA3, 0xAF);

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
    pub skew_x_deg: f32,
    pub pattern_height_scale: f32,
    pub pattern_offset_y_ratio: f32,
    pub mask_radius: f32,
    pub easing: Easing,
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
            num_squares: 50,
            max_opacity: 0.5,
            duration_secs: 4.0,
            repeat_delay_secs: 0.5,
            seed: 7,
            skew_x_deg: 12.0,
            pattern_height_scale: 2.0,
            pattern_offset_y_ratio: -0.3,
            mask_radius: 250.0,
            easing: Easing::InOut,
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
    _theme: &Theme,
) -> Element<'a, Message> {
    let stroke_color = props.stroke_color.unwrap_or(pattern_tone_color());
    let fill_color = props.fill_color.unwrap_or(square_fill_color());

    effect_canvas::<_, Message>(
        AnimatedGridPatternProgram {
            elapsed,
            props,
            stroke_color,
            fill_color,
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

#[derive(Clone, Copy, Debug)]
struct PatternLayout {
    pattern_height: f32,
    transform: PatternTransform,
    mask_center: Point,
    mask_radius: f32,
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
        let layout = PatternLayout::new(bounds, &self.props);
        let pattern_size = Size::new(bounds.width, layout.pattern_height);
        let (columns, rows) = grid_dimensions(pattern_size, spec);
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

        for row in 0..rows {
            for col in 0..columns {
                let x = spec.offset_x + col as f32 * spec.cell_width;
                let y = spec.offset_y + row as f32 * spec.cell_height;
                let start = layout
                    .transform
                    .map_point(Point::new(x + 0.5, y + spec.cell_height));
                let corner = layout
                    .transform
                    .map_point(Point::new(x + 0.5, y + 0.5));
                let end = layout
                    .transform
                    .map_point(Point::new(x + spec.cell_width, y + 0.5));
                let mid = Point::new(
                    (start.x + corner.x + end.x) / 3.0,
                    (start.y + corner.y + end.y) / 3.0,
                );
                let mask = radial_mask_alpha(layout, mid);

                let path = canvas::Path::new(|builder| {
                    builder.move_to(start);
                    builder.line_to(corner);
                    builder.line_to(end);
                });

                frame.stroke(
                    &path,
                    Stroke {
                        width: 1.0,
                        style: canvas::stroke::Style::Solid(Color {
                            a: self.stroke_color.a * mask,
                            ..self.stroke_color
                        }),
                        line_dash: dash,
                        ..Stroke::default()
                    },
                );
            }
        }

        let duration = self.props.duration_secs.max(0.05);
        let repeat_delay = self.props.repeat_delay_secs.max(0.0);
        let cycle = duration * 2.0 + repeat_delay;
        let elapsed = self.elapsed.as_secs_f32();
        let max_opacity = self.props.max_opacity.clamp(0.0, 1.0);

        for slot in 0..self.props.num_squares {
            let start_offset = slot as f32 * 0.1;
            let local = (elapsed + start_offset).max(0.0);
            let cycle_index = (local / cycle).floor() as usize;
            let cycle_time = local.rem_euclid(cycle);
            let wave = pulse_opacity(cycle_time, duration, self.props.easing);
            if wave <= f32::EPSILON {
                continue;
            }

            let cell = stable_grid_cell(self.props.seed, slot, cycle_index, columns, rows);
            let x = spec.offset_x + cell.x as f32 * spec.cell_width + 1.0;
            let y = spec.offset_y + cell.y as f32 * spec.cell_height + 1.0;
            let rect = Rectangle::new(
                Point::new(x, y),
                Size::new(
                    (spec.cell_width - 1.0).max(1.0),
                    (spec.cell_height - 1.0).max(1.0),
                ),
            );
            let corners = layout.transform.map_rect(rect);
            let center = Point::new(
                (corners[0].x + corners[1].x + corners[2].x + corners[3].x) / 4.0,
                (corners[0].y + corners[1].y + corners[2].y + corners[3].y) / 4.0,
            );
            let mask = radial_mask_alpha(layout, center);
            let opacity = max_opacity * wave * mask;

            let path = canvas::Path::new(|builder| {
                builder.move_to(corners[0]);
                builder.line_to(corners[1]);
                builder.line_to(corners[2]);
                builder.line_to(corners[3]);
                builder.close();
            });

            frame.fill(
                &path,
                Color {
                    a: self.fill_color.a * opacity,
                    ..self.fill_color
                },
            );
        }

        vec![frame.into_geometry()]
    }
}

impl PatternLayout {
    fn new(bounds: Rectangle, props: &AnimatedGridPatternProps) -> Self {
        let pattern_height = bounds.height * props.pattern_height_scale.max(0.1);
        let offset_y = bounds.height * props.pattern_offset_y_ratio;

        Self {
            pattern_height,
            transform: PatternTransform::skew_x(
                props.skew_x_deg,
                bounds.width,
                pattern_height,
                offset_y,
            ),
            mask_center: Point::new(bounds.width / 2.0, bounds.height / 2.0),
            mask_radius: props.mask_radius.max(0.0),
        }
    }
}

fn pattern_tone_color() -> Color {
    Color {
        a: 0.3,
        ..GRAY_400
    }
}

fn square_fill_color() -> Color {
    GRAY_400
}

fn radial_mask_alpha(layout: PatternLayout, point: Point) -> f32 {
    if layout.mask_radius <= f32::EPSILON {
        return 1.0;
    }

    let dx = point.x - layout.mask_center.x;
    let dy = point.y - layout.mask_center.y;
    let distance = (dx * dx + dy * dy).sqrt();
    (1.0 - (distance / layout.mask_radius).clamp(0.0, 1.0)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animated_grid_pattern_defaults_match_docs() {
        let props = AnimatedGridPatternProps::new();

        assert_eq!(props.num_squares, 50);
        assert_eq!(props.max_opacity, 0.5);
        assert_eq!(props.duration_secs, 4.0);
        assert_eq!(props.repeat_delay_secs, 0.5);
        assert_eq!(props.skew_x_deg, 12.0);
        assert_eq!(props.pattern_height_scale, 2.0);
        assert_eq!(props.pattern_offset_y_ratio, -0.3);
        assert_eq!(props.easing, Easing::InOut);
    }
}
