mod _shared;

use iced::time::Instant;
use iced::widget::{column, container, stack, text};
use iced::{Alignment, Border, Element, Length, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{MeteorsProps, meteors};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        MeteorsExample::new,
        MeteorsExample::update,
        MeteorsExample::view,
    )
    .title("Meteors")
    .window(iced::window::Settings {
        size: Size::new(900.0, 620.0),
        ..Default::default()
    })
    .subscription(MeteorsExample::subscription)
    .run()
}

struct MeteorsExample {
    theme: Theme,
    start: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl MeteorsExample {
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
        let stage = container(stack(vec![
            container(meteors(
                self.elapsed,
                MeteorsProps {
                    width: Length::Fill,
                    height: Length::Fill,
                    number: 30,
                    ..MeteorsProps::new()
                },
                &self.theme,
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
            container(text("Meteors").size(72))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into(),
        ]))
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

        _shared::shell("Meteors", column![stage].spacing(24).align_x(Alignment::Center))
    }
}
