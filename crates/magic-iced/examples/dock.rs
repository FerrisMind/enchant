mod _shared;

use iced::widget::{column, text};
use iced::time::Instant;
use std::time::Duration;
use iced::{Element, Size, Subscription, Task};
use magic_iced::{DockAction, DockDirection, DockProps, DockState, dock, dock_icon};

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
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                dock_state: DockState::new(),
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
            }
            Message::Dock(action) => self.dock_state.apply_action(&action),
        }
        Task::none()
    }


    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let items = vec![
            dock_icon(text("⌂").size(16)),
            dock_icon(text("✎").size(16)),
            dock_icon(text("GH").size(11)),
            dock_icon(text("in").size(11)),
            dock_icon(text("X").size(12)),
            dock_icon(text("✉").size(16)),
        ];

        let body = column![
            text("Dock").size(72),
            dock(
                items,
                &self.dock_state,
                DockProps {
                    direction: DockDirection::Middle,
                    ..DockProps::new()
                },
                Message::Dock,
                self.theme_shell.theme(),
            ),
        ]
        .spacing(32);

        _shared::themed_shell("Dock", body, &self.theme_shell, Message::ToggleTheme)
    }
}
