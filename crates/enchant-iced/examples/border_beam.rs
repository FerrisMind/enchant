mod _shared;

use enchant_iced::{
    BorderBeam, BorderBeamCanvas, BorderBeamOptions, BorderBeamPalette, BorderBeamSize,
    BorderBeamTheme, ResolvedBeamTheme,
};
use iced::time::Instant;
use iced::widget::{button, column, container, row, stack, text};
use iced::{Alignment, Element, Length, Size, Subscription, Task};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        BorderBeamExample::new,
        BorderBeamExample::update,
        BorderBeamExample::view,
    )
    .title("Border Beam")
    .window(iced::window::Settings {
        size: Size::new(980.0, 720.0),
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
    lit: bool,
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
    NextSize,
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
                size: BorderBeamSize::PulseOutside,
                palette: BorderBeamPalette::Colorful,
                static_colors: false,
                active: true,
                lit: false,
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
                self.elapsed = now.duration_since(self.start);
            }
            Message::EmailChanged(value) => self.email = value,
            Message::PasswordChanged(value) => self.password = value,
            Message::Register | Message::Login => {}
            Message::NextSize => {
                let current = BorderBeamSize::ALL
                    .iter()
                    .position(|size| *size == self.size)
                    .unwrap_or(0);
                self.size = BorderBeamSize::ALL[(current + 1) % BorderBeamSize::ALL.len()];
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
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let now = self.theme_shell.now;
        let transition = self.theme_shell.controller.is_transition_active(now);
        let background = self.theme_shell.blended_theme().palette.background;
        let luminance = 0.299 * background.r + 0.587 * background.g + 0.114 * background.b;
        let system = if luminance < 0.5 {
            ResolvedBeamTheme::Dark
        } else {
            ResolvedBeamTheme::Light
        };
        let login = _shared::shadcn_forms::login_card_dual_footer(
            &self.email,
            &self.password,
            Some(Message::EmailChanged),
            Some(Message::PasswordChanged),
            Some(Message::Register),
            Some(Message::Login),
            self.theme_shell.blended_theme(),
        );

        // Widget backend: owns the clock and the 0.6s/0.5s fade, wraps the child.
        let widget_card = BorderBeam::new(
            container(login)
                .width(Length::Fixed(350.0))
                .height(Length::Fixed(380.0)),
        )
        .size(self.size)
        .palette(self.palette)
        .theme(BorderBeamTheme::Auto)
        .static_colors(self.static_colors)
        .radius(16.0)
        .active(self.active && !transition)
        .on_activate(|| Message::Lit)
        .on_deactivate(|| Message::Dark)
        .pixel_density(2.0);

        // Canvas backend: caller-driven clock, same frame model and rasterizer.
        let canvas_options = BorderBeamOptions::new()
            .size(self.size)
            .palette(self.palette)
            .static_colors(self.static_colors)
            .radius(16.0)
            .active(self.active);
        let canvas_card = stack(vec![
            container(
                text("Canvas backend")
                    .size(18)
                    .color(self.theme_shell.blended_foreground(self.theme_shell.now)),
            )
            .width(Length::Fixed(280.0))
            .height(Length::Fixed(160.0))
            .center(Length::Fill)
            .into(),
            BorderBeamCanvas::new(canvas_options.clone())
                .host(280.0, 160.0)
                .elapsed(self.elapsed)
                .system(system)
                .pixel_density(2.0)
                .into_element(),
        ]);

        let controls = row![
            button(text(format!("size · {}", self.size))).on_press(Message::NextSize),
            button(text(format!("palette · {}", self.palette))).on_press(Message::NextPalette),
            button(text(if self.static_colors {
                "static colors · on"
            } else {
                "static colors · off"
            }))
            .on_press(Message::ToggleStatic),
            button(text(if self.active {
                "active · on"
            } else {
                "active · off"
            }))
            .on_press(Message::ToggleActive),
        ]
        .spacing(12);

        let status = text(if !self.active {
            "beam off (faded out)"
        } else if transition {
            "paused while the theme crossfades"
        } else if self.lit {
            "fade-in finished callback fired"
        } else {
            "beam on"
        })
        .size(14);

        let body = column![
            row![widget_card, container(canvas_card).padding(24)]
                .spacing(48)
                .align_y(Alignment::Center),
            controls,
            status,
        ]
        .spacing(24)
        .align_x(Alignment::Center);

        _shared::themed_shell("Border Beam", body, &self.theme_shell, Message::ToggleTheme)
    }
}
