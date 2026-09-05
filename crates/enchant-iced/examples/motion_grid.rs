mod _shared;

use _shared::ThemeShell;
use enchant_iced::MotionGrid;
use iced::time::Instant;
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Background, Border, Color, Element, Font, Length, Size, Subscription, Task};
use state_frames::{STATE_ORDER, state_at};
use std::time::Duration;

const TICK_MS: u64 = 16;
const STATE_CYCLE_MS: u64 = 3000;
const TAB_LABELS: &[&str] = &[
    "Preview",
    "Manual Switcher",
    "Custom Cells",
    "Child Wrapper",
];

/// Tailwind `h-11` on the docs preview button.
const PREVIEW_BUTTON_HEIGHT: f32 = 44.0;
/// Tailwind `px-3`.
const PREVIEW_BUTTON_PADDING_H: f32 = 12.0;
/// Tailwind `gap-x-3` between grid and label.
const PREVIEW_GRID_LABEL_GAP: f32 = 12.0;
const PREVIEW_CELL_SIZE: f32 = 3.0;
const PREVIEW_GAP: f32 = 2.0;
const PREVIEW_GRID_COLS: u16 = 5;
/// `cols * cell_size + (cols - 1) * gap` with preview defaults.
const PREVIEW_GRID_EXTENT: f32 =
    PREVIEW_GRID_COLS as f32 * PREVIEW_CELL_SIZE + (PREVIEW_GRID_COLS - 1) as f32 * PREVIEW_GAP;

const DEMO_GRID_COLS: u16 = 5;
const DEMO_GAP: f32 = 4.0;
const DEMO_CELL_SIZE: f32 = 10.0;
const MANUAL_ACTIVE_SCALE: f32 = 1.1;

pub fn main() -> iced::Result {
    iced::application(
        MotionGridExample::new,
        MotionGridExample::update,
        MotionGridExample::view,
    )
    .title("Motion Grid")
    .window(iced::window::Settings {
        size: Size::new(1000.0, 560.0),
        ..Default::default()
    })
    .subscription(MotionGridExample::subscription)
    .run()
}

struct MotionGridExample {
    theme_shell: ThemeShell,
    active_tab: usize,
    animation_start: Instant,
    state_start: Instant,
    state_index: usize,
    presses: u32,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Tick(Instant),
    Pressed,
    SelectTab(usize),
    SelectState(usize),
}

