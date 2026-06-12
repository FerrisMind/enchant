mod _shared;

use iced::time::Instant;
use iced::widget::{column, container, text};
use iced::{Alignment, Element, Length, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{WarpBackgroundProps, warp_background};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        WarpBackgroundExample::new,
        WarpBackgroundExample::update,
        WarpBackgroundExample::view,
    )
    .title("Warp Background")
    .window(iced::window::Settings {
        size: Size::new(900.0, 620.0),
        ..Default::default()
    })
    .subscription(WarpBackgroundExample::subscription)
    .run()
}

struct WarpBackgroundExample {
    theme: Theme,
    start: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl WarpBackgroundExample {
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
        let card = container(
            column![
                text("Congratulations on Your Promotion!").size(20),
                text(
                    "Your hard work and dedication have paid off. We're thrilled to see you take this next step in your career. Keep up the fantastic work!"
                )
                .size(14)
                .width(Length::Fixed(280.0)),
            ]
            .spacing(10),
        )
        .padding(16)
        .width(Length::Fixed(320.0));

        let body = warp_background(
            card,
            self.elapsed,
            WarpBackgroundProps {
                width: Length::Fixed(720.0),
                height: Length::Fixed(500.0),
                ..WarpBackgroundProps::new()
            },
            &self.theme,
        );

        _shared::shell("Warp Background", column![body].spacing(24).align_x(Alignment::Center))
    }
}
