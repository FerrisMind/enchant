mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::column;
use iced::{Element, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{KineticTextProps, KineticTextState, kinetic_text};

pub fn main() -> iced::Result {
    iced::application(
        KineticTextExample::new,
        KineticTextExample::update,
        KineticTextExample::view,
    )
    .title("Kinetic Text")
    .subscription(KineticTextExample::subscription)
    .window(iced::window::Settings {
        size: Size::new(1100.0, 420.0),
        ..Default::default()
    })
    .run()
}

struct KineticTextExample {
    theme: Theme,
    state: KineticTextState,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl KineticTextExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme: Theme::default(),
                state: KineticTextState::new(),
                now: Instant::now(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick(now) => self.now = now,
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let body = kinetic_text(
            "Nostalgia",
            &self.state,
            self.now,
            &KineticTextProps::new().font_size(96),
            &self.theme,
        );

        _shared::shell("Kinetic Text", column![body].spacing(24))
    }
}
