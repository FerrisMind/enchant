mod _shared;

use iced::time::Instant;
use iced::widget::column;
use iced::{Alignment, Element, Size, Subscription, Task};
use magic_iced::{GlobeAction, GlobeProps, GlobeState, globe_demo};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(GlobeExample::new, GlobeExample::update, GlobeExample::view)
        .title("Globe")
        .window(iced::window::Settings {
            size: Size::new(900.0, 620.0),
            ..Default::default()
        })
        .subscription(GlobeExample::subscription)
        .run()
}

struct GlobeExample {
    theme_shell: _shared::ThemeShell,
    state: GlobeState,
    props: GlobeProps,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(magic_iced::AnchorMeasurements),
    Globe(GlobeAction),
    Tick(Instant),
}

impl GlobeExample {
    fn new() -> (Self, Task<Message>) {
        let props = GlobeProps::new();
        let state = GlobeState::new(&props);
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                state,
                props,
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
            Message::Globe(action) => self.state.apply_action(&action, &self.props),
            Message::Tick(now) => {
                self.theme_shell.tick(now);
                self.state.tick(now, &self.props);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let colors = self
            .theme_shell
            .blended_globe_demo_colors(self.theme_shell.now);
        let stage = globe_demo(&self.state, &self.props, colors, Message::Globe);

        _shared::themed_shell(
            "Globe",
            column![stage].spacing(24).align_x(Alignment::Center),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
