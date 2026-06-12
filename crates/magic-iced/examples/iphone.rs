mod _shared;

use iced::widget::{column, container};
use iced::{Element, Length, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{IphoneProps, iphone};

pub fn main() -> iced::Result {
    iced::application(
        IphoneExample::new,
        IphoneExample::update,
        IphoneExample::view,
    )
    .title("iPhone Example")
    .window(iced::window::Settings {
        size: Size::new(1200.0, 900.0),
        ..Default::default()
    })
    .subscription(IphoneExample::subscription)
    .run()
}

struct IphoneExample {
    active: usize,
    theme: Theme,
}

#[derive(Debug, Clone)]
enum Message {
    Select(usize),
}

impl IphoneExample {
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
        let labels = ["iPhone Demo", "iPhone Demo 2", "iPhone Demo 3"];
        let content = match self.active {
            0 => iphone(
                IphoneProps::new()
                    .width(Length::Fixed(320.0))
                    .screen_media(false),
                &self.theme,
            ),
            1 => iphone(
                IphoneProps::new()
                    .width(Length::Fixed(320.0))
                    .screen_media(true)
                    .media(
                        include_bytes!("assets/android-demo.jpg"),
                        "image/jpeg",
                    ),
                &self.theme,
            ),
            _ => iphone(
                IphoneProps::new()
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
            "iPhone",
            column![
                _shared::tabs(&labels, self.active, Message::Select),
                container(content).center_x(Length::Fill)
            ]
            .spacing(20),
        )
    }
}
