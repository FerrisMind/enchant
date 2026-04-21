use std::borrow::Cow;

use iced::widget::{container, float, stack, text};
use iced::{Color, Element, Length, Vector};
use iced_shadcn::{AvatarProps, AvatarSize, AvatarVariant, Theme, avatar};

#[derive(Clone, Debug, PartialEq)]
pub struct AvatarCircleEntry<'a> {
    pub label: Cow<'a, str>,
    pub color: Option<Color>,
    pub variant: AvatarVariant,
}

impl<'a> AvatarCircleEntry<'a> {
    pub fn new(label: impl Into<Cow<'a, str>>) -> Self {
        Self {
            label: label.into(),
            color: None,
            variant: AvatarVariant::Soft,
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

#[derive(Clone, Debug, PartialEq)]
pub struct AvatarCirclesProps<'a> {
    pub entries: Vec<AvatarCircleEntry<'a>>,
    pub extra_count: usize,
    pub size: AvatarSize,
    pub overlap: f32,
}

impl Default for AvatarCirclesProps<'_> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            extra_count: 0,
            size: AvatarSize::Size5,
            overlap: 12.0,
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
}

pub fn avatar_circles<'a, Message: Clone + 'a>(
    props: &'a AvatarCirclesProps<'a>,
    theme: &'a Theme,
) -> Element<'a, Message> {
    if props.entries.is_empty() && props.extra_count == 0 {
        return iced::widget::Space::new().into();
    }

    let avatar_px = avatar_size_px(props.size);
    let step = (avatar_px - props.overlap).max(1.0);
    let total_width = total_width(props.entries.len(), props.extra_count > 0, avatar_px, step);
    let mut layers = Vec::with_capacity(props.entries.len() + usize::from(props.extra_count > 0));

    for (index, entry) in props.entries.iter().enumerate() {
        let mut avatar_props = AvatarProps::new(entry.label.as_ref())
            .size(props.size)
            .variant(entry.variant);
        if let Some(color) = entry.color {
            avatar_props = avatar_props.color(color);
        }

        let x = index as f32 * step;
        layers.push(
            float(avatar(avatar_props, theme))
                .translate(move |_bounds, _viewport| Vector::new(x, 0.0))
                .into(),
        );
    }

    if props.extra_count > 0 {
        let x = props.entries.len() as f32 * step;
        layers.push(
            float(extra_chip(props.extra_count, avatar_px, theme))
                .translate(move |_bounds, _viewport| Vector::new(x, 0.0))
                .into(),
        );
    }

    container(stack(layers))
        .width(Length::Fixed(total_width))
        .height(Length::Fixed(avatar_px))
        .into()
}

fn extra_chip<'a, Message: 'a>(
    extra_count: usize,
    avatar_px: f32,
    theme: &'a Theme,
) -> Element<'a, Message> {
    container(text(format!("+{extra_count}")).size((avatar_px * 0.34).max(10.0)))
        .width(Length::Fixed(avatar_px))
        .height(Length::Fixed(avatar_px))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style({
            let background = theme.palette.foreground;
            let text_color = theme.palette.background;
            move |_| iced::widget::container::Style {
                background: Some(iced::Background::Color(background)),
                text_color: Some(text_color),
                border: iced::Border {
                    color: theme.palette.background,
                    width: 2.0,
                    radius: (avatar_px / 2.0).into(),
                },
                ..iced::widget::container::Style::default()
            }
        })
        .into()
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
    fn avatar_circles_defaults_are_stable() {
        let props = AvatarCirclesProps::default();

        assert_eq!(props.size, AvatarSize::Size5);
        assert_eq!(props.overlap, 12.0);
        assert_eq!(props.extra_count, 0);
    }

    #[test]
    fn total_width_accounts_for_overlap_and_extra_chip() {
        let avatar_px = avatar_size_px(AvatarSize::Size5);
        let step = avatar_px - 12.0;

        assert_eq!(total_width(0, false, avatar_px, step), 0.0);
        assert_eq!(total_width(1, false, avatar_px, step), avatar_px);
        assert_eq!(
            total_width(2, true, avatar_px, step),
            avatar_px + step * 2.0
        );
    }
}
