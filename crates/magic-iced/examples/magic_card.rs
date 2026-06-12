mod _shared;

use iced::widget::{button, column, text};
use iced::{Element, Length, Size, Task};
use iced_shadcn::Theme;
use magic_iced::{MagicCardProps, magic_card};

pub fn main() -> iced::Result {
    iced::application(
        MagicCardExample::new,
        MagicCardExample::update,
        MagicCardExample::view,
    )
    .title("Magic Card")
    .window(iced::window::Settings {
        size: Size::new(900.0, 520.0),
        ..Default::default()
    })
    .run()
}

struct MagicCardExample {
    theme: Theme,
}

#[derive(Debug, Clone)]
enum Message {}

impl MagicCardExample {
    fn new() -> (Self, Task<Message>) {
        (Self { theme: Theme::default() }, Task::none())
    }

    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    fn login_card(&self) -> Element<'_, Message> {
        column![
            column![
                text("Login").size(22),
                text("Enter your credentials to access your account").size(14),
            ]
            .spacing(6),
            column![
                text("Email").size(13),
                text("name@example.com").size(14),
                text("Password").size(13),
                text("••••••••").size(14),
            ]
            .spacing(8),
            button(text("Sign In")).width(Length::Fill),
        ]
        .spacing(16)
        .padding(20)
        .into()
    }

    fn view(&self) -> Element<'_, Message> {
        let card = magic_card(
            self.login_card(),
            MagicCardProps::new()
                .width(Length::Fixed(384.0))
                .height(Length::Fixed(360.0))
                .radius(20.0),
            &self.theme,
        );

        _shared::shell("Magic Card", column![card].spacing(24))
    }
}
