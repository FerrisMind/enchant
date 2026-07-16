//! Magic UI `bento-demo-vertical` port — asymmetric 3×3 feature grid.
//!
//! Reference (MIT): `magic-refs/magicui/apps/www/registry/example/bento-demo-vertical.tsx`
//! Layout (lg):
//! ```text
//! | search (2h) | files (3h) | calendar (1h) |
//! |             |            | notifs (2h)   |
//! | multi  (1h) |            |               |
//! ```

mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Color, Element, Font, Length, Size, Subscription, Task};
use lucide_icons::{Icon as LucideIcon, LUCIDE_FONT_BYTES};
use magic_iced::{BENTO_GAP, BentoCardProps, BentoGridProps, bento_card};

pub fn main() -> iced::Result {
    iced::application(
        BentoVerticalExample::new,
        BentoVerticalExample::update,
        BentoVerticalExample::view,
    )
    .title("Bento Grid Vertical — Magic UI bento-demo-vertical")
    .window(iced::window::Settings {
        size: Size::new(1200.0, 1100.0),
        ..Default::default()
    })
    .subscription(BentoVerticalExample::subscription)
    .font(LUCIDE_FONT_BYTES)
    .run()
}

struct BentoVerticalExample {
    theme_shell: _shared::ThemeShell,
    hovered: Option<&'static str>,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(magic_iced::AnchorMeasurements),
    Tick(Instant),
    CardHover(&'static str, bool),
    Cta(&'static str),
}

impl BentoVerticalExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                hovered: None,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleTheme => {
                return _shared::ThemeShell::measure_toggle_anchor().map(Message::AnchorsMeasured);
            }
            Message::AnchorsMeasured(measurements) => {
                self.theme_shell.apply_anchor_and_toggle(&measurements);
            }
            Message::Tick(now) => {
                self.theme_shell.tick(now);
            }
            Message::CardHover(id, entered) => {
                self.hovered = if entered { Some(id) } else { None };
            }
            Message::Cta(id) => {
                let _ = id;
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let theme = self.theme_shell.theme();
        let dark = theme.variant().is_dark();
        let props = BentoGridProps::new();
        let gap = props.gap;

        // Decorative placeholders match Magic UI empty <img> backgrounds.
        let placeholder = |label: &'static str| -> Element<'_, Message> {
            container(text(label).size(12).style(|_| text::Style {
                color: Some(Color {
                    a: 0.28,
                    ..theme.palette.foreground
                }),
            }))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        };

        let card = |id: &'static str,
                    name: &'static str,
                    description: &'static str,
                    icon: LucideIcon,
                    row_span: u16,
                    bg_label: &'static str|
         -> Element<'_, Message> {
            bento_card(
                BentoCardProps::new(
                    name,
                    lucide(icon, 48.0, icon_color(dark)),
                    placeholder(bg_label),
                )
                .description(description)
                .cta("Learn more")
                .row_span(row_span)
                .hovered(self.hovered == Some(id))
                .on_hover(Message::CardHover(id, true), Message::CardHover(id, false))
                .on_cta(Message::Cta(id)),
                props,
                theme,
            )
        };

        // Column 1: Full text search (2 rows) + Multilingual (1 row)
        let col1 = column![
            card(
                "search",
                "Full text search",
                "Search through all your files in one place.",
                LucideIcon::TextCursorInput,
                2,
                "search bg",
            ),
            card(
                "multi",
                "Multilingual",
                "Supports 100+ languages and counting.",
                LucideIcon::Globe,
                1,
                "globe bg",
            ),
        ]
        .spacing(gap)
        .width(Length::FillPortion(1));

        // Column 2: Save your files (3 rows)
        let col2 = column![card(
            "files",
            "Save your files",
            "We automatically save your files as you type.",
            LucideIcon::FileText,
            3,
            "files bg",
        )]
        .width(Length::FillPortion(1));

        // Column 3: Calendar (1 row) + Notifications (2 rows)
        let col3 = column![
            card(
                "calendar",
                "Calendar",
                "Use the calendar to filter your files by date.",
                LucideIcon::Calendar,
                1,
                "calendar bg",
            ),
            card(
                "notifs",
                "Notifications",
                "Get notified when someone shares a file or mentions you in a comment.",
                LucideIcon::Bell,
                2,
                "bell bg",
            ),
        ]
        .spacing(gap)
        .width(Length::FillPortion(1));

        let grid = row![col1, col2, col3]
            .spacing(gap)
            .width(Length::Fill)
            .align_y(Alignment::Start);

        _shared::themed_shell(
            "Bento Grid Vertical",
            column![
                text("Magic UI bento-demo-vertical — asymmetric row/col spans")
                    .size(14)
                    .style(|_| text::Style {
                        color: Some(Color {
                            a: 0.65,
                            ..theme.palette.foreground
                        }),
                    }),
                grid,
                Space::new().height(Length::Fixed(BENTO_GAP)),
            ]
            .spacing(16)
            .width(Length::Fill)
            .padding([0.0, 8.0]),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}

fn icon_color(dark: bool) -> Color {
    if dark {
        Color::from_rgb8(0xD4, 0xD4, 0xD8)
    } else {
        Color::from_rgb8(0x40, 0x40, 0x40)
    }
}

fn lucide<'a, Message: 'a>(icon: LucideIcon, size: f32, color: Color) -> Element<'a, Message> {
    text(char::from(icon).to_string())
        .font(Font::with_name("lucide"))
        .size(size)
        .style(move |_| text::Style { color: Some(color) })
        .into()
}
