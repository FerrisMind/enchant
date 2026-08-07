mod _shared;

use iced::time::Instant;
use iced::widget::{column, container, stack};
use iced::{Element, Length, Size, Subscription, Task};
use enchant_iced::{ShineBorderProps, shine_border};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        ShineBorderExample::new,
        ShineBorderExample::update,
        ShineBorderExample::view,
    )
    .title("Shine Border")
    .window(iced::window::Settings {
        size: Size::new(900.0, 520.0),
        ..Default::default()
    })
    .subscription(ShineBorderExample::subscription)
    .run()
}

struct ShineBorderExample {
    theme_shell: _shared::ThemeShell,
    start: Instant,
    elapsed: Duration,
    email: String,
    password: String,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Tick(Instant),
    EmailChanged(String),
    PasswordChanged(String),
    SignIn,
}

impl ShineBorderExample {
    fn new() -> (Self, Task<Message>) {
        let start = Instant::now();
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                start,
                elapsed: Duration::ZERO,
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

            Message::Tick(now) => {
                self.theme_shell.tick(now);
                self.elapsed = now.duration_since(self.start);
            }
            Message::EmailChanged(value) => self.email = value,
            Message::PasswordChanged(value) => self.password = value,
            Message::SignIn => {}
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let login = _shared::shadcn_forms::login_card_sign_in(
            &self.email,
            &self.password,
            Some(Message::EmailChanged),
            Some(Message::PasswordChanged),
            Some(Message::SignIn),
            self.theme_shell.blended_theme(),
        );

        let card = container(stack(vec![
            login,
            container(shine_border(self.elapsed, &ShineBorderProps::new()))
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        ]))
        .width(Length::Fixed(350.0))
        .height(Length::Fixed(360.0))
        .clip(true);

        _shared::themed_shell(
            "Shine Border",
            column![card].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
