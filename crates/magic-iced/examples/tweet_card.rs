mod _shared;

use iced::widget::{column, text};
use iced::{Element, Size, Task};
use iced_shadcn::Theme;
use magic_iced::{TweetAsset, TweetAuthor, TweetCardData, TweetCardProps, TweetMedia, tweet_card};

pub fn main() -> iced::Result {
    iced::application(
        TweetCardExample::new,
        TweetCardExample::update,
        TweetCardExample::view,
    )
    .title("Tweet Card Example")
    .window(iced::window::Settings {
        size: Size::new(1200.0, 900.0),
        ..Default::default()
    })
    .run()
}

struct TweetCardExample {
    active: usize,
    theme: Theme,
}

#[derive(Debug, Clone)]
enum Message {
    Select(usize),
}

impl TweetCardExample {
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

    fn view(&self) -> Element<'_, Message> {
        let labels = ["Tweet Demo", "Tweet Images", "Tweet Meta"];
        let card = tweet_card(
            &tweet_data(self.active),
            &TweetCardProps::new(),
            &self.theme,
        );

        _shared::shell(
            "Tweet Card",
            column![
                _shared::tabs(&labels, self.active, Message::Select),
                card,
                text("Server-side card surface.")
            ]
            .spacing(20),
        )
    }
}

fn tweet_data(index: usize) -> TweetCardData {
    match index {
        0 => TweetCardData {
            id: "1668408059125702661".into(),
            text: "FerrisMind shipped native examples for the whole component surface.".into(),
            author: TweetAuthor {
                name: "FerrisMind".into(),
                handle: "ferrismind".into(),
                avatar: None,
                verified: true,
            },
            url: Some("https://x.com/ferrismind".into()),
            timestamp: Some("Jun 13, 2023".into()),
            media: None,
        },
        1 => TweetCardData {
            id: "1678577280489234432".into(),
            text: "Image-heavy card with compact metadata.".into(),
            author: TweetAuthor {
                name: "Design Systems".into(),
                handle: "designsystems".into(),
                avatar: Some(TweetAsset::new("Avatar")),
                verified: false,
            },
            url: Some("https://magicui.design".into()),
            timestamp: Some("Jul 7, 2023".into()),
            media: Some(TweetMedia::LinkPreview {
                title: "Magic UI".into(),
                site: "magicui.design".into(),
                thumbnail: None,
                url: "https://magicui.design".into(),
            }),
        },
        _ => TweetCardData {
            id: "1675849118445436929".into(),
            text: "Meta preview card with compact author info.".into(),
            author: TweetAuthor {
                name: "Product".into(),
                handle: "product".into(),
                avatar: None,
                verified: true,
            },
            url: Some("https://magicui.design/r/tweet-card".into()),
            timestamp: Some("Jun 30, 2023".into()),
            media: Some(TweetMedia::VideoPoster {
                poster: None,
                label: "Preview".into(),
            }),
        },
    }
}
