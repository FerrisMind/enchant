mod _shared;

use iced::time::Instant;
use iced::widget::column;
use iced::{Element, Size, Subscription, Task};
use magic_iced::{AnimatedGradientTextProps, animated_gradient_text};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        AnimatedGradientTextExample::new,
        AnimatedGradientTextExample::update,
        AnimatedGradientTextExample::view,
    )
    .title("Animated Gradient Text")
    .window(iced::window::Settings {
        size: Size::new(900.0, 420.0),
        ..Default::default()
    })
    .subscription(AnimatedGradientTextExample::subscription)
    .run()
}

struct AnimatedGradientTextExample {
    start: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl AnimatedGradientTextExample {
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
        let body = animated_gradient_text(
            "Introducing Magic UI",
            self.elapsed,
            AnimatedGradientTextProps::new().font_size(20),
        );

        _shared::shell("Animated Gradient Text", column![body].spacing(24))
    }
}
