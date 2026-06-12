mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::column;
use iced::{Element, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{TypingAnimationProps, TypingAnimationState, typing_animation};

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
    theme: Theme,
    state: TypingAnimationState,
    props: TypingAnimationProps,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl TypingAnimationExample {
    fn new() -> (Self, Task<Message>) {
        let props = TypingAnimationProps::new("Hello World! 👋").font_size(32);
        let now = Instant::now();
        (
            Self {
                theme: Theme::default(),
                state: TypingAnimationState::new(&props),
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
        let body = typing_animation(&self.state, self.now, &self.props, &self.theme);
        _shared::shell("Typing Animation", column![body].spacing(24))
    }
}
