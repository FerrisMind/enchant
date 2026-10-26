//! Border beam around real shadcn cards.
//!
//! ```text
//! cargo run -p enchant-iced --example border_beam --release
//! ```
//!
//! Both backends wrap the same kind of surface the effect is meant for: a real
//! `iced_shadcn::card` with its own background, 1px border, 12px radius and
//! live content.
//!
//! - Left: [`BorderBeam`] (image backend). The widget measures the pricing
//!   card's intrinsic size, owns the animation clock and draws its fade layers
//!   around those exact bounds. Nothing is given a fake size.
//! - Right: [`BorderBeamCanvas`] (canvas backend). The login card is pinned to
//!   a known 360x400 box so the caller can supply the matching host size.
//!
//! Both cards are visible at once, but only the selected backend animates.
//! Rendering two independent software-rasterized effects at once is what made
//! the old demo stutter; the toggle keeps a real card live without paying for
//! both. The image widget owns its clock (no app tick); the canvas gets a
//! 16ms clock, or 33ms for the pulse presets, matching the reference's own
//! pulse driver cadence.
//!
//! The preset row exercises every size on the selected card: `sm`, `md`,
//! `line`, `pulse-inner` and `pulse-outside`. The theme toggle comes from the
//! shared `_shared::themed_shell`, and the beam theme follows the shadcn
//! palette rather than the ambient iced theme.

mod _shared;

use std::time::Duration;

use enchant_iced::{
    BorderBeam, BorderBeamCanvas, BorderBeamOptions, BorderBeamPalette, BorderBeamSize,
    BorderBeamTheme, ResolvedBeamTheme,
};
use iced::time::Instant;
use iced::widget::{column, container, row, stack, text};
use iced::{Alignment, Color, Element, Length, Size, Subscription, Task};
use iced_shadcn::{ButtonProps, ButtonSize, ButtonVariant, button_content};

/// The canvas backend cannot inspect its child, so the login card is laid out
/// at this fixed size and the beam host matches it exactly.
const CANVAS_WIDTH: f32 = 360.0;
const CANVAS_HEIGHT: f32 = 400.0;
/// `shadcn_forms` builds both cards with `CardProps::radius(12.0)`.
const CARD_RADIUS: f32 = 12.0;

pub fn main() -> iced::Result {
    iced::application(
        BorderBeamExample::new,
        BorderBeamExample::update,
        BorderBeamExample::view,
    )
    .title("Border Beam · shadcn cards")
    .window(iced::window::Settings {
        size: Size::new(1080.0, 880.0),
        ..Default::default()
    })
    .subscription(BorderBeamExample::subscription)
    .run()
}

struct BorderBeamExample {
    theme_shell: _shared::ThemeShell,
    start: Instant,
    elapsed: Duration,
    email: String,
    password: String,
    size: BorderBeamSize,
    palette: BorderBeamPalette,
    static_colors: bool,
    active: bool,
    backend: Backend,
    density: f32,
    lit: bool,
}

/// Which backend owns the animated beam. Only one runs at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    Image,
    Canvas,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Tick(Instant),
    EmailChanged(String),
    PasswordChanged(String),
    Register,
    Login,
    GetStarted,
    SetSize(BorderBeamSize),
    SetBackend(Backend),
    ScaleFactor(f32),
    NextPalette,
    ToggleStatic,
    ToggleActive,
    Lit,
    Dark,
}

