mod _shared;

use iced::time::Instant;
use iced::widget::{column, text};
use iced::{Element, Size, Subscription, Task};
use iced_shadcn::{ButtonProps, ButtonSize, ButtonVariant, Theme};
use magic_iced::{ShimmerButtonProps, shimmer_button};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        ShimmerButtonExample::new,
        ShimmerButtonExample::update,
        ShimmerButtonExample::view,
    )
    .title("Shimmer Button")
    .window(iced::window::Settings {
        size: Size::new(900.0, 420.0),
        ..Default::default()
    })
    .subscription(ShimmerButtonExample::subscription)
    .run()
}

struct ShimmerButtonExample {
    theme_shell: _shared::ThemeShell,
    start: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(magic_iced::AnchorMeasurements),
    Tick(Instant),
}

impl ShimmerButtonExample {
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
        let body = shimmer_button(
            text("Shimmer Button").size(18),
            None,
            self.elapsed,
            ShimmerButtonProps::new().button(
                ButtonProps::new()
                    .variant(ButtonVariant::Default)
                    .size(ButtonSize::Size3),
            ),
            self.theme_shell.theme(),
        );

        _shared::themed_shell(
            "Shimmer Button",
            column![body].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
