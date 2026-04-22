use iced::widget::{column, container, text};
use iced::{Element, Length, Size, Task};

pub fn main() -> iced::Result {
    iced::application(
        Text3dFlipExample::new,
        Text3dFlipExample::update,
        Text3dFlipExample::view,
    )
    .title("Text3dFlip Example")
    .window(iced::window::Settings {
        size: Size::new(800.0, 600.0),
        ..Default::default()
    })
    .run()
}

#[derive(Default)]
struct Text3dFlipExample {}

#[derive(Debug, Clone)]
enum Message {}

impl Text3dFlipExample {
    fn new() -> (Self, Task<Message>) {
        (Self::default(), Task::none())
    }

    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        container(column![text("Text3dFlip Example").size(24)])
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}
