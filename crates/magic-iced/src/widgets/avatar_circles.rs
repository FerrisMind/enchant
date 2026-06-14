use std::borrow::Cow;

use iced::widget::{container, image, row, stack, text, Space};
use iced::alignment::{Horizontal, Vertical};
use iced::{Border, Color, ContentFit, Element, Length, Padding};
use iced::border::Radius;
use iced_shadcn::{AvatarProps, AvatarSize, AvatarVariant, Theme, avatar};

#[derive(Clone, Debug)]
pub struct AvatarCircleEntry<'a> {
    pub label: Cow<'a, str>,
    pub image: Option<image::Handle>,
    pub color: Option<Color>,
    pub variant: AvatarVariant,
}

impl<'a> AvatarCircleEntry<'a> {
    pub fn image(handle: image::Handle) -> Self {
        Self {
            label: Cow::Borrowed(""),
            image: Some(handle),
            color: None,
            variant: AvatarVariant::Solid,
        }
    }

    pub fn new(label: impl Into<Cow<'a, str>>) -> Self {
        Self {
            label: label.into(),
            image: None,
            color: None,
            variant: AvatarVariant::Solid,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn variant(mut self, variant: AvatarVariant) -> Self {
        self.variant = variant;
        self
    }
}

#[derive(Clone, Debug)]
pub struct AvatarCirclesProps<'a> {
    pub entries: Vec<AvatarCircleEntry<'a>>,
    pub extra_count: usize,
    pub size: AvatarSize,
    /// Reference `flex -space-x-4` on `size-10` avatars → 16px overlap.
    pub overlap: f32,
    /// Reference `border-2`.
    pub border_width: f32,
}

impl Default for AvatarCirclesProps<'_> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            extra_count: 0,
            size: AvatarSize::Size5,
            overlap: 16.0,
            border_width: 2.0,
        }
    }
}

impl<'a> AvatarCirclesProps<'a> {
    pub fn new<I>(entries: I) -> Self
    where
        I: IntoIterator<Item = AvatarCircleEntry<'a>>,
    {
        Self {
            entries: entries.into_iter().collect(),
            ..Self::default()
        }
    }

    pub fn extra_count(mut self, extra_count: usize) -> Self {
        self.extra_count = extra_count;
        self
    }

    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    pub fn overlap(mut self, overlap: f32) -> Self {
        self.overlap = overlap.max(0.0);
        self
    }

    pub fn border_width(mut self, border_width: f32) -> Self {
        self.border_width = border_width.max(0.0);
        self
    }
}

pub fn avatar_circles<'a, Message: Clone + 'a>(
    props: &'a AvatarCirclesProps<'a>,
    theme: &'a Theme,
) -> Element<'a, Message> {
    if props.entries.is_empty() && props.extra_count == 0 {
        return iced::widget::Space::new().into();
    }

    let avatar_px = avatar_size_px(props.size);
    let border_color = avatar_border_color(theme);
    let step = (avatar_px - props.overlap).max(1.0);

    let avatar_items: Vec<Element<'a, Message>> = props
        .entries
        .iter()
        .map(|entry| circle_avatar(entry, props, theme, border_color))
        .collect();

    if props.extra_count == 0 {
        return row(avatar_items)
            .spacing(-props.overlap)
            .align_y(Vertical::Center)
            .into();
    }

    let chip_offset = props.entries.len() as f32 * step;
    let total_w = total_width(props.entries.len(), true, avatar_px, step);
    let chip = extra_chip(
        props.extra_count,
        avatar_px,
        props.border_width,
        theme,
    );

    // Counter in a higher stack layer (above avatar border rings), positioned via padding.
    stack![
        row(avatar_items)
            .spacing(-props.overlap)
            .align_y(Vertical::Center),
        container(chip)
            .width(Length::Fixed(total_w))
            .height(Length::Fixed(avatar_px))
            .padding(Padding::ZERO.left(chip_offset))
            .align_x(Horizontal::Left)
            .align_y(Vertical::Center),
    ]
    .width(Length::Fixed(total_w))
    .height(Length::Fixed(avatar_px))
    .into()
}

