mod _shared;

use iced::time::Instant;
use iced::widget::{column, container};
use iced::{Alignment, Border, Element, Length, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{DotPatternProps, dot_pattern};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        DotPatternExample::new,
        DotPatternExample::update,
        DotPatternExample::view,
    )
    .title("Dot Pattern")
    .window(iced::window::Settings {
        size: Size::new(900.0, 620.0),
        ..Default::default()
    })
    .subscription(DotPatternExample::subscription)
    .run()
}

struct DotPatternExample {
    theme: Theme,
    start: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl DotPatternExample {
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
        let stage = container(dot_pattern(
            self.elapsed,
            DotPatternProps {
                width: Length::Fill,
                height: Length::Fill,
                cr: 300.0,
                ..DotPatternProps::new()
            },
            &self.theme,
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

        _shared::shell("Dot Pattern", column![stage].spacing(24).align_x(Alignment::Center))
    }
}
