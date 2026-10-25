//! Thinking orbs gallery: all nine states at every size, with live controls.
//!
//! ```text
//! cargo run -p enchant-iced --example thinking_orb
//! ```
//!
//! Snapshot mode renders a fixed, controls-free grid at a frozen clock and
//! writes it as a PNG, for comparing frames against the reference library:
//!
//! ```text
//! THINKING_ORB_SNAPSHOT=out.png THINKING_ORB_CLOCK=1.25 THINKING_ORB_DARK=1 \
//!     cargo run -p enchant-iced --example thinking_orb
//! ```

use std::time::Duration;

use enchant_iced::{
    CursorSprite, Gravity, MotionPreference, OrbSize, OrbState, OrbTheme, ThinkingOrb,
};
use iced::widget::{checkbox, column, container, row, slider, text};
use iced::{Alignment, Color, Element, Length, Size, Subscription, Task, Theme, window};

/// Cell side of the snapshot grid, so crops are trivial.
const CELL: f32 = 96.0;
const SNAPSHOT_FRAMES: u32 = 12;

pub fn main() -> iced::Result {
    let snapshot = Snapshot::from_env();
    let size = if snapshot.is_some() {
        Size::new(
            CELL * OrbState::ALL.len() as f32,
            CELL * OrbSize::ALL.len() as f32,
        )
    } else {
        Size::new(980.0, 640.0)
    };
    iced::application(Gallery::new, Gallery::update, Gallery::view)
        .title("Thinking Orbs")
        .theme(Gallery::theme)
        .antialiasing(true)
        .window(window::Settings {
            size,
            resizable: snapshot.is_none(),
            ..Default::default()
        })
        .subscription(Gallery::subscription)
        .run()
}

#[derive(Clone, Debug)]
struct Snapshot {
    path: String,
    clock: f64,
}

impl Snapshot {
    fn from_env() -> Option<Self> {
        let path = std::env::var("THINKING_ORB_SNAPSHOT").ok()?;
        let clock = std::env::var("THINKING_ORB_CLOCK")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1.0);
        Some(Self { path, clock })
    }
}

struct Gallery {
    snapshot: Option<Snapshot>,
    dark: bool,
    speed: f32,
    dots: f32,
    dot_size: f32,
    paused: bool,
    tinted: bool,
    reduced: bool,
    gravity: bool,
    frames: u32,
}

#[derive(Debug, Clone)]
enum Message {
    Frame,
    Dark(bool),
    Speed(f32),
    Dots(f32),
    DotSize(f32),
    Paused(bool),
    Tinted(bool),
    Reduced(bool),
    Gravity(bool),
    Captured(window::Screenshot),
}

impl Gallery {
    fn new() -> (Self, Task<Message>) {
        let snapshot = Snapshot::from_env();
        let dark = std::env::var("THINKING_ORB_DARK").is_ok_and(|v| v == "1");
        (
            Self {
                snapshot,
                dark,
                speed: 1.0,
                dots: 1.0,
                dot_size: 1.0,
                paused: false,
                tinted: false,
                reduced: false,
                gravity: false,
                frames: 0,
            },
            Task::none(),
        )
    }

