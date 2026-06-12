mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::column;
use iced::{Color, Element, Size, Subscription, Task};
use magic_iced::{HyperTextAction, HyperTextProps, HyperTextState, hyper_text};

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
    state: HyperTextState,
    props: HyperTextProps,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
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
                state: HyperTextState::new(&props),
                props,
                now,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick(now) => {
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

        _shared::shell("Hyper Text", column![body].spacing(24))
    }
}
