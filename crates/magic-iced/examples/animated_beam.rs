mod _shared;

use iced::time::Instant;
use iced::widget::{column, container, stack};
use iced::{Element, Length, Point, Rectangle, Size, Subscription, Task};
use magic_iced::{AnchorBounds, AnimatedBeamProps, animated_beam};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        AnimatedBeamExample::new,
        AnimatedBeamExample::update,
        AnimatedBeamExample::view,
    )
    .title("AnimatedBeam Example")
    .window(iced::window::Settings {
        size: Size::new(1200.0, 820.0),
        ..Default::default()
    })
    .subscription(AnimatedBeamExample::subscription)
    .run()
}

struct AnimatedBeamExample {
    active: usize,
    started_at: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    Select(usize),
    Tick(Instant),
}

impl AnimatedBeamExample {
    fn new() -> (Self, Task<Message>) {
        let started_at = Instant::now();
        (
            Self {
                active: 0,
                started_at,
                elapsed: Duration::ZERO,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Select(index) => {
                self.active = index.min(3);
                self.started_at = Instant::now();
                self.elapsed = Duration::ZERO;
            }
            Message::Tick(now) => self.elapsed = now.duration_since(self.started_at),
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let labels = [
            "Demo",
            "Unidirectional",
            "Bidirectional",
            "Multiple Inputs",
        ];
        let scene = match self.active {
            0 => beam_scene(vec![beam_pair(
                self.elapsed,
                    AnchorBounds::from_bounds(Rectangle::new(
                        Point::new(120.0, 220.0),
                        iced::Size::new(180.0, 88.0),
                    )),
                    AnchorBounds::from_bounds(Rectangle::new(
                        Point::new(820.0, 200.0),
                        iced::Size::new(180.0, 88.0),
                    )),
                    AnimatedBeamProps::new(),
                )]),
            1 => beam_scene(vec![beam_pair(
                self.elapsed,
                    AnchorBounds::from_bounds(Rectangle::new(
                        Point::new(100.0, 160.0),
                        iced::Size::new(180.0, 88.0),
                    )),
                    AnchorBounds::from_bounds(Rectangle::new(
                        Point::new(860.0, 120.0),
                        iced::Size::new(180.0, 88.0),
                    )),
                    AnimatedBeamProps::new().reverse(false),
                )]),
            2 => beam_scene(vec![beam_pair(
                self.elapsed,
                    AnchorBounds::from_bounds(Rectangle::new(
                        Point::new(120.0, 420.0),
                        iced::Size::new(180.0, 88.0),
                    )),
                    AnchorBounds::from_bounds(Rectangle::new(
                        Point::new(840.0, 160.0),
                        iced::Size::new(180.0, 88.0),
                    )),
                    AnimatedBeamProps {
                        curvature: -42.0,
                        reverse: true,
                        delay_secs: 0.6,
                        ..AnimatedBeamProps::new()
                    },
                )]),
            _ => beam_scene(vec![
                    beam_pair(
                        self.elapsed,
                        AnchorBounds::from_bounds(Rectangle::new(
                            Point::new(110.0, 180.0),
                            iced::Size::new(180.0, 88.0),
                        )),
                        AnchorBounds::from_bounds(Rectangle::new(
                            Point::new(700.0, 120.0),
                            iced::Size::new(180.0, 88.0),
                        )),
                        AnimatedBeamProps {
                            curvature: 28.0,
                            ..AnimatedBeamProps::new()
                        },
                    ),
                    beam_pair(
                        self.elapsed,
                        AnchorBounds::from_bounds(Rectangle::new(
                            Point::new(110.0, 180.0),
                            iced::Size::new(180.0, 88.0),
                        )),
                        AnchorBounds::from_bounds(Rectangle::new(
                            Point::new(700.0, 360.0),
                            iced::Size::new(180.0, 88.0),
                        )),
                        AnimatedBeamProps {
                            curvature: -8.0,
                            delay_secs: 0.3,
                            gradient_start_color: iced::Color::from_rgb8(0x6B, 0xC9, 0xFF),
                            gradient_stop_color: iced::Color::from_rgb8(0xFF, 0x8A, 0xC2),
                            ..AnimatedBeamProps::new()
                        },
                    ),
                ]),
        };

        _shared::shell(
            "AnimatedBeam",
            column![
                _shared::tabs(&labels, self.active, Message::Select),
                container(scene)
                    .width(Length::Fill)
                    .height(Length::Fixed(520.0)),
            ]
            .spacing(20),
        )
    }
}

fn beam_pair(
    elapsed: Duration,
    from: AnchorBounds,
    to: AnchorBounds,
    props: AnimatedBeamProps,
) -> Element<'static, Message> {
    animated_beam(from, to, elapsed, props)
}

fn beam_scene(beams: Vec<Element<'static, Message>>) -> Element<'static, Message> {
    let overlay = stack(
        beams
            .into_iter()
            .map(|beam| {
                container(beam)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            })
            .collect::<Vec<_>>(),
    );

    container(overlay)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
