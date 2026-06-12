mod _shared;

use iced::time::Instant;
use iced::widget::column;
use iced::{Element, Size, Subscription, Task};
use magic_iced::{SpinningTextProps, spinning_text};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        SpinningTextExample::new,
        SpinningTextExample::update,
        SpinningTextExample::view,
    )
    .title("Spinning Text")
    .window(iced::window::Settings {
        size: Size::new(900.0, 520.0),
        ..Default::default()
    })
    .subscription(SpinningTextExample::subscription)
    .run()
}

struct SpinningTextExample {
    start: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl SpinningTextExample {
    fn new() -> (Self, Task<Message>) {
        let start = Instant::now();
        (
            Self {
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
        let body = spinning_text(
            "learn more • earn more • grow more •",
            self.elapsed,
            SpinningTextProps::new(),
        );

        _shared::shell("Spinning Text", column![body].spacing(24))
    }
}
