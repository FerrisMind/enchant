mod _shared;

use enchant_iced::{Dock, DockAction, DockDirection, DockEntry, DockState, dock_icon, dock_separator};
use iced::time::Instant;
use iced::widget::{column, text};
use iced::{Alignment, Color, Element, Font, Size, Task};
use lucide_icons::{Icon as LucideIcon, LUCIDE_FONT_BYTES};

pub fn main() -> iced::Result {
    iced::application(DockExample::new, DockExample::update, DockExample::view)
        .title("Dock")
        .window(iced::window::Settings {
            size: Size::new(900.0, 420.0),
            ..Default::default()
        })
        .subscription(DockExample::subscription)
        .font(LUCIDE_FONT_BYTES)
        .run()
}

struct DockExample {
    theme_shell: _shared::ThemeShell,
    dock_state: DockState,
    dock: Dock,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    Tick(Instant),
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Dock(DockAction),
    DockIconPressed(&'static str),
}

impl DockExample {
    fn new() -> (Self, Task<Message>) {
        let dock = Dock::new().direction(DockDirection::Middle);
        let mut dock_state = DockState::new();
        dock_state.prepare(DOCK_ICON_COUNT, dock.values().icon_size);

        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                dock_state,
                dock,
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
                self.dock_state.tick(now, &self.dock);
            }
            Message::Dock(action) => {
                self.dock_state.apply_action(
                    &action,
                    DOCK_ICON_COUNT,
                    self.dock.values().icon_size,
                );
                self.dock_state.tick(self.theme_shell.now, &self.dock);
            }
            Message::DockIconPressed(label) => {
                let _ = label;
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

        // Preview: Home / Blog | GitHub / LinkedIn / X (Mail commented out in ref preview).
        // Glyph stays `size-4` (16px) like the animations preview — only DockIcon layout springs.
        let items: Vec<DockEntry<'_, Message>> = vec![
            dock_demo_icon(LucideIcon::House, "Home", foreground).into(),
            dock_demo_icon(LucideIcon::Pencil, "Blog", foreground).into(),
            dock_separator(),
            dock_demo_icon(LucideIcon::Github, "GitHub", foreground).into(),
            dock_demo_icon(LucideIcon::Linkedin, "LinkedIn", foreground).into(),
            dock_demo_icon(LucideIcon::X, "X", foreground).into(),
        ];

        let body = column![
            title,
            self.dock
                .into_element(items, &self.dock_state, Message::Dock, theme),
        ]
        .spacing(32)
        .align_x(Alignment::Center);

        _shared::themed_shell("Dock", body, &self.theme_shell, Message::ToggleTheme)
    }
}

const DOCK_ICON_COUNT: usize = 5;

/// Lucide glyph inside a pressable round ghost `DockIcon` (tooltip + `on_press` chrome).
fn dock_demo_icon<'a>(
    icon: LucideIcon,
    label: &'static str,
    foreground: Color,
) -> enchant_iced::DockIcon<'a, Message> {
    dock_icon(lucide(icon, foreground))
        .tooltip(label)
        .on_press(Message::DockIconPressed(label))
}

fn lucide<'a>(icon: LucideIcon, foreground: Color) -> Element<'a, Message> {
    text(char::from(icon).to_string())
        .font(Font::with_name("lucide"))
        .size(16)
        .style(move |_| iced::widget::text::Style {
            color: Some(foreground),
        })
        .into()
}

/// Preview-style title: dark/light solid heading (CSS uses vertical gradient; solid matches top stop).
fn dock_title(now: Instant, theme_shell: &_shared::ThemeShell) -> Element<'static, Message> {
    let reveal = if theme_shell.controller.is_transition_active(now) {
        theme_shell.controller.reveal_at(now)
    } else if theme_shell.controller.dark_mode() {
        1.0
    } else {
        0.0
    };

    let light = Color::BLACK;
    let dark = Color::WHITE;
    let color = blend_colors(light, dark, reveal);

    text("Dock")
        .size(96)
        .style(move |_| iced::widget::text::Style {
            color: Some(color),
        })
        .into()
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
