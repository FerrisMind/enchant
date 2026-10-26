//! The liquid-gooey gallery: every reference demo in one iced application.
//!
//! ```text
//! cargo run -p enchant-iced --example gooey --release
//! ```
//!
//! - **Tabs / morph**: selecting a tab changes its intrinsic size, and the
//!   jelly shape springs carry the shared surface across.
//! - **Move chips**: a clock shifts one chip every frame; the liquid surface
//!   lags, stretches and drops its trailing tongue.
//! - **Plus menu**: three satellites spring out of the main button with
//!   staggered delayed transitions.
//! - **Split pair**: one blank circle divides into two and merges back.
//! - **Email field**: the field and submit button share one gooey pill and
//!   grow apart when the field is used.
//! - **Slider**: a draggable thumb rides the track with the `move` effect.
//! - **Drag card**: dragging bows the liquid and stretches it toward the
//!   pointer.
//! - **Melting cards**: three draggable photos dissolve into each other's
//!   edges with turbulence, erosion and hole masks.
//! - **Melt pair**: two photos run molten into one another.
//! - **Avatar group**: a draggable face dissolves into the avatar stack.
//!
//! Every group is a real [`Liquid`]; the buttons, text inputs, labels and
//! images inside it are ordinary iced widgets drawn on top of the liquid
//! silhouette. Image items are decoded and warped by the crate itself.

mod _shared;

use std::time::Duration;

use enchant_iced::{
    Bend, CornerRadii, DissolveOptions, Easing, ItemBox, Liquid, LiquidItem, Morph, Move,
    Transition, IMAGE_MELT_DEFAULTS,
};
use iced::widget::{
    button, column, container, image, row, scrollable, text, text_input, Space,
};
use iced::{
    Alignment, Border, Color, Element, Length, Size, Subscription, Task, Theme, Vector,
};

const PAPER: Color = Color::from_rgb(0.949, 0.953, 0.961);
const INK: Color = Color::from_rgb(0.075, 0.086, 0.106);
const MUTED: Color = Color::from_rgb(0.42, 0.45, 0.50);
const ACCENT: Color = Color::from_rgb(0.30, 0.47, 0.96);
const SURFACE: Color = Color::from_rgb(1.0, 1.0, 1.0);

const SHADOW: &str = "0 1px 2px rgba(0, 0, 0, 0.05), 0 10px 30px rgba(0, 0, 0, 0.10)";
const SLIDER_TRAVEL: f32 = 188.0;

/// Upstream plus-menu / split-pair motion curves.
const BOUNCY_OPEN: Easing = Easing::CubicBezier {
    x1: 0.34,
    y1: 1.56,
    x2: 0.64,
    y2: 1.0,
};
const SNAPPY_CLOSE: Easing = Easing::CubicBezier {
    x1: 0.22,
    y1: 1.0,
    x2: 0.36,
    y2: 1.0,
};

pub fn main() -> iced::Result {
    iced::application(GooeyExample::new, GooeyExample::update, GooeyExample::view)
        .title("Liquid Gooey · enchant-iced")
        .window(iced::window::Settings {
            size: Size::new(880.0, 1180.0),
            ..Default::default()
        })
        .subscription(GooeyExample::subscription)
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    Tab(usize),
    Nudge,
    ToggleMenu,
    EmailChanged(String),
    Submit,
    Tick,
    Slider(Vector),
    ToggleSplit,
}

struct GooeyExample {
    theme_shell: _shared::ThemeShell,
    tab: usize,
    nudged: bool,
    menu_open: bool,
    split_open: bool,
    email_open: bool,
    email: String,
    submitted: bool,
    phase: f32,
    slider: f32,
    avatars: Vec<image::Handle>,
    photos: Vec<image::Handle>,
}

