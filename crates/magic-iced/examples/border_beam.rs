mod _shared;

use iced::time::Instant;
use iced::widget::{button, column, container, row, stack, text};
use iced::{Background, Border, Element, Length, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{BorderBeamProps, border_beam};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        BorderBeamExample::new,
        BorderBeamExample::update,
        BorderBeamExample::view,
    )
    .title("Border Beam")
    .window(iced::window::Settings {
        size: Size::new(900.0, 520.0),
        ..Default::default()
    })
    .subscription(BorderBeamExample::subscription)
    .run()
}

struct BorderBeamExample {
    theme: Theme,
    start: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl BorderBeamExample {
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
        if let Message::Tick(now) = message {
            self.elapsed = now.duration_since(self.start);
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn login_card(&self) -> Element<'_, Message> {
        container(
            column![
                column![
                    text("Login").size(22),
                    text("Enter your credentials to access your account.").size(14),
                ]
                .spacing(6),
                column![
                    text("Email").size(13),
                    text("Enter your email").size(14),
                    text("Password").size(13),
                    text("Enter your password").size(14),
                ]
                .spacing(8),
                row![
                    button(text("Register")),
                    button(text("Login")),
                ]
                .spacing(12),
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
                    radius: 12.0.into(),
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
            container(border_beam(
                self.elapsed,
                BorderBeamProps {
                    size: 100.0,
                    duration_secs: 8.0,
                    radius: 12.0,
                    ..BorderBeamProps::new()
                },
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
        ]))
        .width(Length::Fixed(350.0))
        .height(Length::Fixed(340.0));

        _shared::shell("Border Beam", column![card].spacing(24))
    }
}
