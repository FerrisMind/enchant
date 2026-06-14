mod _shared;

use iced::time::Instant;
use iced::widget::column;
use iced::{Element, Size, Subscription, Task};
use magic_iced::{SparklesTextProps, sparkles_text};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        SparklesTextExample::new,
        SparklesTextExample::update,
        SparklesTextExample::view,
    )
    .title("Sparkles Text")
    .window(iced::window::Settings {
        size: Size::new(900.0, 420.0),
        ..Default::default()
    })
    .subscription(SparklesTextExample::subscription)
    .run()
}

struct SparklesTextExample {
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

impl SparklesTextExample {
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
        let body = sparkles_text(
            "Magic UI",
            self.elapsed,
            SparklesTextProps {
                font_size: 48,
                text_color: Some(self.theme_shell.blended_foreground(self.theme_shell.now)),
                ..SparklesTextProps::new()
            },
            self.theme_shell.theme(),
        );

        _shared::themed_shell("Sparkles Text", column![body].spacing(24), &self.theme_shell, Message::ToggleTheme)
    }
}
