mod _shared;

use iced::widget::{column, text};
use iced::time::Instant;
use std::time::Duration;
use iced::{Element, Size, Subscription, Task};
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
    .subscription(InteractiveHoverButtonExample::subscription)
        
    .run()
}

struct InteractiveHoverButtonExample {
    theme_shell: _shared::ThemeShell,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    Tick(Instant),
    AnchorsMeasured(magic_iced::AnchorMeasurements),
    Clicked,
}

impl InteractiveHoverButtonExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        let _ = message;
        Task::none()
    }


    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        _shared::themed_shell("Interactive Hover Button", column![
                interactive_hover_button(
                    text("Hover Me"),
                    Some(Message::Clicked),
                    InteractiveHoverButtonProps::new(),
                    self.theme_shell.theme(),
                ),
            ]
            .spacing(20),
            &self.theme_shell, Message::ToggleTheme)
    }
}
