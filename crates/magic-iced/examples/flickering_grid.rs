mod _shared;

use iced::time::Instant;
use iced::widget::{column, container};
use iced::{Alignment, Border, Color, Element, Length, Size, Subscription, Task};
use magic_iced::{FlickeringGridProps, flickering_grid};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        FlickeringGridExample::new,
        FlickeringGridExample::update,
        FlickeringGridExample::view,
    )
    .title("Flickering Grid")
    .window(iced::window::Settings {
        size: Size::new(900.0, 620.0),
        ..Default::default()
    })
    .subscription(FlickeringGridExample::subscription)
    .run()
}

struct FlickeringGridExample {
    start: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl FlickeringGridExample {
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
        match message {
            Message::Tick(now) => self.elapsed = now.duration_since(self.start),
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let stage = container(flickering_grid(
            self.elapsed,
            FlickeringGridProps {
                width: Length::Fill,
                height: Length::Fill,
                color: Color::from_rgb8(0x6B, 0x72, 0x80),
                max_opacity: 0.5,
                flicker_chance: 0.1,
                ..FlickeringGridProps::new()
            },
        ))
        .width(Length::Fixed(720.0))
        .height(Length::Fixed(500.0))
        .style(|_| container::Style {
            border: Border {
                width: 1.0,
                radius: 12.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        });

        _shared::shell(
            "Flickering Grid",
            column![stage].spacing(24).align_x(Alignment::Center),
        )
    }
}
