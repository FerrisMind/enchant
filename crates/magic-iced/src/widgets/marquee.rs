use crate::primitives::marquee_offset;
use iced::Color;
use iced::Element;
use iced::Event;
use iced::Length;
use iced::Point;
use iced::Radians;
use iced::Rectangle;
use iced::Size;
use iced::Vector;
use iced::advanced::Clipboard;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Shell, overlay};
use iced::gradient;
use iced::widget::{column, container, mouse_area, row};

use iced_shadcn::{DecorativeSurfaceProps, Theme, decorative_surface};
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
    marquee_indexed(
        items,
        move |item, _copy_index, _item_index| render_item(item),
        elapsed,
        props,
        theme,
        on_hover,
    )
}

pub fn marquee_indexed<'a, Message: 'a + Clone, T>(
    items: &'a [T],
    render_item: impl Fn(&T, usize, usize) -> Element<'a, Message>,
    elapsed: Duration,
    props: MarqueeProps,
    theme: &Theme,
    on_hover: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    let track_extent = marquee_track_extent(items.len(), props);

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

    let content = container(MarqueeTrack::new(items, render_item, elapsed, props))
        .width(Length::Fill)
        .height(Length::Fill);

    let overlays = if props.edge_fade && track_extent > f32::EPSILON {
        vec![edge_fades(props, theme)]
    } else {
        Vec::new()
    };

    decorative_surface(
        mouse_area(content)
            .on_enter(on_hover(true))
            .on_exit(on_hover(false)),
        Vec::new(),
        overlays,
        DecorativeSurfaceProps::new()
            .width(width)
            .height(height)
            .padding(props.padding)
            .clip(true),
        theme,
    )
}

struct MarqueeTrack<'a, Message> {
    children: Vec<Element<'a, Message>>,
    positions: Vec<Point>,
    width: Length,
    height: Length,
    props: MarqueeProps,
}

