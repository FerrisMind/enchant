mod _shared;

use iced::time::Instant;
use iced::widget::{column, text};
use iced::{Element, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{ShinyButtonProps, shiny_button};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        ShinyButtonExample::new,
        ShinyButtonExample::update,
        ShinyButtonExample::view,
    )
    .title("Shiny Button")
    .window(iced::window::Settings {
        size: Size::new(900.0, 420.0),
        ..Default::default()
    })
    .subscription(ShinyButtonExample::subscription)
    .run()
}

struct ShinyButtonExample {
    theme: Theme,
    start: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl ShinyButtonExample {
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
        let body = shiny_button(
            text("Shiny Button").size(16),
            None,
            self.elapsed,
            ShinyButtonProps::new(),
            &self.theme,
        );

        _shared::shell("Shiny Button", column![body].spacing(24))
    }
}
