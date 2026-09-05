mod _shared;

use enchant_iced::{
    ScrollVelocityContainerProps, ScrollVelocityDirection, ScrollVelocityItem,
    ScrollVelocityRowProps, ScrollVelocityRowState, scroll_velocity_container, scroll_velocity_row,
};
use iced::time::Instant;
use iced::widget::column;
use iced::{Element, Size, Subscription, Task};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        ScrollBasedVelocityExample::new,
        ScrollBasedVelocityExample::update,
        ScrollBasedVelocityExample::view,
    )
    .title("Scroll Based Velocity")
    .window(iced::window::Settings {
        size: Size::new(900.0, 420.0),
        ..Default::default()
    })
    .subscription(ScrollBasedVelocityExample::subscription)
    .run()
}

struct ScrollBasedVelocityExample {
    theme_shell: _shared::ThemeShell,
    forward_state: ScrollVelocityRowState,
    reverse_state: ScrollVelocityRowState,
    start: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Tick(Instant),
}

impl ScrollBasedVelocityExample {
    fn new() -> (Self, Task<Message>) {
        let start = Instant::now();
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                forward_state: ScrollVelocityRowState::new(),
                reverse_state: ScrollVelocityRowState::new(),
                start,
                elapsed: Duration::ZERO,
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
                self.forward_state.tick();
                self.reverse_state.tick();
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn row_props(direction: ScrollVelocityDirection) -> ScrollVelocityRowProps {
        ScrollVelocityRowProps {
            items: vec![ScrollVelocityItem::Text("Velocity Scroll".into())],
            base_velocity: 20.0,
            direction,
            item_extent: 280.0,
            item_cross: 48.0,
            ..ScrollVelocityRowProps::new()
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let rows = scroll_velocity_container(
            vec![
                scroll_velocity_row(
                    &self.forward_state,
                    self.elapsed,
                    &Self::row_props(ScrollVelocityDirection::Forward),
                    self.theme_shell.theme(),
                ),
                scroll_velocity_row(
                    &self.reverse_state,
                    self.elapsed,
                    &Self::row_props(ScrollVelocityDirection::Reverse),
                    self.theme_shell.theme(),
                ),
            ],
            ScrollVelocityContainerProps::new(),
        );

        _shared::themed_shell(
            "Scroll Based Velocity",
            column![rows].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