fn circle_avatar<'a, Message: Clone + 'a>(
    entry: &'a AvatarCircleEntry<'a>,
    props: &'a AvatarCirclesProps<'a>,
    theme: &Theme,
    border_color: Color,
) -> Element<'a, Message> {
    let avatar_px = avatar_size_px(props.size);
    let border_width = props.border_width;
    let content_px = (avatar_px - border_width * 2.0).max(0.0);

    let (inner, inner_px) = if let Some(handle) = entry.image.clone() {
        (
            image(handle)
                .width(Length::Fixed(content_px))
                .height(Length::Fixed(content_px))
                .content_fit(ContentFit::Cover)
                .border_radius(Radius::from(content_px / 2.0))
                .into(),
            content_px,
        )
    } else {
        let mut avatar_props = AvatarProps::new(entry.label.as_ref())
            .size(props.size)
            .variant(entry.variant);
        if let Some(color) = entry.color {
            avatar_props = avatar_props.color(color);
        }
        (avatar(avatar_props, theme), avatar_px)
    };

    bordered_circle(inner, avatar_px, inner_px, border_color, border_width)
}

fn bordered_circle<'a, Message: 'a>(
    inner: Element<'a, Message>,
    diameter: f32,
    inner_px: f32,
    border_color: Color,
    border_width: f32,
) -> Element<'a, Message> {
    let outer_radius = Radius::from(diameter / 2.0);

    let content = container(inner)
        .width(Length::Fixed(inner_px))
        .height(Length::Fixed(inner_px))
        .center_x(Length::Fill)
        .center_y(Length::Fill);

    let ring = container(Space::new())
        .width(Length::Fixed(diameter))
        .height(Length::Fixed(diameter))
        .style(move |_| iced::widget::container::Style {
            border: Border {
                color: border_color,
                width: border_width,
                radius: outer_radius,
            },
            ..iced::widget::container::Style::default()
        });

    stack![
        container(Space::new())
            .width(Length::Fixed(diameter))
            .height(Length::Fixed(diameter)),
        content,
        ring,
    ]
    .width(Length::Fixed(diameter))
    .height(Length::Fixed(diameter))
    .into()
}

fn extra_chip<'a, Message: 'a>(
    extra_count: usize,
    avatar_px: f32,
    border_width: f32,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let background = theme.palette.foreground;
    let text_color = theme.palette.background;
    let border_color = avatar_border_color(theme);
    let outer_radius = Radius::from(avatar_px / 2.0);

    // Background + border in one quad so outer anti-aliasing uses border_color,
    // not a separate black fill halo (see iced solid.wgsl quad_alpha).
    container(text(format!("+{extra_count}")).size((avatar_px * 0.34).max(10.0)))
        .width(Length::Fixed(avatar_px))
        .height(Length::Fixed(avatar_px))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(move |_| iced::widget::container::Style {
            background: Some(iced::Background::Color(background)),
            text_color: Some(text_color),
            border: Border {
                color: border_color,
                width: border_width,
                radius: outer_radius,
            },
            snap: true,
            ..iced::widget::container::Style::default()
        })
        .into()
}

fn avatar_border_color(theme: &Theme) -> Color {
    // Reference: `border-white dark:border-gray-800` on each avatar/chip.
    if theme.palette.background.relative_luminance() < 0.5 {
        Color::from_rgb8(0x1f, 0x29, 0x37)
    } else {
        Color::WHITE
    }
}

fn avatar_size_px(size: AvatarSize) -> f32 {
    match size {
        AvatarSize::Size1 => 16.0,
        AvatarSize::Size2 => 20.0,
        AvatarSize::Size3 => 24.0,
        AvatarSize::Size4 => 32.0,
        AvatarSize::Size5 => 40.0,
        AvatarSize::Size6 => 48.0,
        AvatarSize::Size7 => 64.0,
        AvatarSize::Size8 => 80.0,
        AvatarSize::Size9 => 96.0,
    }
}

fn total_width(count: usize, has_extra_chip: bool, avatar_px: f32, step: f32) -> f32 {
    let items = count + usize::from(has_extra_chip);
    if items == 0 {
        0.0
    } else {
        avatar_px + step * items.saturating_sub(1) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avatar_circles_defaults_match_reference() {
        let props = AvatarCirclesProps::default();

        assert_eq!(props.size, AvatarSize::Size5);
        assert_eq!(props.overlap, 16.0);
        assert_eq!(props.border_width, 2.0);
        assert_eq!(props.extra_count, 0);
    }

    #[test]
    fn total_width_accounts_for_overlap_and_extra_chip() {
        let avatar_px = avatar_size_px(AvatarSize::Size5);
        let step = avatar_px - 16.0;

        assert_eq!(total_width(0, false, avatar_px, step), 0.0);
        assert_eq!(total_width(1, false, avatar_px, step), avatar_px);
        assert_eq!(
            total_width(2, true, avatar_px, step),
            avatar_px + step * 2.0
        );
        assert_eq!(
            total_width(6, true, avatar_px, step),
            avatar_px + step * 6.0
        );
    }
}
