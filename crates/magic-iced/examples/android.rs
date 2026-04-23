mod _shared;

use iced::widget::{column, container, text};
use iced::{Element, Length, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{AndroidProps, android};

pub fn main() -> iced::Result {
    iced::application(AndroidExample::new, AndroidExample::update, AndroidExample::view)
        .title("Android Example")
        .window(iced::window::Settings {
            size: Size::new(1200.0, 900.0),
            ..Default::default()
        })
        .subscription(AndroidExample::subscription)
        .run()
}

struct AndroidExample {
    active: usize,
    theme: Theme,
}

#[derive(Debug, Clone)]
enum Message {
    Select(usize),
}

impl AndroidExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                active: 0,
                theme: Theme::default(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Select(index) => self.active = index.min(2),
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let labels = ["Android Demo", "Android Demo 2", "Android Demo 3"];
        let content = match self.active {
            0 => android_screen(
                "Messages",
                "Clean bottom nav, profile chip, and dark content rail.",
                "85%",
                "8 new notifications",
                &self.theme,
            ),
            1 => android_screen(
                "Media",
                "Split tiles with stronger chrome label.",
                "4K",
                "Live now",
                &self.theme,
            ),
            _ => android_screen(
                "Wallet",
                "Compact mobile panel with status chips.",
                "$12,480",
                "Synced 2 min ago",
                &self.theme,
            ),
        };

        _shared::shell(
            "Android",
            column![_shared::tabs(&labels, self.active, Message::Select), content].spacing(20),
        )
    }
}

fn android_screen<'a>(
    title: &'a str,
    subtitle: &'a str,
    metric: &'a str,
    footer: &'a str,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let body = column![
        text(title).size(24),
        text(subtitle).size(14),
        container(
            column![
                text(metric).size(38),
                text(footer).size(14),
            ]
            .spacing(8),
        )
        .padding(24)
        .width(Length::Fill)
    ]
    .spacing(16);

    android(
        container(body)
            .padding(20)
            .width(Length::Fill)
            .height(Length::Fill),
        AndroidProps::new().width(Length::Fixed(360.0)),
        theme,
    )
}