impl GooeyExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                tab: 0,
                nudged: false,
                menu_open: false,
                split_open: false,
                email_open: false,
                email: String::new(),
                submitted: false,
                phase: 0.0,
                slider: 84.0,
                avatars: (0..6).map(make_avatar).collect(),
                photos: (0..4).map(make_photo).collect(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tab(index) => self.tab = index,
            Message::Nudge => self.nudged = !self.nudged,
            Message::ToggleMenu => self.menu_open = !self.menu_open,
            Message::EmailChanged(value) => {
                self.email = value;
                self.email_open = true;
            }
            Message::Submit => self.submitted = !self.email.is_empty(),
            Message::Tick => {
                self.phase += 0.028;
                if self.phase > std::f32::consts::TAU {
                    self.phase -= std::f32::consts::TAU;
                }
            }
            Message::Slider(offset) => {
                self.slider = offset.x.clamp(0.0, SLIDER_TRAVEL);
            }
            Message::ToggleSplit => self.split_open = !self.split_open,
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let _ = &self.theme_shell;
        if let Ok(name) = std::env::var("GOOEY_PROBE") {
            return container(self.probe_demo(&name))
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }
        let content = column(vec![
            text("Liquid gooey").size(30).color(INK).into(),
            text("A full Rust port of Libraries.dev liquid-gooey.")
                .size(14)
                .color(MUTED)
                .into(),
            self.tabs_demo(),
            self.move_demo(),
            self.plus_menu_demo(),
            self.split_pair_demo(),
            self.email_demo(),
            self.slider_demo(),
            self.drag_card_demo(),
            self.cards_melt_demo(),
            self.melt_pair_demo(),
            self.avatar_group_demo(),
        ])
        .spacing(26)
        .padding(36);
        container(scrollable(content))
            .style(page_style)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn probe_demo(&self, name: &str) -> Element<'_, Message> {
        match name {
            "tabs" => self.tabs_demo(),
            "move" => self.move_demo(),
            "plus" => self.plus_menu_demo(),
            "split" => self.split_pair_demo(),
            "email" => self.email_demo(),
            "slider" => self.slider_demo(),
            "drag" => self.drag_card_demo(),
            "cards" => self.cards_melt_demo(),
            "melt" => self.melt_pair_demo(),
            "avatars" => self.avatar_group_demo(),
            _ => self.tabs_demo(),
        }
    }

    fn tabs_demo(&self) -> Element<'_, Message> {
        let labels = ["Design", "Build", "Ship"];
        let mut liquid = Liquid::row(4.0)
            .padding(4.0)
            .blur(4.0)
            .contrast(20.0)
            .fill(SURFACE)
            .shadow(SHADOW);
        for (index, label) in labels.iter().enumerate() {
            let selected = self.tab == index;
            liquid = liquid.push(
                LiquidItem::new(
                    button(text(*label).size(14))
                        .padding([9, if selected { 22 } else { 16 }])
                        .style(move |theme, status| {
                            if selected {
                                selected_button(theme, status)
                            } else {
                                transparent_button(theme, status)
                            }
                        })
                        .on_press(Message::Tab(index)),
                )
                .morph(Morph::new().shape(true).speed(1.0).bounce(0.62)),
            );
        }
        section(
            "Morph tabs",
            "The selected tab changes size; the jelly surface springs across.",
            liquid.into_element(),
        )
    }

    fn move_demo(&self) -> Element<'_, Message> {
        let shift = if self.nudged {
            self.phase.sin() * 18.0
        } else {
            0.0
        };
        let mut liquid = Liquid::row(6.0)
            .padding(5.0)
            .blur(4.5)
            .contrast(19.0)
            .fill(SURFACE)
            .shadow(SHADOW);
        for (index, label) in ["Inbox", "Drafts", "Sent"].iter().enumerate() {
            liquid = liquid.push(
                LiquidItem::new(
                    button(text(*label).size(13))
                        .padding([9, 17])
                        .style(transparent_button)
                        .on_press(Message::Nudge),
                )
                .move_effect(
                    Move::new()
                        .springiness(0.62)
                        .wobble(0.7)
                        .stretch(0.5)
                        .trail(0.7),
                )
                .offset(Vector::new(if index == 1 { shift } else { 0.0 }, 0.0)),
            );
        }
        let header = row(vec![
            text(if self.nudged { "Move · live" } else { "Move · idle" })
                .size(13)
                .color(MUTED)
                .into(),
            Space::new().width(Length::Fill).into(),
            button(text("Nudge").size(12))
                .padding([6, 12])
                .style(transparent_button)
                .on_press(Message::Nudge)
                .into(),
        ])
        .align_y(Alignment::Center);
        section(
            "Move chips",
            "The surface lags, stretches and drops a trailing tongue.",
            column(vec![header.into(), centered(liquid.into_element())])
                .spacing(10)
                .into(),
        )
    }

    fn plus_menu_demo(&self) -> Element<'_, Message> {
        let main = (80.0, 80.0);
        let open = self.menu_open;
        let transition = if open {
            Transition::Timed {
                duration: Duration::from_millis(550),
                easing: BOUNCY_OPEN,
            }
        } else {
            Transition::Timed {
                duration: Duration::from_millis(250),
                easing: SNAPPY_CLOSE,
            }
        };
        let mut liquid = Liquid::free()
            .size(Size::new(200.0, 140.0))
            .blur(6.0)
            .contrast(18.0)
            .fill(SURFACE)
            .shadow("0 2px 6px rgba(0, 0, 0, 0.08)");
        for (index, label) in ["A", "B", "C"].iter().enumerate() {
            let targets = [(26.0, 46.0), (80.0, 16.0), (134.0, 46.0)];
            let (x, y) = if open { targets[index] } else { main };
            liquid = liquid.push(
                LiquidItem::new(
                    button(text(if open { *label } else { "" }).size(15).color(INK))
                        .padding(0)
                        .width(Length::Fixed(40.0))
                        .height(Length::Fixed(40.0))
                        .style(transparent_button)
                        .on_press_maybe(open.then_some(Message::ToggleMenu)),
                )
                .position(x, y)
                .transition(transition)
                .delay(Duration::from_millis(if open {
                    40 * index as u64
                } else {
                    0
                })),
            );
        }
        liquid = liquid.push(
            LiquidItem::new(
                button(text(if open { "×" } else { "+" }).size(18).color(INK))
                    .padding(0)
                    .width(Length::Fixed(40.0))
                    .height(Length::Fixed(40.0))
                    .style(transparent_button)
                    .on_press(Message::ToggleMenu),
            )
            .position(main.0, main.1)
            .morph(Morph::new().shape(true).bounce(0.7)),
        );
        section(
            "Plus menu",
            "Three satellites spring out with staggered delayed transitions.",
            liquid.into_element(),
        )
    }

    fn split_pair_demo(&self) -> Element<'_, Message> {
        let open = self.split_open;
        let transition = if open {
            Transition::Timed {
                duration: Duration::from_millis(825),
                easing: BOUNCY_OPEN,
            }
        } else {
            Transition::Timed {
                duration: Duration::from_millis(375),
                easing: SNAPPY_CLOSE,
            }
        };
        let mut liquid = Liquid::free()
            .size(Size::new(200.0, 140.0))
            .blur(6.0)
            .contrast(18.0)
            .fill(SURFACE)
            .shadow("0 2px 6px rgba(0, 0, 0, 0.08)");
        for direction in [-1.0_f32, 1.0] {
            liquid = liquid.push(
                LiquidItem::new(
                    button(text("").size(15))
                        .padding(0)
                        .width(Length::Fixed(40.0))
                        .height(Length::Fixed(40.0))
                        .style(transparent_button)
                        .on_press(Message::ToggleSplit),
                )
                .position(80.0, 80.0)
                .offset(Vector::new(if open { direction * 34.0 } else { 0.0 }, 0.0))
                .transition(transition),
            );
        }
        section(
            "Split pair",
            "One circle divides into two; the liquid necks, stretches and lets go.",
            liquid.into_element(),
        )
    }

    fn email_demo(&self) -> Element<'_, Message> {
        let open = self.email_open;
        let transition = Transition::Timed {
            duration: Duration::from_millis(600),
            easing: Easing::CubicBezier {
                x1: 0.22,
                y1: 1.3,
                x2: 0.71,
                y2: 1.0,
            },
        };
        let field = container(
            text_input("you@example.com", &self.email)
                .size(14)
                .padding(iced::Padding { top: 0.0, right: 44.0, bottom: 0.0, left: 18.0 })
                .style(typed_input)
                .on_input(Message::EmailChanged)
                .on_submit(Message::Submit),
        )
        .width(Length::Fixed(202.0))
        .height(Length::Fixed(48.0))
        .align_y(Alignment::Center);
        let field = LiquidItem::new(field)
            .position(44.0, 81.0)
            .offset(Vector::new(if open { -32.0 } else { 0.0 }, 0.0))
            .transition(transition)
            .radius(CornerRadii::uniform(24.0));
        let submit = LiquidItem::new(
            button(
                text(if !open {
                    ""
                } else if self.submitted {
                    "✓"
                } else {
                    "→"
                })
                .size(17)
                .color(INK),
            )
            .padding(0)
            .width(Length::Fixed(44.0))
            .height(Length::Fixed(44.0))
            .style(transparent_button)
            .on_press_maybe(open.then_some(Message::Submit)),
        )
        .position(200.0, 83.0)
        .offset(Vector::new(if open { 34.0 } else { 0.0 }, 0.0))
        .transition(transition)
        .radius(CornerRadii::uniform(22.0));
        let liquid = Liquid::free()
            .size(Size::new(290.0, 210.0))
            .blur(8.0)
            .contrast(22.0)
            .fill(SURFACE)
            .shadow(SHADOW)
            .extend([field, submit]);
        section(
            "Email input",
            "Using the field widens it; the submit button stays attached.",
            liquid.into_element(),
        )
    }

    fn slider_demo(&self) -> Element<'_, Message> {
        let thumb = LiquidItem::new(
            Space::new()
                .width(Length::Fixed(24.0))
                .height(Length::Fixed(24.0)),
        )
        .position(14.0, 30.0)
        .offset(Vector::new(self.slider, 0.0))
        .radius(12.0)
        .draggable(true)
        .drag_bounds(ItemBox::new(0.0, 0.0, SLIDER_TRAVEL, 0.0))
        .on_drag(|_, offset| Message::Slider(offset))
        .move_effect(
            Move::new()
                .springiness(0.5)
                .stretch(0.6)
                .trail(0.35),
        );
        let track = LiquidItem::content(
            container(Space::new().width(Length::Fixed(212.0)).height(Length::Fixed(8.0))).style(
                |_theme| container::Style {
                    background: Some(Color::from_rgba8(234, 234, 234, 179.0 / 255.0).into()),
                    border: Border {
                        color: Color::from_rgba8(0, 0, 0, 14.0 / 255.0),
                        width: 1.0,
                        radius: 4.0.into(),
                    },
                    ..container::Style::default()
                },
            ),
        )
        .position(14.0, 38.0)
        .behind(true);
        let liquid = Liquid::free()
            .size(Size::new(240.0, 80.0))
            .blur(6.0)
            .contrast(18.0)
            .fill(SURFACE)
            .shadow("0 0 0 1px rgba(0, 0, 0, 0.08), 0 1px 5px rgba(0, 0, 0, 0.08)")
            .extend([track, thumb]);
        let percent = (self.slider / SLIDER_TRAVEL * 100.0).round();
        let header = row(vec![
            text("Slider").size(13).color(MUTED).into(),
            Space::new().width(Length::Fill).into(),
            text(format!("{percent:.0}%")).size(13).color(INK).into(),
        ])
        .align_y(Alignment::Center);
        section(
            "Move thumb",
            "Drag the thumb: the liquid trails, stretches and settles.",
            column(vec![
                header.into(),
                centered(liquid.into_element()),
            ])
            .spacing(10)
            .into(),
        )
    }

    fn drag_card_demo(&self) -> Element<'_, Message> {
        let card = container(
            row(vec![
                container(text("G").size(11).color(MUTED))
                    .width(Length::Fixed(32.0))
                    .height(Length::Fixed(32.0))
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .style(chip_style)
                    .into(),
                text("Gooey Project").size(13).color(INK).into(),
                text("45 files").size(13).color(MUTED).into(),
            ])
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding(iced::Padding { top: 0.0, right: 16.0, bottom: 0.0, left: 8.0 })
        .width(Length::Fixed(261.0))
        .height(Length::Fixed(46.0))
        .align_y(Alignment::Center);
        let liquid = Liquid::free()
            .size(Size::new(380.0, 210.0))
            .blur(6.0)
            .contrast(18.0)
            .fill(SURFACE)
            .shadow("0 0 0 1px rgba(0, 0, 0, 0.08), 0 1px 3px rgba(0, 0, 0, 0.04)")
            .push(
                LiquidItem::new(card)
                    .position(59.5, 82.0)
                    .radius(23.0)
                    .draggable(true)
                    .drag_bounds(ItemBox::new(-59.5, -82.0, 119.0, 164.0))
                    .move_effect(
                        Move::new()
                            .springiness(1.0)
                            .stretch(0.0)
                            .trail(0.0),
                    )
                    .bend(Bend::new().vertical(0.6).horizontal(0.35)),
            );
        section(
            "Drag card",
            "The dragged card bows and stretches its liquid toward the pointer.",
            liquid.into_element(),
        )
    }

    fn cards_melt_demo(&self) -> Element<'_, Message> {
        let dissolve = DissolveOptions {
            warp: 26.0,
            blur: 5.0,
            mix: 0.7,
            gravity: 60.0,
            taper: 0.95,
            warp_freq: 1.0,
            flow_speed: 26.0,
            detail: 2,
            zone: Some(26.0),
            range: Some(44.0),
            release_ms: 110.0,
            fade_ms: Some(320.0),
            ..DissolveOptions::default()
        };
        let mut liquid = Liquid::free()
            .size(Size::new(320.0, 210.0))
            .blur(6.0)
            .contrast(18.0)
            .fill(SURFACE)
            .shadow(SHADOW);
        for (index, handle) in self.photos.iter().take(2).enumerate() {
            let rest = if index == 0 { -78.0 } else { 78.0 };
            liquid = liquid.push(
                LiquidItem::image(handle.clone())
                    .size(96.0, 96.0)
                    .position(112.0, 57.0)
                    .offset(Vector::new(rest, 0.0))
                    .radius(16.0)
                    .morph(Morph::new().blob_inset(2.0).bridge_grow(10.0))
                    .dissolve(dissolve)
                    .draggable(true)
                    .drag_bounds(ItemBox::new(-106.0, -51.0, 212.0, 102.0)),
            );
        }
        section(
            "Melting cards",
            "Draggable photos dissolve into each other with turbulence, erosion and hole masks.",
            liquid.into_element(),
        )
    }

    fn melt_pair_demo(&self) -> Element<'_, Message> {
        let mut liquid = Liquid::free()
            .size(Size::new(380.0, 200.0))
            .blur(6.0)
            .contrast(18.0)
            .fill(SURFACE)
            .shadow(SHADOW);
        for (index, handle) in self.photos.iter().skip(1).take(2).enumerate() {
            let (x, bounds) = if index == 0 {
                (96.0, ItemBox::new(-96.0, -58.0, 296.0, 116.0))
            } else {
                (200.0, ItemBox::new(-200.0, -58.0, 296.0, 116.0))
            };
            liquid = liquid.push(
                LiquidItem::image(handle.clone())
                    .size(84.0, 84.0)
                    .position(x, 58.0)
                    .radius(16.0)
                    .melt(IMAGE_MELT_DEFAULTS)
                    .draggable(true)
                    .drag_bounds(bounds),
            );
        }
        section(
            "Melt pair",
            "Two photos run molten into one another, then pull back apart.",
            liquid.into_element(),
        )
    }

    fn avatar_group_demo(&self) -> Element<'_, Message> {
        let label = LiquidItem::new(
            container(text("Share").size(14).color(INK))
                .padding([10, 16])
                .height(Length::Fixed(44.0))
                .align_y(Alignment::Center),
        )
        .radius(22.0);
        let mut liquid = Liquid::row(2.0)
            .padding(5.0)
            .blur(3.5)
            .contrast(22.0)
            .fill(SURFACE)
            .shadow(SHADOW)
            .push(label);
        for handle in self.avatars.iter().take(3) {
            liquid = liquid.push(
                LiquidItem::image(handle.clone())
                    .size(44.0, 44.0)
                    .radius(22.0),
            );
        }
        liquid = liquid.push(
            LiquidItem::image(self.avatars[4].clone())
                .size(52.0, 52.0)
                .radius(26.0)
                .morph(Morph::new().blob_inset(2.0).bridge_grow(8.0))
                .dissolve(DissolveOptions::new().range(88.0))
                .draggable(true)
                .drag_bounds(ItemBox::new(-110.0, -55.0, 220.0, 110.0)),
        );
        section(
            "Avatar group",
            "Drag the extra face into the stack; its edges liquefy as it joins.",
            liquid.into_element(),
        )
    }
}

