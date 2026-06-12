mod _shared;

use iced::time::Instant;
use iced::widget::{column, container, stack, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Size, Subscription, Task};
use magic_iced::{OrbitingCirclesProps, orbiting_circles};
use std::time::Duration;

const ICON_COLORS: [Color; 4] = [
    Color::from_rgb8(0x25, 0xD3, 0x66),
    Color::from_rgb8(0x3B, 0x82, 0xF6),
    Color::from_rgb8(0xA8, 0x55, 0xF7),
    Color::from_rgb8(0xF9, 0x73, 0x16),
];

pub fn main() -> iced::Result {
    iced::application(
        OrbitingCirclesExample::new,
        OrbitingCirclesExample::update,
        OrbitingCirclesExample::view,
    )
    .title("Orbiting Circles")
    .window(iced::window::Settings {
        size: Size::new(900.0, 620.0),
        ..Default::default()
    })
    .subscription(OrbitingCirclesExample::subscription)
    .run()
}

struct OrbitingCirclesExample {
    start: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl OrbitingCirclesExample {
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

    fn orbit_icon(&self, color: Color, size: f32) -> Element<'_, Message> {
        container(text(" "))
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .style(move |_| container::Style {
                background: Some(Background::Color(color)),
                border: Border {
                    radius: (size / 2.0).into(),
                    ..Border::default()
                },
                ..container::Style::default()
            })
            .into()
    }

    fn view(&self) -> Element<'_, Message> {
        let outer: Vec<Element<'_, Message>> = ICON_COLORS
            .iter()
            .map(|&color| self.orbit_icon(color, 30.0))
            .collect();
        let inner: Vec<Element<'_, Message>> = ICON_COLORS
            .iter()
            .map(|&color| self.orbit_icon(color, 40.0))
            .collect();

        let stage = container(stack(vec![
            orbiting_circles(
                outer,
                self.elapsed,
                OrbitingCirclesProps {
                    width: Length::Fill,
                    height: Length::Fill,
                    icon_size: 30.0,
                    radius: 100.0,
                    reverse: true,
                    speed: 2.0,
                    ..OrbitingCirclesProps::new()
                },
            ),
            orbiting_circles(
                inner,
                self.elapsed,
                OrbitingCirclesProps {
                    width: Length::Fill,
                    height: Length::Fill,
                    icon_size: 40.0,
                    radius: 160.0,
                    ..OrbitingCirclesProps::new()
                },
            ),
        ]))
        .width(Length::Fixed(720.0))
        .height(Length::Fixed(500.0));

        _shared::shell(
            "Orbiting Circles",
            column![stage].spacing(24).align_x(Alignment::Center),
        )
    }
}
