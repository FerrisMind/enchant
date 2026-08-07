//! Magic UI `bento-demo` port — feature showcase grid.
//!
//! Reference (MIT): `magic-refs/magicui/apps/www/registry/example/bento-demo.tsx`
//! Reuses: marquee, animated_list, animated_beam (multiple outputs), iced-shadcn calendar.

mod _shared;

use chrono::NaiveDate;
use iced::time::{Duration, Instant};
use iced::widget::scrollable::Scrollbar;
use iced::widget::{column, container, scrollable, stack, text};
use iced::{
    Alignment, Background, Border, Color, Element, Font, Length, Padding, Size, Subscription, Task,
};
use iced_shadcn::{CalendarAction, CalendarMode, CalendarProps, CalendarState, calendar};
use lucide_icons::{Icon as LucideIcon, LUCIDE_FONT_BYTES};
use enchant_iced::{
    AnchorMeasurements, AnchorRemeasure, AnimatedListProps, AnimatedListState, AnimatedValue,
    BentoCardProps, BentoGridProps, MarqueeOrientation, MarqueeProps, MotionSpec, animated_list,
    bento_grid, marquee_indexed, measure_anchors,
};
use twill::prelude::{Easing, TransitionDuration};

use _shared::animated_list_notifications::{
    bottom_background_fade, notification_card, repeated_notifications,
};
use _shared::beam_demo;
use _shared::beam_scene::BeamSceneKind;

const LIST_STAGE_H: f32 = 280.0;
const LIST_FADE_H: f32 = LIST_STAGE_H / 5.0;
const GEIST_FONT_BYTES: &[u8] = include_bytes!("../assets/Geist/Geist-VariableFont_wght.ttf");
const GEIST_FONT: Font = Font::with_name("Geist");
const CARD_COUNT: usize = 4;

struct FileItem {
    name: &'static str,
    body: &'static str,
}

const FILES: &[FileItem] = &[
    FileItem {
        name: "bitcoin.pdf",
        body: "Bitcoin is a cryptocurrency invented in 2008 by an unknown person or group of people using the name Satoshi Nakamoto.",
    },
    FileItem {
        name: "finances.xlsx",
        body: "A spreadsheet or worksheet is a file made of rows and columns that help sort data, arrange data easily, and calculate numerical data.",
    },
    FileItem {
        name: "logo.svg",
        body: "Scalable Vector Graphics is an Extensible Markup Language-based vector image format for two-dimensional graphics with support for interactivity and animation.",
    },
    FileItem {
        name: "keys.gpg",
        body: "GPG keys are used to encrypt and decrypt email, files, directories, and whole disk partitions and to authenticate messages.",
    },
    FileItem {
        name: "seed.txt",
        body: "A seed phrase, seed recovery phrase or backup seed phrase is a list of words which store all the information needed to recover Bitcoin funds on-chain.",
    },
];

pub fn main() -> iced::Result {
    iced::application(
        BentoGridExample::new,
        BentoGridExample::update,
        BentoGridExample::view,
    )
    .title("Bento Grid — Magic UI bento-demo")
    .window(iced::window::Settings {
        // 758 - shell padding (48) - body padding (16) = browser reference 694px.
        size: Size::new(758.0, 920.0),
        ..Default::default()
    })
    .subscription(BentoGridExample::subscription)
    .font(GEIST_FONT_BYTES)
    .font(LUCIDE_FONT_BYTES)
    .default_font(GEIST_FONT)
    .run()
}