impl MotionGridExample {
    fn new() -> (Self, Task<Message>) {
        let start = Instant::now();
        (
            Self {
                theme_shell: ThemeShell::new(),
                active_tab: 0,
                animation_start: start,
                state_start: start,
                state_index: 0,
                presses: 0,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleTheme => {
                return ThemeShell::measure_toggle_anchor().map(Message::AnchorsMeasured);
            }
            Message::AnchorsMeasured(measurements) => {
                self.theme_shell.apply_anchor_and_toggle(&measurements);
            }
            Message::SelectTab(index) => {
                self.active_tab = index;
            }
            Message::SelectState(index) => {
                self.state_index = index % STATE_ORDER.len();
                self.state_start = self.theme_shell.now;
            }
            Message::Tick(now) => {
                self.theme_shell.tick(now);
                if self.active_tab == 0 {
                    let state_elapsed = now.duration_since(self.state_start);
                    if state_elapsed >= Duration::from_millis(STATE_CYCLE_MS) {
                        self.state_index = (self.state_index + 1) % STATE_ORDER.len();
                        self.state_start = now;
                    }
                }
            }
            Message::Pressed => {
                self.presses = self.presses.saturating_add(1);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(TICK_MS)).map(Message::Tick)
    }

    fn animation_elapsed(&self) -> Duration {
        self.theme_shell.now.duration_since(self.animation_start)
    }

    fn state_elapsed(&self) -> Duration {
        self.theme_shell.now.duration_since(self.state_start)
    }

    fn view(&self) -> Element<'_, Message> {
        let tabs = _shared::tabs(TAB_LABELS, self.active_tab, Message::SelectTab);
        let body = match self.active_tab {
            0 => self.preview_tab(),
            1 => self.manual_switcher_tab(),
            2 => self.custom_cells_tab(),
            _ => self.child_wrapper_tab(),
        };

        _shared::themed_shell(
            "Motion Grid",
            column![tabs, _shared::stage(body)].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }

    fn preview_tab(&self) -> Element<'_, Message> {
        let now = self.theme_shell.now;
        let state = state_at(self.state_index);
        let dark = self.theme_shell.controller.dark_mode();
        let (inactive_color, active_color) = grid_dot_colors(dark);

        let grid = MotionGrid::new(PREVIEW_GRID_COLS, PREVIEW_GRID_COLS)
            .frames(state.frames.iter().map(|frame| frame.iter().copied()))
            .cell_size(PREVIEW_CELL_SIZE)
            .gap(PREVIEW_GAP)
            .inactive_color(inactive_color)
            .active_color(active_color)
            .into_element(self.state_elapsed());

        let label_color = self.theme_shell.blended_theme().palette.primary_foreground;
        let label = text(state.label)
            .size(14)
            .font(Font {
                weight: iced::font::Weight::Medium,
                ..Font::DEFAULT
            })
            .style(move |_| iced::widget::text::Style {
                color: Some(label_color),
            });

        let primary = self.theme_shell.blended_primary(now);
        let primary_fg = self.theme_shell.blended_theme().palette.primary_foreground;
        let content = row![grid, label]
            .spacing(PREVIEW_GRID_LABEL_GAP)
            .align_y(Alignment::Center);

        let preview_button = button(content)
            .padding([
                (PREVIEW_BUTTON_HEIGHT - PREVIEW_GRID_EXTENT) * 0.5,
                PREVIEW_BUTTON_PADDING_H,
            ])
            .on_press(Message::Pressed)
            .style(move |_theme, status| primary_button_style(primary, primary_fg, status));

        let feedback = text(format!("clicks: {}", self.presses))
            .size(14)
            .color(self.theme_shell.blended_foreground(now));

        column![preview_button, feedback]
            .spacing(16)
            .align_x(Alignment::Center)
            .into()
    }

    fn manual_switcher_tab(&self) -> Element<'_, Message> {
        let now = self.theme_shell.now;
        let palette = self.theme_shell.blended_theme().palette;
        let state = state_at(self.state_index);
        let inactive_color = palette.muted;
        let active_color = palette.foreground;

        let chips: Vec<Element<'_, Message>> = STATE_ORDER
            .iter()
            .enumerate()
            .map(|(index, motion_state)| {
                let selected = index == self.state_index;
                let primary = palette.primary;
                let primary_fg = palette.primary_foreground;
                let border = self.theme_shell.blended_border(now);
                let background = palette.background;
                let foreground = palette.foreground;
                button(text(motion_state.label).size(12))
                    .padding([4.0, 10.0])
                    .on_press(Message::SelectState(index))
                    .style(move |_theme, _status| {
                        if selected {
                            iced::widget::button::Style {
                                background: Some(Background::Color(primary)),
                                text_color: primary_fg,
                                border: Border {
                                    radius: 6.0.into(),
                                    width: 1.0,
                                    color: primary,
                                },
                                ..iced::widget::button::Style::default()
                            }
                        } else {
                            iced::widget::button::Style {
                                background: Some(Background::Color(background)),
                                text_color: foreground,
                                border: Border {
                                    radius: 6.0.into(),
                                    width: 1.0,
                                    color: border,
                                },
                                ..iced::widget::button::Style::default()
                            }
                        }
                    })
                    .into()
            })
            .collect();

        let grid = MotionGrid::new(DEMO_GRID_COLS, DEMO_GRID_COLS)
            .frames(state.frames.iter().map(|frame| frame.iter().copied()))
            .cell_size(DEMO_CELL_SIZE)
            .gap(DEMO_GAP)
            .inactive_color(inactive_color)
            .active_color(active_color)
            .active_scale(MANUAL_ACTIVE_SCALE)
            .into_element(self.state_elapsed());

        let label = text(state.label)
            .size(14)
            .font(Font {
                weight: iced::font::Weight::Medium,
                ..Font::DEFAULT
            })
            .color(palette.foreground);

        column![
            row(chips).spacing(8),
            row![grid, label].spacing(12).align_y(Alignment::Center)
        ]
        .spacing(16)
        .align_x(Alignment::Center)
        .into()
    }

