mod _shared;

use enchant_iced::{
    Dock, DockAction, DockDirection, DockEntry, DockState, dock_icon, dock_separator,
};
use iced::time::Instant;
use iced::widget::{column, text};
use iced::{Alignment, Color, Element, Font, Size, Task};
use lucide_icons::{Icon as LucideIcon, LUCIDE_FONT_BYTES};

const TAB_LABELS: &[&str] = &[
    "Preview",
    "Direction",
    "Custom Magnification",
    "Colored Icons",
    "Disabled Magnification",
];

const PREVIEW_ICON_COUNT: usize = 5;
const DIRECTION_ICON_COUNT: usize = 4;
const CUSTOM_ICON_COUNT: usize = 5;
const COLORED_ICON_COUNT: usize = 6;
const DISABLED_ICON_COUNT: usize = 5;

/// Lucide `size-4` inside preview `size-12` buttons.
const PREVIEW_GLYPH_SIZE: f32 = 16.0;
/// Lucide `size-full` inside decorative DockIcon padded content
/// (`icon_size` 40 − 2 × `max(6, icon_size * 0.2)` ≈ 24).
const DECORATIVE_GLYPH_SIZE: f32 = 24.0;

/// Light `bg-black/10`.
const SURFACE_LIGHT: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.1);
/// Dark `bg-white/10`.
const SURFACE_DARK: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.1);

pub fn main() -> iced::Result {
    iced::application(DockExample::new, DockExample::update, DockExample::view)
        .title("Dock")
        .window(iced::window::Settings {
            size: Size::new(1100.0, 720.0),
            ..Default::default()
        })
        .subscription(DockExample::subscription)
        .font(LUCIDE_FONT_BYTES)
        .run()
}

