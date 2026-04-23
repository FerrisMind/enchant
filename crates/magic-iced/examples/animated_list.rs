mod _shared;

use iced::time::Instant;
use iced::widget::{column, container, text};
use iced::{Element, Length, Size, Subscription, Task};
use magic_iced::{AnimatedListProps, AnimatedListState, animated_list};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        AnimatedListExample::new,
        AnimatedListExample::update,
        AnimatedListExample::view,
    )
    .title("AnimatedList Example")
    .window(iced::window::Settings {
        size: Size::new(1000.0, 760.0),
        ..Default::default()
    })
    .subscription(AnimatedListExample::subscription)
    .run()
}

struct AnimatedListExample {
    active: usize,
    props: AnimatedListProps,
    state: AnimatedListState,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    Select(usize),
    Tick(Instant),
}

impl AnimatedListExample {
    fn new() -> (Self, Task<Message>) {
        let props = Self::props_for(0);
        let now = Instant::now();
        (
            Self {
                active: 0,
                state: AnimatedListState::new(&props),
                props,
                now,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Select(index) => {
                self.active = index.min(2);
                self.props = Self::props_for(self.active);
                self.state = AnimatedListState::new(&self.props);
                self.now = Instant::now();
            }
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
        let labels = ["Feed", "Tasks", "Notes"];
        let items = Self::items_for(self.active);

        _shared::shell(
            "Animated List",
            column![
                _shared::tabs(&labels, self.active, Message::Select),
                text("Rows reveal one by one and the newest row scales in."),
                animated_list(items, &self.state, self.now, &self.props),
            ]
            .spacing(20),
        )
    }
}

impl AnimatedListExample {
    fn props_for(active: usize) -> AnimatedListProps {
        match active {
            0 => AnimatedListProps {
                delay_ms: 700,
                gap: 12.0,
                scale_from: 0.88,
                width: Length::Fill,
            },
            1 => AnimatedListProps {
                delay_ms: 1200,
                gap: 16.0,
                scale_from: 0.84,
                width: Length::Fill,
            },
            _ => AnimatedListProps {
                delay_ms: 900,
                gap: 10.0,
                scale_from: 0.9,
                width: Length::Fill,
            },
        }
    }

    fn items_for(active: usize) -> Vec<Element<'static, Message>> {
        match active {
            0 => vec![
                container(text("Inbox sync"))
                    .padding(16)
                    .width(Length::Fill)
                    .into(),
                container(text("Theme update"))
                    .padding(16)
                    .width(Length::Fill)
                    .into(),
                container(text("Motion review"))
                    .padding(16)
                    .width(Length::Fill)
                    .into(),
                container(text("Release prep"))
                    .padding(16)
                    .width(Length::Fill)
                    .into(),
            ],
            1 => vec![
                container(text("Write tests"))
                    .padding(16)
                    .width(Length::Fill)
                    .into(),
                container(text("Open PR"))
                    .padding(16)
                    .width(Length::Fill)
                    .into(),
                container(text("Merge queue"))
                    .padding(16)
                    .width(Length::Fill)
                    .into(),
            ],
            _ => vec![
                container(text("Monday"))
                    .padding(16)
                    .width(Length::Fill)
                    .into(),
                container(text("Tuesday"))
                    .padding(16)
                    .width(Length::Fill)
                    .into(),
                container(text("Wednesday"))
                    .padding(16)
                    .width(Length::Fill)
                    .into(),
                container(text("Thursday"))
                    .padding(16)
                    .width(Length::Fill)
                    .into(),
                container(text("Friday"))
                    .padding(16)
                    .width(Length::Fill)
                    .into(),
            ],
        }
    }
}
