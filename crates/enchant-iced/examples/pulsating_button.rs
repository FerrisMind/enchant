mod _shared;

use enchant_iced::{PulsatingButtonProps, pulsating_button};
use iced::time::Instant;
use iced::widget::{column, text};
use iced::{Element, Size, Subscription, Task};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        PulsatingButtonExample::new,
        PulsatingButtonExample::update,
        PulsatingButtonExample::view,
    )
    .title("Pulsating Button")
    .window(iced::window::Settings {
        size: Size::new(900.0, 420.0),
        ..Default::default()
    })
    .subscription(PulsatingButtonExample::subscription)
    .run()
}

struct PulsatingButtonExample {
    theme_shell: _shared::ThemeShell,
    start: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Tick(Instant),
}

impl PulsatingButtonExample {
    fn new() -> (Self, Task<Message>) {
        let start = Instant::now();
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                start,
                elapsed: Duration::ZERO,
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

            Message::Tick(now) => self.elapsed = now.duration_since(self.start),
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let body = pulsating_button(
            text("Join Affiliate Program").size(16),
            None,
            self.elapsed,
            PulsatingButtonProps::new(),
            self.theme_shell.theme(),
        );

        _shared::themed_shell(
            "Pulsating Button",
            column![body].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
