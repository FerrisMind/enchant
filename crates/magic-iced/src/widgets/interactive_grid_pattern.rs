use crate::primitives::{EffectCanvasProps, EffectProgram, GridCellIndex, effect_canvas};
use iced::mouse;
use iced::widget::mouse_area;
use iced::{Color, Element, Length, Point, Rectangle, Renderer};
use iced_shadcn::Theme;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InteractiveGridPatternAction {
    Hover(Option<GridCellIndex>),
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct InteractiveGridPatternState {
    hovered: Option<GridCellIndex>,
}

impl InteractiveGridPatternState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_action(&mut self, action: &InteractiveGridPatternAction) {
        match action {
            InteractiveGridPatternAction::Hover(cell) => self.hovered = *cell,
        }
    }

    pub fn hovered(&self) -> Option<GridCellIndex> {
        self.hovered
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InteractiveGridPatternProps {
    pub width: Length,
    pub height: Length,
    pub cell_width: f32,
    pub cell_height: f32,
    pub squares: (u16, u16),
    pub stroke_color: Option<Color>,
    pub hover_fill_color: Option<Color>,
}

impl Default for InteractiveGridPatternProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            height: Length::Fixed(320.0),
            cell_width: 40.0,
            cell_height: 40.0,
            squares: (24, 24),
            stroke_color: None,
            hover_fill_color: None,
        }
    }
}

impl InteractiveGridPatternProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn interactive_grid_pattern<'a, Message: Clone + 'a>(
    state: &InteractiveGridPatternState,
    props: InteractiveGridPatternProps,
    theme: &Theme,
    on_action: impl Fn(InteractiveGridPatternAction) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let width = props.width;
    let height = props.height;
    let stroke_color = props.stroke_color.unwrap_or(Color {
        a: 0.3,
        ..theme.palette.foreground
    });
    let hover_fill_color = props.hover_fill_color.unwrap_or(Color {
        a: 0.3,
        ..theme.palette.primary
    });

    let canvas: Element<'a, Message> = effect_canvas::<_, Message>(
        InteractiveGridPatternProgram {
            props,
            hovered: state.hovered(),
            stroke_color,
            hover_fill_color,
        },
        EffectCanvasProps::new().width(width).height(height),
    )
    .into();

    mouse_area(canvas)
        .on_move({
            let on_action = on_action.clone();
            move |position| {
                on_action(InteractiveGridPatternAction::Hover(cell_at(
                    position, props,
                )))
            }
        })
        .on_exit(on_action(InteractiveGridPatternAction::Hover(None)))
        .interaction(mouse::Interaction::Pointer)
        .into()
}

fn cell_at(position: Point, props: InteractiveGridPatternProps) -> Option<GridCellIndex> {
    if props.cell_width <= f32::EPSILON || props.cell_height <= f32::EPSILON {
        return None;
    }

    let column = (position.x / props.cell_width).floor() as i32;
    let row = (position.y / props.cell_height).floor() as i32;

    if column < 0 || row < 0 || column >= props.squares.0 as i32 || row >= props.squares.1 as i32 {
        None
    } else {
        Some(GridCellIndex { x: column, y: row })
    }
}

struct InteractiveGridPatternProgram {
    props: InteractiveGridPatternProps,
    hovered: Option<GridCellIndex>,
    stroke_color: Color,
    hover_fill_color: Color,
}

impl<Message> EffectProgram<Message> for InteractiveGridPatternProgram {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<iced::widget::canvas::Geometry> {
        let mut frame = iced::widget::canvas::Frame::new(renderer, bounds.size());
        let stroke = iced::widget::canvas::Stroke {
            width: 1.0,
            style: iced::widget::canvas::stroke::Style::Solid(self.stroke_color),
            ..iced::widget::canvas::Stroke::default()
        };

        for column in 0..self.props.squares.0 {
            for row in 0..self.props.squares.1 {
                let top_left = Point::new(
                    column as f32 * self.props.cell_width,
                    row as f32 * self.props.cell_height,
                );
                let rect = iced::widget::canvas::Path::rectangle(
                    top_left,
                    iced::Size::new(self.props.cell_width, self.props.cell_height),
                );
                if self.hovered
                    == Some(GridCellIndex {
                        x: column as i32,
                        y: row as i32,
                    })
                {
                    frame.fill(&rect, self.hover_fill_color);
                }
                frame.stroke(&rect, stroke);
            }
        }

        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interactive_grid_pattern_defaults_match_docs() {
        let props = InteractiveGridPatternProps::new();

        assert_eq!(props.cell_width, 40.0);
        assert_eq!(props.cell_height, 40.0);
        assert_eq!(props.squares, (24, 24));
    }

    #[test]
    fn cell_at_resolves_hovered_square() {
        let props = InteractiveGridPatternProps::new();
        assert_eq!(
            cell_at(Point::new(80.0, 120.0), props),
            Some(GridCellIndex { x: 2, y: 3 })
        );
    }
}