fn centered<'a>(body: Element<'a, Message>) -> Element<'a, Message> {
    container(body)
        .width(Length::Fill)
        .align_x(Alignment::Center)
        .into()
}

fn section<'a>(
    title: &'static str,
    subtitle: &'static str,
    body: Element<'a, Message>,
) -> Element<'a, Message> {
    container(
        column(vec![
            text(title).size(17).color(INK).into(),
            text(subtitle).size(12).color(MUTED).into(),
            Space::new().height(Length::Fixed(2.0)).into(),
            container(body)
                .width(Length::Fill)
                .align_x(Alignment::Center)
                .into(),
        ])
        .spacing(5)
        .padding(20),
    )
    .style(section_style)
    .width(Length::Fill)
    .into()
}

fn page_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(PAPER.into()),
        ..container::Style::default()
    }
}

fn section_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.55).into()),
        border: Border {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.06),
            width: 1.0,
            radius: 20.0.into(),
        },
        ..container::Style::default()
    }
}

fn transparent_button(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: INK,
        border: Border::default(),
        shadow: iced::Shadow::default(),
        snap: false,
    }
}

fn selected_button(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: ACCENT,
        border: Border::default(),
        shadow: iced::Shadow::default(),
        snap: false,
    }
}

fn typed_input(_theme: &Theme, _status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: Color::TRANSPARENT.into(),
        border: Border::default(),
        icon: MUTED,
        placeholder: MUTED,
        value: INK,
        selection: ACCENT.scale_alpha(0.3),
    }
}

