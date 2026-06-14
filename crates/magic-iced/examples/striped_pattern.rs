mod _shared;

use iced::time::Instant;
use std::time::Duration;

use iced::widget::{column, container};
use iced::{Alignment, Border, Element, Length, Size, Task};
use magic_iced::{StripedPatternProps, striped_pattern};

pub fn main() -> iced::Result {
    iced::application(
        StripedPatternExample::new,
        StripedPatternExample::update,
        StripedPatternExample::view,
    )
    .title("Striped Pattern")
    .window(iced::window::Settings {
        size: Size::new(900.0, 620.0),
        ..Default::default()
    })
    .subscription(StripedPatternExample::subscription)
        
    .run()
}

struct StripedPatternExample {
    theme_shell: _shared::ThemeShell,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    Tick(Instant),
    AnchorsMeasured(magic_iced::AnchorMeasurements),
}

impl StripedPatternExample {
    fn new() -> (Self, Task<Message>) {
        (Self { theme_shell: _shared::ThemeShell::new() }, Task::none())
    }

    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }


    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let stage = container(striped_pattern(
            StripedPatternProps {
                width: Length::Fill,
                height: Length::Fill,
                ..StripedPatternProps::new()
            },
            self.theme_shell.theme(),
        ))
        .width(Length::Fixed(720.0))
        .height(Length::Fixed(500.0))
        .style(|_| container::Style {
            border: Border {
                width: 1.0,
                radius: 12.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        });

        _shared::themed_shell("Striped Pattern", column![stage].spacing(24).align_x(Alignment::Center), &self.theme_shell, Message::ToggleTheme,
        )
    }
}
