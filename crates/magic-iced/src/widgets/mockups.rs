use crate::primitives::{MockupFrameKind, MockupFrameProps, mockup_frame_with_props};
use iced::Element;
use iced::Length;
use iced::widget::{container, space};
use iced_shadcn::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SafariMode {
    #[default]
    Default,
    Simple,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SafariProps {
    pub width: Length,
    pub chrome_label: Option<&'static str>,
    pub mode: SafariMode,
    pub screen_media: bool,
    pub media_bytes: Option<&'static [u8]>,
    pub media_mime: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IphoneProps {
    pub width: Length,
    pub chrome_label: Option<&'static str>,
    pub screen_media: bool,
    pub media_bytes: Option<&'static [u8]>,
    pub media_mime: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AndroidProps {
    pub width: Length,
    pub chrome_label: Option<&'static str>,
    pub screen_media: bool,
    pub media_bytes: Option<&'static [u8]>,
    pub media_mime: Option<&'static str>,
}

impl Default for SafariProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            chrome_label: None,
            mode: SafariMode::Default,
            screen_media: false,
            media_bytes: None,
            media_mime: None,
        }
    }
}

impl Default for IphoneProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            chrome_label: None,
            screen_media: false,
            media_bytes: None,
            media_mime: None,
        }
    }
}

impl Default for AndroidProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            chrome_label: None,
            screen_media: false,
            media_bytes: None,
            media_mime: None,
        }
    }
}

impl SafariProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn chrome_label(mut self, chrome_label: &'static str) -> Self {
        self.chrome_label = Some(chrome_label);
        self
    }

    pub fn mode(mut self, mode: SafariMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn screen_media(mut self, screen_media: bool) -> Self {
        self.screen_media = screen_media;
        self
    }

    pub fn media(mut self, bytes: &'static [u8], mime: &'static str) -> Self {
        self.media_bytes = Some(bytes);
        self.media_mime = Some(mime);
        self
    }

    pub const fn frame_kind(&self) -> MockupFrameKind {
        MockupFrameKind::Safari
    }
}

impl IphoneProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn chrome_label(mut self, chrome_label: &'static str) -> Self {
        self.chrome_label = Some(chrome_label);
        self
    }

    pub fn screen_media(mut self, screen_media: bool) -> Self {
        self.screen_media = screen_media;
        self
    }

    pub fn media(mut self, bytes: &'static [u8], mime: &'static str) -> Self {
        self.media_bytes = Some(bytes);
        self.media_mime = Some(mime);
        self
    }

    pub const fn frame_kind(&self) -> MockupFrameKind {
        MockupFrameKind::Iphone
    }
}

impl AndroidProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn chrome_label(mut self, chrome_label: &'static str) -> Self {
        self.chrome_label = Some(chrome_label);
        self
    }

    pub fn screen_media(mut self, screen_media: bool) -> Self {
        self.screen_media = screen_media;
        self
    }

    pub fn media(mut self, bytes: &'static [u8], mime: &'static str) -> Self {
        self.media_bytes = Some(bytes);
        self.media_mime = Some(mime);
        self
    }

    pub const fn frame_kind(&self) -> MockupFrameKind {
        MockupFrameKind::Android
    }
}

pub fn safari<'a, Message: Clone + 'a + 'static>(
    props: SafariProps,
    theme: &Theme,
) -> Element<'a, Message> {
    container(mockup_frame_with_props(
        props.frame_kind(),
        space().width(Length::Fill).height(Length::Fill),
        &to_frame_props(
            props.chrome_label,
            matches!(props.mode, SafariMode::Simple),
            props.screen_media,
            props.media_bytes,
            props.media_mime,
        ),
        theme,
    ))
    .width(props.width)
    .into()
}

pub fn iphone<'a, Message: Clone + 'a + 'static>(
    props: IphoneProps,
    theme: &Theme,
) -> Element<'a, Message> {
    container(mockup_frame_with_props(
        props.frame_kind(),
        space().width(Length::Fill).height(Length::Fill),
        &to_frame_props(
            props.chrome_label,
            false,
            props.screen_media,
            props.media_bytes,
            props.media_mime,
        ),
        theme,
    ))
    .width(props.width)
    .into()
}

pub fn android<'a, Message: Clone + 'a + 'static>(
    props: AndroidProps,
    theme: &Theme,
) -> Element<'a, Message> {
    container(mockup_frame_with_props(
        props.frame_kind(),
        space().width(Length::Fill).height(Length::Fill),
        &to_frame_props(
            props.chrome_label,
            false,
            props.screen_media,
            props.media_bytes,
            props.media_mime,
        ),
        theme,
    ))
    .width(props.width)
    .into()
}

fn to_frame_props(
    chrome_label: Option<&'static str>,
    safari_simple: bool,
    screen_media: bool,
    media_bytes: Option<&'static [u8]>,
    media_mime: Option<&'static str>,
) -> MockupFrameProps {
    let mut props = MockupFrameProps::new().screen_media(screen_media);

    if let Some(chrome_label) = chrome_label {
        props = props.chrome_label(chrome_label);
    }

    if safari_simple {
        props = props.safari_simple(true);
    }

    if let (Some(bytes), Some(mime)) = (media_bytes, media_mime) {
        props = props.media(bytes, mime);
    }

    props
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mockup_wrappers_map_to_expected_kinds() {
        assert_eq!(SafariProps::new().frame_kind(), MockupFrameKind::Safari);
        assert_eq!(IphoneProps::new().frame_kind(), MockupFrameKind::Iphone);
        assert_eq!(AndroidProps::new().frame_kind(), MockupFrameKind::Android);
    }

    #[test]
    fn mockup_wrappers_forward_media_bytes() {
        let bytes = b"image";
        let props = IphoneProps::new()
            .screen_media(true)
            .media(bytes, "image/png");
        let frame = to_frame_props(
            props.chrome_label,
            false,
            props.screen_media,
            props.media_bytes,
            props.media_mime,
        );

        assert_eq!(frame.media_bytes, Some(bytes.as_slice()));
        assert_eq!(frame.media_mime, Some("image/png"));
    }
}
