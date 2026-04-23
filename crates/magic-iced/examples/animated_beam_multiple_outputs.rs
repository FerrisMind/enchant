mod _shared;

use iced::time::Instant;
use iced::widget::{column, container, stack, text};
use iced::{Element, Length, Point, Rectangle, Size, Subscription, Task};
use magic_iced::{AnchorBounds, AnimatedBeamProps, animated_beam};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        AnimatedBeamMultipleOutputsExample::new,
        AnimatedBeamMultipleOutputsExample::update,
        AnimatedBeamMultipleOutputsExample::view,
    )
    .title("Animated Beam Multiple Outputs")
    .window(iced::window::Settings {
        size: Size::new(1200.0, 820.0),
        ..Default::default()
    })
    .subscription(AnimatedBeamMultipleOutputsExample::subscription)
    .run()
}

struct AnimatedBeamMultipleOutputsExample {
    started_at: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl AnimatedBeamMultipleOutputsExample {
    fn new() -> (Self, Task<Message>) {
        let started_at = Instant::now();
        (
            Self {
                started_at,
                elapsed: Duration::ZERO,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick(now) => self.elapsed = now.duration_since(self.started_at),
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let source = AnchorBounds::from_bounds(Rectangle::new(
            Point::new(140.0, 210.0),
            Size::new(180.0, 88.0),
        ));
        let output_a = AnchorBounds::from_bounds(Rectangle::new(
            Point::new(820.0, 120.0),
            Size::new(160.0, 76.0),
        ));
        let output_b = AnchorBounds::from_bounds(Rectangle::new(
            Point::new(840.0, 280.0),
            Size::new(160.0, 76.0),
        ));
        let output_c = AnchorBounds::from_bounds(Rectangle::new(
            Point::new(800.0, 440.0),
            Size::new(180.0, 88.0),
        ));

        let beams = stack(vec![
            animated_beam(
                source,
                output_a,
                self.elapsed,
                AnimatedBeamProps::new().reverse(false),
            ),
            animated_beam(
                source,
                output_b,
                self.elapsed,
                AnimatedBeamProps {
                    curvature: 24.0,
                    delay_secs: 0.4,
                    path_width: 2.4,
                    ..AnimatedBeamProps::new()
                },
            ),
            animated_beam(
                source,
                output_c,
                self.elapsed,
                AnimatedBeamProps {
                    curvature: -16.0,
                    delay_secs: 0.8,
                    reverse: true,
                    gradient_start_color: iced::Color::from_rgb8(0x6B, 0xC9, 0xFF),
                    gradient_stop_color: iced::Color::from_rgb8(0xFF, 0x8A, 0xC2),
                    ..AnimatedBeamProps::new()
                },
            ),
        ]);

        _shared::shell(
            "Animated Beam Multiple Outputs",
            column![
                text("One source fans out into three targets."),
                container(beams)
                    .width(Length::Fill)
                    .height(Length::Fixed(520.0)),
            ]
            .spacing(20),
        )
    }
}