struct BentoGridExample {
    theme_shell: _shared::ThemeShell,
    started_at: Instant,
    elapsed: Duration,
    now: Instant,
    list_props: AnimatedListProps,
    list_state: AnimatedListState,
    beam_measurements: Option<AnchorMeasurements>,
    beam_remeasure: AnchorRemeasure,
    calendar_state: CalendarState,
    calendar_selected: Option<NaiveDate>,
    hover_motion: [AnimatedValue; CARD_COUNT],
    marquee_paused: bool,
    marquee_pause_started: Option<Instant>,
    marquee_paused_total: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Tick(Instant),
    BeamMeasured(AnchorMeasurements),
    Calendar(CalendarAction),
    CardHover(usize, bool),
    MarqueeHover(bool),
    Cta(&'static str),
}

impl BentoGridExample {
    fn new() -> (Self, Task<Message>) {
        let list_props = AnimatedListProps {
            max_width: 360.0,
            item_height: 72.0,
            gap: 12.0,
            ..AnimatedListProps::new()
        };
        let now = Instant::now();
        let selected = NaiveDate::from_ymd_opt(2022, 5, 11);
        let hover_spec = MotionSpec::new()
            .duration(TransitionDuration::Ms300)
            .easing(Easing::InOut);
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                started_at: now,
                elapsed: Duration::ZERO,
                now,
                list_props,
                list_state: AnimatedListState::new(&list_props),
                beam_measurements: None,
                beam_remeasure: AnchorRemeasure::new(),
                calendar_state: CalendarState::new(
                    selected.unwrap_or_else(|| NaiveDate::from_ymd_opt(2022, 5, 1).unwrap()),
                ),
                calendar_selected: selected,
                hover_motion: std::array::from_fn(|_| AnimatedValue::with_spec(0.0, hover_spec)),
                marquee_paused: false,
                marquee_pause_started: None,
                marquee_paused_total: Duration::ZERO,
            },
            measure_anchors(beam_demo::measure_ids(BeamSceneKind::MultipleOutputs))
                .map(Message::BeamMeasured),
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
                self.now = now;
                if !self.marquee_paused {
                    let pause = self.marquee_paused_total
                        + self
                            .marquee_pause_started
                            .map(|start| now.duration_since(start))
                            .unwrap_or(Duration::ZERO);
                    self.elapsed = now.duration_since(self.started_at).saturating_sub(pause);
                }
                self.list_state.tick(now, &self.list_props);
                return if self.beam_remeasure.take() {
                    measure_anchors(beam_demo::measure_ids(BeamSceneKind::MultipleOutputs))
                        .map(Message::BeamMeasured)
                } else {
                    Task::none()
                };
            }
            Message::BeamMeasured(measurements) => {
                self.beam_measurements = Some(measurements);
            }
            Message::Calendar(action) => match action {
                CalendarAction::MonthChanged(month) => {
                    self.calendar_state.current_month = month;
                }
                CalendarAction::Selected(date) => {
                    self.calendar_selected = date;
                }
                CalendarAction::RangeSelected(_, _) => {}
            },
            Message::CardHover(index, entered) => {
                if let Some(motion) = self.hover_motion.get_mut(index) {
                    motion.transition(if entered { 1.0 } else { 0.0 }, self.now);
                }
            }
            Message::MarqueeHover(entered) => {
                let now = Instant::now();
                if entered {
                    if !self.marquee_paused {
                        self.marquee_paused = true;
                        self.marquee_pause_started = Some(now);
                    }
                } else if self.marquee_paused {
                    if let Some(start) = self.marquee_pause_started.take() {
                        self.marquee_paused_total += now.duration_since(start);
                    }
                    self.marquee_paused = false;
                }
            }
            Message::Cta(_label) => {}
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let theme = self.theme_shell.theme();
        let dark = theme.variant().is_dark();
        let hover_progress = self
            .hover_motion
            .each_ref()
            .map(|motion| motion.value_at(self.now));

        let marquee_bg = self.files_marquee(dark);
        let list_bg = self.notifications_bg(hover_progress[1]);
        let beam_bg = self.integrations_bg(hover_progress[2]);
        let calendar_bg = self.calendar_bg(hover_progress[3]);

