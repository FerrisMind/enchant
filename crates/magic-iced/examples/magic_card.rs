mod _shared;

use iced::widget::column;
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
    email: String,
    password: String,
}

#[derive(Debug, Clone)]
enum Message {
    EmailChanged(String),
    PasswordChanged(String),
    SignIn,
}

impl MagicCardExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme: Theme::default(),
                email: String::new(),
                password: String::new(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::EmailChanged(value) => self.email = value,
            Message::PasswordChanged(value) => self.password = value,
            Message::SignIn => {}
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let inner = _shared::shadcn_forms::login_form_sign_in(
            &self.email,
            &self.password,
            Some(Message::EmailChanged),
            Some(Message::PasswordChanged),
            Some(Message::SignIn),
            &self.theme,
        );

        let card = magic_card(
            inner,
            MagicCardProps::new()
                .width(Length::Fixed(384.0))
                .height(Length::Fixed(360.0))
                .radius(20.0),
            &self.theme,
        );

        _shared::shell("Magic Card", column![card].spacing(24))
    }
}
