mod _shared;

use iced::widget::{column, container, text};
use iced::{Alignment, Element, Length, Size, Task};
use iced_shadcn::Theme;
use magic_iced::{BentoCardProps, BentoGridProps, bento_grid};

pub fn main() -> iced::Result {
    iced::application(BentoGridExample::new, BentoGridExample::update, BentoGridExample::view)
        .title("Bento Grid Example")
        .window(iced::window::Settings {
            size: Size::new(1400.0, 900.0),
            ..Default::default()
        })
        .run()
}

struct BentoGridExample {
    theme: Theme,
}

#[derive(Debug, Clone)]
enum Message {}

impl BentoGridExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme: Theme::default(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let cards = vec![
            BentoCardProps {
                name: "Marquee".into(),
                description: "Social proof rail".into(),
                cta: "Open".into(),
                icon: "✦".into(),
                col_span: 2,
                background: card_bg("Customers", "2,400+", &self.theme),
                on_press: None,
                accent: None,
            },
            BentoCardProps {
                name: "Beam".into(),
                description: "Connector flow".into(),
                cta: "Inspect".into(),
                icon: "↗".into(),
                col_span: 1,
                background: card_bg("Flow", "5 hops", &self.theme),
                on_press: None,
                accent: None,
            },
            BentoCardProps {
                name: "Ticker".into(),
                description: "Live metrics".into(),
                cta: "View".into(),
                icon: "●".into(),
                col_span: 1,
                background: card_bg("Revenue", "$128k", &self.theme),
                on_press: None,
                accent: None,
            },
            BentoCardProps {
                name: "Tweet".into(),
                description: "Social card".into(),
                cta: "Open".into(),
                icon: "✉".into(),
                col_span: 2,
                background: card_bg("Tweet", "Pinned", &self.theme),
                on_press: None,
                accent: None,
            },
        ];

        _shared::shell(
            "Bento Grid",
            column![
                text("One section, multiple cells").size(14),
                bento_grid(cards, BentoGridProps::new(), &self.theme),
            ]
            .spacing(16)
            .align_x(Alignment::Start),
        )
    }
}

fn card_bg<'a>(title: &'a str, metric: &'a str, theme: &'a Theme) -> Element<'a, Message> {
    container(
        column![
            text(title).size(22),
            text(metric).size(34),
        ]
        .spacing(8),
    )
    .padding(20)
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .style(move |_| iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color {
            a: 0.08,
            ..theme.palette.primary
        })),
        border: iced::Border {
            color: iced::Color {
                a: 0.18,
                ..theme.palette.primary
            },
            width: 1.0,
            radius: theme.radius.md.into(),
        },
        ..iced::widget::container::Style::default()
    })
    .into()
}
