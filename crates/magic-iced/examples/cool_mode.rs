mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::{column, text};
use iced::{Element, Size, Subscription, Task};
use iced_shadcn::{ButtonProps, ButtonSize, ButtonVariant, Theme, button_content};
use magic_iced::{CoolModeAction, CoolModeProps, CoolModeState, cool_mode};

pub fn main() -> iced::Result {
    iced::application(CoolModeExample::new, CoolModeExample::update, CoolModeExample::view)
        .title("Cool Mode")
        .window(iced::window::Settings {
            size: Size::new(900.0, 360.0),
            ..Default::default()
        })
        .subscription(CoolModeExample::subscription)
        .run()
}

struct CoolModeExample {
    theme_shell: _shared::ThemeShell,
    state: CoolModeState,
    props: CoolModeProps,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(magic_iced::AnchorMeasurements),
    CoolMode(CoolModeAction),
    Tick(Instant),
}

impl CoolModeExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                state: CoolModeState::new(),
                props: CoolModeProps::new(),
                now: Instant::now(),
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

            Message::CoolMode(action) => {
                self.state.apply_action(&action, self.now, &self.props);
            }
            Message::Tick(now) => {
                self.theme_shell.tick(now);
                self.now = now;
                self.state.tick(now, &self.props);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let trigger = button_content(
            text("Click Me!"),
            None::<Message>,
            ButtonProps::new()
                .variant(ButtonVariant::Default)
                .size(ButtonSize::Size2),
            self.theme_shell.theme(),
        );

        let body = cool_mode(trigger, &self.state, &self.props, Message::CoolMode);
        _shared::themed_shell("Cool Mode", column![body].spacing(24), &self.theme_shell, Message::ToggleTheme)
    }
}
