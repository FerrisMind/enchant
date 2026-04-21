use crate::primitives::{MockupFrameKind, MockupFrameProps, mockup_frame_with_props};
use iced::Element;
use iced::Length;
use iced::widget::container;
use iced_shadcn::Theme;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SafariProps {
    pub width: Length,
    pub chrome_label: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IphoneProps {
    pub width: Length,
    pub chrome_label: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AndroidProps {
    pub width: Length,
    pub chrome_label: Option<&'static str>,
}

impl Default for SafariProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            chrome_label: None,
        }
    }
}

impl Default for IphoneProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            chrome_label: None,
        }
    }
}

impl Default for AndroidProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            chrome_label: None,
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

    pub const fn frame_kind(&self) -> MockupFrameKind {
        MockupFrameKind::Android
    }
}

pub fn safari<'a, Message: Clone + 'a + 'static>(
    content: impl Into<Element<'a, Message>>,
    props: SafariProps,
    theme: &Theme,
) -> Element<'a, Message> {
    container(mockup_frame_with_props(
        props.frame_kind(),
        content,
        &frame_props(props.chrome_label),
        theme,
    ))
    .width(props.width)
    .into()
}

pub fn iphone<'a, Message: Clone + 'a + 'static>(
    content: impl Into<Element<'a, Message>>,
    props: IphoneProps,
    theme: &Theme,
) -> Element<'a, Message> {
    container(mockup_frame_with_props(
        props.frame_kind(),
        content,
        &frame_props(props.chrome_label),
        theme,
    ))
    .width(props.width)
    .into()
}

pub fn android<'a, Message: Clone + 'a + 'static>(
    content: impl Into<Element<'a, Message>>,
    props: AndroidProps,
    theme: &Theme,
) -> Element<'a, Message> {
    container(mockup_frame_with_props(
        props.frame_kind(),
        content,
        &frame_props(props.chrome_label),
        theme,
    ))
    .width(props.width)
    .into()
}

fn frame_props(chrome_label: Option<&'static str>) -> MockupFrameProps {
    let mut props = MockupFrameProps::new();

    if let Some(chrome_label) = chrome_label {
        props = props.chrome_label(chrome_label);
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
    fn mockup_wrappers_accept_generic_chrome_labels() {
        assert_eq!(
            SafariProps::new()
                .chrome_label("workspace.local")
                .chrome_label,
            Some("workspace.local")
        );
        assert_eq!(
            IphoneProps::new().chrome_label("preview").chrome_label,
            Some("preview")
        );
        assert_eq!(
            AndroidProps::new()
                .chrome_label("android-demo")
                .chrome_label,
            Some("android-demo")
        );
    }
}