    fn custom_cells_tab(&self) -> Element<'_, Message> {
        let palette = self.theme_shell.blended_theme().palette;
        let muted_fg = palette.muted_foreground;
        let searching = state_frames::motion_state_by_label("Searching");

        let inactive_bg = Color {
            a: 0.40,
            ..palette.muted
        };
        let inactive_border = Color {
            a: 0.30,
            ..muted_fg
        };
        let active_fill = Color::from_rgb8(0x34, 0xD3, 0x99);
        let active_border = Color::from_rgba(0.431, 0.906, 0.718, 0.70);
        let active_glow = Color::from_rgba(0.204, 0.827, 0.600, 0.65);

        let grid = MotionGrid::new(DEMO_GRID_COLS, DEMO_GRID_COLS)
            .frames(searching.frames.iter().map(|frame| frame.iter().copied()))
            .duration_ms(220)
            .cell_size(DEMO_CELL_SIZE)
            .gap(DEMO_GAP)
            .cell_radius(2.0)
            .inactive_color(inactive_bg)
            .active_color(active_fill)
            .inactive_border_color(inactive_border)
            .active_border_color(active_border)
            .cell_border_width(1.0)
            .active_glow(active_glow)
            .active_pulse(true)
            .inactive_scale(0.9)
            .into_element(self.animation_elapsed());

        let helper = text(
            "Use builder setters (active_color, inactive_color, active_pulse, …) \
             to style and animate cell states without changing frame data.",
        )
        .size(14)
        .color(muted_fg)
        .width(Length::Fixed(320.0));

        row![grid, helper]
            .spacing(16)
            .align_y(Alignment::Center)
            .into()
    }

    fn child_wrapper_tab(&self) -> Element<'_, Message> {
        let now = self.theme_shell.now;
        let palette = self.theme_shell.blended_theme().palette;
        let muted_fg = palette.muted_foreground;
        let syncing = state_frames::motion_state_by_label("Syncing");
        let border = self.theme_shell.blended_border(now);

        let inactive_color = Color {
            a: 0.15,
            ..palette.foreground
        };
        let active_color = Color {
            a: 0.90,
            ..palette.foreground
        };

        let grid = MotionGrid::new(DEMO_GRID_COLS, DEMO_GRID_COLS)
            .frames(syncing.frames.iter().map(|frame| frame.iter().copied()))
            .duration_ms(180)
            .cell_size(8.0)
            .gap(DEMO_GAP)
            .inactive_color(inactive_color)
            .active_color(active_color)
            .into_element(self.animation_elapsed());

        let wrapped = container(grid).padding(8).style(move |_| container::Style {
            background: Some(Background::Color(palette.muted)),
            border: Border {
                width: 1.0,
                radius: 8.0.into(),
                color: border,
            },
            ..container::Style::default()
        });

        let helper = text(
            "This wraps MotionGrid in a custom container (iced analogue of the child render-prop).",
        )
        .size(14)
        .color(muted_fg);

        row![wrapped, helper]
            .spacing(16)
            .align_y(Alignment::Center)
            .into()
    }
}

fn grid_dot_colors(dark: bool) -> (Color, Color) {
    if dark {
        (
            Color::from_rgba(0.0, 0.0, 0.0, 0.20),
            Color::from_rgba(0.0, 0.0, 0.0, 0.70),
        )
    } else {
        (
            Color::from_rgba(1.0, 1.0, 1.0, 0.20),
            Color::from_rgba(1.0, 1.0, 1.0, 0.70),
        )
    }
}

fn primary_button_style(
    primary: Color,
    primary_fg: Color,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let mut style = iced::widget::button::Style {
        background: Some(Background::Color(primary)),
        text_color: primary_fg,
        border: Border {
            radius: 0.0.into(),
            ..Border::default()
        },
        ..iced::widget::button::Style::default()
    };

    if matches!(status, iced::widget::button::Status::Pressed) {
        style.text_color = Color {
            a: 0.92,
            ..primary_fg
        };
    }

    style
}

mod state_frames {
    type Dots = &'static [(u16, u16)];
    pub type Frames = &'static [Dots];

