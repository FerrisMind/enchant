mod _shared;

use iced::time::Instant;
use iced::widget::{column, container, stack, text};
use iced::{Alignment, Border, Color, Element, Length, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{RetroGridProps, retro_grid};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        RetroGridExample::new,
        RetroGridExample::update,
        RetroGridExample::view,
    )
    .title("Retro Grid")
    .window(iced::window::Settings {
        size: Size::new(900.0, 620.0),
        ..Default::default()
    })
    .subscription(RetroGridExample::subscription)
    .run()
}

struct RetroGridExample {
    theme: Theme,
    start: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl RetroGridExample {
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
            container(retro_grid(
                self.elapsed,
                RetroGridProps {
                    width: Length::Fill,
                    height: Length::Fill,
                    ..RetroGridProps::new()
                },
                &self.theme,
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
            container(
                text("Retro Grid")
                    .size(64)
                    .style(|_| iced::widget::text::Style {
                        color: Some(Color::from_rgb8(0xFF, 0x29, 0x75)),
                    }),
            )
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

        _shared::shell("Retro Grid", column![stage].spacing(24).align_x(Alignment::Center))
    }
}
