mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::column;
use iced::{Element, Size, Subscription, Task};
use magic_iced::{ComicTextProps, ComicTextState, comic_text};

pub fn main() -> iced::Result {
    iced::application(
        ComicTextExample::new,
        ComicTextExample::update,
        ComicTextExample::view,
    )
    .title("Comic Text")
    .window(iced::window::Settings {
        size: Size::new(900.0, 420.0),
        ..Default::default()
    })
    .subscription(ComicTextExample::subscription)
    .run()
}

struct ComicTextExample {
    state: ComicTextState,
    props: ComicTextProps,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl ComicTextExample {
    fn new() -> (Self, Task<Message>) {
        let props = ComicTextProps::new();
        let now = Instant::now();
        (
            Self {
                state: ComicTextState::new(&props),
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
        let body = comic_text("BOOM!", &self.state, self.now, &self.props);
        _shared::shell("Comic Text", column![body].spacing(24))
    }
}
