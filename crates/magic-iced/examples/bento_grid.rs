mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::{column, container, row, stack, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Point, Rectangle, Size, Subscription, Task};
use magic_iced::{
    animated_beam, bento_grid, marquee_indexed, number_ticker, AnchorBounds,
    AnimatedBeamProps, BentoCardProps, BentoGridProps, MarqueeOrientation, MarqueeProps,
    NumberTickerProps, NumberTickerState,
};

pub fn main() -> iced::Result {
    iced::application(
        BentoGridExample::new,
        BentoGridExample::update,
        BentoGridExample::view,
    )
    .title("Bento Grid Example")
    .window(iced::window::Settings {
        size: Size::new(1400.0, 900.0),
        ..Default::default()
    })
    .subscription(BentoGridExample::subscription)
    .run()
}

struct FileItem {
    name: &'static str,
    body: &'static str,
}

struct BentoGridExample {
    theme_shell: _shared::ThemeShell,
    started_at: Instant,
    elapsed: Duration,
    ticker_state: NumberTickerState,
    ticker_props: NumberTickerProps,
    now: Instant,
    files: Vec<FileItem>,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(magic_iced::AnchorMeasurements),
    Tick(Instant),
    MarqueeHover(bool),
}

impl BentoGridExample {
    fn new() -> (Self, Task<Message>) {
        let ticker_props = NumberTickerProps::new(128400.0).font_size(24);
        let ticker_state = NumberTickerState::new(&ticker_props);
        let now = Instant::now();
        let files = vec![
            FileItem {
                name: "bitcoin.pdf",
                body: "Bitcoin is a cryptocurrency invented in 2008...",
            },
            FileItem {
                name: "finances.xlsx",
                body: "A spreadsheet or worksheet is a file made of rows...",
            },
            FileItem {
                name: "logo.svg",
                body: "Scalable Vector Graphics is an XML-based format...",
            },
            FileItem {
                name: "keys.gpg",
                body: "GPG keys are used to encrypt and decrypt email...",
            },
            FileItem {
                name: "seed.txt",
                body: "A seed phrase is a list of words which store...",
            },
        ];
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                started_at: now,
                elapsed: Duration::ZERO,
                ticker_state,
                ticker_props,
                now,
                files,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleTheme => {
                return _shared::ThemeShell::measure_toggle_anchor()
                    .map(Message::AnchorsMeasured);
            }
            Message::AnchorsMeasured(measurements) => {
                self.theme_shell.apply_anchor_and_toggle(&measurements);
            }

            Message::Tick(now) => {
                self.theme_shell.tick(now);
                self.now = now;
                self.elapsed = now.duration_since(self.started_at);
                self.ticker_state.tick(now, &self.ticker_props);
            }
            Message::MarqueeHover(_) => {}
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        // --- 1. Marquee file list background ---
        let render_file = |file: &FileItem, _copy_idx: usize, _item_idx: usize| -> Element<'_, Message> {
            container(
                column![
                    text(file.name).size(12).style(|_: &iced::Theme| text::Style {
                        color: Some(self.theme_shell.theme().palette.foreground),
                    }),
                    text(file.body).size(9).style(|_: &iced::Theme| text::Style {
                        color: Some(Color { a: 0.6, ..self.theme_shell.theme().palette.foreground }),
                    }),
                ]
                .spacing(4)
            )
            .width(Length::Fixed(120.0))
            .height(Length::Fixed(80.0))
            .padding(10)
            .style(|_: &iced::Theme| container::Style {
                background: Some(Background::Color(Color { a: 0.05, ..self.theme_shell.theme().palette.foreground })),
                border: Border {
                    color: Color { a: 0.1, ..self.theme_shell.theme().palette.foreground },
                    width: 1.0,
                    radius: self.theme_shell.theme().radius.sm.into(),
                },
                ..Default::default()
            })
            .into()
        };

        let marquee_bg = marquee_indexed(
            &self.files,
            render_file,
            self.elapsed,
            MarqueeProps {
                orientation: MarqueeOrientation::Horizontal,
                reverse: false,
                pause_on_hover: false,
                repeat: 4,
                gap: 12.0,
                speed_px_per_sec: 25.0,
                item_extent: 120.0,
                item_cross: 80.0,
                padding: 4.0,
                edge_fade: false,
            },
            self.theme_shell.theme(),
            |_| Message::MarqueeHover(false),
        );

        let marquee_card_bg = container(marquee_bg)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_y(Length::Fill)
            .style(|_: &iced::Theme| container::Style::default());

