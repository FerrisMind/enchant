use crate::primitives::marquee_offset;
use iced::Alignment;
use iced::Color;
use iced::Element;
use iced::Length;
use iced::Radians;
use iced::Rectangle;
use iced::Vector;
use iced::gradient;
use iced::widget::{Space, column, container, float, mouse_area, row, stack};

use iced_shadcn::Theme;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MarqueeOrientation {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MarqueeItem {
    pub title: String,
    pub subtitle: String,
    pub body: String,
    pub accent: Option<Color>,
}

impl MarqueeItem {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: String::new(),
            body: String::new(),
            accent: None,
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = subtitle.into();
        self
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    pub fn accent(mut self, accent: Color) -> Self {
        self.accent = Some(accent);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarqueeProps {
    pub orientation: MarqueeOrientation,
    pub reverse: bool,
    pub pause_on_hover: bool,
    pub repeat: usize,
    pub gap: f32,
    pub speed_px_per_sec: f32,
    pub item_extent: f32,
    pub item_cross: f32,
    pub padding: f32,
    pub edge_fade: bool,
}

impl Default for MarqueeProps {
    fn default() -> Self {
        Self {
            orientation: MarqueeOrientation::Horizontal,
            reverse: false,
            pause_on_hover: false,
            repeat: 4,
            gap: 16.0,
            speed_px_per_sec: 40.0,
            item_extent: 260.0,
            item_cross: 112.0,
            padding: 8.0,
            edge_fade: true,
        }
    }
}

impl MarqueeProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn marquee<'a, Message: 'a + Clone, T>(
    items: &'a [T],
    render_item: impl Fn(&T) -> Element<'a, Message>,
    elapsed: Duration,
    props: MarqueeProps,
    theme: &Theme,
    on_hover: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    let track_extent = marquee_track_extent(items.len(), props);
    let repeats = props.repeat.max(2);

    let (width, height) = match props.orientation {
        MarqueeOrientation::Horizontal => (
            Length::Fill,
            Length::Fixed(props.item_cross + props.padding * 2.0),
        ),
        MarqueeOrientation::Vertical => (
            Length::Fixed(props.item_cross + props.padding * 2.0),
            Length::Fill,
        ),
    };

    let mut layers: Vec<Element<'a, Message>> = Vec::with_capacity(repeats + 1);
    layers.push(
        container(Space::new().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
    );

    for index in 0..repeats {
        let copy = render_copy(items, &render_item, props);
        layers.push(
            float(copy)
                .translate(move |bounds, _viewport| {
                    marquee_copy_position(index, elapsed, items.len(), props, bounds)
                })
                .into(),
        );
    }

    let content = container(stack(layers))
        .width(Length::Fill)
        .height(Length::Fill);

    let frame = container(
        mouse_area(content)
            .on_enter(on_hover(true))
            .on_exit(on_hover(false)),
    )
    .width(width)
    .height(height)
    .padding(props.padding)
    .clip(true);

    if !props.edge_fade || track_extent <= f32::EPSILON {
        return frame.into();
    }

    let overlays = edge_fades(props, theme);

    // Stack overlays on top but ensure they are correctly sized
    container(stack(vec![frame.into(), overlays]))
        .width(width)
        .height(height)
        .into()
}

fn render_copy<'a, Message: 'a, T>(
    items: &'a [T],
    render_item: &impl Fn(&T) -> Element<'a, Message>,
    props: MarqueeProps,
) -> Element<'a, Message> {
    let children = items.iter().map(render_item).collect::<Vec<_>>();

    match props.orientation {
        MarqueeOrientation::Horizontal => row(children)
            .width(Length::Shrink)
            .spacing(props.gap)
            .align_y(Alignment::Center)
            .into(),
        MarqueeOrientation::Vertical => column(children)
            .height(Length::Shrink)
            .spacing(props.gap)
            .align_x(Alignment::Center)
            .into(),
    }
}

fn edge_fades<'a, Message: 'a>(props: MarqueeProps, theme: &Theme) -> Element<'a, Message> {
    let background = theme.palette.background;

    match props.orientation {
        MarqueeOrientation::Horizontal => {
            let left = container(iced::widget::Space::new())
                .width(Length::FillPortion(1))
                .height(Length::Fill)
                .style(move |_| {
                    gradient::Linear::new(Radians(0.0))
                        .add_stop(0.0, background)
                        .add_stop(0.45, background)
                        .add_stop(
                            1.0,
                            Color {
                                a: 0.0,
                                ..background
                            },
                        )
                        .into()
                });
            let right = container(iced::widget::Space::new())
                .width(Length::FillPortion(1))
                .height(Length::Fill)
                .style(move |_| {
                    gradient::Linear::new(Radians(std::f32::consts::PI))
                        .add_stop(0.0, background)
                        .add_stop(0.45, background)
                        .add_stop(
                            1.0,
                            Color {
                                a: 0.0,
                                ..background
                            },
                        )
                        .into()
                });

            row![
                left,
                iced::widget::Space::new().width(Length::FillPortion(2)),
                right
            ]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
        MarqueeOrientation::Vertical => {
            let top = container(iced::widget::Space::new())
                .width(Length::Fill)
                .height(Length::FillPortion(1))
                .style(move |_| {
                    gradient::Linear::new(Radians(std::f32::consts::FRAC_PI_2))
                        .add_stop(0.0, background)
                        .add_stop(0.45, background)
                        .add_stop(
                            1.0,
                            Color {
                                a: 0.0,
                                ..background
                            },
                        )
                        .into()
                });
            let bottom = container(iced::widget::Space::new())
                .width(Length::Fill)
                .height(Length::FillPortion(1))
                .style(move |_| {
                    gradient::Linear::new(Radians(-std::f32::consts::FRAC_PI_2))
                        .add_stop(0.0, background)
                        .add_stop(0.45, background)
                        .add_stop(
                            1.0,
                            Color {
                                a: 0.0,
                                ..background
                            },
                        )
                        .into()
                });

            column![
                top,
                iced::widget::Space::new().height(Length::FillPortion(2)),
                bottom
            ]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
    }
}

fn marquee_track_extent(item_count: usize, props: MarqueeProps) -> f32 {
    if item_count == 0 {
        return 0.0;
    }
    item_count as f32 * props.item_extent
        + item_count.saturating_sub(1) as f32 * props.gap
}

pub fn marquee_translation(elapsed: Duration, item_count: usize, props: MarqueeProps) -> Vector {
    marquee_copy_translation(0, elapsed, item_count, props)
}

fn marquee_copy_translation(
    copy_index: usize,
    elapsed: Duration,
    item_count: usize,
    props: MarqueeProps,
) -> Vector {
    let track_extent = marquee_track_extent(item_count, props);
    let cycle_extent = track_extent + props.gap;
    if cycle_extent <= f32::EPSILON {
        return Vector::default();
    }

    let offset = marquee_offset(elapsed, props.speed_px_per_sec, cycle_extent);
    let copy_index = copy_index as f32;

    match (props.orientation, props.reverse) {
        (MarqueeOrientation::Horizontal, false) => {
            Vector::new(copy_index * cycle_extent - offset, 0.0)
        }
        (MarqueeOrientation::Horizontal, true) => {
            Vector::new((copy_index - 1.0) * cycle_extent + offset, 0.0)
        }
        (MarqueeOrientation::Vertical, false) => {
            Vector::new(0.0, copy_index * cycle_extent - offset)
        }
        (MarqueeOrientation::Vertical, true) => {
            Vector::new(0.0, (copy_index - 1.0) * cycle_extent + offset)
        }
    }
}

fn marquee_copy_position(
    copy_index: usize,
    elapsed: Duration,
    item_count: usize,
    props: MarqueeProps,
    bounds: Rectangle,
) -> Vector {
    let translation = marquee_copy_translation(copy_index, elapsed, item_count, props);
    let copy_extent = marquee_track_extent(item_count, props);

    match props.orientation {
        MarqueeOrientation::Horizontal => {
            Vector::new(translation.x - (bounds.width - copy_extent) / 2.0, 0.0)
        }
        MarqueeOrientation::Vertical => {
            Vector::new(0.0, translation.y - (bounds.height - copy_extent) / 2.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn horizontal_translation_starts_at_origin_and_wraps_on_gap_boundary() {
        let props = MarqueeProps {
            orientation: MarqueeOrientation::Horizontal,
            reverse: false,
            pause_on_hover: false,
            repeat: 4,
            gap: 16.0,
            speed_px_per_sec: 40.0,
            item_extent: 256.0,
            item_cross: 112.0,
            padding: 8.0,
            edge_fade: true,
        };

        let start = marquee_translation(Duration::ZERO, 3, props);
        let cycle = marquee_track_extent(3, props) + props.gap;
        let wrapped = marquee_translation(
            Duration::from_secs_f32(cycle / props.speed_px_per_sec),
            3,
            props,
        );

        assert_eq!(start.x, 0.0);
        assert_eq!(start.y, 0.0);
        assert!(wrapped.x.abs() < 0.001);
        assert_eq!(wrapped.y, 0.0);
    }

    #[test]
    fn reverse_translation_starts_at_negative_cycle_extent() {
        let props = MarqueeProps {
            orientation: MarqueeOrientation::Horizontal,
            reverse: true,
            pause_on_hover: false,
            repeat: 4,
            gap: 16.0,
            speed_px_per_sec: 40.0,
            item_extent: 256.0,
            item_cross: 112.0,
            padding: 8.0,
            edge_fade: true,
        };

        let cycle = marquee_track_extent(3, props) + props.gap;
        let start = marquee_translation(Duration::ZERO, 3, props);

        assert_eq!(start.x, -cycle);
        assert_eq!(start.y, 0.0);
    }

    #[test]
    fn adjacent_copies_stay_one_cycle_apart() {
        let props = MarqueeProps {
            orientation: MarqueeOrientation::Horizontal,
            reverse: false,
            pause_on_hover: false,
            repeat: 4,
            gap: 16.0,
            speed_px_per_sec: 40.0,
            item_extent: 256.0,
            item_cross: 112.0,
            padding: 8.0,
            edge_fade: true,
        };

        let cycle = marquee_track_extent(3, props) + props.gap;
        let first = marquee_copy_translation(0, Duration::from_millis(375), 3, props);
        let second = marquee_copy_translation(1, Duration::from_millis(375), 3, props);

        assert!((second.x - first.x - cycle).abs() < 0.001);
        assert_eq!(first.y, 0.0);
        assert_eq!(second.y, 0.0);
    }
}
