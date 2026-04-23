mod _shared;

use iced::time::Instant;
use iced::widget::{column, text};
use iced::{Element, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{RippleProps, ripple};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        RippleExample::new,
        RippleExample::update,
        RippleExample::view,
    )
    .title("Ripple")
    .window(iced::window::Settings {
        size: Size::new(1000.0, 760.0),
        ..Default::default()
    })
    .subscription(RippleExample::subscription)
    .run()
}

struct RippleExample {
    active: usize,
    theme: Theme,
    started_at: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    Select(usize),
    Tick(Instant),
}

impl RippleExample {
    fn new() -> (Self, Task<Message>) {
        let started_at = Instant::now();
        (
            Self {
                active: 0,
                theme: Theme::default(),
                started_at,
                elapsed: Duration::ZERO,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Select(index) => self.active = index.min(1),
            Message::Tick(now) => self.elapsed = now.duration_since(self.started_at),
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let labels = ["Calm", "Dense"];
        let props = if self.active == 0 {
            RippleProps {
                main_circle_size: 180.0,
                main_circle_opacity: 0.24,
                num_circles: 6,
                duration_secs: 1.8,
                color: None,
            }
        } else {
            RippleProps {
                main_circle_size: 240.0,
                main_circle_opacity: 0.18,
                num_circles: 10,
                duration_secs: 1.2,
                color: None,
            }
        };

        _shared::shell(
            "Ripple",
            column![
                _shared::tabs(&labels, self.active, Message::Select),
                ripple(self.elapsed, props, &self.theme),
                text("A concentric background effect for hero surfaces."),
            ]
            .spacing(20),
        )
    }
}
