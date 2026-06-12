mod _shared;

use iced::widget::{column, text};
use iced::{Element, Size, Task};
use iced_shadcn::Theme;
use magic_iced::{DockAction, DockDirection, DockProps, DockState, dock, dock_icon};

pub fn main() -> iced::Result {
    iced::application(DockExample::new, DockExample::update, DockExample::view)
        .title("Dock")
        .window(iced::window::Settings {
            size: Size::new(900.0, 420.0),
            ..Default::default()
        })
        .run()
}

struct DockExample {
    theme: Theme,
    dock_state: DockState,
}

#[derive(Debug, Clone)]
enum Message {
    Dock(DockAction),
}

impl DockExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme: Theme::default(),
                dock_state: DockState::new(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Dock(action) => self.dock_state.apply_action(&action),
        }
        Task::none()
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
                &self.theme,
            ),
        ]
        .spacing(32);

        _shared::shell("Dock", body)
    }
}
