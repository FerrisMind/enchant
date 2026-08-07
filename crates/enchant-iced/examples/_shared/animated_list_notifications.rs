#![allow(dead_code)]

use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Shadow, Vector};
use iced_shadcn::Theme;

const GRAY_500: Color = Color::from_rgb8(0x6B, 0x72, 0x80);
const CARD_RADIUS: f32 = 16.0;

#[derive(Clone, Copy, Debug)]
pub struct NotificationItem {
    pub name: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
    pub color: Color,
    pub time: &'static str,
}

pub const NOTIFICATIONS: &[NotificationItem] = &[
    NotificationItem {
        name: "Payment received",
        description: "Magic UI",
        time: "15m ago",
        icon: "💸",
        color: Color::from_rgb8(0x00, 0xC9, 0xA7),
    },
    NotificationItem {
        name: "User signed up",
        description: "Magic UI",
        time: "10m ago",
        icon: "👤",
        color: Color::from_rgb8(0xFF, 0xB8, 0x00),
    },
    NotificationItem {
        name: "New message",
        description: "Magic UI",
        time: "5m ago",
        icon: "💬",
        color: Color::from_rgb8(0xFF, 0x3D, 0x71),
    },
    NotificationItem {
        name: "New event",
        description: "Magic UI",
        time: "2m ago",
        icon: "🗞️",
        color: Color::from_rgb8(0x1E, 0x86, 0xFF),
    },
];

pub fn repeated_notifications(repeat: usize) -> Vec<NotificationItem> {
    NOTIFICATIONS
        .iter()
        .copied()
        .cycle()
        .take(NOTIFICATIONS.len() * repeat.max(1))
        .collect()
}

/// Matches magicui `animated-list-demo.tsx` `<figure>` notification card.
pub fn notification_card<'a, Message: 'a>(
    item: NotificationItem,
    theme: &Theme,
) -> Element<'a, Message> {
    let dark = theme.variant().is_dark();
    let title_color = if dark {
        theme.palette.foreground
    } else {
        Color::from_rgb8(0x11, 0x18, 0x27)
    };
    let description_color = if dark {
        Color {
            a: 0.6,
            ..theme.palette.foreground
        }
    } else {
        GRAY_500
    };
    let time_color = GRAY_500;

    let icon_badge = container(text(item.icon).size(18))
        .width(Length::Fixed(40.0))
        .height(Length::Fixed(40.0))
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(move |_| container::Style {
            background: Some(Background::Color(item.color)),
            border: Border {
                radius: 16.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        });

    let title = row![
        text(item.name)
            .size(16)
            .style(move |_| iced::widget::text::Style {
                color: Some(title_color),
            }),
        text(" · ")
            .size(14)
            .style(move |_| iced::widget::text::Style {
                color: Some(title_color),
            }),
        text(item.time)
            .size(12)
            .style(move |_| iced::widget::text::Style {
                color: Some(time_color),
            }),
    ]
    .spacing(0)
    .align_y(Alignment::Center);

    let body = row![
        icon_badge,
        column![
            title,
            text(item.description)
                .size(14)
                .style(move |_| iced::widget::text::Style {
                    color: Some(description_color),
                }),
        ]
        .spacing(4)
        .width(Length::Fill),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .width(Length::Fill);

    container(body)
        .width(Length::Fill)
        .padding(16)
        .style(move |_| {
            if dark {
                dark_figure_style()
            } else {
                light_figure_style()
            }
        })
        .into()
}

fn dark_figure_style() -> container::Style {
    container::Style {
        // `dark:bg-transparent` — gaps between cards show the stage background.
        background: None,
        border: Border {
            width: 1.0,
            color: Color {
                a: 0.1,
                ..Color::WHITE
            },
            radius: CARD_RADIUS.into(),
        },
        // Rounded outer glow on all sides; positive Y offset weights it toward the bottom.
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.32),
            offset: Vector::new(0.0, 12.0),
            blur_radius: 32.0,
        },
        ..container::Style::default()
    }
}

fn light_figure_style() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::WHITE)),
        border: Border {
            width: 1.0,
            color: Color {
                a: 0.03,
                ..Color::BLACK
            },
            radius: CARD_RADIUS.into(),
        },
        shadow: Shadow {
            color: Color {
                a: 0.05,
                ..Color::BLACK
            },
            offset: Vector::new(0.0, 12.0),
            blur_radius: 24.0,
        },
        ..container::Style::default()
    }
}

/// `bg-gradient-to-t from-background` fade at the bottom quarter of the stage.
pub fn bottom_background_fade<'a, Message: 'a>(
    height: f32,
    background: Color,
) -> Element<'a, Message> {
    const STEPS: usize = 32;
    let strip_height = height / STEPS as f32;

    let mut strips = Vec::with_capacity(STEPS);
    for step in 0..STEPS {
        let progress = (step as f32 + 0.5) / STEPS as f32;
        strips.push(
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fixed(strip_height))
                .style(move |_| container::Style {
                    background: Some(Background::Color(Color {
                        a: progress,
                        ..background
                    })),
                    ..container::Style::default()
                })
                .into(),
        );
    }

    column(strips).width(Length::Fill).into()
}
