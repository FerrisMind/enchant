mod _shared;

use iced::widget::{column, container, text};
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
            0 => iphone_screen(
                "Camera",
                "Focus on top notch chrome.",
                "Live preview",
                &self.theme,
            ),
            1 => iphone_screen(
                "Music",
                "Tall content rail and compact card stack.",
                "Now playing",
                &self.theme,
            ),
            _ => iphone_screen(
                "Fitness",
                "Activity summary with clean spacing.",
                "3 goals met",
                &self.theme,
            ),
        };

        _shared::shell(
            "iPhone",
            column![
                _shared::tabs(&labels, self.active, Message::Select),
                content
            ]
            .spacing(20),
        )
    }
}

fn iphone_screen<'a>(
    title: &'a str,
    subtitle: &'a str,
    metric: &'a str,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let body = column![
        text(title).size(24),
        text(subtitle).size(14),
        container(column![
            text(metric).size(34),
            text("Swipe to explore").size(14)
        ])
        .padding(24)
        .width(Length::Fill),
    ]
    .spacing(16);

    iphone(
        container(body)
            .padding(20)
            .width(Length::Fill)
            .height(Length::Fill),
        IphoneProps::new().width(Length::Fixed(320.0)),
        theme,
    )
}
