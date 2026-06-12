mod _shared;

use iced::widget::{column, container};
use iced::{Element, Length, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{SafariMode, SafariProps, safari};

pub fn main() -> iced::Result {
    iced::application(
        SafariExample::new,
        SafariExample::update,
        SafariExample::view,
    )
    .title("Safari Example")
    .window(iced::window::Settings {
        size: Size::new(1200.0, 900.0),
        ..Default::default()
    })
    .subscription(SafariExample::subscription)
    .run()
}

struct SafariExample {
    active: usize,
    theme: Theme,
}

#[derive(Debug, Clone)]
enum Message {
    Select(usize),
}

impl SafariExample {
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
            Message::Select(index) => self.active = index.min(3),
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let labels = [
            "Safari Demo",
            "Safari Demo 2",
            "Safari Demo 3",
            "Safari Demo 4",
        ];
        let content = match self.active {
            0 => safari(
                SafariProps::new()
                    .width(Length::Fixed(680.0))
                    .chrome_label("magicui.design")
                    .screen_media(false),
                &self.theme,
            ),
            1 => safari(
                SafariProps::new()
                    .width(Length::Fixed(680.0))
                    .chrome_label("magicui.design")
                    .screen_media(true)
                    .media(include_bytes!("assets/hero-light.png"), "image/png"),
                &self.theme,
            ),
            2 => safari(
                SafariProps::new()
                    .width(Length::Fixed(680.0))
                    .chrome_label("magicui.design")
                    .screen_media(true)
                    .media(include_bytes!("assets/hero-dark.png"), "image/png"),
                &self.theme,
            ),
            _ => safari(
                SafariProps::new()
                    .width(Length::Fixed(680.0))
                    .chrome_label("magicui.design")
                    .mode(SafariMode::Simple)
                    .screen_media(true)
                    .media(include_bytes!("assets/hero-light.png"), "image/png"),
                &self.theme,
            ),
        };

        _shared::shell(
            "Safari",
            column![
                _shared::tabs(&labels, self.active, Message::Select),
                container(content).center_x(Length::Fill)
            ]
            .spacing(20),
        )
    }
}
