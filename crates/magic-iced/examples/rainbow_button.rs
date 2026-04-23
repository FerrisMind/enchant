mod _shared;

use iced::time::Instant;
use iced::widget::{column, text};
use iced::{Element, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{RainbowButtonProps, RainbowButtonVariant, rainbow_button};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        RainbowButtonExample::new,
        RainbowButtonExample::update,
        RainbowButtonExample::view,
    )
    .title("Rainbow Button")
    .window(iced::window::Settings {
        size: Size::new(1000.0, 760.0),
        ..Default::default()
    })
    .subscription(RainbowButtonExample::subscription)
    .run()
}

struct RainbowButtonExample {
    active: usize,
    theme: Theme,
    started_at: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    Select(usize),
    Tick(Instant),
    Pressed,
}

impl RainbowButtonExample {
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
            Message::Pressed => {}
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let labels = ["Default", "Outline"];
        let props = RainbowButtonProps {
            variant: if self.active == 0 {
                RainbowButtonVariant::Default
            } else {
                RainbowButtonVariant::Outline
            },
            duration_secs: 2.0,
            radius: 10.0,
            ..RainbowButtonProps::new()
        };

        _shared::shell(
            "Rainbow Button",
            column![
                _shared::tabs(&labels, self.active, Message::Select),
                rainbow_button(text("Rainbow"), Some(Message::Pressed), self.elapsed, props, &self.theme),
            ]
            .spacing(20),
        )
    }
}
