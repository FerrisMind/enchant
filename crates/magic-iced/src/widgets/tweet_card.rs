use iced::widget::{column, container, image, row, scrollable, text};
use iced::{Alignment, Background, Color, Element, Length};
use iced_shadcn::{CardProps, CardSize, Theme, card};

#[derive(Clone, Debug)]
pub struct TweetAsset {
    pub alt: String,
    pub handle: Option<image::Handle>,
    pub source_url: Option<String>,
}

impl TweetAsset {
    pub fn new(alt: impl Into<String>) -> Self {
        Self {
            alt: alt.into(),
            handle: None,
            source_url: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TweetAuthor {
    pub name: String,
    pub handle: String,
    pub avatar: Option<TweetAsset>,
    pub verified: bool,
}

#[derive(Clone, Debug)]
pub enum TweetMedia {
    Images(Vec<TweetAsset>),
    VideoPoster {
        poster: Option<TweetAsset>,
        label: String,
    },
    LinkPreview {
        title: String,
        site: String,
        thumbnail: Option<TweetAsset>,
        url: String,
    },
}

#[derive(Clone, Debug)]
pub struct TweetCardData {
    pub id: String,
    pub text: String,
    pub author: TweetAuthor,
    pub url: Option<String>,
    pub timestamp: Option<String>,
    pub media: Option<TweetMedia>,
}

#[derive(Clone, Debug)]
pub struct TweetCardProps {
    pub width: Length,
    pub media_height: f32,
    pub accent_color: Option<Color>,
}

impl Default for TweetCardProps {
    fn default() -> Self {
        Self {
            width: Length::Fixed(540.0),
            media_height: 208.0,
            accent_color: None,
        }
    }
}

impl TweetCardProps {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ClientTweetCardStatus {
    Idle,
    Loading,
    Ready,
    Error(String),
}

#[derive(Clone, Debug)]
pub struct ClientTweetCardState {
    pub status: ClientTweetCardStatus,
    pub data: Option<TweetCardData>,
}

impl Default for ClientTweetCardState {
    fn default() -> Self {
        Self {
            status: ClientTweetCardStatus::Idle,
            data: None,
        }
    }
}

impl ClientTweetCardState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_action(&mut self, action: &ClientTweetCardAction) {
        match action {
            ClientTweetCardAction::Loading => {
                self.status = ClientTweetCardStatus::Loading;
                self.data = None;
            }
            ClientTweetCardAction::Loaded(data) => {
                self.status = ClientTweetCardStatus::Ready;
                self.data = Some(*data.clone());
            }
            ClientTweetCardAction::Failed(error) => {
                self.status = ClientTweetCardStatus::Error(error.clone());
                self.data = None;
            }
            ClientTweetCardAction::Reset => {
                self.status = ClientTweetCardStatus::Idle;
                self.data = None;
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum ClientTweetCardAction {
    Loading,
    Loaded(Box<TweetCardData>),
    Failed(String),
    Reset,
}

#[derive(Clone, Debug)]
pub struct ClientTweetCardProps {
    pub width: Length,
}

impl Default for ClientTweetCardProps {
    fn default() -> Self {
        Self {
            width: Length::Fixed(540.0),
        }
    }
}

impl ClientTweetCardProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn tweet_card<'a, Message: 'a>(
    data: &TweetCardData,
    props: &TweetCardProps,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let accent = props.accent_color.unwrap_or(theme.palette.primary);
    let media = data
        .media
        .as_ref()
        .map(|media| render_media(media, props.media_height, accent, theme))
        .unwrap_or_else(|| iced::widget::Space::new().height(Length::Shrink).into());

    let body = column![
        tweet_header(&data.author, data.timestamp.as_deref(), theme),
        text(data.text.clone()).size(15),
        media,
    ]
    .spacing(16)
    .width(Length::Fill);

    container(card(body, CardProps::new().size(CardSize::Size3), theme))
        .width(props.width)
        .into()
}

pub fn client_tweet_card<'a, Message: 'a>(
    state: &ClientTweetCardState,
    props: &ClientTweetCardProps,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let body: Element<'a, Message> = match (&state.status, state.data.as_ref()) {
        (ClientTweetCardStatus::Ready, Some(data)) => tweet_card(
            data,
            &TweetCardProps {
                width: props.width,
                ..TweetCardProps::new()
            },
            theme,
        ),
        (ClientTweetCardStatus::Error(error), _) => error_card(error, props.width, theme),
        (ClientTweetCardStatus::Loading, _) => skeleton_card(props.width, theme),
        _ => skeleton_card(props.width, theme),
    };

    body
}

fn tweet_header<'a, Message: 'a>(
    author: &TweetAuthor,
    timestamp: Option<&str>,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let timestamp_line: Element<'a, Message> = text(timestamp.unwrap_or_default().to_string())
        .size(12)
        .into();

    let avatar: Element<'a, Message> = if let Some(asset) = &author.avatar {
        if let Some(handle) = &asset.handle {
            image(handle.clone())
                .width(Length::Fixed(48.0))
                .height(Length::Fixed(48.0))
                .into()
        } else {
            avatar_placeholder(&author.name, theme)
        }
    } else {
        avatar_placeholder(&author.name, theme)
    };

    row![
        avatar,
        column![
            row![
                text(author.name.clone()).size(16),
                if author.verified {
                    text("●").size(12).style({
                        let color = theme.palette.primary;
                        move |_| iced::widget::text::Style { color: Some(color) }
                    })
                } else {
                    text("").size(1)
                }
            ]
            .spacing(6)
            .align_y(Alignment::Center),
            text(format!("@{}", author.handle)).size(13).style({
                let color = theme.palette.muted_foreground;
                move |_| iced::widget::text::Style { color: Some(color) }
            }),
            timestamp_line,
        ]
        .spacing(2),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

fn avatar_placeholder<'a, Message: 'a>(name: &str, theme: &'a Theme) -> Element<'a, Message> {
    let initials = name
        .split_whitespace()
        .take(2)
        .filter_map(|part| part.chars().next())
        .collect::<String>();

    container(text(initials).size(18))
        .width(Length::Fixed(48.0))
        .height(Length::Fixed(48.0))
        .center_x(Length::Shrink)
        .center_y(Length::Shrink)
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(Color {
                a: 0.12,
                ..theme.palette.primary
            })),
            text_color: Some(theme.palette.foreground),
            border: iced::Border {
                color: Color {
                    a: 0.16,
                    ..theme.palette.primary
                },
                width: 1.0,
                radius: 24.0.into(),
            },
            ..iced::widget::container::Style::default()
        })
        .into()
}

fn render_media<'a, Message: 'a>(
    media: &TweetMedia,
    media_height: f32,
    accent: Color,
    theme: &'a Theme,
) -> Element<'a, Message> {
    match media {
        TweetMedia::Images(items) => {
            let cards = items
                .iter()
                .map(|asset| media_tile(asset, media_height, accent, theme))
                .collect::<Vec<_>>();
            scrollable(row(cards).spacing(12))
                .width(Length::Fill)
                .height(Length::Fixed(media_height))
                .into()
        }
        TweetMedia::VideoPoster { poster, label } => poster
            .as_ref()
            .map(|asset| media_tile_with_label(asset, label, media_height, accent, theme))
            .unwrap_or_else(|| media_placeholder(label, media_height, accent, theme)),
        TweetMedia::LinkPreview {
            title,
            site,
            thumbnail,
            url,
        } => {
            let thumb = thumbnail
                .as_ref()
                .map(|asset| media_tile(asset, media_height, accent, theme))
                .unwrap_or_else(|| media_placeholder(site, media_height, accent, theme));

            row![
                thumb,
                column![
                    text(title.clone()).size(16),
                    text(site.clone()).size(13).style({
                        let color = theme.palette.muted_foreground;
                        move |_| iced::widget::text::Style { color: Some(color) }
                    }),
                    text(url.clone()).size(12),
                ]
                .spacing(8)
                .width(Length::Fill)
            ]
            .spacing(12)
            .into()
        }
    }
}

fn media_tile<'a, Message: 'a>(
    asset: &TweetAsset,
    media_height: f32,
    accent: Color,
    theme: &'a Theme,
) -> Element<'a, Message> {
    asset.handle.as_ref().map_or_else(
        || media_placeholder(&asset.alt, media_height, accent, theme),
        |handle| {
            container(
                image(handle.clone())
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .width(Length::Fixed(media_height * 1.2))
            .height(Length::Fixed(media_height))
            .clip(true)
            .into()
        },
    )
}

fn media_tile_with_label<'a, Message: 'a>(
    asset: &TweetAsset,
    label: &str,
    media_height: f32,
    accent: Color,
    theme: &'a Theme,
) -> Element<'a, Message> {
    column![
        media_tile(asset, media_height, accent, theme),
        text(label.to_string()).size(12)
    ]
    .spacing(8)
    .into()
}

fn media_placeholder<'a, Message: 'a>(
    label: &str,
    media_height: f32,
    accent: Color,
    theme: &'a Theme,
) -> Element<'a, Message> {
    container(text(label.to_string()).size(14))
        .width(Length::Fixed(media_height * 1.2))
        .height(Length::Fixed(media_height))
        .center_x(Length::Shrink)
        .center_y(Length::Shrink)
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(Color { a: 0.08, ..accent })),
            text_color: Some(theme.palette.foreground),
            border: iced::Border {
                color: Color { a: 0.2, ..accent },
                width: 1.0,
                radius: theme.radius.lg.into(),
            },
            ..iced::widget::container::Style::default()
        })
        .into()
}