    pub struct MotionState {
        pub label: &'static str,
        pub frames: Frames,
    }

    const IMPORTING_FRAMES: Frames = &[
        &[(2, 2)],
        &[(1, 2), (2, 1), (2, 3), (3, 2)],
        &[
            (2, 2),
            (0, 2),
            (1, 1),
            (1, 3),
            (2, 0),
            (2, 4),
            (3, 1),
            (3, 3),
            (4, 2),
        ],
        &[
            (0, 1),
            (0, 3),
            (1, 0),
            (1, 2),
            (1, 4),
            (2, 1),
            (2, 3),
            (3, 0),
            (3, 2),
            (3, 4),
            (4, 1),
            (4, 3),
        ],
        &[
            (0, 0),
            (0, 2),
            (0, 4),
            (1, 1),
            (1, 3),
            (2, 0),
            (2, 2),
            (2, 4),
            (3, 1),
            (3, 3),
            (4, 0),
            (4, 2),
            (4, 4),
        ],
        &[
            (0, 1),
            (0, 3),
            (1, 0),
            (1, 2),
            (1, 4),
            (2, 1),
            (2, 3),
            (3, 0),
            (3, 2),
            (3, 4),
            (4, 1),
            (4, 3),
        ],
        &[
            (0, 0),
            (0, 2),
            (0, 4),
            (1, 1),
            (1, 3),
            (2, 0),
            (2, 4),
            (3, 1),
            (3, 3),
            (4, 0),
            (4, 2),
            (4, 4),
        ],
        &[
            (0, 1),
            (1, 0),
            (3, 0),
            (4, 1),
            (0, 3),
            (1, 4),
            (3, 4),
            (4, 3),
        ],
        &[(0, 0), (0, 4), (4, 0), (4, 4)],
        &[],
    ];

    #[allow(dead_code)]
    const ARROW_DOWN_FRAMES: Frames = &[
        &[(2, 0)],
        &[(1, 0), (2, 0), (3, 0), (2, 1)],
        &[(2, 0), (1, 1), (2, 1), (3, 1), (2, 2)],
        &[(2, 0), (2, 1), (1, 2), (2, 2), (3, 2), (2, 3)],
        &[(2, 1), (2, 2), (1, 3), (2, 3), (3, 3), (2, 4)],
        &[(2, 2), (2, 3), (1, 4), (2, 4), (3, 4)],
        &[(2, 3), (2, 4)],
        &[(2, 4)],
        &[],
    ];

    #[allow(dead_code)]
    const ARROW_UP_FRAMES: Frames = &[
        &[(2, 4)],
        &[(1, 4), (2, 4), (3, 4), (2, 3)],
        &[(2, 4), (1, 3), (2, 3), (3, 3), (2, 2)],
        &[(2, 4), (2, 3), (1, 2), (2, 2), (3, 2), (2, 1)],
        &[(2, 3), (2, 2), (1, 1), (2, 1), (3, 1), (2, 0)],
        &[(2, 2), (2, 1), (1, 0), (2, 0), (3, 0)],
        &[(2, 1), (2, 0)],
        &[(2, 0)],
        &[],
    ];

    const SYNCING_FRAMES: Frames = &[
        &[(2, 0)],
        &[(1, 0), (2, 0), (3, 0), (2, 1)],
        &[(2, 0), (1, 1), (2, 1), (3, 1), (2, 2)],
        &[(2, 0), (2, 1), (1, 2), (2, 2), (3, 2), (2, 3)],
        &[(2, 1), (2, 2), (1, 3), (2, 3), (3, 3), (2, 4)],
        &[(2, 2), (2, 3), (1, 4), (2, 4), (3, 4)],
        &[(2, 3), (2, 4)],
        &[(2, 4)],
        &[],
        &[(2, 4)],
        &[(1, 4), (2, 4), (3, 4), (2, 3)],
        &[(2, 4), (1, 3), (2, 3), (3, 3), (2, 2)],
        &[(2, 4), (2, 3), (1, 2), (2, 2), (3, 2), (2, 1)],
        &[(2, 3), (2, 2), (1, 1), (2, 1), (3, 1), (2, 0)],
        &[(2, 2), (2, 1), (1, 0), (2, 0), (3, 0)],
        &[(2, 1), (2, 0)],
        &[(2, 0)],
        &[],
    ];

