use crate::primitives::{
    EffectCanvasProps, EffectProgram, GridCellIndex, GridPatternSpec, effect_canvas,
};
use iced::widget::canvas::{self, Stroke};
use iced::{Color, Element, Length, Rectangle, Renderer};
use iced_shadcn::Theme;

#[derive(Clone, Debug, PartialEq)]
pub struct GridPatternProps {
    pub width: Length,
    pub height: Length,
    pub pattern: GridPatternSpec,
    pub squares: Vec<GridCellIndex>,
    pub stroke_dasharray: f32,
    pub stroke_width: f32,
    pub stroke_color: Option<Color>,
    pub fill_color: Option<Color>,
}

impl Default for GridPatternProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            height: Length::Fixed(220.0),
            pattern: GridPatternSpec::new(),
            squares: Vec::new(),
            stroke_dasharray: 0.0,
            stroke_width: 1.0,
            stroke_color: None,
            fill_color: None,
        }
    }
}

impl GridPatternProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn grid_pattern<'a, Message: 'a>(
    props: GridPatternProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let width = props.width;
    let height = props.height;
    let stroke_color = props
        .stroke_color
        .or_else(|| theme.registry.color("magic.pattern.grid"))
        .unwrap_or(Color::from_rgba(1.0, 1.0, 1.0, 0.14));
    let fill_color = props
        .fill_color
        .or_else(|| theme.registry.color("magic.pattern.fill"))
        .unwrap_or(Color {
            a: 0.18,
            ..theme.palette.primary
        });

    effect_canvas::<_, Message>(
        GridPatternProgram {
            props,
            stroke_color,
            fill_color,
        },
        EffectCanvasProps::new().width(width).height(height),
    )
    .into()
}

struct GridPatternProgram {
    props: GridPatternProps,
    stroke_color: Color,
    fill_color: Color,
}

impl<Message> EffectProgram<Message> for GridPatternProgram {
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
            width: self.props.stroke_width.max(0.5),
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

        for cell in &self.props.squares {
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
                self.fill_color,
            );
        }

        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_pattern_defaults_match_docs() {
        let props = GridPatternProps::new();

        assert_eq!(props.pattern.cell_width, 40.0);
        assert_eq!(props.pattern.cell_height, 40.0);
        assert_eq!(props.pattern.offset_x, -1.0);
        assert_eq!(props.pattern.offset_y, -1.0);
    }
}
