mod _shared;

use iced::time::Instant;
use iced::widget::{column, text};
use iced::{Element, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{PulsatingButtonProps, pulsating_button};
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
    theme: Theme,
    start: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl PulsatingButtonExample {
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
        let body = pulsating_button(
            text("Join Affiliate Program").size(16),
            None,
            self.elapsed,
            PulsatingButtonProps::new(),
            &self.theme,
        );

        _shared::shell("Pulsating Button", column![body].spacing(24))
    }
}
