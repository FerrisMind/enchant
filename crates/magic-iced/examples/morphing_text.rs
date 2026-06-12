mod _shared;

use iced::time::Instant;
use iced::widget::column;
use iced::{Element, Length, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{MorphingTextProps, morphing_text};
use std::time::Duration;

const MORPHING_TEXTS: &[&str] = &[
    "Hello",
    "Morphing",
    "Text",
    "Animation",
    "React",
    "Component",
    "Smooth",
    "Transition",
    "Engaging",
];

pub fn main() -> iced::Result {
    iced::application(
        MorphingTextExample::new,
        MorphingTextExample::update,
        MorphingTextExample::view,
    )
    .title("Morphing Text")
    .window(iced::window::Settings {
        size: Size::new(900.0, 420.0),
        ..Default::default()
    })
    .subscription(MorphingTextExample::subscription)
    .run()
}

struct MorphingTextExample {
    theme: Theme,
    start: Instant,
    elapsed: Duration,
    props: MorphingTextProps,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl MorphingTextExample {
    fn new() -> (Self, Task<Message>) {
        let start = Instant::now();
        (
            Self {
                theme: Theme::default(),
                start,
                elapsed: Duration::ZERO,
                props: MorphingTextProps {
                    width: Length::Shrink,
                    ..MorphingTextProps::new(MORPHING_TEXTS.iter().copied())
                },
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        if let Message::Tick(now) = message {
            self.elapsed = now.duration_since(self.start);
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let body = morphing_text(self.elapsed, &self.props, &self.theme);
        _shared::shell("Morphing Text", column![body].spacing(24))
    }
}
