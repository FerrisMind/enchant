use crate::primitives::marquee_offset;
use iced::Alignment;
use iced::Color;
use iced::Element;
use iced::Length;
use iced::Radians;
use iced::Vector;
use iced::gradient;
use iced::widget::{column, container, float, mouse_area, row, stack};

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
    let translation = marquee_translation(elapsed, items.len(), props);
    let repeats = props.repeat.max(2);

    // Wrap items in a track that handles hover internally
    let base = render_track(items, &render_item, &on_hover, repeats, props);
    let track = float(base).translate(move |_bounds, _viewport| translation);

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

    let frame = container(
        mouse_area(track)
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

fn render_track<'a, Message: 'a + Clone, T>(
    items: &'a [T],
    render_item: &impl Fn(&T) -> Element<'a, Message>,
    on_hover: &impl Fn(bool) -> Message,
    repeats: usize,
    props: MarqueeProps,
) -> Element<'a, Message> {
    let sequence = (0..repeats)
        .flat_map(|_| {
            items.iter().map(|item| {
                // Wrap each item in its own mouse area to ensure hover/pause works everywhere
                mouse_area(render_item(item))
                    .on_enter(on_hover(true))
                    .on_exit(on_hover(false))
                    .into()
            })
        })
        .collect::<Vec<_>>();

    match props.orientation {
        MarqueeOrientation::Horizontal => row(sequence)
            .spacing(props.gap)
            .align_y(Alignment::Center)
            .into(),
        MarqueeOrientation::Vertical => column(sequence)
            .spacing(props.gap)
            .align_x(Alignment::Center)
            .into(),
    }
}

fn edge_fades<'a, Message: 'a>(props: MarqueeProps, theme: &Theme) -> Element<'a, Message> {
    let background = theme.palette.background;
    let fade_size = 64.0;

    match props.orientation {
        MarqueeOrientation::Horizontal => {
            let left = container(iced::widget::Space::new())
                .width(Length::Fixed(fade_size))
                .height(Length::Fill)
                .style(move |_| {
                    gradient::Linear::new(Radians(0.0))
                        .add_stop(0.0, background)
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
                .width(Length::Fixed(fade_size))
                .height(Length::Fill)
                .style(move |_| {
                    gradient::Linear::new(Radians(std::f32::consts::PI))
                        .add_stop(0.0, background)
                        .add_stop(
                            1.0,
                            Color {
                                a: 0.0,
                                ..background
                            },
                        )
                        .into()
                });

            row![left, iced::widget::Space::new().width(Length::Fill), right]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
        MarqueeOrientation::Vertical => {
            let top = container(iced::widget::Space::new())
                .width(Length::Fill)
                .height(Length::Fixed(fade_size))
                .style(move |_| {
                    gradient::Linear::new(Radians(std::f32::consts::FRAC_PI_2))
                        .add_stop(0.0, background)
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
                .height(Length::Fixed(fade_size))
                .style(move |_| {
                    gradient::Linear::new(Radians(-std::f32::consts::FRAC_PI_2))
                        .add_stop(0.0, background)
                        .add_stop(
                            1.0,
                            Color {
                                a: 0.0,
                                ..background
                            },
                        )
                        .into()
                });

            column![top, iced::widget::Space::new().height(Length::Fill), bottom]
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
    (props.item_extent + props.gap) * item_count as f32
}

pub fn marquee_translation(elapsed: Duration, item_count: usize, props: MarqueeProps) -> Vector {
    let extent = marquee_track_extent(item_count, props);
    if extent <= f32::EPSILON {
        return Vector::default();
    }

    let offset = marquee_offset(elapsed, props.speed_px_per_sec, extent);

    match (props.orientation, props.reverse) {
        (MarqueeOrientation::Horizontal, false) => Vector::new(-extent - offset, 0.0),
        (MarqueeOrientation::Horizontal, true) => Vector::new(-extent + offset, 0.0),
        (MarqueeOrientation::Vertical, false) => Vector::new(0.0, -extent - offset),
        (MarqueeOrientation::Vertical, true) => Vector::new(0.0, -extent + offset),
    }
}
