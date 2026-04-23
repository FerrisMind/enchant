use iced::gradient;
use iced::widget::{column, container, image, row, stack, text, Space};
use iced::{Alignment, Background, Border, Color, Element, Length, Radians, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{marquee, MagicTheme, MarqueeItem, MarqueeOrientation, MarqueeProps};
use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

static REVIEWS: LazyLock<Vec<MarqueeItem>> = LazyLock::new(reviews);

pub fn main() -> iced::Result {
    iced::application(MarqueeDemo::new, MarqueeDemo::update, MarqueeDemo::view)
        .title("Marquee Example")
        .window(iced::window::Settings {
            size: Size::new(980.0, 620.0),
            ..Default::default()
        })
        .subscription(MarqueeDemo::subscription)
        .run()
}

struct MarqueeDemo {
    theme: Theme,
    started_at: Instant,
    elapsed: Duration,
    paused_total: Duration,
    pause_started_at: Option<Instant>,
    hover_depth: usize,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
    Hover(bool),
}

impl Default for MarqueeDemo {
    fn default() -> Self {
        Self {
            theme: MagicTheme::light(),
            started_at: Instant::now(),
            elapsed: Duration::ZERO,
            paused_total: Duration::ZERO,
            pause_started_at: None,
            hover_depth: 0,
        }
    }
}

impl MarqueeDemo {
    fn new() -> (Self, Task<Message>) {
        (Self::default(), Task::none())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick(now) => {
                self.elapsed = if self.hover_depth > 0 {
                    self.pause_started_at
                        .map(|pause_started_at| {
                            pause_started_at
                                .saturating_duration_since(self.started_at)
                                .saturating_sub(self.paused_total)
                        })
                        .unwrap_or(self.elapsed)
                } else {
                    now.saturating_duration_since(self.started_at)
                        .saturating_sub(self.paused_total)
                };
            }
            Message::Hover(entered) => {
                if entered {
                    self.hover_depth = self.hover_depth.saturating_add(1);
                    if self.hover_depth == 1 {
                        let now = Instant::now();
                        self.pause_started_at = Some(now);
                        self.elapsed = now
                            .saturating_duration_since(self.started_at)
                            .saturating_sub(self.paused_total);
                    }
                } else if self.hover_depth > 0 {
                    self.hover_depth -= 1;
                    if self.hover_depth == 0
                        && let Some(pause_started_at) = self.pause_started_at.take()
                    {
                        self.paused_total +=
                            Instant::now().saturating_duration_since(pause_started_at);
                    }
                }
            }
        }

        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::window::frames().map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let first_row = &REVIEWS[..3];
        let second_row = &REVIEWS[3..];

        let content = container(
            stack(vec![
                column![
                    self.marquee_row(first_row, false),
                    self.marquee_row(second_row, true),
                ]
                .spacing(16)
                .width(Length::Fill)
                .align_x(Alignment::Center)
                .into(),
                horizontal_fade_overlay(&self.theme),
            ])
        )
        .width(Length::Fixed(780.0))
        .height(Length::Fixed(360.0))
        .padding(24)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(preview_frame_style());

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(page_style(&self.theme))
            .into()
    }

    fn marquee_row<'a>(&'a self, items: &'a [MarqueeItem], reverse: bool) -> Element<'a, Message> {
        let render_item = |item: &MarqueeItem| -> Element<'a, Message> {
            let avatar = container(image(avatar_handle(item.title.as_str())))
                .width(Length::Fixed(32.0))
                .height(Length::Fixed(32.0))
                .clip(true)
                .style(|_| iced::widget::container::Style {
                    border: Border {
                        color: Color::TRANSPARENT,
                        width: 0.0,
                        radius: 16.0.into(),
                    },
                    ..iced::widget::container::Style::default()
                });

            container(
                column![
                    row![
                        avatar,
                        column![
                            text(item.title.clone()).size(14),
                            text(item.subtitle.clone()).size(12).style(|_| {
                                iced::widget::text::Style {
                                    color: Some(Color::from_rgba8(0x00, 0x00, 0x00, 0.40)),
                                }
                            }),
                        ]
                        .spacing(2),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    text(item.body.clone()).size(14),
                ]
                .spacing(8),
            )
            .width(Length::Fixed(256.0))
            .height(Length::Fill)
            .padding(16)
            .style(|_| iced::widget::container::Style {
                background: Some(Background::Color(Color::from_rgba8(0x00, 0x00, 0x00, 0.01))),
                border: Border {
                    color: Color::from_rgba8(0x00, 0x00, 0x00, 0.10),
                    width: 1.0,
                    radius: 12.0.into(),
                },
                ..iced::widget::container::Style::default()
            })
            .into()
        };

        marquee(
            items,
            render_item,
            self.elapsed,
            marquee_props(reverse),
            &self.theme,
            Message::Hover,
        )
    }
}

