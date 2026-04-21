use iced::widget::canvas::{self, Fill, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size};
use iced_shadcn::Theme;

use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};

#[derive(Clone, Debug, PartialEq)]
pub struct HexagonPatternProps {
    pub width: Length,
    pub height: Length,
    pub radius: f32,
    pub gap: f32,
    pub x: f32,
    pub y: f32,
    pub direction: HexagonDirection,
    pub stroke_dasharray: f32,
    pub hexagons: Vec<(i32, i32)>,
    pub stroke_color: Option<Color>,
    pub fill_color: Option<Color>,
    pub stroke_width: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HexagonDirection {
    #[default]
    Horizontal,
    Vertical,
}

impl Default for HexagonPatternProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            height: Length::Fixed(240.0),
            radius: 40.0,
            gap: 0.0,
            x: -1.0,
            y: -1.0,
            direction: HexagonDirection::Horizontal,
            stroke_dasharray: 0.0,
            hexagons: Vec::new(),
            stroke_color: None,
            fill_color: None,
            stroke_width: 1.0,
        }
    }
}

impl HexagonPatternProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn hexagon_pattern<'a, Message: 'a>(
    props: HexagonPatternProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let width = props.width;
    let height = props.height;
    let stroke_color = props
        .stroke_color
        .or_else(|| theme.registry.color("magic.pattern.grid"))
        .unwrap_or(Color {
            a: 0.3,
            ..theme.palette.foreground
        });
    let fill_color = props
        .fill_color
        .or_else(|| theme.registry.color("magic.pattern.fill"))
        .unwrap_or(Color {
            a: 0.3,
            ..theme.palette.primary
        });

    effect_canvas::<_, Message>(
        HexagonPatternProgram {
            props,
            stroke_color,
            fill_color,
        },
        EffectCanvasProps::new().width(width).height(height),
    )
    .into()
}

struct HexagonPatternProgram {
    props: HexagonPatternProps,
    stroke_color: Color,
    fill_color: Color,
}

impl<Message> EffectProgram<Message> for HexagonPatternProgram {
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
        let dash_segments = [self.props.stroke_dasharray, self.props.stroke_dasharray];
        let stroke = Stroke {
            width: self.props.stroke_width.max(0.5),
            style: canvas::stroke::Style::Solid(self.stroke_color),
            line_dash: if self.props.stroke_dasharray > 0.0 {
                canvas::LineDash {
                    offset: 0,
                    segments: &dash_segments,
                }
            } else {
                canvas::LineDash::default()
            },
            ..Stroke::default()
        };

        let centers = tile_geometry(
            self.props.radius,
            self.props.direction,
            self.props.gap,
            bounds.size(),
            self.props.x,
            self.props.y,
        );

        for center in centers {
            frame.stroke(
                &hexagon_path(center, self.props.radius, self.props.direction),
                stroke,
            );
        }

        for &(col, row) in &self.props.hexagons {
            let center = hexagon_center(
                col,
                row,
                self.props.radius,
                self.props.direction,
                self.props.gap,
                self.props.x,
                self.props.y,
            );
            frame.fill(
                &hexagon_path(
                    center,
                    (self.props.radius - 1.0).max(2.0),
                    self.props.direction,
                ),
                Fill::from(self.fill_color),
            );
        }

        vec![frame.into_geometry()]
    }
}

fn hexagon_vertices(center: Point, radius: f32, direction: HexagonDirection) -> [Point; 6] {
    let start_angle = match direction {
        HexagonDirection::Horizontal => 0.0f32,
        HexagonDirection::Vertical => 30.0f32,
    };

    std::array::from_fn(|index| {
        let angle = (start_angle + index as f32 * 60.0).to_radians();
        Point::new(
            center.x + radius * angle.cos(),
            center.y + radius * angle.sin(),
        )
    })
}

fn hexagon_path(center: Point, radius: f32, direction: HexagonDirection) -> canvas::Path {
    let vertices = hexagon_vertices(center, radius, direction);
    canvas::Path::new(|builder| {
        builder.move_to(vertices[0]);
        for vertex in vertices.iter().skip(1) {
            builder.line_to(*vertex);
        }
        builder.close();
    })
}

fn tile_geometry(
    radius: f32,
    direction: HexagonDirection,
    gap: f32,
    bounds: Size,
    x: f32,
    y: f32,
) -> Vec<Point> {
    let sqrt3 = 3.0f32.sqrt();
    let row_step = match direction {
        HexagonDirection::Horizontal => sqrt3 * radius + gap,
        HexagonDirection::Vertical => (3.0 * radius) / 2.0 + (sqrt3 * gap) / 2.0,
    };

    let mut centers = Vec::new();
    let mut row = 0i32;
    loop {
        let mut col = 0i32;
        let mut added = false;
        loop {
            let center = hexagon_center(col, row, radius, direction, gap, x, y);
            if center.x - radius > bounds.width + radius
                || center.y - radius > bounds.height + radius
            {
                break;
            }
            if center.x + radius >= 0.0 && center.y + radius >= 0.0 {
                centers.push(center);
                added = true;
            }
            col += 1;
        }

        if !added && (row as f32) * row_step > bounds.height + radius * 2.0 {
            break;
        }
        row += 1;
    }

    centers
}

fn hexagon_center(
    col: i32,
    row: i32,
    radius: f32,
    direction: HexagonDirection,
    gap: f32,
    x: f32,
    y: f32,
) -> Point {
    let sqrt3 = 3.0f32.sqrt();
    match direction {
        HexagonDirection::Horizontal => {
            let col_step = (3.0 * radius) / 2.0 + (sqrt3 * gap) / 2.0;
            let row_step = sqrt3 * radius + gap;
            Point::new(
                x + col as f32 * col_step + col_step / 2.0,
                y + row as f32 * row_step
                    + row_step / 2.0
                    + if col % 2 != 0 { row_step / 2.0 } else { 0.0 },
            )
        }
        HexagonDirection::Vertical => {
            let col_step = sqrt3 * radius + gap;
            let row_step = (3.0 * radius) / 2.0 + (sqrt3 * gap) / 2.0;
            Point::new(
                x + col as f32 * col_step
                    + col_step / 2.0
                    + if row % 2 != 0 { col_step / 2.0 } else { 0.0 },
                y + row as f32 * row_step + row_step / 2.0,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hexagon_pattern_defaults_match_docs() {
        let props = HexagonPatternProps::new();

        assert_eq!(props.radius, 40.0);
        assert_eq!(props.gap, 0.0);
        assert_eq!(props.direction, HexagonDirection::Horizontal);
    }

    #[test]
    fn hexagon_center_offsets_stagger_rows() {
        let left = hexagon_center(0, 0, 40.0, HexagonDirection::Horizontal, 0.0, 0.0, 0.0);
        let right = hexagon_center(1, 0, 40.0, HexagonDirection::Horizontal, 0.0, 0.0, 0.0);

        assert!(right.x > left.x);
        assert!(right.y > left.y);
    }
}
