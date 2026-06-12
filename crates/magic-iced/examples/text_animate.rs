mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::column;
use iced::{Element, Size, Subscription, Task};
use magic_iced::{
    TextAnimateBy, TextAnimatePreset, TextAnimateProps, TextAnimateState, text_animate,
};

pub fn main() -> iced::Result {
    iced::application(
        TextAnimateExample::new,
        TextAnimateExample::update,
        TextAnimateExample::view,
    )
    .title("Text Animate")
    .window(iced::window::Settings {
        size: Size::new(900.0, 360.0),
        ..Default::default()
    })
    .subscription(TextAnimateExample::subscription)
    .run()
}

struct TextAnimateExample {
    props: TextAnimateProps,
    state: TextAnimateState,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl TextAnimateExample {
    fn new() -> (Self, Task<Message>) {
        let props = TextAnimateProps {
            animation: TextAnimatePreset::BlurInUp,
            by: TextAnimateBy::Character,
            once: true,
            font_size: Some(28),
            ..TextAnimateProps::new()
        };
        let state = TextAnimateState::new(&props);
        (
            Self {
                props,
                state,
                now: Instant::now(),
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
        let body = text_animate("Blur in by character", &self.state, self.now, &self.props);
        _shared::shell("Text Animate", column![body].spacing(24))
    }
}
