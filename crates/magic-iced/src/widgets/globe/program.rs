use crate::widgets::globe::primitive::GlobePrimitive;
use crate::widgets::globe::{GlobeAction, GlobeProps, GlobeState};
use iced::mouse;
use iced::widget::shader::{self, Action};
use iced::{Event, Rectangle};

pub struct GlobeShaderProgram<'a, Message> {
    pub state: &'a GlobeState,
    pub props: &'a GlobeProps,
    pub on_action: Box<dyn Fn(GlobeAction) -> Message + 'a>,
}

impl<Message: Clone + 'static> shader::Program<Message> for GlobeShaderProgram<'_, Message> {
    type State = ();
    type Primitive = GlobePrimitive;

    fn update(
        &self,
        _state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        let position = cursor.position_over(bounds)?;

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                Some(
                    Action::publish((self.on_action)(GlobeAction::Press))
                        .and_capture(),
                )
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                Some(
                    Action::publish((self.on_action)(GlobeAction::Release))
                        .and_capture(),
                )
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                Some(Action::publish((self.on_action)(GlobeAction::Move(position))))
            }
            Event::Mouse(mouse::Event::CursorLeft) => {
                Some(Action::publish((self.on_action)(GlobeAction::Exit)))
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        _bounds: Rectangle,
    ) -> Self::Primitive {
        GlobePrimitive::new(self.state, self.props)
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        mouse::Interaction::Grab
    }
}
