use iced::widget::{column, text, container};
use iced::{Element, Length, Task, Size};

pub fn main() -> iced::Result {
    iced::application(
        TextRevealExample::new,
        TextRevealExample::update,
        TextRevealExample::view,
    )
    .title("TextReveal Example")
    .window(iced::window::Settings {
        size: Size::new(800.0, 600.0),
        ..Default::default()
    })
    .run()
}

#[derive(Default)]
struct TextRevealExample {}

#[derive(Debug, Clone)]
enum Message {}

impl TextRevealExample {
    fn new() -> (Self, Task<Message>) {
        (Self::default(), Task::none())
    }

    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        container(
            column![
                text("TextReveal Example").size(24)
            ]
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }
}
