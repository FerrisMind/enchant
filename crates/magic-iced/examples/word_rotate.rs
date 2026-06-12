mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::column;
use iced::{Element, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{WordRotateProps, WordRotateState, word_rotate};

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
    theme: Theme,
    state: WordRotateState,
    props: WordRotateProps,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl WordRotateExample {
    fn new() -> (Self, Task<Message>) {
        let props = WordRotateProps::new(["Word", "Rotate"]).font_size(36);
        let now = Instant::now();
        (
            Self {
                theme: Theme::default(),
                state: WordRotateState::new(&props),
                props,
                now,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        if let Message::Tick(now) = message {
            self.now = now;
            self.state.tick(now, &self.props);
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let body = word_rotate(&self.state, self.now, &self.props, &self.theme);
        _shared::shell("Word Rotate", column![body].spacing(24))
    }
}