struct DockExample {
    theme_shell: _shared::ThemeShell,
    active_tab: usize,
    preview_dock: Dock,
    preview_state: DockState,
    direction_top_dock: Dock,
    direction_top_state: DockState,
    direction_middle_dock: Dock,
    direction_middle_state: DockState,
    direction_bottom_dock: Dock,
    direction_bottom_state: DockState,
    custom_dock: Dock,
    custom_state: DockState,
    colored_dock: Dock,
    colored_state: DockState,
    disabled_dock: Dock,
    disabled_state: DockState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LiveDock {
    Preview,
    DirectionTop,
    DirectionMiddle,
    DirectionBottom,
    Custom,
    Colored,
    Disabled,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    Tick(Instant),
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    SelectTab(usize),
    Dock(LiveDock, DockAction),
    DockIconPressed(&'static str),
}

impl DockExample {
    fn new() -> (Self, Task<Message>) {
        let preview_dock = Dock::new().direction(DockDirection::Middle);
        let mut preview_state = DockState::new();
        preview_state.prepare(PREVIEW_ICON_COUNT, preview_dock.values().icon_size);

        let direction_top_dock = Dock::new().direction(DockDirection::Top);
        let mut direction_top_state = DockState::new();
        direction_top_state.prepare(DIRECTION_ICON_COUNT, direction_top_dock.values().icon_size);

        let direction_middle_dock = Dock::new().direction(DockDirection::Middle);
        let mut direction_middle_state = DockState::new();
        direction_middle_state.prepare(DIRECTION_ICON_COUNT, direction_middle_dock.values().icon_size);

        let direction_bottom_dock = Dock::new().direction(DockDirection::Bottom);
        let mut direction_bottom_state = DockState::new();
        direction_bottom_state.prepare(DIRECTION_ICON_COUNT, direction_bottom_dock.values().icon_size);

        let custom_dock = Dock::new()
            .icon_magnification(60.0)
            .icon_distance(100.0);
        let mut custom_state = DockState::new();
        custom_state.prepare(CUSTOM_ICON_COUNT, custom_dock.values().icon_size);

        let colored_dock = Dock::new()
            .icon_magnification(70.0)
            .icon_distance(120.0);
        let mut colored_state = DockState::new();
        colored_state.prepare(COLORED_ICON_COUNT, colored_dock.values().icon_size);

        let disabled_dock = Dock::new().disable_magnification(true);
        let mut disabled_state = DockState::new();
        disabled_state.prepare(DISABLED_ICON_COUNT, disabled_dock.values().icon_size);

        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                active_tab: 0,
                preview_dock,
                preview_state,
                direction_top_dock,
                direction_top_state,
                direction_middle_dock,
                direction_middle_state,
                direction_bottom_dock,
                direction_bottom_state,
                custom_dock,
                custom_state,
                colored_dock,
                colored_state,
                disabled_dock,
                disabled_state,
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
            Message::SelectTab(index) => {
                self.active_tab = index;
            }
            Message::Tick(now) => {
                self.theme_shell.tick(now);
                self.tick_active_docks(now);
            }
            Message::Dock(which, action) => {
                self.apply_dock_action(which, &action);
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

    fn tick_active_docks(&mut self, now: Instant) {
        match self.active_tab {
            0 => self.preview_state.tick(now, &self.preview_dock),
            1 => {
                self.direction_top_state
                    .tick(now, &self.direction_top_dock);
                self.direction_middle_state
                    .tick(now, &self.direction_middle_dock);
                self.direction_bottom_state
                    .tick(now, &self.direction_bottom_dock);
            }
            2 => self.custom_state.tick(now, &self.custom_dock),
            3 => self.colored_state.tick(now, &self.colored_dock),
            _ => self.disabled_state.tick(now, &self.disabled_dock),
        }
    }

    fn apply_dock_action(&mut self, which: LiveDock, action: &DockAction) {
        let now = self.theme_shell.now;
        match which {
            LiveDock::Preview => {
                self.preview_state.apply_action(
                    action,
                    PREVIEW_ICON_COUNT,
                    self.preview_dock.values().icon_size,
                );
                self.preview_state.tick(now, &self.preview_dock);
            }
            LiveDock::DirectionTop => {
                self.direction_top_state.apply_action(
                    action,
                    DIRECTION_ICON_COUNT,
                    self.direction_top_dock.values().icon_size,
                );
                self.direction_top_state
                    .tick(now, &self.direction_top_dock);
            }
            LiveDock::DirectionMiddle => {
                self.direction_middle_state.apply_action(
                    action,
                    DIRECTION_ICON_COUNT,
                    self.direction_middle_dock.values().icon_size,
                );
                self.direction_middle_state
                    .tick(now, &self.direction_middle_dock);
            }
            LiveDock::DirectionBottom => {
                self.direction_bottom_state.apply_action(
                    action,
                    DIRECTION_ICON_COUNT,
                    self.direction_bottom_dock.values().icon_size,
                );
                self.direction_bottom_state
                    .tick(now, &self.direction_bottom_dock);
            }
            LiveDock::Custom => {
                self.custom_state.apply_action(
                    action,
                    CUSTOM_ICON_COUNT,
                    self.custom_dock.values().icon_size,
                );
                self.custom_state.tick(now, &self.custom_dock);
            }
            LiveDock::Colored => {
                self.colored_state.apply_action(
                    action,
                    COLORED_ICON_COUNT,
                    self.colored_dock.values().icon_size,
                );
                self.colored_state.tick(now, &self.colored_dock);
            }
            LiveDock::Disabled => {
                self.disabled_state.apply_action(
                    action,
                    DISABLED_ICON_COUNT,
                    self.disabled_dock.values().icon_size,
                );
                self.disabled_state.tick(now, &self.disabled_dock);
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let tabs = _shared::tabs(TAB_LABELS, self.active_tab, Message::SelectTab);
        let body = match self.active_tab {
            0 => self.preview_tab(),
            1 => self.direction_tab(),
            2 => self.custom_magnification_tab(),
            3 => self.colored_icons_tab(),
            _ => self.disabled_magnification_tab(),
        };

        _shared::themed_shell(
            "Dock",
            column![tabs, _shared::stage(body)].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }

    fn preview_tab(&self) -> Element<'_, Message> {
        let now = self.theme_shell.now;
        let theme = self.theme_shell.blended_theme();
        let foreground = self.theme_shell.blended_foreground(now);
        let title = dock_title(now, &self.theme_shell);

        // Preview: Home / Blog | GitHub / LinkedIn / X (Mail commented out in ref preview).
        let items: Vec<DockEntry<'_, Message>> = vec![
            preview_icon(LucideIcon::House, "Home", foreground).into(),
            preview_icon(LucideIcon::Pencil, "Blog", foreground).into(),
            dock_separator(),
            preview_icon(LucideIcon::Github, "GitHub", foreground).into(),
            preview_icon(LucideIcon::Linkedin, "LinkedIn", foreground).into(),
            preview_icon(LucideIcon::X, "X", foreground).into(),
        ];

        column![
            title,
            self.preview_dock.into_element(
                items,
                &self.preview_state,
                |action| Message::Dock(LiveDock::Preview, action),
                theme,
            ),
        ]
        .spacing(32)
        .align_x(Alignment::Center)
        .into()
    }

    fn direction_tab(&self) -> Element<'_, Message> {
        let theme = self.theme_shell.blended_theme();
        let muted = theme.palette.muted_foreground;
        let surface = decorative_surface(&self.theme_shell);

        column![
            direction_row(
                "Direction: Top",
                muted,
                self.direction_top_dock,
                &self.direction_top_state,
                LiveDock::DirectionTop,
                surface,
                theme,
            ),
            direction_row(
                "Direction: Middle (Default)",
                muted,
                self.direction_middle_dock,
                &self.direction_middle_state,
                LiveDock::DirectionMiddle,
                surface,
                theme,
            ),
            direction_row(
                "Direction: Bottom",
                muted,
                self.direction_bottom_dock,
                &self.direction_bottom_state,
                LiveDock::DirectionBottom,
                surface,
                theme,
            ),
        ]
        .spacing(32)
        .align_x(Alignment::Center)
        .into()
    }

    fn custom_magnification_tab(&self) -> Element<'_, Message> {
        let theme = self.theme_shell.blended_theme();
        let foreground = self.theme_shell.blended_foreground(self.theme_shell.now);
        let surface = decorative_surface(&self.theme_shell);

        let items: Vec<DockEntry<'_, Message>> = vec![
            decorative_icon(LucideIcon::Github, foreground, surface).into(),
            decorative_icon(LucideIcon::Cloud, foreground, surface).into(),
            decorative_icon(LucideIcon::FileText, foreground, surface).into(),
            decorative_icon(LucideIcon::MessageCircle, foreground, surface).into(),
            decorative_icon(LucideIcon::Music, foreground, surface).into(),
        ];

        self.custom_dock.into_element(
            items,
            &self.custom_state,
            |action| Message::Dock(LiveDock::Custom, action),
            theme,
        )
    }

    fn colored_icons_tab(&self) -> Element<'_, Message> {
        let theme = self.theme_shell.blended_theme();

        // Brand fills: hex/20 (Instagram solid per plan — no gradient API).
        // Brand fills: hex/20 → alpha 0.2 (Instagram solid rgba(225,48,108,0.2) per plan).
        let items: Vec<DockEntry<'_, Message>> = vec![
            decorative_icon(
                LucideIcon::Twitter,
                Color::from_rgb8(0x1D, 0xA1, 0xF2),
                Color::from_rgba8(0x1D, 0xA1, 0xF2, 0.2),
            )
            .into(),
            decorative_icon(
                LucideIcon::Instagram,
                Color::from_rgb8(0xE1, 0x30, 0x6C),
                Color::from_rgba8(225, 48, 108, 0.2),
            )
            .into(),
            decorative_icon(
                LucideIcon::Youtube,
                Color::from_rgb8(0xFF, 0x00, 0x00),
                Color::from_rgba8(0xFF, 0x00, 0x00, 0.2),
            )
            .into(),
            decorative_icon(
                LucideIcon::Twitch,
                Color::from_rgb8(0x91, 0x46, 0xFF),
                Color::from_rgba8(0x91, 0x46, 0xFF, 0.2),
            )
            .into(),
            decorative_icon(
                LucideIcon::Linkedin,
                Color::from_rgb8(0x00, 0x77, 0xB5),
                Color::from_rgba8(0x00, 0x77, 0xB5, 0.2),
            )
            .into(),
            decorative_icon(
                LucideIcon::Facebook,
                Color::from_rgb8(0x18, 0x77, 0xF2),
                Color::from_rgba8(0x18, 0x77, 0xF2, 0.2),
            )
            .into(),
        ];

        self.colored_dock.into_element(
            items,
            &self.colored_state,
            |action| Message::Dock(LiveDock::Colored, action),
            theme,
        )
    }

