mod _shared;

use iced::widget::{column, text};
use iced::{Element, Size, Task};
use iced_shadcn::Theme;
use magic_iced::{InteractiveHoverButtonProps, interactive_hover_button};

pub fn main() -> iced::Result {
    iced::application(
        InteractiveHoverButtonExample::new,
        InteractiveHoverButtonExample::update,
        InteractiveHoverButtonExample::view,
    )
    .title("Interactive Hover Button")
    .window(iced::window::Settings {
        size: Size::new(1000.0, 700.0),
        ..Default::default()
    })
    .run()
}

struct InteractiveHoverButtonExample {
    theme: Theme,
}

#[derive(Debug, Clone)]
enum Message {
    Clicked,
}

impl InteractiveHoverButtonExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme: Theme::default(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        let _ = message;
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        _shared::shell(
            "Interactive Hover Button",
            column![
                interactive_hover_button(
                    text("Hover Me"),
                    Some(Message::Clicked),
                    InteractiveHoverButtonProps::new(),
                    &self.theme,
                ),
            ]
            .spacing(20),
        )
    }
}
