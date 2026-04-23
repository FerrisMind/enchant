mod _shared;

use iced::widget::{column, text};
use iced::{Element, Size, Task};
use iced_shadcn::Theme;
use magic_iced::{
    ClientTweetCardProps, ClientTweetCardState, ClientTweetCardStatus, TweetAsset, TweetAuthor,
    TweetCardData, TweetMedia, client_tweet_card,
};

pub fn main() -> iced::Result {
    iced::application(
        ClientTweetCardExample::new,
        ClientTweetCardExample::update,
        ClientTweetCardExample::view,
    )
    .title("Client Tweet Card Example")
    .window(iced::window::Settings {
        size: Size::new(1200.0, 900.0),
        ..Default::default()
    })
    .run()
}

struct ClientTweetCardExample {
    active: usize,
    theme: Theme,
}

#[derive(Debug, Clone)]
enum Message {
    Select(usize),
}

impl ClientTweetCardExample {
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
        let labels = ["Loading", "Ready", "Error"];
        let state = client_state(self.active);
        let card = client_tweet_card(&state, &ClientTweetCardProps::new(), &self.theme);

        _shared::shell(
            "Client Tweet Card",
            column![
                _shared::tabs(&labels, self.active, Message::Select),
                card,
                text("Client surface state demo.")
            ]
            .spacing(20),
        )
    }
}

fn client_state(index: usize) -> ClientTweetCardState {
    match index {
        0 => ClientTweetCardState {
            status: ClientTweetCardStatus::Loading,
            data: None,
        },
        1 => ClientTweetCardState {
            status: ClientTweetCardStatus::Ready,
            data: Some(sample_tweet()),
        },
        _ => ClientTweetCardState {
            status: ClientTweetCardStatus::Error("Tweet unavailable".into()),
            data: None,
        },
    }
}

fn sample_tweet() -> TweetCardData {
    TweetCardData {
        id: "1675849118445436929".into(),
        text: "Client tweet card renders card state and remote-ready content.".into(),
        author: TweetAuthor {
            name: "FerrisMind".into(),
            handle: "ferrismind".into(),
            avatar: Some(TweetAsset::new("Avatar")),
            verified: true,
        },
        url: Some("https://x.com/ferrismind".into()),
        timestamp: Some("Jul 10, 2023".into()),
        media: Some(TweetMedia::LinkPreview {
            title: "Magic UI".into(),
            site: "magicui.design".into(),
            thumbnail: None,
            url: "https://magicui.design".into(),
        }),
    }
}
