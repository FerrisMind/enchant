//! Metal FX gallery — Libraries.dev `metal-fx` sections (buttons, circle, text,
//! badge, neighbour reflection).
//!
//! ```text
//! cargo run -p enchant-iced --example metal_fx
//! ```
//!
//! Snapshot (frozen clock, dark page):
//!
//! ```text
//! METAL_FX_SNAPSHOT=out.png METAL_FX_CLOCK=1.25 \
//!     cargo run -p enchant-iced --example metal_fx
//! ```

use enchant_iced::widgets::metal_fx::{
    METAL_TEXT_PAD_CSS, MetalBadge, MetalFx, MetalFxPreset, MetalFxTheme, MetalFxVariant,
    MetalReflection, MetalReflectionStyle, MetalText, MetalTextFont, glyph_mask_image,
    rasterize_text_mask, set_cursor_light_config,
};
use iced::widget::{button, checkbox, column, container, row, slider, space, text};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Size, Subscription, Task, Theme, window,
};

pub fn main() -> iced::Result {
    // Demo: catch-light follows the pointer (off by default upstream).
    set_cursor_light_config(|c| {
        c.catch_light = true;
        c.spill = true;
    });

    let snapshot = Snapshot::from_env();
    let size = if snapshot.is_some() {
        Size::new(900.0, 720.0)
    } else {
        Size::new(980.0, 780.0)
    };
    iced::application(Gallery::new, Gallery::update, Gallery::view)
        .title("Metal FX")
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
    #[allow(dead_code)]
    clock: f64,
}

impl Snapshot {
    fn from_env() -> Option<Self> {
        let path = std::env::var("METAL_FX_SNAPSHOT").ok()?;
        let clock = std::env::var("METAL_FX_CLOCK")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1.25);
        Some(Self { path, clock })
    }
}

struct Gallery {
    snapshot: Option<Snapshot>,
    dark: bool,
    strength: f32,
    paused: bool,
    frames: u32,
}

#[derive(Debug, Clone)]
enum Message {
    Frame,
    Dark(bool),
    Strength(f32),
    Paused(bool),
    Captured(window::Screenshot),
    Nop,
}