impl BorderBeamExample {
    fn new() -> (Self, Task<Message>) {
        let start = Instant::now();
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                start,
                elapsed: Duration::ZERO,
                email: String::new(),
                password: String::new(),
                size: BorderBeamSize::Md,
                palette: BorderBeamPalette::Colorful,
                static_colors: false,
                active: true,
                backend: Backend::Image,
                density: 1.0,
                lit: false,
            },
            iced::window::latest()
                .and_then(iced::window::scale_factor)
                .map(Message::ScaleFactor),
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
                self.elapsed = now.duration_since(self.start);
            }
            Message::EmailChanged(value) => self.email = value,
            Message::PasswordChanged(value) => self.password = value,
            Message::Register | Message::Login | Message::GetStarted => {}
            Message::ScaleFactor(scale) => {
                // The effect is a CPU rasterizer, so density is a direct cost
                // multiplier: at 1.5x a 360x400 card already takes ~13ms per
                // rotating frame in release and pulse-outside takes ~30ms,
                // against a 33ms pulse cadence. 1.25 keeps a 60fps rotating
                // beam and a 30fps pulse on HiDPI displays while still
                // matching the compositor closely enough to stay crisp.
                self.density = scale.clamp(1.0, 1.25);
            }
            Message::SetSize(size) => self.size = size,
            Message::SetBackend(backend) => {
                if self.backend != backend {
                    self.backend = backend;
                    // The canvas is stateless and starts from its own elapsed
                    // time, so give it a fresh clock instead of jumping to the
                    // app's uptime.
                    self.start = Instant::now();
                    self.elapsed = Duration::ZERO;
                    self.lit = false;
                }
            }
            Message::NextPalette => {
                let current = BorderBeamPalette::ALL
                    .iter()
                    .position(|palette| *palette == self.palette)
                    .unwrap_or(0);
                self.palette = BorderBeamPalette::ALL[(current + 1) % BorderBeamPalette::ALL.len()];
            }
            Message::ToggleStatic => self.static_colors = !self.static_colors,
            Message::ToggleActive => self.active = !self.active,
            Message::Lit => self.lit = true,
            Message::Dark => self.lit = false,
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        let rescale = iced::window::events().filter_map(|(_, event)| match event {
            iced::window::Event::Rescaled(scale) => Some(Message::ScaleFactor(scale)),
            _ => None,
        });
        // The image widget advances its clock on RedrawRequested and asks for
        // the next frame itself, so it needs no application tick. The canvas
        // is stateless, so it gets a clock only while it is the live backend.
        // Theme crossfades need display-rate ticks regardless of backend.
        let transition = self
            .theme_shell
            .controller
            .is_transition_active(self.theme_shell.now);
        let ticks = if transition {
            Some(iced::time::every(Duration::from_millis(16)).map(Message::Tick))
        } else if self.active && self.backend == Backend::Canvas {
            let period = match self.size {
                BorderBeamSize::PulseInner | BorderBeamSize::PulseOutside => 33,
                _ => 16,
            };
            Some(iced::time::every(Duration::from_millis(period)).map(Message::Tick))
        } else {
            None
        };
        match ticks {
            Some(ticks) => Subscription::batch([rescale, ticks]),
            None => rescale,
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let now = self.theme_shell.now;
        let transition = self.theme_shell.controller.is_transition_active(now);
        let theme = self.theme_shell.blended_theme();
        let muted = theme.palette.muted_foreground;
        let dark = self.theme_shell.controller.dark_mode();
        let beam_theme = if dark {
            BorderBeamTheme::Dark
        } else {
            BorderBeamTheme::Light
        };
        let system = if dark {
            ResolvedBeamTheme::Dark
        } else {
            ResolvedBeamTheme::Light
        };

        // ── image backend: real pricing card, intrinsic bounds ─────────────
        let pricing = _shared::shadcn_forms::pricing_card_pro(Some(Message::GetStarted), theme);
        let widget_card = BorderBeam::new(pricing)
            .size(self.size)
            .palette(self.palette)
            .theme(beam_theme)
            .static_colors(self.static_colors)
            .radius(CARD_RADIUS)
            .active(self.active && self.backend == Backend::Image && !transition)
            .on_activate(|| Message::Lit)
            .on_deactivate(|| Message::Dark)
            .pixel_density(self.density);

        // ── canvas backend: real login card, pinned bounds ─────────────────
        let login = _shared::shadcn_forms::login_card_dual_footer(
            &self.email,
            &self.password,
            Some(Message::EmailChanged),
            Some(Message::PasswordChanged),
            Some(Message::Register),
            Some(Message::Login),
            theme,
        );
        let login = container(login)
            .width(Length::Fixed(CANVAS_WIDTH))
            .height(Length::Fixed(CANVAS_HEIGHT));
        let canvas_options = BorderBeamOptions::new()
            .size(self.size)
            .palette(self.palette)
            .theme(BorderBeamTheme::Auto)
            .static_colors(self.static_colors)
            .radius(CARD_RADIUS)
            .active(self.active && self.backend == Backend::Canvas && !transition);
        let canvas_beam = BorderBeamCanvas::new(canvas_options)
            .host(CANVAS_WIDTH, CANVAS_HEIGHT)
            .elapsed(self.elapsed)
            .system(system)
            .pixel_density(self.density)
            .into_element();
        // pulse-outside paints its core and halo *behind* an opaque child; the
        // only way to reproduce that with one canvas is to layer it under the
        // card. Every other preset draws over the card.
        let mut layers: Vec<Element<'_, Message>> = Vec::with_capacity(2);
        if self.size == BorderBeamSize::PulseOutside {
            layers.push(canvas_beam);
            layers.push(login.into());
        } else {
            layers.push(login.into());
            layers.push(canvas_beam);
        }
        let canvas_card: Element<'_, Message> = stack(layers).into();

        let widget_stage = column![
            note("BorderBeam · image backend · intrinsic bounds", muted),
            widget_card,
        ]
        .spacing(10)
        .align_x(Alignment::Center);
        let canvas_stage = column![
            note("BorderBeamCanvas · caller clock · fixed 360x400", muted),
            canvas_card,
        ]
        .spacing(10)
        .align_x(Alignment::Center);

        let mut preset_buttons: Vec<Element<'_, Message>> = Vec::with_capacity(5);
        for size in BorderBeamSize::ALL {
            preset_buttons.push(control(
                size.to_string(),
                Message::SetSize(size),
                size == self.size,
                theme,
            ));
        }
        let controls = row![
            control(
                "image backend",
                Message::SetBackend(Backend::Image),
                self.backend == Backend::Image,
                theme,
            ),
            control(
                "canvas backend",
                Message::SetBackend(Backend::Canvas),
                self.backend == Backend::Canvas,
                theme,
            ),
            row(preset_buttons).spacing(8).align_y(Alignment::Center),
            control(
                format!("palette · {}", self.palette),
                Message::NextPalette,
                false,
                theme,
            ),
            control(
                format!(
                    "static colors · {}",
                    if self.static_colors { "on" } else { "off" }
                ),
                Message::ToggleStatic,
                self.static_colors,
                theme,
            ),
            control(
                format!("active · {}", if self.active { "on" } else { "off" }),
                Message::ToggleActive,
                self.active,
                theme,
            ),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        let status = if !self.active {
            "beams off (fade-out complete)".to_owned()
        } else if transition {
            "paused while the theme crossfades".to_owned()
        } else if self.lit {
            format!(
                "fade-in finished callback fired · {} · {} backend",
                self.size,
                match self.backend {
                    Backend::Image => "image",
                    Backend::Canvas => "canvas",
                }
            )
        } else {
            format!(
                "{} · {} · {} beam · {} backend",
                self.size,
                self.palette,
                if dark { "dark" } else { "light" },
                match self.backend {
                    Backend::Image => "image",
                    Backend::Canvas => "canvas",
                }
            )
        };
        let status = note(status, muted);
        let hint = note(
            "Both cards are real shadcn components. Only the selected backend animates: the image \
             widget measures its card and owns its clock, the canvas matches the login card's \
             360x400 host. pulse-outside layers under the opaque card so only the halo spills.",
            muted,
        );

        let body = column![
            row![widget_stage, canvas_stage]
                .spacing(56)
                .align_y(Alignment::Start),
            controls,
            status,
            hint,
        ]
        .spacing(22)
        .align_x(Alignment::Center);

        _shared::themed_shell("Border Beam", body, &self.theme_shell, Message::ToggleTheme)
    }
}

fn note<'a>(label: impl Into<String>, color: Color) -> Element<'a, Message> {
    let label: String = label.into();
    text(label)
        .size(13)
        .style(move |_| iced::widget::text::Style { color: Some(color) })
        .into()
}

fn control<'a>(
    label: impl Into<String>,
    message: Message,
    selected: bool,
    theme: &'a iced_shadcn::Theme,
) -> Element<'a, Message> {
    let label: String = label.into();
    button_content(
        text(label).size(13),
        Some(message),
        ButtonProps::new()
            .variant(if selected {
                ButtonVariant::Default
            } else {
                ButtonVariant::Outline
            })
            .size(ButtonSize::Size2),
        theme,
    )
    .into()
}