fn chip_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Color::from_rgba8(235, 235, 236, 77.0 / 255.0).into()),
        border: Border {
            color: Color::from_rgba8(0, 0, 0, 13.0 / 255.0),
            width: 1.0,
            radius: 40.0.into(),
        },
        ..container::Style::default()
    }
}

fn make_avatar(seed: usize) -> image::Handle {
    const SIZE: u32 = 96;
    const PALETTE: [(f32, f32, f32); 6] = [
        (0.96, 0.45, 0.42),
        (0.98, 0.65, 0.30),
        (0.36, 0.72, 0.52),
        (0.30, 0.55, 0.95),
        (0.55, 0.42, 0.92),
        (0.94, 0.40, 0.66),
    ];
    let (red, green, blue) = PALETTE[seed % PALETTE.len()];
    let mut pixels = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let nx = (x as f32 + 0.5) / SIZE as f32 * 2.0 - 1.0;
            let ny = (y as f32 + 0.5) / SIZE as f32 * 2.0 - 1.0;
            let radius = (nx * nx + ny * ny).sqrt();
            let alpha = ((1.04 - radius) * SIZE as f32 * 0.5).clamp(0.0, 1.0);
            let light = 1.06 - 0.30 * (nx * 0.5 + ny * 0.75);
            let glow = (1.0 - radius).max(0.0).powi(2) * 0.18;
            pixels.extend_from_slice(&[
                ((red * light + glow).clamp(0.0, 1.0) * 255.0) as u8,
                ((green * light + glow).clamp(0.0, 1.0) * 255.0) as u8,
                ((blue * light + glow).clamp(0.0, 1.0) * 255.0) as u8,
                (alpha * 255.0) as u8,
            ]);
        }
    }
    image::Handle::from_rgba(SIZE, SIZE, pixels)
}

