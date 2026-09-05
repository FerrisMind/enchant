mod _shared;

use enchant_iced::{WordRotateProps, WordRotateState, word_rotate};
use iced::time::{Duration, Instant};
use iced::widget::column;
use iced::{Element, Size, Subscription, Task};

pub fn main() -> iced::Result {
    iced::application(
        WordRotateExample::new,
        WordRotateExample::update,
        WordRotateExample::view,
    )
    .title("Word Rotate")
    .window(iced::window::Settings {
        size: Size::new(900.0, 420.0),
        ..Default::default()
    })
    .subscription(WordRotateExample::subscription)
    .run()
}

struct WordRotateExample {
    theme_shell: _shared::ThemeShell,
    state: WordRotateState,
    props: WordRotateProps,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Tick(Instant),
}

impl WordRotateExample {
    fn new() -> (Self, Task<Message>) {
        let props = WordRotateProps::new(["Word", "Rotate"]).font_size(36);
        let now = Instant::now();
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                state: WordRotateState::new(&props),
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
        let body = word_rotate(&self.state, self.now, &self.props, self.theme_shell.theme());
        _shared::themed_shell(
            "Word Rotate",
            column![body].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