    const SEARCHING_FRAMES: Frames = &[
        &[(1, 0), (0, 1), (1, 1), (2, 1), (1, 2)],
        &[(2, 0), (1, 1), (2, 1), (3, 1), (2, 2)],
        &[(3, 0), (2, 1), (3, 1), (4, 1), (3, 2)],
        &[(3, 1), (2, 2), (3, 2), (4, 2), (3, 3)],
        &[(3, 2), (2, 3), (3, 3), (4, 3), (3, 4)],
        &[(1, 2), (0, 3), (1, 3), (2, 3), (1, 4)],
        &[
            (0, 0),
            (0, 1),
            (0, 2),
            (1, 0),
            (1, 2),
            (2, 0),
            (2, 1),
            (2, 2),
        ],
        &[],
    ];

    const BUSY_FRAMES: Frames = &[
        &[(0, 1), (0, 2), (0, 3), (1, 2), (4, 1), (4, 2), (4, 3)],
        &[(0, 1), (0, 2), (0, 3), (2, 3), (4, 2), (4, 3), (4, 4)],
        &[(0, 1), (0, 2), (0, 3), (3, 4), (4, 2), (4, 3), (4, 4)],
        &[(0, 1), (0, 2), (0, 3), (2, 3), (4, 2), (4, 3), (4, 4)],
        &[(0, 0), (0, 1), (0, 2), (1, 2), (4, 2), (4, 3), (4, 4)],
        &[(0, 0), (0, 1), (0, 2), (2, 1), (4, 1), (4, 2), (4, 3)],
        &[(0, 0), (0, 1), (0, 2), (3, 0), (4, 0), (4, 1), (4, 2)],
        &[(0, 1), (0, 2), (0, 3), (2, 1), (4, 0), (4, 1), (4, 2)],
    ];

    const SAVING_FRAMES: Frames = &[
        &[
            (0, 0),
            (0, 1),
            (0, 2),
            (0, 3),
            (0, 4),
            (1, 0),
            (1, 1),
            (1, 2),
            (1, 3),
            (2, 0),
            (2, 1),
            (2, 2),
            (2, 3),
            (2, 4),
            (3, 0),
            (3, 1),
            (3, 2),
            (3, 3),
            (4, 0),
            (4, 1),
            (4, 2),
            (4, 3),
            (4, 4),
        ],
        &[
            (0, 0),
            (0, 1),
            (0, 2),
            (0, 3),
            (1, 0),
            (1, 1),
            (1, 2),
            (2, 0),
            (2, 1),
            (2, 2),
            (2, 3),
            (3, 0),
            (3, 1),
            (3, 2),
            (4, 0),
            (4, 1),
            (4, 2),
            (4, 3),
        ],
        &[
            (0, 0),
            (0, 1),
            (0, 2),
            (1, 0),
            (1, 1),
            (2, 0),
            (2, 1),
            (2, 2),
            (3, 0),
            (3, 1),
            (4, 0),
            (4, 1),
            (4, 2),
            (4, 4),
            (3, 4),
            (2, 4),
            (1, 4),
            (0, 4),
        ],
        &[
            (0, 0),
            (0, 1),
            (1, 0),
            (2, 0),
            (2, 1),
            (3, 0),
            (4, 0),
            (4, 1),
            (4, 3),
            (3, 3),
            (2, 3),
            (1, 3),
            (0, 3),
            (4, 4),
            (3, 4),
            (2, 4),
            (1, 4),
            (0, 4),
        ],
        &[
            (0, 0),
            (2, 0),
            (4, 0),
            (4, 2),
            (3, 2),
            (2, 2),
            (1, 2),
            (0, 2),
            (4, 3),
            (3, 3),
            (2, 3),
            (1, 3),
            (0, 3),
            (4, 4),
            (3, 4),
            (2, 4),
            (1, 4),
            (0, 4),
        ],
        &[
            (0, 0),
            (1, 0),
            (2, 0),
            (3, 0),
            (4, 0),
            (4, 1),
            (3, 1),
            (2, 1),
            (1, 1),
            (0, 1),
            (4, 2),
            (3, 2),
            (2, 2),
            (1, 2),
            (0, 2),
            (4, 3),
            (3, 3),
            (2, 3),
            (1, 3),
            (0, 3),
            (4, 4),
            (3, 4),
            (2, 4),
            (1, 4),
            (0, 4),
        ],
        &[
            (0, 0),
            (1, 0),
            (2, 0),
            (3, 0),
            (4, 0),
            (4, 1),
            (3, 1),
            (2, 1),
            (1, 1),
            (0, 1),
            (4, 2),
            (3, 2),
            (2, 2),
            (1, 2),
            (0, 2),
            (4, 3),
            (3, 3),
            (2, 3),
            (1, 3),
            (0, 3),
            (4, 4),
            (3, 4),
            (2, 4),
            (1, 4),
            (0, 4),
        ],
        &[
            (0, 0),
            (1, 0),
            (2, 0),
            (3, 0),
            (4, 0),
            (4, 1),
            (3, 1),
            (2, 1),
            (1, 1),
            (0, 1),
            (4, 2),
            (3, 2),
            (2, 2),
            (1, 2),
            (0, 2),
            (4, 3),
            (3, 3),
            (2, 3),
            (1, 3),
            (0, 3),
            (4, 4),
            (3, 4),
            (2, 4),
            (1, 4),
            (0, 4),
        ],
    ];