fn marquee_props(reverse: bool) -> MarqueeProps {
        MarqueeProps {
            orientation: MarqueeOrientation::Horizontal,
            reverse,
            pause_on_hover: true,
            repeat: 4,
            gap: 16.0,
            speed_px_per_sec: 40.0,
            item_extent: 256.0,
            item_cross: 112.0,
            padding: 8.0,
            edge_fade: false,
        }
    }

fn page_style(theme: &Theme) -> impl Fn(&iced::Theme) -> iced::widget::container::Style + '_ {
    let background = theme.palette.background;

    move |_| iced::widget::container::Style {
        background: Some(Background::Color(background)),
        ..iced::widget::container::Style::default()
    }
}

fn preview_frame_style() -> impl Fn(&iced::Theme) -> iced::widget::container::Style {
    move |_| iced::widget::container::Style {
        background: Some(Background::Color(Color::WHITE)),
        border: Border {
            color: Color::from_rgba8(0x00, 0x00, 0x00, 0.10),
            width: 1.0,
            radius: 16.0.into(),
        },
        ..iced::widget::container::Style::default()
    }
}

fn horizontal_fade_overlay<'a>(theme: &'a Theme) -> Element<'a, Message> {
    let background = theme.palette.background;
    let left = container(Space::new())
        .width(Length::FillPortion(1))
        .height(Length::Fill)
        .style(move |_| {
            gradient::Linear::new(Radians(0.0))
                .add_stop(0.0, background)
                .add_stop(
                    1.0,
                    Color {
                        a: 0.0,
                        ..background
                    },
                )
                .into()
        });
    let right = container(Space::new())
        .width(Length::FillPortion(1))
        .height(Length::Fill)
        .style(move |_| {
            gradient::Linear::new(Radians(std::f32::consts::PI))
                .add_stop(0.0, background)
                .add_stop(
                    1.0,
                    Color {
                        a: 0.0,
                        ..background
                    },
                )
                .into()
        });

    row![
        left,
        Space::new().width(Length::FillPortion(2)),
        right,
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn reviews() -> Vec<MarqueeItem> {
    vec![
        MarqueeItem::new("Jack")
            .subtitle("@jack")
            .body("I've never seen anything like this before. It's amazing. I love it."),
        MarqueeItem::new("Jill")
            .subtitle("@jill")
            .body("I don't know what to say. I'm speechless. This is amazing."),
        MarqueeItem::new("John")
            .subtitle("@john")
            .body("I'm at a loss for words. This is amazing. I love it."),
        MarqueeItem::new("Jane")
            .subtitle("@jane")
            .body("I'm at a loss for words. This is amazing. I love it."),
        MarqueeItem::new("Jenny")
            .subtitle("@jenny")
            .body("I'm at a loss for words. This is amazing. I love it."),
        MarqueeItem::new("James")
            .subtitle("@james")
            .body("I'm at a loss for words. This is amazing. I love it."),
    ]
}

fn avatar_handle(name: &str) -> iced::widget::image::Handle {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("examples");
    path.push("assets");
    path.push("marquee");
    path.push(format!("{}.png", name.to_lowercase()));
    iced::widget::image::Handle::from_path(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marquee_demo_uses_reference_reviews() {
        let reviews = reviews();
        let expected = [
            (
                "Jack",
                "@jack",
                "I've never seen anything like this before. It's amazing. I love it.",
            ),
            (
                "Jill",
                "@jill",
                "I don't know what to say. I'm speechless. This is amazing.",
            ),
            (
                "John",
                "@john",
                "I'm at a loss for words. This is amazing. I love it.",
            ),
            (
                "Jane",
                "@jane",
                "I'm at a loss for words. This is amazing. I love it.",
            ),
            (
                "Jenny",
                "@jenny",
                "I'm at a loss for words. This is amazing. I love it.",
            ),
            (
                "James",
                "@james",
                "I'm at a loss for words. This is amazing. I love it.",
            ),
        ];

        assert_eq!(reviews.len(), expected.len());
        for (review, (name, username, body)) in reviews.iter().zip(expected.iter()) {
            assert_eq!(review.title, *name);
            assert_eq!(review.subtitle, *username);
            assert_eq!(review.body, *body);
        }
    }

    #[test]
    fn marquee_demo_uses_two_rows_like_reference() {
        let reviews = reviews();
        assert_eq!(reviews[..3].len(), 3);
        assert_eq!(reviews[3..].len(), 3);
    }
}
