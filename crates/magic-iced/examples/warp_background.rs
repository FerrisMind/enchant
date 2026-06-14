mod _shared;

use iced::time::Instant;
use iced::widget::{column, container, text};
use iced::{Alignment, Element, Length, Size, Subscription, Task};
use magic_iced::{WarpBackgroundProps, warp_background};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        WarpBackgroundExample::new,
        WarpBackgroundExample::update,
        WarpBackgroundExample::view,
    )
    .title("Warp Background")
    .window(iced::window::Settings {
        size: Size::new(900.0, 620.0),
        ..Default::default()
    })
    .subscription(WarpBackgroundExample::subscription)
    .run()
}

struct WarpBackgroundExample {
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

impl WarpBackgroundExample {
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
                return _shared::ThemeShell::measure_toggle_anchor()
                    .map(Message::AnchorsMeasured);
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
        let card = container(
            column![
                text("Congratulations on Your Promotion!").size(20),
                text(
                    "Your hard work and dedication have paid off. We're thrilled to see you take this next step in your career. Keep up the fantastic work!"
                )
                .size(14)
                .width(Length::Fixed(280.0)),
            ]
            .spacing(10),
        )
        .padding(16)
        .width(Length::Fixed(320.0));

        let body = warp_background(
            card,
            self.elapsed,
            WarpBackgroundProps {
                width: Length::Fixed(720.0),
                height: Length::Fixed(500.0),
                ..WarpBackgroundProps::new()
            },
            self.theme_shell.theme(),
        );

        _shared::themed_shell("Warp Background", column![body].spacing(24).align_x(Alignment::Center), &self.theme_shell, Message::ToggleTheme)
    }
}