fn make_photo(seed: usize) -> image::Handle {
    const SIZE: u32 = 160;
    let palettes = [
        [(0.10, 0.16, 0.28), (0.30, 0.63, 0.90), (0.94, 0.68, 0.40)],
        [(0.26, 0.10, 0.24), (0.86, 0.33, 0.47), (0.98, 0.73, 0.55)],
        [(0.06, 0.20, 0.18), (0.30, 0.72, 0.61), (0.92, 0.94, 0.72)],
        [(0.18, 0.16, 0.32), (0.53, 0.47, 0.91), (0.96, 0.83, 0.72)],
    ];
    let palette = palettes[seed % palettes.len()];
    let mut pixels = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let u = (x as f32 + 0.5) / SIZE as f32;
            let v = (y as f32 + 0.5) / SIZE as f32;
            let wave = ((u * 7.0 + seed as f32).sin() * 0.5 + 0.5) * 0.35
                + ((v * 9.0 - u * 4.0).cos() * 0.5 + 0.5) * 0.25;
            let blend = (u * 0.55 + v * 0.45 + wave * 0.18).clamp(0.0, 1.0);
            let t = if blend < 0.55 {
                blend / 0.55
            } else {
                (blend - 0.55) / 0.45
            };
            let (a, b) = if blend < 0.55 {
                (palette[0], palette[1])
            } else {
                (palette[1], palette[2])
            };
            let halo = (1.0 - ((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt())
                .max(0.0)
                .powi(3);
            pixels.extend_from_slice(&[
                ((a.0 + (b.0 - a.0) * t + halo * 0.05).clamp(0.0, 1.0) * 255.0) as u8,
                ((a.1 + (b.1 - a.1) * t + halo * 0.05).clamp(0.0, 1.0) * 255.0) as u8,
                ((a.2 + (b.2 - a.2) * t + halo * 0.05).clamp(0.0, 1.0) * 255.0) as u8,
                255,
            ]);
        }
    }
    image::Handle::from_rgba(SIZE, SIZE, pixels)
}
