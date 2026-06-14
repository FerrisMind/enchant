mod _shared;

use iced::time::Instant;
use iced::widget::{column, text};
use iced::{Element, Size, Subscription, Task};
use magic_iced::{ScrollProgressProps, scroll_progress};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        ScrollProgressExample::new,
        ScrollProgressExample::update,
        ScrollProgressExample::view,
    )
    .title("Scroll Progress")
    .window(iced::window::Settings {
        size: Size::new(900.0, 360.0),
        ..Default::default()
    })
    .subscription(ScrollProgressExample::subscription)
    .run()
}

struct ScrollProgressExample {
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

impl ScrollProgressExample {
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
            Message::Tick(now) => {
                self.theme_shell.tick(now);

                self.elapsed = now.duration_since(self.start);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let progress = (self.elapsed.as_secs_f32() * 0.15).sin() * 0.5 + 0.5;
        let bar = scroll_progress(
            ScrollProgressProps {
                height: 3.0,
                ..ScrollProgressProps::new(progress)
            },
            self.theme_shell.theme(),
        );

        let body = column![
            bar,
            text("Note: The scroll progress animates along the top edge.").size(14),
        ]
        .spacing(16);

        _shared::themed_shell(
            "Scroll Progress",
            body,
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
