use iced::widget::{column, container, text};
use iced::{Element, Length, Size, Task};

pub fn main() -> iced::Result {
    iced::application(
        MeteorsExample::new,
        MeteorsExample::update,
        MeteorsExample::view,
    )
    .title("Meteors Example")
    .window(iced::window::Settings {
        size: Size::new(800.0, 600.0),
        ..Default::default()
    })
    .run()
}

#[derive(Default)]
struct MeteorsExample {}

#[derive(Debug, Clone)]
enum Message {}

impl MeteorsExample {
    fn new() -> (Self, Task<Message>) {
        (Self::default(), Task::none())
    }

    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        container(column![text("Meteors Example").size(24)])
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}
