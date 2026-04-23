mod _shared;

use iced::time::Instant;
use iced::widget::{column, container, text};
use iced::{Element, Length, Size, Subscription, Task};
use magic_iced::{AnimatedListProps, AnimatedListState, animated_list};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        AnimatedListDemoExample::new,
        AnimatedListDemoExample::update,
        AnimatedListDemoExample::view,
    )
    .title("Animated List Demo")
    .window(iced::window::Settings {
        size: Size::new(1000.0, 760.0),
        ..Default::default()
    })
    .subscription(AnimatedListDemoExample::subscription)
    .run()
}

struct AnimatedListDemoExample {
    props: AnimatedListProps,
    state: AnimatedListState,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl AnimatedListDemoExample {
    fn new() -> (Self, Task<Message>) {
        let props = AnimatedListProps::new();
        let now = Instant::now();
        (
            Self {
                state: AnimatedListState::new(&props),
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
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let items = vec![
            container(text("Inbox sync"))
                .padding(16)
                .width(Length::Fill)
                .into(),
            container(text("Automations"))
                .padding(16)
                .width(Length::Fill)
                .into(),
            container(text("Release notes"))
                .padding(16)
                .width(Length::Fill)
                .into(),
            container(text("Design review"))
                .padding(16)
                .width(Length::Fill)
                .into(),
        ];

        _shared::shell(
            "Animated List Demo",
            column![
                text("Cards reveal in sequence with a focus scale on the newest item."),
                animated_list(items, &self.state, self.now, &self.props),
            ]
            .spacing(20),
        )
    }
}
