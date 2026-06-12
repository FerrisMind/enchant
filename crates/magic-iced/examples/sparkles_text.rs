mod _shared;

use iced::time::Instant;
use iced::widget::column;
use iced::{Element, Size, Subscription, Task};
use iced_shadcn::Theme;
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
    theme: Theme,
    start: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl SparklesTextExample {
    fn new() -> (Self, Task<Message>) {
        let start = Instant::now();
        (
            Self {
                theme: Theme::default(),
                start,
                elapsed: Duration::ZERO,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        if let Message::Tick(now) = message {
            self.elapsed = now.duration_since(self.start);
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
                ..SparklesTextProps::new()
            },
            &self.theme,
        );

        _shared::shell("Sparkles Text", column![body].spacing(24))
    }
}
