mod _shared;

use iced::time::Instant;
use iced::widget::{column, text};
use iced::{Element, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{RainbowButtonProps, rainbow_button};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        RainbowButtonExample::new,
        RainbowButtonExample::update,
        RainbowButtonExample::view,
    )
    .title("Rainbow Button")
    .window(iced::window::Settings {
        size: Size::new(900.0, 420.0),
        ..Default::default()
    })
    .subscription(RainbowButtonExample::subscription)
    .run()
}

struct RainbowButtonExample {
    theme: Theme,
    start: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl RainbowButtonExample {
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
        match message {
            Message::Tick(now) => self.elapsed = now.duration_since(self.start),
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let body = rainbow_button(
            text("Get Unlimited Access").size(16),
            None,
            self.elapsed,
            RainbowButtonProps::new(),
            &self.theme,
        );

        _shared::shell("Rainbow Button", column![body].spacing(24))
    }
}