fn skeleton_card<'a, Message: 'a>(width: Length, theme: &'a Theme) -> Element<'a, Message> {
    container(card(
        column![
            text("Loading tweet…").size(14),
            container(iced::widget::Space::new())
                .width(Length::Fill)
                .height(Length::Fixed(96.0))
                .style(move |_| iced::widget::container::Style {
                    background: Some(Background::Color(Color {
                        a: 0.08,
                        ..theme.palette.primary
                    })),
                    ..iced::widget::container::Style::default()
                }),
        ]
        .spacing(12),
        CardProps::new().size(CardSize::Size3),
        theme,
    ))
    .width(width)
    .into()
}

fn error_card<'a, Message: 'a>(
    error: &str,
    width: Length,
    theme: &'a Theme,
) -> Element<'a, Message> {
    container(card(
        column![
            text("Tweet unavailable").size(16),
            text(error.to_string()).size(13)
        ]
        .spacing(8),
        CardProps::new().size(CardSize::Size3),
        theme,
    ))
    .width(width)
    .into()
}

#[cfg(feature = "tweet-adapter")]
#[allow(dead_code)]
pub mod adapter {
    use super::*;
    use reqwest::StatusCode;
    use serde::Deserialize;

    #[derive(Debug)]
    pub enum TweetAdapterError {
        Request(reqwest::Error),
        UnexpectedStatus(StatusCode),
    }

