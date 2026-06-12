mod _shared;

use iced::widget::{column, container};
use iced::{Alignment, Border, Element, Length, Size, Task};
use iced_shadcn::Theme;
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
    .run()
}

struct StripedPatternExample {
    theme: Theme,
}

#[derive(Debug, Clone)]
enum Message {}

impl StripedPatternExample {
    fn new() -> (Self, Task<Message>) {
        (Self { theme: Theme::default() }, Task::none())
    }

    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let stage = container(striped_pattern(
            StripedPatternProps {
                width: Length::Fill,
                height: Length::Fill,
                ..StripedPatternProps::new()
            },
            &self.theme,
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

        _shared::shell(
            "Striped Pattern",
            column![stage].spacing(24).align_x(Alignment::Center),
        )
    }
}