    fn disabled_magnification_tab(&self) -> Element<'_, Message> {
        let theme = self.theme_shell.blended_theme();
        let foreground = self.theme_shell.blended_foreground(self.theme_shell.now);
        let surface = decorative_surface(&self.theme_shell);

        let items: Vec<DockEntry<'_, Message>> = vec![
            decorative_icon(LucideIcon::Folder, foreground, surface).into(),
            decorative_icon(LucideIcon::Mail, foreground, surface).into(),
            decorative_icon(LucideIcon::Calendar, foreground, surface).into(),
            decorative_icon(LucideIcon::Camera, foreground, surface).into(),
            decorative_icon(LucideIcon::Gamepad2, foreground, surface).into(),
        ];

        self.disabled_dock.into_element(
            items,
            &self.disabled_state,
            |action| Message::Dock(LiveDock::Disabled, action),
            theme,
        )
    }
}

fn direction_row<'a>(
    label: &'static str,
    muted: Color,
    dock: Dock,
    state: &'a DockState,
    which: LiveDock,
    surface: Color,
    theme: &'a iced_shadcn::Theme,
) -> Element<'a, Message> {
    let foreground = theme.palette.foreground;
    let items: Vec<DockEntry<'_, Message>> = vec![
        decorative_icon(LucideIcon::House, foreground, surface).into(),
        decorative_icon(LucideIcon::Search, foreground, surface).into(),
        decorative_icon(LucideIcon::Bell, foreground, surface).into(),
        decorative_icon(LucideIcon::Settings, foreground, surface).into(),
    ];

    column![
        text(label)
            .size(14)
            .style(move |_| iced::widget::text::Style {
                color: Some(muted),
            }),
        dock.into_element(
            items,
            state,
            move |action| Message::Dock(which, action),
            theme,
        ),
    ]
    .spacing(8)
    .align_x(Alignment::Center)
    .into()
}