impl<'a, Message> MarqueeTrack<'a, Message> {
    fn new<T>(
        items: &'a [T],
        render_item: impl Fn(&T, usize, usize) -> Element<'a, Message>,
        elapsed: Duration,
        props: MarqueeProps,
    ) -> Self {
        let repeats = props.repeat.max(2);
        let item_count = items.len();
        let total = repeats.saturating_mul(item_count);
        let (width, height) = match props.orientation {
            MarqueeOrientation::Horizontal => (Length::Fill, Length::Fixed(props.item_cross)),
            MarqueeOrientation::Vertical => (Length::Fixed(props.item_cross), Length::Fill),
        };

        let mut children = Vec::with_capacity(total);
        let mut positions = Vec::with_capacity(total);

        for copy_index in 0..repeats {
            for (item_index, item) in items.iter().enumerate() {
                children.push(render_item(item, copy_index, item_index));
                positions.push(marquee_item_position(
                    copy_index, item_index, elapsed, item_count, props,
                ));
            }
        }

        Self {
            children,
            positions,
            width,
            height,
            props,
        }
    }
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for MarqueeTrack<'_, Message>
where
    Message: Clone,
{
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(self.width).height(self.height);
        let size = limits.resolve(self.width, self.height, Size::ZERO);
        let item_size = marquee_item_size(self.props);
        let child_limits = layout::Limits::new(Size::ZERO, item_size)
            .width(Length::Fixed(item_size.width))
            .height(Length::Fixed(item_size.height));

        let nodes = self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(self.positions.iter().copied())
            .map(|((child, state), position)| {
                child
                    .as_widget_mut()
                    .layout(state, renderer, &child_limits)
                    .move_to(position)
            })
            .collect();

        layout::Node::with_children(size, nodes)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
                .for_each(|((child, state), layout)| {
                    child
                        .as_widget_mut()
                        .operate(state, layout, renderer, operation);
                });
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: iced::mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        for ((child, state), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child.as_widget_mut().update(
                state, event, layout, cursor, renderer, clipboard, shell, viewport,
            );

            if shell.is_event_captured() {
                return;
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: iced::mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> iced::mouse::Interaction {
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .filter(|(_, layout)| layout.bounds().intersects(viewport))
            .map(|((child, state), layout)| {
                child
                    .as_widget()
                    .mouse_interaction(state, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: iced::mouse::Cursor,
        viewport: &Rectangle,
    ) {
        if let Some(clipped_viewport) = layout.bounds().intersection(viewport) {
            for ((child, state), layout) in self
                .children
                .iter()
                .zip(&tree.children)
                .zip(layout.children())
                .filter(|(_, layout)| layout.bounds().intersects(&clipped_viewport))
            {
                child.as_widget().draw(
                    state,
                    renderer,
                    theme,
                    style,
                    layout,
                    cursor,
                    &clipped_viewport,
                );
            }
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, iced::Theme, iced::Renderer>> {
        overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message> From<MarqueeTrack<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(widget: MarqueeTrack<'a, Message>) -> Self {
        Element::new(widget)
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
    item_count as f32 * props.item_extent + item_count.saturating_sub(1) as f32 * props.gap
}

pub fn marquee_translation(elapsed: Duration, item_count: usize, props: MarqueeProps) -> Vector {
    marquee_copy_translation(0, elapsed, item_count, props)
}

fn marquee_item_size(props: MarqueeProps) -> Size {
    match props.orientation {
        MarqueeOrientation::Horizontal => Size::new(props.item_extent, props.item_cross),
        MarqueeOrientation::Vertical => Size::new(props.item_cross, props.item_extent),
    }
}

fn marquee_item_position(
    copy_index: usize,
    item_index: usize,
    elapsed: Duration,
    item_count: usize,
    props: MarqueeProps,
) -> Point {
    let translation = marquee_copy_translation(copy_index, elapsed, item_count, props);
    let offset = item_index as f32 * (props.item_extent + props.gap);

    match props.orientation {
        MarqueeOrientation::Horizontal => Point::new(translation.x + offset, 0.0),
        MarqueeOrientation::Vertical => Point::new(0.0, translation.y + offset),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
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

    #[test]
    fn copy_translations_ignore_viewport_width() {
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

        let first = marquee_copy_translation(0, Duration::from_millis(375), 3, props);
        let second = marquee_copy_translation(1, Duration::from_millis(375), 3, props);
        let cycle = marquee_track_extent(3, props) + props.gap;

        assert!(first.x.is_finite());
        assert!(second.x.is_finite());
        assert!((second.x - first.x - cycle).abs() < 0.001);
    }

    #[test]
    fn render_copy_passes_distinct_instance_indices() {
        let seen = RefCell::new(Vec::new());
        let items = ["one", "two", "three"];
        let props = MarqueeProps::new();

        let _: iced::Element<'_, ()> = MarqueeTrack::new(
            &items,
            |_item, copy_index, item_index| -> iced::Element<'_, ()> {
                seen.borrow_mut().push((copy_index, item_index));
                iced::widget::Space::new().into()
            },
            Duration::ZERO,
            MarqueeProps { repeat: 3, ..props },
        )
        .into();

        assert_eq!(
            *seen.borrow(),
            vec![
                (0, 0),
                (0, 1),
                (0, 2),
                (1, 0),
                (1, 1),
                (1, 2),
                (2, 0),
                (2, 1),
                (2, 2),
            ]
        );
    }

    #[test]
    fn marquee_item_positions_keep_full_item_stride() {
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

        let first = marquee_item_position(0, 0, Duration::ZERO, 3, props);
        let second = marquee_item_position(0, 1, Duration::ZERO, 3, props);
        let third = marquee_item_position(0, 2, Duration::ZERO, 3, props);

        assert_eq!(first.x, 0.0);
        assert!((second.x - 272.0).abs() < 0.001);
        assert!((third.x - 544.0).abs() < 0.001);
    }

    #[test]
    fn marquee_item_size_matches_horizontal_contract() {
        let props = MarqueeProps {
            orientation: MarqueeOrientation::Horizontal,
            item_extent: 256.0,
            item_cross: 112.0,
            ..MarqueeProps::new()
        };

        assert_eq!(marquee_item_size(props), Size::new(256.0, 112.0));
    }
}
