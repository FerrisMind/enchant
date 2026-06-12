mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::{column, container};
use iced::{Alignment, Border, Element, Length, Size, Subscription, Task};
use magic_iced::{
    DottedMapMarker, DottedMapOverlay, DottedMapProps, DottedMapState, dotted_map,
};

pub fn main() -> iced::Result {
    iced::application(
        DottedMapExample::new,
        DottedMapExample::update,
        DottedMapExample::view,
    )
    .title("Dotted Map")
    .window(iced::window::Settings {
        size: Size::new(900.0, 620.0),
        ..Default::default()
    })
    .subscription(DottedMapExample::subscription)
    .run()
}

struct DottedMapExample {
    props: DottedMapProps,
    state: DottedMapState,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl DottedMapExample {
    fn new() -> (Self, Task<Message>) {
        let props = DottedMapProps {
            width: Length::Fill,
            height: Length::Fill,
            map_samples: 2500,
            markers: vec![
                DottedMapMarker {
                    lat: 37.5665,
                    lng: 126.978,
                    size: 2.8,
                    pulse: true,
                    overlay: Some(DottedMapOverlay::Label {
                        text: "Seoul".into(),
                        badge: None,
                    }),
                },
                DottedMapMarker {
                    lat: 40.7128,
                    lng: -74.006,
                    size: 2.8,
                    pulse: true,
                    overlay: Some(DottedMapOverlay::Label {
                        text: "NYC".into(),
                        badge: None,
                    }),
                },
            ],
            ..DottedMapProps::new()
        };
        let state = DottedMapState::new(&props);
        (
            Self {
                props,
                state,
                now: Instant::now(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        if let Message::Tick(now) = message {
            self.now = now;
            self.state.tick(now, &self.props);
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let stage = container(dotted_map(&self.state, self.now, &self.props))
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

        _shared::shell("Dotted Map", column![stage].spacing(24).align_x(Alignment::Center))
    }
}