    const INITIALIZING_FRAMES: Frames = &[
        &[],
        &[(1, 0), (3, 0)],
        &[(1, 0), (3, 0), (0, 1), (1, 1), (2, 1), (3, 1), (4, 1)],
        &[
            (1, 0),
            (3, 0),
            (0, 1),
            (1, 1),
            (2, 1),
            (3, 1),
            (4, 1),
            (0, 2),
            (1, 2),
            (2, 2),
            (3, 2),
            (4, 2),
        ],
        &[
            (1, 0),
            (3, 0),
            (0, 1),
            (1, 1),
            (2, 1),
            (3, 1),
            (4, 1),
            (0, 2),
            (1, 2),
            (2, 2),
            (3, 2),
            (4, 2),
            (1, 3),
            (2, 3),
            (3, 3),
        ],
        &[
            (1, 0),
            (3, 0),
            (0, 1),
            (1, 1),
            (2, 1),
            (3, 1),
            (4, 1),
            (0, 2),
            (1, 2),
            (2, 2),
            (3, 2),
            (4, 2),
            (1, 3),
            (2, 3),
            (3, 3),
            (2, 4),
        ],
        &[(1, 2), (2, 1), (2, 2), (2, 3), (3, 2)],
        &[(2, 2)],
        &[],
    ];

    pub const STATE_ORDER: &[MotionState] = &[
        MotionState {
            label: "Importing",
            frames: IMPORTING_FRAMES,
        },
        MotionState {
            label: "Syncing",
            frames: SYNCING_FRAMES,
        },
        MotionState {
            label: "Searching",
            frames: SEARCHING_FRAMES,
        },
        MotionState {
            label: "Busy",
            frames: BUSY_FRAMES,
        },
        MotionState {
            label: "Saving",
            frames: SAVING_FRAMES,
        },
        MotionState {
            label: "Initializing",
            frames: INITIALIZING_FRAMES,
        },
    ];

    pub fn state_at(index: usize) -> &'static MotionState {
        &STATE_ORDER[index % STATE_ORDER.len()]
    }

    pub fn motion_state_by_label(label: &str) -> &'static MotionState {
        STATE_ORDER
            .iter()
            .find(|state| state.label == label)
            .unwrap_or(&STATE_ORDER[0])
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn syncing_frames_match_arrow_concatenation() {
            assert_eq!(
                SYNCING_FRAMES.len(),
                ARROW_DOWN_FRAMES.len() + ARROW_UP_FRAMES.len()
            );
            for (index, frame) in ARROW_DOWN_FRAMES.iter().enumerate() {
                assert_eq!(SYNCING_FRAMES[index], *frame);
            }
            let offset = ARROW_DOWN_FRAMES.len();
            for (index, frame) in ARROW_UP_FRAMES.iter().enumerate() {
                assert_eq!(SYNCING_FRAMES[offset + index], *frame);
            }
        }
    }
}
