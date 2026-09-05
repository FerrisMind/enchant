mod _shared;

use enchant_iced::BorderBeam;
use iced::time::Instant;
use iced::widget::{column, container, stack};
use iced::{Element, Length, Size, Subscription, Task};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        BorderBeamExample::new,
        BorderBeamExample::update,
        BorderBeamExample::view,
    )
    .title("Border Beam")
    .window(iced::window::Settings {
        size: Size::new(900.0, 520.0),
        ..Default::default()
    })
    .subscription(BorderBeamExample::subscription)
    .run()
}

struct BorderBeamExample {
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
    Register,
    Login,
}

impl BorderBeamExample {
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
            Message::Register | Message::Login => {}
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let now = self.theme_shell.now;
        let transition = self.theme_shell.controller.is_transition_active(now);
        let login = _shared::shadcn_forms::login_card_dual_footer(
            &self.email,
            &self.password,
            Some(Message::EmailChanged),
            Some(Message::PasswordChanged),
            Some(Message::Register),
            Some(Message::Login),
            self.theme_shell.blended_theme(),
        );

        let card = container(stack(vec![
            login,
            container(
                BorderBeam::new()
                    .size(100.0)
                    .duration_secs(8.0)
                    .radius(12.0)
                    .beam_enabled(!transition)
                    .into_element(self.elapsed),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
        ]))
        .width(Length::Fixed(350.0))
        .height(Length::Fixed(380.0))
        .clip(true);

        _shared::themed_shell(
            "Border Beam",
            column![card].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
