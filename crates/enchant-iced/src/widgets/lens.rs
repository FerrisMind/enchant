use iced::advanced::Renderer as _;
use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{self, Tree, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::keyboard;
use iced::mouse;
use iced::widget::{canvas, mouse_area};
use iced::{
    Color, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Transformation, Vector,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LensAction {
    Move(Point),
    Enter,
    Exit,
    Dismiss,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct LensState {
    hovered: bool,
    position: Option<Point>,
    dismissed: bool,
}

impl LensState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_action(&mut self, action: &LensAction) {
        match action {
            LensAction::Move(position) => {
                self.hovered = true;
                self.position = Some(*position);
                self.dismissed = false;
            }
            LensAction::Enter => {
                self.hovered = true;
                self.dismissed = false;
            }
            LensAction::Exit => {
                self.hovered = false;
            }
            LensAction::Dismiss => {
                self.dismissed = true;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LensProps {
    pub width: Length,
    pub height: Length,
    pub zoom_factor: f32,
    pub lens_size: f32,
    pub position: Option<Point>,
    pub default_position: Option<Point>,
    pub is_static: bool,
    pub duration_secs: f32,
    pub lens_color: Option<Color>,
}

impl Default for LensProps {
    fn default() -> Self {
        Self {
            width: Length::Shrink,
            height: Length::Shrink,
            zoom_factor: 1.3,
            lens_size: 170.0,
            position: None,
            default_position: None,
            is_static: false,
            duration_secs: 0.1,
            lens_color: None,
        }
    }
}

impl LensProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn lens<'a, Message: Clone + 'a + 'static>(
    content: impl Into<Element<'a, Message>>,
    state: &'a LensState,
    props: LensProps,
    on_action: impl Fn(LensAction) -> Message + Clone + 'a + 'static,
) -> Element<'a, Message> {
    let widget = LensWidget {
        content: content.into(),
        state,
        props,
        on_action: on_action.clone(),
    };

    mouse_area(widget)
        .on_move({
            let on_action = on_action.clone();
            move |position| on_action(LensAction::Move(position))
        })
        .on_enter(on_action(LensAction::Enter))
        .on_exit(on_action(LensAction::Exit))
        .interaction(if lens_active(state, props) {
            mouse::Interaction::ZoomIn
        } else {
            mouse::Interaction::Pointer
        })
        .into()
}

struct LensWidget<'a, Message, F>
where
    F: Fn(LensAction) -> Message + Clone + 'a,
{
    content: Element<'a, Message>,
    state: &'a LensState,
    props: LensProps,
    on_action: F,
}

impl<Message, F> Widget<Message, Theme, Renderer> for LensWidget<'_, Message, F>
where
    Message: 'static,
    F: Fn(LensAction) -> Message + Clone + 'static,
{
    fn tag(&self) -> widget::tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> widget::tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if let Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event
            && *key == keyboard::Key::Named(keyboard::key::Named::Escape)
            && lens_active(self.state, self.props)
        {
            shell.publish((self.on_action)(LensAction::Dismiss));
            shell.capture_event();
        }

        self.content.as_widget_mut().update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);

        if !lens_active(self.state, self.props) {
            return;
        }

        let bounds = layout.bounds();
        let Some(position) = lens_position(self.state, self.props, bounds) else {
            return;
        };
        let lens_bounds = lens_bounds(position, self.props.lens_size, bounds);

        renderer.with_layer(lens_bounds, |renderer| {
            renderer.with_transformation(
                Transformation::translate(
                    lens_bounds.x + lens_bounds.width * 0.5 - position.x * self.props.zoom_factor,
                    lens_bounds.y + lens_bounds.height * 0.5 - position.y * self.props.zoom_factor,
                ) * Transformation::scale(self.props.zoom_factor)
                    * Transformation::translate(-bounds.x, -bounds.y),
                |renderer| {
                    self.content.as_widget().draw(
                        tree,
                        renderer,
                        theme,
                        style,
                        layout,
                        mouse::Cursor::Unavailable,
                        viewport,
                    );
                },
            );
        });

        let lens_color = self.props.lens_color.unwrap_or(Color::BLACK);
        let mut frame = canvas::Frame::new(renderer, lens_bounds.size());
        let radius = self.props.lens_size.max(24.0) * 0.5;
        let path = canvas::Path::rounded_rectangle(
            Point::ORIGIN,
            lens_bounds.size(),
            iced::border::Radius::from(radius),
        );
        frame.fill(
            &path,
            Color {
                a: 0.06,
                ..lens_color
            },
        );
        frame.stroke(
            &path,
            canvas::Stroke::default().with_width(2.0).with_color(Color {
                a: 0.38,
                ..lens_color
            }),
        );
        let geometry = frame.into_geometry();
        renderer.with_translation(Vector::new(lens_bounds.x, lens_bounds.y), |renderer| {
            renderer.draw_geometry(geometry);
        });
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::ZoomIn
        } else {
            self.content
                .as_widget()
                .mouse_interaction(tree, layout, cursor, viewport, renderer)
        }
    }
}

impl<'a, Message, F> From<LensWidget<'a, Message, F>> for Element<'a, Message>
where
    Message: 'a + 'static,
    F: Fn(LensAction) -> Message + Clone + 'a + 'static,
{
    fn from(widget: LensWidget<'a, Message, F>) -> Self {
        Element::new(widget)
    }
}

fn lens_active(state: &LensState, props: LensProps) -> bool {
    (props.is_static
        || state.hovered
        || props.default_position.is_some()
        || props.position.is_some())
        && !state.dismissed
}

fn lens_position(state: &LensState, props: LensProps, bounds: Rectangle) -> Option<Point> {
    let fallback = Point::new(bounds.width * 0.5, bounds.height * 0.5);
    if props.is_static {
        return Some(
            props
                .position
                .or(props.default_position)
                .unwrap_or(fallback),
        );
    }

    if state.hovered {
        return state
            .position
            .or(props.position)
            .or(props.default_position)
            .or(Some(fallback));
    }

    props.default_position.or(props.position)
}

fn lens_bounds(position: Point, lens_size: f32, bounds: Rectangle) -> Rectangle {
    let lens_size = lens_size.max(24.0);
    Rectangle::new(
        Point::new(
            (position.x - lens_size * 0.5).clamp(0.0, (bounds.width - lens_size).max(0.0)),
            (position.y - lens_size * 0.5).clamp(0.0, (bounds.height - lens_size).max(0.0)),
        ),
        iced::Size::new(lens_size, lens_size),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lens_defaults_match_docs() {
        let props = LensProps::new();

        assert_eq!(props.zoom_factor, 1.3);
        assert_eq!(props.lens_size, 170.0);
        assert_eq!(props.duration_secs, 0.1);
    }

    #[test]
    fn lens_state_dismiss_hides_overlay_until_next_move() {
        let mut state = LensState::new();
        let props = LensProps::new();
        state.apply_action(&LensAction::Enter);
        state.apply_action(&LensAction::Move(Point::new(50.0, 40.0)));
        assert!(lens_active(&state, props));

        state.apply_action(&LensAction::Dismiss);
        assert!(!lens_active(&state, props));

        state.apply_action(&LensAction::Move(Point::new(60.0, 44.0)));
        assert!(lens_active(&state, props));
    }
}
