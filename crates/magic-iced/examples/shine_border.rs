mod _shared;

use iced::time::Instant;
use iced::widget::{button, column, container, stack, text};
use iced::{Background, Border, Element, Length, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{ShineBorderProps, shine_border};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        ShineBorderExample::new,
        ShineBorderExample::update,
        ShineBorderExample::view,
    )
    .title("Shine Border")
    .window(iced::window::Settings {
        size: Size::new(900.0, 520.0),
        ..Default::default()
    })
    .subscription(ShineBorderExample::subscription)
    .run()
}

struct ShineBorderExample {
    theme: Theme,
    start: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl ShineBorderExample {
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

    fn login_card(&self) -> Element<'_, Message> {
        container(
            column![
                text("Login").size(22),
                text("Enter your credentials to access your account").size(14),
                column![
                    text("Email").size(13),
                    text("name@example.com").size(14),
                    text("Password").size(13),
                    text("••••••••").size(14),
                ]
                .spacing(8),
                button(text("Sign In")).width(Length::Fill),
            ]
            .spacing(16)
            .padding(24),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style({
            let card = self.theme.palette.card;
            move |_| container::Style {
                background: Some(Background::Color(card)),
                border: Border {
                    radius: 20.0.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            }
        })
        .into()
    }

    fn view(&self) -> Element<'_, Message> {
        let card = container(stack(vec![
            self.login_card(),
            container(shine_border(self.elapsed, &ShineBorderProps::new()))
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        ]))
        .width(Length::Fixed(350.0))
        .height(Length::Fixed(320.0));

        _shared::shell("Shine Border", column![card].spacing(24))
    }
}