    fn theme(&self) -> Theme {
        if self.dark { Theme::Dark } else { Theme::Light }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Frame => {
                self.frames += 1;
                if self.snapshot.is_some() && self.frames == SNAPSHOT_FRAMES {
                    return window::latest()
                        .and_then(window::screenshot)
                        .map(Message::Captured);
                }
            }
            Message::Dark(v) => self.dark = v,
            Message::Speed(v) => self.speed = v,
            Message::Dots(v) => self.dots = v,
            Message::DotSize(v) => self.dot_size = v,
            Message::Paused(v) => self.paused = v,
            Message::Tinted(v) => self.tinted = v,
            Message::Reduced(v) => self.reduced = v,
            Message::Gravity(v) => {
                self.gravity = v;
                // gravity needs the real pointer raster; the stock arrow at 2x
                // is a convenient stand-in on hi-dpi screens
                enchant_iced::gravity::set_sprite(v.then(|| CursorSprite::classic_arrow(2)));
            }
            Message::Captured(shot) => {
                let Some(snapshot) = &self.snapshot else {
                    return Task::none();
                };
                if let Err(err) = image::save_buffer(
                    &snapshot.path,
                    &shot.rgba,
                    shot.size.width,
                    shot.size.height,
                    image::ColorType::Rgba8,
                ) {
                    eprintln!("could not write {}: {err}", snapshot.path);
                }
                return iced::exit();
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        // keeps the snapshot frame counter moving; interactive orbs redraw themselves
        iced::time::every(Duration::from_millis(16)).map(|_| Message::Frame)
    }

    fn orb(&self, state: OrbState, size: OrbSize) -> ThinkingOrb {
        let mut orb = ThinkingOrb::new()
            .state(state)
            .size(size)
            .theme(OrbTheme::Auto)
            .speed(self.speed)
            .dots(self.dots)
            .dot_size(self.dot_size)
            .paused(self.paused);
        if self.tinted {
            orb = orb.color(Color::from_rgb8(0x7c, 0x3a, 0xed));
        }
        if self.reduced {
            orb = orb.motion(MotionPreference::Reduced);
        }
        if self.gravity {
            orb = orb.gravity(Gravity::new());
        }
        if let Some(snapshot) = &self.snapshot {
            orb = orb.elapsed(Duration::from_secs_f64(snapshot.clock));
        }
        orb
    }

    fn view(&self) -> Element<'_, Message> {
        if self.snapshot.is_some() {
            return self.snapshot_grid();
        }

        let header = row![
            text("Thinking Orbs").size(28),
            checkbox(self.dark).label("Dark").on_toggle(Message::Dark),
            checkbox(self.paused)
                .label("Paused")
                .on_toggle(Message::Paused),
            checkbox(self.tinted)
                .label("Tinted")
                .on_toggle(Message::Tinted),
            checkbox(self.reduced)
                .label("Reduced motion")
                .on_toggle(Message::Reduced),
            checkbox(self.gravity)
                .label("Gravity")
                .on_toggle(Message::Gravity),
        ]
        .spacing(18)
        .align_y(Alignment::Center);

        let sliders = row![
            labelled(
                "Speed",
                slider(0.0..=3.0, self.speed, Message::Speed).step(0.05_f32)
            ),
            labelled(
                "Dots",
                slider(0.1..=3.0, self.dots, Message::Dots).step(0.05_f32)
            ),
            labelled(
                "Dot size",
                slider(0.1..=3.0, self.dot_size, Message::DotSize).step(0.05_f32)
            ),
        ]
        .spacing(24);

        let mut grid = column![].spacing(18);
        for size in OrbSize::ALL {
            let mut line = row![text(size.to_string()).width(56)]
                .spacing(18)
                .align_y(Alignment::Center);
            for state in OrbState::ALL {
                line = line.push(
                    column![
                        self.orb(state, size).into_element(),
                        text(state.name()).size(11)
                    ]
                    .spacing(6)
                    .width(86)
                    .align_x(Alignment::Center),
                );
            }
            grid = grid.push(line);
        }

        container(column![header, sliders, grid].spacing(24))
            .padding(24)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Fixed grid, no chrome: one cell per (size, state), background pinned
    /// to pure white or black so crops compare cleanly.
    fn snapshot_grid(&self) -> Element<'_, Message> {
        let ground = if self.dark {
            Color::BLACK
        } else {
            Color::WHITE
        };
        let mut rows = column![];
        for size in OrbSize::ALL {
            let mut line = row![];
            for state in OrbState::ALL {
                line = line.push(
                    container(self.orb(state, size).into_element())
                        .width(CELL)
                        .height(CELL)
                        .center_x(CELL)
                        .center_y(CELL),
                );
            }
            rows = rows.push(line);
        }
        container(rows)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| container::Style::default().background(ground))
            .into()
    }
}

fn labelled<'a>(label: &'a str, control: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    column![text(label).size(12), control.into()]
        .spacing(4)
        .width(200)
        .into()
}
