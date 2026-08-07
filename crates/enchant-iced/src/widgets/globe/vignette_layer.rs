//! Bottom vignette on the demo card — `fill_quad` + `border.radius` (same path as the
//! outer shell background). Canvas overlays ignore rounded container styles in iced
//! `stack` layers, which is why dark-theme vignette corners looked square.

use super::DEMO_CARD_INNER_RADIUS;
use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer::{self, Quad};
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Clipboard, Renderer as AdvancedRenderer, Shell};
use iced::gradient::{self, Gradient};
use iced::mouse;
use iced::{
    Background, Border, Color, Element, Event, Length, Radians, Rectangle, Renderer, Shadow, Size,
    Vector,
};

/// Magic UI `globe-demo`: `radial-gradient(circle_at_50%_200%, rgba(0,0,0,0.2), …)`.
const VIGNETTE_ALPHA: f32 = 0.2;

pub struct GlobeVignetteLayer;

pub fn globe_vignette_layer<'a, Message: 'a>() -> Element<'a, Message> {
    Element::new(GlobeVignetteLayer)
}

impl<Message> Widget<Message, iced::Theme, Renderer> for GlobeVignetteLayer {
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, Length::Fill, Length::Fill)
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        _theme: &iced::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if bounds.width < 1.0 || bounds.height < 1.0 {
            return;
        }

        let gradient = Background::Gradient(Gradient::Linear(
            gradient::Linear::new(Radians(0.0))
                .add_stop(0.0, Color::from_rgba(0.0, 0.0, 0.0, VIGNETTE_ALPHA))
                .add_stop(0.45, Color::TRANSPARENT)
                .add_stop(1.0, Color::TRANSPARENT),
        ));

        AdvancedRenderer::fill_quad(
            renderer,
            Quad {
                bounds,
                border: Border {
                    radius: iced::border::radius(DEMO_CARD_INNER_RADIUS),
                    ..Border::default()
                },
                shadow: Shadow::default(),
                snap: false,
            },
            gradient,
        );
    }

    fn operate(
        &mut self,
        _tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
    }

    fn update(
        &mut self,
        _tree: &mut Tree,
        _event: &Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        _shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        mouse::Interaction::None
    }

    fn overlay<'b>(
        &'b mut self,
        _tree: &'b mut Tree,
        _layout: Layout<'b>,
        _renderer: &Renderer,
        _viewport: &Rectangle,
        _translation: Vector,
    ) -> Option<overlay::Element<'b, Message, iced::Theme, Renderer>> {
        None
    }
}
