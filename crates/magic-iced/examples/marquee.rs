use iced::Background;
use iced::Color;
use iced::widget::{button, column, container, row, text};
use iced::{Element, Length, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::widgets::{MarqueeItem, MarqueeOrientation, MarqueeProps, marquee};
use std::time::Instant;

pub fn main() -> iced::Result {
    iced::application(
        MarqueeExample::new,
        MarqueeExample::update,
        MarqueeExample::view,
    )
    .title("Marquee Example")
    .window(iced::window::Settings {
        size: Size::new(1000.0, 800.0),
        ..Default::default()
    })
    .subscription(MarqueeExample::subscription)
    .run()
}

struct MarqueeExample {
    last_tick: Instant,
    elapsed_times: [std::time::Duration; 4],
    is_paused: [bool; 4],
    theme: Theme,
    reviews: Vec<MarqueeItem>,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
    TogglePause(usize, bool),
}

impl Default for MarqueeExample {
    fn default() -> Self {
        let reviews = vec![
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
        ];

        Self {
            last_tick: Instant::now(),
            elapsed_times: [std::time::Duration::ZERO; 4],
            is_paused: [false; 4],
            theme: Theme::default(),
            reviews,
        }
    }
}

impl MarqueeExample {
    fn new() -> (Self, Task<Message>) {
        (Self::default(), Task::none())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick(instant) => {
                let delta = instant.duration_since(self.last_tick);
                for i in 0..4 {
                    if !self.is_paused[i] {
                        self.elapsed_times[i] += delta;
                    }
                }
                self.last_tick = instant;
            }
            Message::TogglePause(index, paused) => {
                if index < 4 {
                    self.is_paused[index] = paused;
                }
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::window::frames().map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let first_row = &self.reviews[0..3];
        let second_row = &self.reviews[3..6];

        let render_card = |item: &MarqueeItem| {
            let accent = item.accent.unwrap_or(self.theme.palette.primary);
            let foreground = self.theme.palette.foreground;
            let muted = self.theme.palette.muted_foreground;
            let radius = self.theme.radius.lg;

            let content = column![
                text(item.title.clone()).size(16),
                text(item.subtitle.clone())
                    .size(12)
                    .style(move |_| { iced::widget::text::Style { color: Some(muted) } }),
                text(item.body.clone()).size(13),
            ]
            .spacing(6);

            button(content)
                .padding([14.0, 16.0])
                .width(Length::Fixed(260.0))
                .height(Length::Fixed(112.0))
                .on_press(Message::Tick(Instant::now()))
                .style(move |_theme, status| {
                    let base_bg = Color { a: 0.08, ..accent };
                    let hover_bg = Color { a: 0.15, ..accent };

                    let bg = match status {
                        iced::widget::button::Status::Hovered
                        | iced::widget::button::Status::Pressed => hover_bg,
                        _ => base_bg,
                    };

                    let border_alpha = match status {
                        iced::widget::button::Status::Hovered
                        | iced::widget::button::Status::Pressed => 0.8,
                        _ => 0.4,
                    };

                    iced::widget::button::Style {
                        background: Some(Background::Color(bg)),
                        text_color: foreground,
                        border: iced::Border {
                            color: Color {
                                a: border_alpha,
                                ..accent
                            },
                            width: 1.0,
                            radius: radius.into(),
                        },
                        ..iced::widget::button::Style::default()
                    }
                })
                .into()
        };

        let horizontal_demo = column![
            text("Horizontal Marquee").size(24),
            marquee(
                first_row,
                render_card,
                self.elapsed_times[0],
                MarqueeProps::default(),
                &self.theme,
                |hover| Message::TogglePause(0, hover),
            ),
            marquee(
                second_row,
                render_card,
                self.elapsed_times[1],
                MarqueeProps {
                    reverse: true,
                    ..MarqueeProps::default()
                },
                &self.theme,
                |hover| Message::TogglePause(1, hover),
            ),
        ]
        .spacing(16);

        let vertical_demo = column![
            text("Vertical Marquee").size(24),
            row![
                marquee(
                    first_row,
                    render_card,
                    self.elapsed_times[2],
                    MarqueeProps {
                        orientation: MarqueeOrientation::Vertical,
                        ..MarqueeProps::default()
                    },
                    &self.theme,
                    |hover| Message::TogglePause(2, hover),
                ),
                marquee(
                    second_row,
                    render_card,
                    self.elapsed_times[3],
                    MarqueeProps {
                        orientation: MarqueeOrientation::Vertical,
                        reverse: true,
                        ..MarqueeProps::default()
                    },
                    &self.theme,
                    |hover| Message::TogglePause(3, hover),
                ),
            ]
            .spacing(16)
            .height(Length::Fixed(400.0)),
        ]
        .spacing(16);

        container(
            column![horizontal_demo, vertical_demo]
                .spacing(32)
                .padding(32),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .into()
    }
}
