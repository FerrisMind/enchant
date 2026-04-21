use crate::primitives::{AnimatedValue, MotionSpec};
use iced::Background;
use iced::Color;
use iced::Element;
use iced::Length;
use iced::Point;
use iced::Rectangle;
use iced::Size;
use iced::Vector;
use iced::time::Instant;
use iced::widget::{center, container, float, row, stack, text};
use iced_shadcn::{DialogProps, Theme, dialog};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HeroVideoAnimationStyle {
    FromBottom,
    #[default]
    FromCenter,
    FromTop,
    FromLeft,
    FromRight,
    Fade,
    TopInBottomOut,
    LeftInRightOut,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeroVideoDialogProps {
    pub animation_style: HeroVideoAnimationStyle,
    pub thumbnail_radius: f32,
    pub dialog_max_width: f32,
    pub overlay_opacity: f32,
    pub content_padding: f32,
    pub motion: MotionSpec,
    pub show_close_button: bool,
}

impl Default for HeroVideoDialogProps {
    fn default() -> Self {
        Self {
            animation_style: HeroVideoAnimationStyle::FromCenter,
            thumbnail_radius: 20.0,
            dialog_max_width: 960.0,
            overlay_opacity: 0.7,
            content_padding: 16.0,
            motion: MotionSpec::new()
                .duration(twill::prelude::TransitionDuration::Ms300)
                .easing(twill::prelude::Easing::Out),
            show_close_button: true,
        }
    }
}

impl HeroVideoDialogProps {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug)]
pub struct HeroVideoDialogState {
    open: bool,
    present: bool,
    animation: AnimatedValue,
}

impl Default for HeroVideoDialogState {
    fn default() -> Self {
        Self {
            open: false,
            present: false,
            animation: AnimatedValue::new(0.0),
        }
    }
}

impl HeroVideoDialogState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_open(&mut self, open: bool, now: Instant, props: &HeroVideoDialogProps) {
        self.open = open;
        self.animation.set_spec(props.motion);
        if open {
            self.present = true;
        }
        self.animation.transition(if open { 1.0 } else { 0.0 }, now);
    }

    pub fn tick(&mut self, now: Instant) {
        if self.present
            && !self.open
            && !self.animation.is_animating(now)
            && self.progress(now) <= 0.001
        {
            self.present = false;
        }
    }

    pub fn progress(&self, now: Instant) -> f32 {
        self.animation.value_at(now)
    }

    pub fn is_present(&self) -> bool {
        self.present
    }
}

#[allow(clippy::too_many_arguments)]
pub fn hero_video_dialog<'a, Message: Clone + 'a>(
    state: &HeroVideoDialogState,
    now: Instant,
    thumbnail: impl Into<Element<'a, Message>>,
    content: impl Into<Element<'a, Message>>,
    on_open: Message,
    on_close: Message,
    props: HeroVideoDialogProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let thumbnail = hero_thumbnail(thumbnail.into(), on_open, props, theme);
    if !state.is_present() {
        return thumbnail;
    }

    let progress = state.progress(now);
    let target_open = state.open;
    let animated = animated_dialog_content(
        content.into(),
        progress,
        target_open,
        props,
        on_close.clone(),
    );

    dialog(
        thumbnail,
        state.is_present(),
        center(animated),
        on_close,
        dialog_props(props),
        theme,
    )
}

fn hero_thumbnail<'a, Message: Clone + 'a>(
    thumbnail: Element<'a, Message>,
    on_open: Message,
    props: HeroVideoDialogProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let poster = container(thumbnail).clip(true).style({
        let border = theme.palette.border;
        let radius = props.thumbnail_radius;
        move |_| iced::widget::container::Style {
            border: iced::Border {
                color: border,
                width: 1.0,
                radius: radius.into(),
            },
            ..iced::widget::container::Style::default()
        }
    });

    let play = center(
        container(text("▶").size(36))
            .width(Length::Fixed(88.0))
            .height(Length::Fixed(88.0))
            .style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.14))),
                border: iced::Border {
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.24),
                    width: 1.0,
                    radius: 9999.0.into(),
                },
                ..iced::widget::container::Style::default()
            }),
    );

    iced::widget::button(stack![poster, play])
        .padding(0)
        .style(iced::widget::button::text)
        .on_press(on_open)
        .into()
}

