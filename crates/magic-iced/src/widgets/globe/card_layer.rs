//! Positions the full-size Cobe canvas inside the demo card without shrinking it to the card height.

use super::program::GlobeShaderProgram;
use super::{GlobeAction, GlobeProps, GlobeState, GLOBE_CANVAS_SIDE};
use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::widget::shader;
use iced::{Element, Event, Length, Point, Rectangle, Renderer, Size, Vector};

pub struct GlobeCardLayer<'a, Message> {
    globe: Element<'a, Message>,
    top_offset: f32,
}

pub fn globe_card_layer<'a, Message: Clone + 'static>(
    state: &'a GlobeState,
    props: &'a GlobeProps,
    top_offset: f32,
    on_action: impl Fn(GlobeAction) -> Message + 'static + 'a,
) -> Element<'a, Message> {
    let globe = shader(GlobeShaderProgram {
        state,
        props,
        on_action: Box::new(on_action),
    })
    .width(Length::Fixed(GLOBE_CANVAS_SIDE))
    .height(Length::Fixed(GLOBE_CANVAS_SIDE))
    .into();

    Element::new(GlobeCardLayer { globe, top_offset })
}

impl<Message: Clone + 'static> Widget<Message, iced::Theme, Renderer>
    for GlobeCardLayer<'_, Message>
{
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.globe)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.globe));
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = limits.resolve(Length::Fill, Length::Fill, Size::ZERO);
        let canvas = Size::new(GLOBE_CANVAS_SIDE, GLOBE_CANVAS_SIDE);
        let child_limits = layout::Limits::new(Size::ZERO, canvas)
            .width(Length::Fixed(canvas.width))
            .height(Length::Fixed(canvas.height));

        let globe_node = self.globe.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &child_limits,
        );

        let x = (size.width - canvas.width) / 2.0;
        layout::Node::with_children(
            size,
            vec![globe_node.move_to(Point::new(x, self.top_offset))],
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        self.globe.as_widget_mut().operate(
            &mut tree.children[0],
            layout.children().next().unwrap(),
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
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.globe.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout.children().next().unwrap(),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: iced::mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> iced::mouse::Interaction {
        self.globe.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().unwrap(),
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &iced::Theme,
        style: &iced::advanced::renderer::Style,
        layout: Layout<'_>,
        cursor: iced::mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let child_layout = layout.children().next().expect("globe child layout");
        let Some(clip) = child_layout.bounds().intersection(viewport) else {
            return;
        };

        self.globe.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            child_layout,
            cursor,
            &clip,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        _tree: &'b mut Tree,
        _layout: Layout<'b>,
        _renderer: &Renderer,
        _viewport: &Rectangle,
        _translation: Vector,
    ) -> Option<overlay::Element<'b, Message, iced::Theme, Renderer>> {
        // Shader draws via `draw_primitive` only — no overlay (would bypass card clip).
        None
    }
}
