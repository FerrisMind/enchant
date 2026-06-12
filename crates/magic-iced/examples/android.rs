mod _shared;

use iced::widget::{column, container};
use iced::{Element, Length, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{AndroidProps, android};

pub fn main() -> iced::Result {
    iced::application(
        AndroidExample::new,
        AndroidExample::update,
        AndroidExample::view,
    )
    .title("Android Example")
    .window(iced::window::Settings {
        size: Size::new(1200.0, 900.0),
        ..Default::default()
    })
    .subscription(AndroidExample::subscription)
    .run()
}

struct AndroidExample {
    active: usize,
    theme: Theme,
}

#[derive(Debug, Clone)]
enum Message {
    Select(usize),
}

impl AndroidExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                active: 0,
                theme: Theme::default(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Select(index) => self.active = index.min(2),
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let labels = ["Android Demo", "Android Demo 2", "Android Demo 3"];
        let content = match self.active {
            0 => android(
                AndroidProps::new()
                    .width(Length::Fixed(320.0))
                    .screen_media(false),
                &self.theme,
            ),
            1 => android(
                AndroidProps::new()
                    .width(Length::Fixed(320.0))
                    .screen_media(true)
                    .media(
                        include_bytes!("assets/android-demo.jpg"),
                        "image/jpeg",
                    ),
                &self.theme,
            ),
            _ => android(
                AndroidProps::new()
                    .width(Length::Fixed(320.0))
                    .screen_media(true)
                    .media(
                        include_bytes!("assets/android-demo-video.jpg"),
                        "image/jpeg",
                    ),
                &self.theme,
            ),
        };

        _shared::shell(
            "Android",
            column![
                _shared::tabs(&labels, self.active, Message::Select),
                container(content).center_x(Length::Fill)
            ]
            .spacing(20),
        )
    }
}