fn animated_dialog_content<'a, Message: Clone + 'a>(
    content: Element<'a, Message>,
    progress: f32,
    target_open: bool,
    props: HeroVideoDialogProps,
    on_close: Message,
) -> Element<'a, Message> {
    let scale = animation_scale(props.animation_style, progress);
    let offset = animation_offset(
        props.animation_style,
        progress,
        target_open,
        Size::new(props.dialog_max_width, props.dialog_max_width * 0.56),
    );

    let close_row: Element<'a, Message> = if props.show_close_button {
        row![
            iced::widget::Space::new().width(Length::Fill),
            iced::widget::button(text("×").size(20))
                .padding([4.0, 10.0])
                .style(iced::widget::button::text)
                .on_press(on_close)
        ]
        .into()
    } else {
        iced::widget::Space::new().height(Length::Fixed(0.0)).into()
    };

    let card = container(iced::widget::column![close_row, content].spacing(8))
        .width(Length::Shrink)
        .height(Length::Shrink);

    float(card)
        .scale(scale)
        .translate(move |_bounds: Rectangle, _viewport: Rectangle| Vector::new(offset.x, offset.y))
        .into()
}

fn dialog_props(props: HeroVideoDialogProps) -> DialogProps {
    DialogProps::new()
        .max_width(props.dialog_max_width.round().max(1.0) as u32)
        .overlay_opacity(props.overlay_opacity)
        .padding(props.content_padding.round().max(0.0) as u16)
}

fn animation_scale(style: HeroVideoAnimationStyle, progress: f32) -> f32 {
    let progress = progress.clamp(0.0, 1.0);
    match style {
        HeroVideoAnimationStyle::FromCenter => 0.85 + progress * 0.15,
        HeroVideoAnimationStyle::Fade => 1.0,
        _ => 0.96 + progress * 0.04,
    }
}

fn animation_offset(
    style: HeroVideoAnimationStyle,
    progress: f32,
    target_open: bool,
    viewport: Size,
) -> Point {
    let progress = progress.clamp(0.0, 1.0);
    let x = viewport.width * 0.12 * (1.0 - progress);
    let y = viewport.height * 0.12 * (1.0 - progress);

    match style {
        HeroVideoAnimationStyle::FromBottom => Point::new(0.0, y),
        HeroVideoAnimationStyle::FromTop => Point::new(0.0, -y),
        HeroVideoAnimationStyle::FromLeft => Point::new(-x, 0.0),
        HeroVideoAnimationStyle::FromRight => Point::new(x, 0.0),
        HeroVideoAnimationStyle::TopInBottomOut => {
            Point::new(0.0, if target_open { -y } else { y })
        }
        HeroVideoAnimationStyle::LeftInRightOut => {
            Point::new(if target_open { -x } else { x }, 0.0)
        }
        HeroVideoAnimationStyle::Fade | HeroVideoAnimationStyle::FromCenter => Point::ORIGIN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::time::Duration;

    #[test]
    fn hero_dialog_state_opens_and_closes_with_presence() {
        let props = HeroVideoDialogProps::new();
        let now = Instant::now();
        let mut state = HeroVideoDialogState::new();

        state.set_open(true, now, &props);
        assert!(state.is_present());

        state.set_open(false, now + Duration::from_millis(400), &props);
        state.tick(now + Duration::from_secs(1));
        assert!(!state.is_present());
    }

    #[test]
    fn animation_offset_maps_styles_to_expected_axes() {
        let size = Size::new(800.0, 600.0);
        let from_bottom = animation_offset(HeroVideoAnimationStyle::FromBottom, 0.5, true, size);
        let from_left = animation_offset(HeroVideoAnimationStyle::FromLeft, 0.5, true, size);
        let top_out = animation_offset(HeroVideoAnimationStyle::TopInBottomOut, 0.0, false, size);

        assert!(from_bottom.y > 0.0);
        assert!(from_left.x < 0.0);
        assert!(top_out.y > 0.0);
    }

    #[test]
    fn hero_dialog_props_map_to_generic_dialog_shell() {
        let dialog = dialog_props(HeroVideoDialogProps {
            dialog_max_width: 720.0,
            overlay_opacity: 0.45,
            content_padding: 24.0,
            ..HeroVideoDialogProps::new()
        });

        assert_eq!(dialog.max_width, 720);
        assert_eq!(dialog.overlay_opacity, 0.45);
        assert_eq!(dialog.padding, Some(24));
    }
}
