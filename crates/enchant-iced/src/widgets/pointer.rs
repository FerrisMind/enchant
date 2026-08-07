use crate::primitives::{EffectCanvasProps, EffectProgram, PointerMotion, effect_canvas};
use iced::mouse;
use iced::time::Instant;
use iced::widget::{container, mouse_area, stack};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Vector};
use iced_shadcn::Theme;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PointerAction {
    Move(Point),
    Exit,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct PointerState {
    hovered: bool,
    motion: PointerMotion,
}

impl PointerState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_action(&mut self, action: &PointerAction, now: Instant) {
        match action {
            PointerAction::Move(position) => {
                self.hovered = true;
                self.motion.update(*position, now);
            }
            PointerAction::Exit => {
                self.hovered = false;
                self.motion.reset();
            }
        }
    }

    pub fn is_hovered(&self) -> bool {
        self.hovered
    }

    pub fn position(&self) -> Option<Point> {
        self.motion.position()
    }
}

pub struct PointerProps<'a, Message> {
    pub pointer: Option<Element<'a, Message>>,
    pub hide_system_cursor: bool,
    pub offset: Vector,
    pub size: f32,
    pub color: Option<Color>,
    pub interaction: mouse::Interaction,
}

impl<Message> Default for PointerProps<'_, Message> {
    fn default() -> Self {
        Self {
            pointer: None,
            hide_system_cursor: true,
            offset: Vector::new(10.0, 10.0),
            size: 24.0,
            color: None,
            interaction: mouse::Interaction::Pointer,
        }
    }
}

impl<'a, Message> PointerProps<'a, Message> {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn pointer<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    state: &PointerState,
    props: PointerProps<'a, Message>,
    on_action: impl Fn(PointerAction) -> Message + Clone + 'a,
    theme: &Theme,
) -> Element<'a, Message> {
    let overlay = pointer_overlay(
        state.position(),
        props.pointer,
        props.offset,
        props.size,
        props.color,
        theme,
    );
    let interaction = if props.hide_system_cursor && state.is_hovered() {
        mouse::Interaction::Hidden
    } else {
        props.interaction
    };

    mouse_area(stack(vec![content.into(), overlay]))
        .on_move({
            let on_action = on_action.clone();
            move |position| on_action(PointerAction::Move(position))
        })
        .on_exit(on_action(PointerAction::Exit))
        .interaction(interaction)
        .into()
}

fn pointer_overlay<'a, Message: 'a>(
    position: Option<Point>,
    pointer: Option<Element<'a, Message>>,
    offset: Vector,
    size: f32,
    color: Option<Color>,
    theme: &Theme,
) -> Element<'a, Message> {
    let (position, overlay): (Point, Element<'a, Message>) = if let Some(position) = position {
        (
            position,
            pointer.unwrap_or_else(|| {
                default_pointer_overlay(
                    size,
                    color
                        .or_else(|| theme.registry.color("magic.pointer.color"))
                        .unwrap_or(theme.palette.foreground),
                )
            }),
        )
    } else {
        (Point::ORIGIN, iced::widget::Space::new().into())
    };

    iced::widget::float(
        container(overlay)
            .width(Length::Shrink)
            .height(Length::Shrink),
    )
    .translate(move |_bounds, _viewport| Vector::new(position.x + offset.x, position.y + offset.y))
    .into()
}

pub(crate) fn default_pointer_overlay<'a, Message: 'a>(
    size: f32,
    color: Color,
) -> Element<'a, Message> {
    effect_canvas::<_, Message>(
        DefaultPointerProgram { color, size },
        EffectCanvasProps::new()
            .width(Length::Fixed(size.max(8.0)))
            .height(Length::Fixed(size.max(8.0))),
    )
    .into()
}

struct DefaultPointerProgram {
    color: Color,
    size: f32,
}

impl<Message> EffectProgram<Message> for DefaultPointerProgram {
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
        let size = self.size.max(8.0);
        let path = iced::widget::canvas::Path::new(|builder| {
            builder.move_to(Point::new(0.0, 0.0));
            builder.line_to(Point::new(size * 0.75, size * 0.46));
            builder.line_to(Point::new(size * 0.38, size * 0.54));
            builder.line_to(Point::new(size * 0.52, size));
            builder.line_to(Point::new(size * 0.30, size * 0.93));
            builder.line_to(Point::new(size * 0.18, size * 0.50));
            builder.close();
        });
        frame.fill(&path, self.color);
        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn pointer_state_tracks_move_and_exit() {
        let mut state = PointerState::new();
        let start = Instant::now();
        state.apply_action(&PointerAction::Move(Point::new(12.0, 14.0)), start);
        assert!(state.is_hovered());
        assert_eq!(state.position(), Some(Point::new(12.0, 14.0)));

        state.apply_action(&PointerAction::Exit, start + Duration::from_millis(16));
        assert!(!state.is_hovered());
        assert_eq!(state.position(), None);
    }
}
