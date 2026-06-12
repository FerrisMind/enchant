mod _shared;

use iced::time::Instant;
use iced::widget::{column, text};
use iced::{Element, Size, Subscription, Task};
use iced_shadcn::Theme;
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
    theme: Theme,
    start: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl ScrollProgressExample {
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
        let progress = (self.elapsed.as_secs_f32() * 0.15).sin() * 0.5 + 0.5;
        let bar = scroll_progress(
            ScrollProgressProps {
                height: 3.0,
                ..ScrollProgressProps::new(progress)
            },
            &self.theme,
        );

        let body = column![
            bar,
            text("Note: The scroll progress animates along the top edge.").size(14),
        ]
        .spacing(16);

        _shared::shell("Scroll Progress", body)
    }
}