    impl std::fmt::Display for TweetAdapterError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Request(error) => write!(f, "{error}"),
                Self::UnexpectedStatus(status) => write!(f, "unexpected status: {status}"),
            }
        }
    }

    impl std::error::Error for TweetAdapterError {}

    #[derive(Debug, Deserialize)]
    struct RawTweetAsset {
        alt: Option<String>,
        source_url: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    struct RawTweetAuthor {
        name: String,
        handle: String,
        verified: bool,
        avatar: Option<RawTweetAsset>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(tag = "kind", rename_all = "snake_case")]
    enum RawTweetMedia {
        Images {
            items: Vec<RawTweetAsset>,
        },
        VideoPoster {
            poster: Option<RawTweetAsset>,
            label: String,
        },
        LinkPreview {
            title: String,
            site: String,
            thumbnail: Option<RawTweetAsset>,
            url: String,
        },
    }

    #[derive(Debug, Deserialize)]
    struct RawTweetCard {
        id: String,
        text: String,
        author: RawTweetAuthor,
        url: Option<String>,
        timestamp: Option<String>,
        media: Option<RawTweetMedia>,
    }

    pub async fn fetch_tweet_card_data(endpoint: &str) -> Result<TweetCardData, TweetAdapterError> {
        let response = reqwest::get(endpoint)
            .await
            .map_err(TweetAdapterError::Request)?;
        if !response.status().is_success() {
            return Err(TweetAdapterError::UnexpectedStatus(response.status()));
        }

        let raw = response
            .json::<RawTweetCard>()
            .await
            .map_err(TweetAdapterError::Request)?;

        Ok(TweetCardData {
            id: raw.id,
            text: raw.text,
            author: TweetAuthor {
                name: raw.author.name,
                handle: raw.author.handle,
                avatar: raw.author.avatar.map(map_asset),
                verified: raw.author.verified,
            },
            url: raw.url,
            timestamp: raw.timestamp,
            media: raw.media.map(map_media),
        })
    }

    fn map_asset(asset: RawTweetAsset) -> TweetAsset {
        TweetAsset {
            alt: asset.alt.unwrap_or_else(|| "tweet asset".into()),
            handle: None,
            source_url: asset.source_url,
        }
    }

    fn map_media(media: RawTweetMedia) -> TweetMedia {
        match media {
            RawTweetMedia::Images { items } => {
                TweetMedia::Images(items.into_iter().map(map_asset).collect())
            }
            RawTweetMedia::VideoPoster { poster, label } => TweetMedia::VideoPoster {
                poster: poster.map(map_asset),
                label,
            },
            RawTweetMedia::LinkPreview {
                title,
                site,
                thumbnail,
                url,
            } => TweetMedia::LinkPreview {
                title,
                site,
                thumbnail: thumbnail.map(map_asset),
                url,
            },
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn adapter_maps_remote_shape_into_core_shape() {
            let media = map_media(RawTweetMedia::VideoPoster {
                poster: Some(RawTweetAsset {
                    alt: Some("poster".into()),
                    source_url: Some("https://example.com/poster.png".into()),
                }),
                label: "Demo".into(),
            });

            match media {
                TweetMedia::VideoPoster { poster, label } => {
                    assert_eq!(label, "Demo");
                    assert!(poster.is_some());
                }
                _ => panic!("expected video poster"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_data() -> TweetCardData {
        TweetCardData {
            id: "1".into(),
            text: "Shipping phase 5 without fake browser semantics.".into(),
            author: TweetAuthor {
                name: "FerrisMind".into(),
                handle: "ferrismind".into(),
                avatar: None,
                verified: true,
            },
            url: None,
            timestamp: Some("2026-04-13".into()),
            media: None,
        }
    }

    #[test]
    fn client_tweet_card_state_tracks_loading_ready_and_error() {
        let mut state = ClientTweetCardState::new();
        state.apply_action(&ClientTweetCardAction::Loading);
        assert_eq!(state.status, ClientTweetCardStatus::Loading);

        state.apply_action(&ClientTweetCardAction::Loaded(Box::new(sample_data())));
        assert_eq!(state.status, ClientTweetCardStatus::Ready);
        assert!(state.data.is_some());

        state.apply_action(&ClientTweetCardAction::Failed("boom".into()));
        assert_eq!(state.status, ClientTweetCardStatus::Error("boom".into()));
    }
}
