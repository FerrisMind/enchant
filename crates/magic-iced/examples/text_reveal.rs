mod _shared;

use iced::time::Instant;
use iced::widget::column;
use iced::{Element, Size, Subscription, Task};
use magic_iced::{TextRevealProps, text_reveal};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        TextRevealExample::new,
        TextRevealExample::update,
        TextRevealExample::view,
    )
    .title("Text Reveal")
    .window(iced::window::Settings {
        size: Size::new(1000.0, 420.0),
        ..Default::default()
    })
    .subscription(TextRevealExample::subscription)
    .run()
}

struct TextRevealExample {
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

impl TextRevealExample {
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
        let progress = (self.elapsed.as_secs_f32() * 0.18).rem_euclid(1.0);
        let body = text_reveal(
            "Magic UI will change the way you design.",
            TextRevealProps::new().progress(progress).font_size(36),
            self.theme_shell.theme(),
        );

        _shared::themed_shell("Text Reveal", column![body].spacing(24), &self.theme_shell, Message::ToggleTheme)
    }
}