        // --- 2. Animated Beam background ---
        let user_node = container(text("👤").center())
            .width(36.0)
            .height(36.0)
            .style(|_: &iced::Theme| container::Style {
                background: Some(Background::Color(self.theme_shell.theme().palette.card)),
                border: Border {
                    color: Color { a: 0.15, ..self.theme_shell.theme().palette.foreground },
                    width: 1.0,
                    radius: 18.0.into(),
                },
                ..Default::default()
            });

        let ai_node = container(text("AI").center())
            .width(36.0)
            .height(36.0)
            .style(|_: &iced::Theme| container::Style {
                background: Some(Background::Color(self.theme_shell.theme().palette.card)),
                border: Border {
                    color: Color { a: 0.15, ..self.theme_shell.theme().palette.foreground },
                    width: 1.0,
                    radius: 18.0.into(),
                },
                ..Default::default()
            });

        let beam_canvas = animated_beam(
            AnchorBounds::from_bounds(Rectangle::new(Point::new(20.0, 32.0), Size::new(36.0, 36.0))),
            AnchorBounds::from_bounds(Rectangle::new(Point::new(244.0, 32.0), Size::new(36.0, 36.0))),
            self.elapsed,
            AnimatedBeamProps::new(),
        );

        let beam_card_bg = container(
            stack![
                container(user_node).padding(Padding { top: 32.0, right: 0.0, bottom: 0.0, left: 20.0 }),
                container(ai_node).padding(Padding { top: 32.0, right: 0.0, bottom: 0.0, left: 244.0 }),
                beam_canvas,
            ]
            .width(Length::Fixed(300.0))
            .height(Length::Fixed(100.0))
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_: &iced::Theme| container::Style::default());

        // --- 3. Number Ticker background ---
        let ticker_bg = container(
            row![
                text("$").size(24).style(|_: &iced::Theme| text::Style {
                    color: Some(self.theme_shell.theme().palette.foreground),
                }),
                number_ticker(&self.ticker_state, self.now, &self.ticker_props, self.theme_shell.theme()),
            ]
            .align_y(Alignment::Center)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_: &iced::Theme| container::Style::default());

        // --- 4. Interactive Tweet Card background ---
        let tweet_card_bg = container(
            column![
                row![
                    container(text("✦").center())
                        .width(32.0)
                        .height(32.0)
                        .style(|_: &iced::Theme| container::Style {
                            background: Some(Background::Color(Color { a: 0.1, ..self.theme_shell.theme().palette.primary })),
                            border: Border {
                                radius: 16.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                    column![
                        text("Magic UI").size(13).style(|_: &iced::Theme| text::Style {
                            color: Some(self.theme_shell.theme().palette.foreground),
                        }),
                        text("@magicui_io").size(10).style(|_: &iced::Theme| text::Style {
                            color: Some(Color { a: 0.5, ..self.theme_shell.theme().palette.foreground }),
                        }),
                    ]
                    .spacing(2)
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                text("Bento grids are excellent for displaying component catalogs and feature lists side-by-side.")
                    .size(11)
                    .style(|_: &iced::Theme| text::Style {
                        color: Some(Color { a: 0.82, ..self.theme_shell.theme().palette.foreground }),
                    }),
            ]
            .spacing(12)
            .width(Length::Fixed(280.0))
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_: &iced::Theme| container::Style::default());

        let cards = vec![
            BentoCardProps {
                name: "Marquee".into(),
                description: "Social proof rail".into(),
                cta: "Open".into(),
                icon: "✦".into(),
                col_span: 2,
                background: marquee_card_bg.into(),
                on_press: None,
                accent: None,
            },
            BentoCardProps {
                name: "Beam".into(),
                description: "Connector flow".into(),
                cta: "Inspect".into(),
                icon: "↗".into(),
                col_span: 1,
                background: beam_card_bg.into(),
                on_press: None,
                accent: None,
            },
            BentoCardProps {
                name: "Ticker".into(),
                description: "Live metrics".into(),
                cta: "View".into(),
                icon: "●".into(),
                col_span: 1,
                background: ticker_bg.into(),
                on_press: None,
                accent: None,
            },
            BentoCardProps {
                name: "Tweet".into(),
                description: "Social card".into(),
                cta: "Open".into(),
                icon: "✉".into(),
                col_span: 2,
                background: tweet_card_bg.into(),
                on_press: None,
                accent: None,
            },
        ];

        _shared::themed_shell("Bento Grid", column![
                text("One section, multiple cells").size(14),
                bento_grid(cards, BentoGridProps::new(), self.theme_shell.theme()),
            ]
            .spacing(16)
            .align_x(Alignment::Start),
            &self.theme_shell, Message::ToggleTheme)
    }
}
