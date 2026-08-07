mod _shared;

use iced::time::Instant;

use iced::widget::column;
use iced::{Element, Length, Size, Task};
use enchant_iced::{MagicCardProps, magic_card};

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
    .subscription(MagicCardExample::subscription)
    .run()
}

struct MagicCardExample {
    theme_shell: _shared::ThemeShell,
    email: String,
    password: String,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    Tick(Instant),
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    EmailChanged(String),
    PasswordChanged(String),
    SignIn,
}

impl MagicCardExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                email: String::new(),
                password: String::new(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleTheme => {
                return _shared::ThemeShell::measure_toggle_anchor().map(Message::AnchorsMeasured);
            }
            Message::AnchorsMeasured(measurements) => {
                self.theme_shell.apply_anchor_and_toggle(&measurements);
            }

            Message::EmailChanged(value) => self.email = value,
            Message::PasswordChanged(value) => self.password = value,
            Message::SignIn => {}
            Message::Tick(now) => {
                self.theme_shell.tick(now);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let inner = _shared::shadcn_forms::login_form_sign_in(
            &self.email,
            &self.password,
            Some(Message::EmailChanged),
            Some(Message::PasswordChanged),
            Some(Message::SignIn),
            self.theme_shell.theme(),
        );

        let card = magic_card(
            inner,
            MagicCardProps::new()
                .width(Length::Fixed(384.0))
                .height(Length::Fixed(360.0))
                .radius(20.0),
            self.theme_shell.theme(),
        );

        _shared::themed_shell(
            "Magic Card",
            column![card].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