        let grid_props = BentoGridProps::new();
        let cards = vec![
            // col-span-3 lg:col-span-1
            BentoCardProps::new(
                "Save your files",
                lucide(
                    LucideIcon::FileText,
                    icon_size(hover_progress[0]),
                    icon_color(dark),
                ),
                marquee_bg,
            )
            .description("We automatically save your files as you type.")
            .cta("Learn more")
            .col_span(1)
            .hover_progress(hover_progress[0])
            .on_hover(Message::CardHover(0, true), Message::CardHover(0, false))
            .on_cta(Message::Cta("files")),
            // col-span-3 lg:col-span-2
            BentoCardProps::new(
                "Notifications",
                lucide(
                    LucideIcon::Bell,
                    icon_size(hover_progress[1]),
                    icon_color(dark),
                ),
                list_bg,
            )
            .description("Get notified when something happens.")
            .cta("Learn more")
            .col_span(2)
            .hover_progress(hover_progress[1])
            .on_hover(Message::CardHover(1, true), Message::CardHover(1, false))
            .on_cta(Message::Cta("notifications")),
            // col-span-3 lg:col-span-2
            BentoCardProps::new(
                "Integrations",
                lucide(
                    LucideIcon::Share2,
                    icon_size(hover_progress[2]),
                    icon_color(dark),
                ),
                beam_bg,
            )
            .description("Supports 100+ integrations and counting.")
            .cta("Learn more")
            .col_span(2)
            .hover_progress(hover_progress[2])
            .on_hover(Message::CardHover(2, true), Message::CardHover(2, false))
            .on_cta(Message::Cta("integrations")),
            // col-span-3 lg:col-span-1
            BentoCardProps::new(
                "Calendar",
                lucide(
                    LucideIcon::Calendar,
                    icon_size(hover_progress[3]),
                    icon_color(dark),
                ),
                calendar_bg,
            )
            .description("Use the calendar to filter your files by date.")
            .cta("Learn more")
            .col_span(1)
            .hover_progress(hover_progress[3])
            .on_hover(Message::CardHover(3, true), Message::CardHover(3, false))
            .on_cta(Message::Cta("calendar")),
        ];

        _shared::themed_shell(
            "Bento Grid",
            column![
                text("Magic UI bento-demo — feature showcase layout")
                    .size(14)
                    .style(|_| text::Style {
                        color: Some(Color {
                            a: 0.65,
                            ..theme.palette.foreground
                        }),
                    }),
                bento_grid(cards, grid_props, theme),
            ]
            .spacing(16)
            .width(Length::Fill)
            .padding([0.0, 8.0]),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }

    fn files_marquee(&self, dark: bool) -> Element<'_, Message> {
        let theme = self.theme_shell.theme();
        let fg = theme.palette.foreground;
        let render_file = |file: &FileItem, _copy: usize, _item: usize| -> Element<'_, Message> {
            let title = file.name;
            let body = file.body;
            container(
                column![
                    text(title)
                        .size(13)
                        .style(move |_| text::Style { color: Some(fg) }),
                    text(body).size(11).style(move |_| text::Style {
                        color: Some(Color { a: 0.65, ..fg }),
                    }),
                ]
                .spacing(6),
            )
            .width(Length::Fixed(128.0))
            .height(Length::Fixed(96.0))
            .padding(12)
            .style(move |_| {
                let (bg_a, border_a) = if dark { (0.10, 0.10) } else { (0.01, 0.10) };
                container::Style {
                    background: Some(Background::Color(Color {
                        a: bg_a,
                        ..if dark { Color::WHITE } else { Color::BLACK }
                    })),
                    border: Border {
                        color: Color {
                            a: border_a,
                            ..if dark { Color::WHITE } else { Color::BLACK }
                        },
                        width: 1.0,
                        radius: 12.0.into(),
                    },
                    ..Default::default()
                }
            })
            .into()
        };

        let marquee = marquee_indexed(
            FILES,
            render_file,
            self.elapsed,
            MarqueeProps {
                orientation: MarqueeOrientation::Horizontal,
                reverse: false,
                pause_on_hover: true,
                repeat: 4,
                gap: 12.0,
                // Magic UI --duration:20s over typical track
                speed_px_per_sec: 40.0,
                item_extent: 128.0,
                item_cross: 96.0,
                padding: 4.0,
                edge_fade: false,
            },
            theme,
            Message::MarqueeHover,
        );

        // Mask toward bottom so meta text stays readable (mask-image linear-gradient to top).
        let fade = container(bottom_background_fade(120.0, theme.palette.background))
            .width(Length::Fill)
            .height(Length::Fixed(120.0));

        container(
            stack![
                container(marquee)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(Padding {
                        top: 40.0,
                        right: 0.0,
                        bottom: 0.0,
                        left: 0.0,
                    }),
                container(fade)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_y(Alignment::End),
            ]
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn notifications_bg(&self, hover_progress: f32) -> Element<'_, Message> {
        let theme = self.theme_shell.theme();
        let items = repeated_notifications(6)
            .into_iter()
            .map(|item| notification_card(item, theme))
            .collect();

        // Magic UI: scale-75 → group-hover:scale-90
        let scale = 0.75 + 0.15 * hover_progress.clamp(0.0, 1.0);
        let list_h = LIST_STAGE_H * scale;

        let list = scrollable(
            container(animated_list(
                items,
                &self.list_state,
                self.now,
                &self.list_props,
            ))
            .padding(12),
        )
        .width(Length::Fill)
        .height(Length::Fixed(list_h))
        .direction(scrollable::Direction::Vertical(Scrollbar::hidden()));

        let fade = container(bottom_background_fade(
            LIST_FADE_H,
            theme.palette.background,
        ))
        .width(Length::Fill)
        .height(Length::Fixed(LIST_FADE_H));

        container(
            stack![
                container(list)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(Padding {
                        top: 8.0,
                        right: 8.0,
                        bottom: 0.0,
                        left: 0.0,
                    }),
                container(fade)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_y(Alignment::End),
            ]
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .clip(true)
        .into()
    }

    fn integrations_bg(&self, hover_progress: f32) -> Element<'_, Message> {
        let theme = self.theme_shell.theme();
        let scene =
            beam_demo::multiple_outputs_scene(self.elapsed, self.beam_measurements.as_ref());

        // Fit multi-output scene into the card (CSS scale-75 / scale-105 on hover).
        let scale = 0.62 + 0.08 * hover_progress.clamp(0.0, 1.0);
        let w = beam_demo::SCENE_WIDTH * scale;
        let h = 300.0 * scale;

        let fade = container(bottom_background_fade(90.0, theme.palette.background))
            .width(Length::Fill)
            .height(Length::Fixed(90.0));

        container(
            stack![
                container(
                    container(scene)
                        .width(Length::Fixed(w))
                        .height(Length::Fixed(h)),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::End)
                .align_y(Alignment::Start)
                .padding(Padding {
                    top: 12.0,
                    right: 8.0,
                    bottom: 0.0,
                    left: 0.0,
                }),
                container(fade)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_y(Alignment::End),
            ]
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .clip(true)
        .into()
    }

    fn calendar_bg(&self, hover_progress: f32) -> Element<'_, Message> {
        let theme = self.theme_shell.theme();
        let border = theme.palette.border;
        let card = theme.palette.card;
        let radius = theme.radius.md;

        // Magic UI: origin-top scale-75 → group-hover:scale-90
        let scale = 0.75 + 0.15 * hover_progress.clamp(0.0, 1.0);

        let cal = calendar(
            CalendarProps::new("bento-calendar")
                .mode(CalendarMode::Single)
                .selected(self.calendar_selected),
            self.calendar_state,
            Some(Message::Calendar),
            theme,
        );

        let fade = container(bottom_background_fade(100.0, theme.palette.background))
            .width(Length::Fill)
            .height(Length::Fixed(100.0));

        container(
            stack![
                container(
                    container(cal)
                        .style(move |_| container::Style {
                            border: Border {
                                color: border,
                                width: 1.0,
                                radius: radius.into(),
                            },
                            background: Some(Background::Color(card)),
                            ..Default::default()
                        })
                        // Approximate CSS scale via fixed visual width so hover grows.
                        .width(Length::Fixed(280.0 * scale)),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::End)
                .align_y(Alignment::Start)
                .padding(Padding {
                    top: 32.0,
                    right: 0.0,
                    bottom: 0.0,
                    left: 0.0,
                }),
                container(fade)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_y(Alignment::End),
            ]
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .clip(true)
        .into()
    }
}

fn icon_color(dark: bool) -> Color {
    if dark {
        Color::from_rgb8(0xD4, 0xD4, 0xD8)
    } else {
        Color::from_rgb8(0x40, 0x40, 0x40)
    }
}

fn icon_size(hover_progress: f32) -> f32 {
    48.0 - 12.0 * hover_progress.clamp(0.0, 1.0)
}

fn lucide<'a, Message: 'a>(icon: LucideIcon, size: f32, color: Color) -> Element<'a, Message> {
    text(char::from(icon).to_string())
        .font(Font::with_name("lucide"))
        .size(size)
        .style(move |_| text::Style { color: Some(color) })
        .into()
}