fn decorative_surface(theme_shell: &_shared::ThemeShell) -> Color {
    let now = theme_shell.now;
    let reveal = if theme_shell.controller.is_transition_active(now) {
        theme_shell.controller.reveal_at(now)
    } else if theme_shell.controller.dark_mode() {
        1.0
    } else {
        0.0
    };
    blend_colors(SURFACE_LIGHT, SURFACE_DARK, reveal)
}

/// Pressable preview icon: tooltip + `on_press` + Fixed 48 ghost button; glyph `size-4`.
fn preview_icon<'a>(
    icon: LucideIcon,
    label: &'static str,
    foreground: Color,
) -> enchant_iced::DockIcon<'a, Message> {
    dock_icon(lucide(icon, foreground, PREVIEW_GLYPH_SIZE))
        .tooltip(label)
        .on_press(Message::DockIconPressed(label))
}

/// Decorative DockIcon: surface `background`, no button chrome; glyph `size-full`.
fn decorative_icon<'a>(
    icon: LucideIcon,
    foreground: Color,
    background: Color,
) -> enchant_iced::DockIcon<'a, Message> {
    dock_icon(lucide(icon, foreground, DECORATIVE_GLYPH_SIZE)).background(background)
}

fn lucide<'a>(icon: LucideIcon, foreground: Color, size: f32) -> Element<'a, Message> {
    text(char::from(icon).to_string())
        .font(Font::with_name("lucide"))
        .size(size)
        .line_height(iced::widget::text::LineHeight::Absolute(iced::Pixels(size)))
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
