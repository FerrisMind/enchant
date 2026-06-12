mod _shared;

use iced::time::Instant;
use iced::widget::column;
use iced::{Element, Size, Subscription, Task};
use magic_iced::{AuroraTextProps, aurora_text};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        AuroraTextExample::new,
        AuroraTextExample::update,
        AuroraTextExample::view,
    )
    .title("Aurora Text")
    .window(iced::window::Settings {
        size: Size::new(1000.0, 760.0),
        ..Default::default()
    })
    .subscription(AuroraTextExample::subscription)
    .run()
}

struct AuroraTextExample {
    start: Instant,
    elapsed: Duration,
    props: AuroraTextProps,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl AuroraTextExample {
    fn new() -> (Self, Task<Message>) {
        let start = Instant::now();
        (
            Self {
                start,
                elapsed: Duration::ZERO,
                props: AuroraTextProps::new()
                    .prefix("Ship ")
                    .font_size(72),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick(now) => {
                self.elapsed = now.duration_since(self.start);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let body = column![aurora_text::<Message>("beautiful", self.elapsed, &self.props)]
            .spacing(24)
            .align_x(iced::Alignment::Center);

        _shared::shell("Aurora Text", body)
    }
}
