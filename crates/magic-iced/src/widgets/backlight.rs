use crate::primitives::{glow_from_handle, glow_from_svg_bytes, glow_spread};
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{self, Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::widget::image::{self, FilterMethod, Handle};
use iced::{ContentFit, Element, Event, Length, Point, Rectangle, Rotation, Size};
use iced_shadcn::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BacklightSvgGlow {
    pub bytes: &'static [u8],
    /// When true, glow + display follow theme text color.
    pub mono: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BacklightProps {
    /// Reference `feGaussianBlur stdDeviation` (default `20`).
    pub blur: f32,
    /// Reference `feColorMatrix saturate` (default `4`).
    pub saturation: f32,
    /// Encoded image used to build the blurred glow layer.
    pub glow_source: Option<Handle>,
    /// SVG bytes for supersampled glow (display stays vector `svg()`).
    pub glow_svg: Option<BacklightSvgGlow>,
    /// Corner radius applied to the sharp content (reference `rounded-xl`).
    pub radius: f32,
}

impl Default for BacklightProps {
    fn default() -> Self {
        Self {
            blur: 20.0,
            saturation: 4.0,
            glow_source: None,
            glow_svg: None,
            radius: 16.0,
        }
    }
}

impl BacklightProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn blur(mut self, blur: f32) -> Self {
        self.blur = blur.max(0.0);
        self
    }

    pub fn saturation(mut self, saturation: f32) -> Self {
        self.saturation = saturation.max(0.0);
        self
    }

    pub fn glow_source(mut self, handle: Handle) -> Self {
        self.glow_source = Some(handle);
        self.glow_svg = None;
        self
    }

    pub fn glow_svg(mut self, bytes: &'static [u8], mono: bool) -> Self {
        self.glow_svg = Some(BacklightSvgGlow { bytes, mono });
        self.glow_source = None;
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius.max(0.0);
        self
    }
}

pub fn backlight<'a, Message: Clone + 'static>(
    content: impl Into<Element<'a, Message>>,
    props: BacklightProps,
    _theme: &Theme,
) -> Element<'a, Message> {
    Element::new(Backlight {
        content: content.into(),
        props,
    })
}

struct Backlight<'a, Message> {
    content: Element<'a, Message>,
    props: BacklightProps,
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for Backlight<'_, Message>
where
    Message: Clone + 'static,
{
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<()>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::None
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let spread = glow_spread(self.props.blur);
        let content_node = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        let content_size = content_node.size();

        layout::Node::with_children(
            Size::new(
                content_size.width + spread * 2.0,
                content_size.height + spread * 2.0,
            ),
            vec![content_node.move_to(Point::new(spread, spread))],
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            layout.children().next().expect("content layout"),
            renderer,
            operation,
        );
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
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout.children().next().expect("content layout"),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
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
        let outer_bounds = layout.bounds();
        let content_layout = layout.children().next().expect("content layout");
        let content_bounds = content_layout.bounds();

        let glow = if let Some(svg) = self.props.glow_svg {
            let tint = svg.mono.then(|| theme.palette().text);
            glow_from_svg_bytes(
                svg.bytes,
                self.props.blur,
                self.props.saturation,
                self.props.radius,
                content_bounds.width,
                content_bounds.height,
                tint,
            )
        } else if let Some(source) = self.props.glow_source.as_ref() {
            glow_from_handle(
                source,
                self.props.blur,
                self.props.saturation,
                self.props.radius,
                content_bounds.width,
                content_bounds.height,
            )
        } else {
            None
        };

        if let Some(handle) = glow {
            let glow_node = layout::Node::new(outer_bounds.size()).move_to(outer_bounds.position());
            let glow_layout = Layout::new(&glow_node);

            image::draw(
                renderer,
                glow_layout,
                &handle,
                None,
                iced::border::Radius::default(),
                ContentFit::Fill,
                FilterMethod::Linear,
                Rotation::default(),
                1.0,
                1.0,
            );
        }

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            content_layout,
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: iced::mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> iced::mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().expect("content layout"),
            cursor,
            viewport,
            renderer,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backlight_defaults_match_reference() {
        let props = BacklightProps::new();

        assert_eq!(props.blur, 20.0);
        assert_eq!(props.saturation, 4.0);
        assert_eq!(props.radius, 16.0);
        assert!(props.glow_source.is_none());
        assert!(props.glow_svg.is_none());
    }

    #[test]
    fn glow_spread_scales_with_blur() {
        assert_eq!(glow_spread(20.0), 50.0);
    }
}