impl Gallery {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                snapshot: Snapshot::from_env(),
                dark: true,
                strength: 1.0,
                paused: false,
                frames: 0,
            },
            Task::none(),
        )
    }

    fn theme(&self) -> Theme {
        if self.dark {
            Theme::Dark
        } else {
            Theme::Light
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Frame => {
                if self.snapshot.is_some() {
                    self.frames = self.frames.saturating_add(1);
                    if self.frames == 18 {
                        return window::latest()
                            .and_then(window::screenshot)
                            .map(Message::Captured);
                    }
                }
            }
            Message::Dark(v) => self.dark = v,
            Message::Strength(v) => self.strength = v,
            Message::Paused(v) => self.paused = v,
            Message::Captured(shot) => {
                if let Some(snap) = &self.snapshot {
                    let _ = image::save_buffer(
                        &snap.path,
                        &shot.rgba,
                        shot.size.width,
                        shot.size.height,
                        image::ColorType::Rgba8,
                    );
                    return iced::exit();
                }
            }
            Message::Nop => {}
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        window::frames().map(|_| Message::Frame)
    }

    fn view(&self) -> Element<'_, Message> {
        let theme = if self.dark {
            MetalFxTheme::Dark
        } else {
            MetalFxTheme::Light
        };
        let s = self.strength;
        let paused = self.paused;

        let pill = |label: &'static str| {
            button(text(label).size(14))
                .padding([10, 22])
                .style(|_t, _s| button::Style {
                    background: Some(Background::Color(Color::from_rgba(0.12, 0.12, 0.12, 0.85))),
                    text_color: Color::from_rgb(0.95, 0.95, 0.95),
                    border: Border {
                        radius: 20.0.into(),
                        ..Border::default()
                    },
                    ..button::Style::default()
                })
                .on_press(Message::Nop)
        };

        let chromatic = MetalFx::new(pill("Chromatic"))
            .preset(MetalFxPreset::Chromatic)
            .theme(theme)
            .strength(s)
            .paused(paused)
            .inner_shadow(true)
            .anchor_id("chromatic");

        let silver = MetalFx::new(pill("Silver"))
            .preset(MetalFxPreset::Silver)
            .theme(theme)
            .strength(s)
            .paused(paused)
            .inner_shadow(true);

        let gold = MetalFx::new(pill("Gold"))
            .preset(MetalFxPreset::Gold)
            .theme(theme)
            .strength(s)
            .paused(paused)
            .inner_shadow(true);

        let circle = MetalFx::new(
            container(text("↑").size(18))
                .center_x(Length::Fixed(40.0))
                .center_y(Length::Fixed(40.0)),
        )
        .variant(MetalFxVariant::Circle)
        .preset(MetalFxPreset::Chromatic)
        .theme(theme)
        .strength(s * 0.9)
        .paused(paused)
        .inner_shadow(true)
        .anchor_id("send");

        let text_font = MetalTextFont {
            size: 24.0,
            weight: 500.0,
        };
        let pro = MetalText::new("Pro")
            .font(text_font.clone())
            .theme(theme)
            .strength(s)
            .paused(paused)
            .anchor_id("pro");

        // Neighbour catch on "Plan" — same swash mask for label + reflection
        // (iced `text` would sit lower than the mask ink).
        let plan_mask = rasterize_text_mask("Plan", &text_font, 2.0, METAL_TEXT_PAD_CSS);
        let (plan_pw, plan_ph) = plan_mask.pixel_size();
        let plan_w = plan_pw as f32 / 2.0;
        let plan_h = plan_ph as f32 / 2.0;
        let plan_label = glyph_mask_image(
            &plan_mask,
            Color::from_rgba(0.9, 0.9, 0.9, 0.55),
        );
        let plan = MetalReflection::new(
            "pro",
            container(plan_label)
                .width(Length::Fixed(plan_w))
                .height(Length::Fixed(plan_h)),
        )
        .strength(0.64)
        .style(MetalReflectionStyle::Glyphs)
        .glyph_mask(plan_mask);

        let badge = MetalBadge::new("New").theme(theme).strength(s).paused(paused);

        let chip = MetalReflection::new(
            "send",
            container(text("Auto").size(12))
                .padding([8, 14])
                .style(|_t| container::Style {
                    background: Some(Background::Color(Color::from_rgb(0.14, 0.14, 0.14))),
                    text_color: Some(Color::from_rgb(0.85, 0.85, 0.85)),
                    border: Border {
                        radius: 999.0.into(),
                        ..Border::default()
                    },
                    ..container::Style::default()
                }),
        )
        .strength(1.0);

        let card_bg = if self.dark {
            Color::from_rgb(0.055, 0.055, 0.055)
        } else {
            Color::from_rgb(0.96, 0.96, 0.96)
        };
        let card_style = move |_t: &Theme| container::Style {
            background: Some(Background::Color(card_bg)),
            border: Border {
                radius: 28.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        };

        let controls = row![
            checkbox(self.dark).label("Dark").on_toggle(Message::Dark),
            checkbox(self.paused)
                .label("Paused")
                .on_toggle(Message::Paused),
            text(format!("Strength {:.2}", self.strength)).size(13),
            slider(0.0..=1.0, self.strength, Message::Strength).step(0.01_f32),
        ]
        .spacing(16)
        .align_y(Alignment::Center);

        let presets = container(
            column![
                text("Presets").size(14),
                row![chromatic, silver, gold, circle]
                    .spacing(16)
                    .align_y(Alignment::Center),
            ]
            .spacing(16),
        )
        .padding(28)
        .width(Length::Fill)
        .style(card_style);

        let glyphs = container(
            row![plan, pro, space().width(24), badge, space().width(16), chip]
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .padding(28)
        .width(Length::Fill)
        .style(card_style);

        let body = column![
            text("Metal FX").size(28),
            text("liquid metal ring · glow · bend · cursor light · reflections").size(13),
            space().height(8),
            controls,
            space().height(12),
            presets,
            glyphs,
        ]
        .spacing(14)
        .padding(24)
        .width(Length::Fill);

        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_t| container::Style {
                background: Some(Background::Color(if self.dark {
                    Color::from_rgb(0.04, 0.04, 0.04)
                } else {
                    Color::WHITE
                })),
                ..container::Style::default()
            })
            .into()
    }
}
