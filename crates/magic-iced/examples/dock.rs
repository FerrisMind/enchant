mod _shared;

use iced::time::Instant;
use iced::widget::{column, text};
use iced::{Color, Element, Size, Task};
use magic_iced::{
    AnimatedGradientTextProps, DockAction, DockDirection, DockProps, DockState, animated_gradient_text,
    dock, dock_icon,
};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(DockExample::new, DockExample::update, DockExample::view)
        .title("Dock")
        .window(iced::window::Settings {
            size: Size::new(900.0, 420.0),
            ..Default::default()
        })
        .subscription(DockExample::subscription)
        .run()
}

struct DockExample {
    theme_shell: _shared::ThemeShell,
    dock_state: DockState,
    dock_props: DockProps,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    Tick(Instant),
    AnchorsMeasured(magic_iced::AnchorMeasurements),
    Dock(DockAction),
}

impl DockExample {
    fn new() -> (Self, Task<Message>) {
        let dock_props = DockProps {
            direction: DockDirection::Middle,
            ..DockProps::new()
        };
        let mut dock_state = DockState::new();
        dock_state.prepare(DOCK_ICON_COUNT, dock_props.icon_size);

        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                dock_state,
                dock_props,
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
                self.dock_state.tick(now, &self.dock_props);
            }
            Message::Dock(action) => {
                self.dock_state.apply_action(
                    &action,
                    DOCK_ICON_COUNT,
                    self.dock_props.icon_size,
                );
                self.dock_state.tick(self.theme_shell.now, &self.dock_props);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let now = self.theme_shell.now;
        let theme = self.theme_shell.blended_theme();
        let foreground = self.theme_shell.blended_foreground(now);
        let title = dock_title(now, &self.theme_shell);

        let items = vec![
            dock_icon(icon_label("⌂", foreground, 16)),
            dock_icon(icon_label("✎", foreground, 16)),
            dock_icon(icon_label("GH", foreground, 11)),
            dock_icon(icon_label("in", foreground, 11)),
            dock_icon(icon_label("X", foreground, 12)),
            dock_icon(icon_label("✉", foreground, 16)),
        ];

        let body = column![
            title,
            dock(
                items,
                &self.dock_state,
                self.dock_props,
                Message::Dock,
                theme,
            ),
        ]
        .spacing(32);

        _shared::themed_shell("Dock", body, &self.theme_shell, Message::ToggleTheme)
    }
}

const DOCK_ICON_COUNT: usize = 6;

fn icon_label<'a>(
    label: &'static str,
    foreground: Color,
    size: u16,
) -> Element<'a, Message> {
    text(label)
        .size(u32::from(size))
        .style(move |_| iced::widget::text::Style {
            color: Some(foreground),
        })
        .into()
}

fn dock_title(now: Instant, theme_shell: &_shared::ThemeShell) -> Element<'static, Message> {
    let reveal = if theme_shell.controller.is_transition_active(now) {
        theme_shell.controller.reveal_at(now)
    } else if theme_shell.controller.dark_mode() {
        1.0
    } else {
        0.0
    };

    let light_from = Color::BLACK;
    let light_to = Color {
        r: 0.83,
        g: 0.83,
        b: 0.83,
        a: 0.8,
    };
    let dark_from = Color::WHITE;
    let dark_to = Color {
        r: 0.07,
        g: 0.09,
        b: 0.15,
        a: 0.1,
    };

    let from = blend_colors(light_from, dark_from, reveal);
    let to = blend_colors(light_to, dark_to, reveal);

    animated_gradient_text(
        "Dock",
        Duration::ZERO,
        AnimatedGradientTextProps::new()
            .font_size(72)
            .speed(0.0)
            .color_from(from)
            .color_to(to),
    )
}

fn blend_colors(from: Color, to: Color, progress: f32) -> Color {
    let t = progress.clamp(0.0, 1.0);
    Color {
        r: from.r + (to.r - from.r) * t,
        g: from.g + (to.g - from.g) * t,
        b: from.b + (to.b - from.b) * t,
        a: from.a + (to.a - from.a) * t,
    }
}
