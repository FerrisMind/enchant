mod _shared;

use iced::widget::{column, container, text};
use iced::{Element, Length, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{SafariMode, SafariProps, safari};

pub fn main() -> iced::Result {
    iced::application(SafariExample::new, SafariExample::update, SafariExample::view)
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
        let labels = ["Safari Demo", "Safari Demo 2", "Safari Demo 3", "Safari Demo 4"];
        let content = match self.active {
            0 => safari_screen("Docs", "Browser chrome with hero preview.", "magicui.design", SafariMode::Default, &self.theme),
            1 => safari_screen("Dashboard", "Simple mode for cleaner framing.", "Analytics", SafariMode::Simple, &self.theme),
            2 => safari_screen("Landing", "Wide screen with polished meta bar.", "Launch page", SafariMode::Default, &self.theme),
            _ => safari_screen("Composer", "Tall browser frame with content stack.", "Product copy", SafariMode::Simple, &self.theme),
        };

        _shared::shell(
            "Safari",
            column![_shared::tabs(&labels, self.active, Message::Select), content].spacing(20),
        )
    }
}

fn safari_screen<'a>(
    title: &'a str,
    subtitle: &'a str,
    footer: &'static str,
    mode: SafariMode,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let body = column![
        text(title).size(24),
        text(subtitle).size(14),
        container(column![text(footer).size(34), text("Preview in browser frame").size(14)])
            .padding(24)
            .width(Length::Fill),
    ]
    .spacing(16);

    safari(
        container(body)
            .padding(20)
            .width(Length::Fill)
            .height(Length::Fill),
        SafariProps::new()
            .width(Length::Fixed(680.0))
            .mode(mode)
            .chrome_label(footer),
        theme,
    )
}
