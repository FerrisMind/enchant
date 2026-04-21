use iced::widget::{column, text, container};
use iced::{Element, Length, Task, Size};

pub fn main() -> iced::Result {
    iced::application(
        BorderTrackExample::new,
        BorderTrackExample::update,
        BorderTrackExample::view,
    )
    .title("BorderTrack Example")
    .window(iced::window::Settings {
        size: Size::new(800.0, 600.0),
        ..Default::default()
    })
    .run()
}

#[derive(Default)]
struct BorderTrackExample {}

#[derive(Debug, Clone)]
enum Message {}

impl BorderTrackExample {
    fn new() -> (Self, Task<Message>) {
        (Self::default(), Task::none())
    }

    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        container(
            column![
                text("BorderTrack Example").size(24)
            ]
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }
}
