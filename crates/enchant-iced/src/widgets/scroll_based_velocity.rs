use crate::primitives::{ScrollSample, marquee_offset};
use iced::widget::{column, container, float, row, text};
use iced::{Alignment, Background, Color, Element, Length, Vector};
use iced_shadcn::Theme;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub enum ScrollVelocityItem {
    Text(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollVelocityDirection {
    Forward,
    Reverse,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollVelocityRowAction {
    ScrollSample(ScrollSample),
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ScrollVelocityRowState {
    velocity_factor: f32,
    last_relative_y: Option<f32>,
}

impl ScrollVelocityRowState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_action(&mut self, action: &ScrollVelocityRowAction) {
        match action {
            ScrollVelocityRowAction::ScrollSample(sample) => {
                let current = sample.relative.y;
                let delta = current - self.last_relative_y.unwrap_or(current);
                self.velocity_factor = (delta * 32.0).clamp(-5.0, 5.0);
                self.last_relative_y = Some(current);
            }
        }
    }

    pub fn tick(&mut self) {
        self.velocity_factor *= 0.92;
        if self.velocity_factor.abs() < 0.01 {
            self.velocity_factor = 0.0;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScrollVelocityContainerProps {
    pub width: Length,
    pub gap: f32,
}

impl Default for ScrollVelocityContainerProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            gap: 12.0,
        }
    }
}

impl ScrollVelocityContainerProps {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScrollVelocityRowProps {
    pub items: Vec<ScrollVelocityItem>,
    pub width: Length,
    pub height: Length,
    pub base_velocity: f32,
    pub direction: ScrollVelocityDirection,
    pub scroll_reactivity: bool,
    pub repeat: usize,
    pub gap: f32,
    pub item_extent: f32,
    pub item_cross: f32,
}

impl Default for ScrollVelocityRowProps {
    fn default() -> Self {
        Self {
            items: vec![ScrollVelocityItem::Text("Velocity Scroll".into())],
            width: Length::Fill,
            height: Length::Fixed(64.0),
            base_velocity: 5.0,
            direction: ScrollVelocityDirection::Forward,
            scroll_reactivity: true,
            repeat: 4,
            gap: 18.0,
            item_extent: 240.0,
            item_cross: 54.0,
        }
    }
}

impl ScrollVelocityRowProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn scroll_velocity_container<'a, Message: 'a>(
    rows: Vec<Element<'a, Message>>,
    props: ScrollVelocityContainerProps,
) -> Element<'a, Message> {
    column(rows).spacing(props.gap).width(props.width).into()
}

pub fn scroll_velocity_row<'a, Message: 'a>(
    state: &ScrollVelocityRowState,
    elapsed: Duration,
    props: &ScrollVelocityRowProps,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let track_extent = scroll_velocity_track_extent(props);
    let velocity_factor = if props.scroll_reactivity {
        state.velocity_factor.abs()
    } else {
        0.0
    };
    let speed = props.base_velocity.max(0.0) * (1.0 + velocity_factor);
    let offset = marquee_offset(
        elapsed,
        speed * props.item_extent / 100.0,
        track_extent.max(1.0),
    );
    let translation = match props.direction {
        ScrollVelocityDirection::Forward => Vector::new(-track_extent + offset, 0.0),
        ScrollVelocityDirection::Reverse => Vector::new(-offset, 0.0),
    };

    let repeats = props.repeat.max(3);
    let items = (0..repeats)
        .flat_map(|_| {
            props
                .items
                .iter()
                .map(|item| render_velocity_item(item, props, theme))
        })
        .collect::<Vec<_>>();

    container(
        float(row(items).spacing(props.gap).align_y(Alignment::Center))
            .translate(move |_bounds, _viewport| translation),
    )
    .width(props.width)
    .height(props.height)
    .clip(true)
    .into()
}

fn render_velocity_item<'a, Message: 'a>(
    item: &ScrollVelocityItem,
    props: &ScrollVelocityRowProps,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let label = match item {
        ScrollVelocityItem::Text(label) => label.clone(),
    };

    container(text(label).size(28))
        .width(Length::Fixed(props.item_extent))
        .height(Length::Fixed(props.item_cross))
        .padding([8.0, 18.0])
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(Color {
                a: 0.08,
                ..theme.palette.primary
            })),
            text_color: Some(theme.palette.foreground),
            border: iced::Border {
                color: Color {
                    a: 0.18,
                    ..theme.palette.primary
                },
                width: 1.0,
                radius: theme.radius.lg.into(),
            },
            ..iced::widget::container::Style::default()
        })
        .into()
}

fn scroll_velocity_track_extent(props: &ScrollVelocityRowProps) -> f32 {
    let count = props.items.len().max(1);
    count as f32 * props.item_extent + count.saturating_sub(1) as f32 * props.gap
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_velocity_defaults_match_docs() {
        let props = ScrollVelocityRowProps::new();

        assert_eq!(props.base_velocity, 5.0);
        assert_eq!(props.direction, ScrollVelocityDirection::Forward);
        assert!(props.scroll_reactivity);
    }

    #[test]
    fn scroll_velocity_state_tracks_scroll_delta() {
        let mut state = ScrollVelocityRowState::new();
        let sample = ScrollSample::new(
            iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(100.0, 100.0)),
            iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(100.0, 400.0)),
            iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: 100.0 },
            iced::widget::scrollable::RelativeOffset { x: 0.0, y: 0.5 },
        );

        state.apply_action(&ScrollVelocityRowAction::ScrollSample(sample));
        assert_eq!(state.velocity_factor, 0.0);
        state.apply_action(&ScrollVelocityRowAction::ScrollSample(ScrollSample::new(
            iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(100.0, 100.0)),
            iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(100.0, 400.0)),
            iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: 150.0 },
            iced::widget::scrollable::RelativeOffset { x: 0.0, y: 0.75 },
        )));
        assert!(state.velocity_factor > 0.0);
    }
}
