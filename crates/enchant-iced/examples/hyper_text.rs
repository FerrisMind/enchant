mod _shared;

use enchant_iced::{HyperTextAction, HyperTextProps, HyperTextState, hyper_text};
use iced::time::{Duration, Instant};
use iced::widget::column;
use iced::{Color, Element, Size, Subscription, Task};

pub fn main() -> iced::Result {
    iced::application(
        HyperTextExample::new,
        HyperTextExample::update,
        HyperTextExample::view,
    )
    .title("Hyper Text")
    .window(iced::window::Settings {
        size: Size::new(900.0, 420.0),
        ..Default::default()
    })
    .subscription(HyperTextExample::subscription)
    .run()
}

struct HyperTextExample {
    theme_shell: _shared::ThemeShell,
    state: HyperTextState,
    props: HyperTextProps,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Tick(Instant),
    Hyper(HyperTextAction),
}

impl HyperTextExample {
    fn new() -> (Self, Task<Message>) {
        let props = HyperTextProps {
            font_size: Some(32),
            color: Some(Color::from_rgb8(0x17, 0x17, 0x17)),
            ..HyperTextProps::new()
        };
        let now = Instant::now();
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                state: HyperTextState::new(&props),
                props,
                now,
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
                self.now = now;
                self.state.tick(now, &self.props);
            }
            Message::Hyper(action) => {
                self.state.apply_action(&action, self.now, &self.props);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let body = hyper_text(
            "Hover Me!",
            &self.state,
            self.now,
            &self.props,
            Message::Hyper,
        );

        _shared::themed_shell(
            "Hyper Text",
            column![body].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
