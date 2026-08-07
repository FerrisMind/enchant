mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::column;
use iced::{Element, Size, Subscription, Task};
use enchant_iced::{TypingAnimationProps, TypingAnimationState, typing_animation};

pub fn main() -> iced::Result {
    iced::application(
        TypingAnimationExample::new,
        TypingAnimationExample::update,
        TypingAnimationExample::view,
    )
    .title("Typing Animation")
    .window(iced::window::Settings {
        size: Size::new(900.0, 420.0),
        ..Default::default()
    })
    .subscription(TypingAnimationExample::subscription)
    .run()
}

struct TypingAnimationExample {
    theme_shell: _shared::ThemeShell,
    state: TypingAnimationState,
    props: TypingAnimationProps,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Tick(Instant),
}

impl TypingAnimationExample {
    fn new() -> (Self, Task<Message>) {
        let props = TypingAnimationProps::new("Hello World! 👋").font_size(32);
        let now = Instant::now();
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                state: TypingAnimationState::new(&props),
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
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let body = typing_animation(&self.state, self.now, &self.props, self.theme_shell.theme());
        _shared::themed_shell(
            "Typing Animation",
            column![body].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
