//! Neighbour reflection overlay — iOS `metalReflection` / web `reflectionTargets`.

use std::cell::RefCell;
use std::time::Duration;

use iced::advanced::image::{self, Renderer as _};
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{self, Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, mouse, window};

use super::reflection::{MetalReflectionPaint, MetalReflectionStyle, paint_reflection};
use super::reflection_anchor::reflection_anchor;

/// Reflects a published [`MetalFx`] anchor onto a neighbour child.
///
/// Register the source with [`crate::widgets::metal_fx::MetalFx::anchor_id`], then
/// wrap the catch surface:
///
/// ```ignore
/// MetalFx::new(button).anchor_id("plan")
/// MetalReflection::new("plan", chip).strength(0.64)
/// ```
pub struct MetalReflection<'a, Message> {
    anchor_id: String,
    strength: f32,
    style: MetalReflectionStyle,
    density: f32,
    child: Element<'a, Message>,
}

impl<'a, Message> MetalReflection<'a, Message> {
    /// Reflects `anchor_id` onto `child`.
    pub fn new(anchor_id: impl Into<String>, child: impl Into<Element<'a, Message>>) -> Self {
        Self {
            anchor_id: anchor_id.into(),
            strength: 1.0,
            style: MetalReflectionStyle::Surface,
            density: 1.0,
            child: child.into(),
        }
    }

    /// Per-target alpha multiplier (demo "Plan" uses `0.64`).
    pub fn strength(mut self, strength: f32) -> Self {
        if strength.is_finite() {
            self.strength = strength.clamp(0.0, 4.0);
        }
        self
    }

    /// Surface strip vs glyph-masked catch.
    pub fn style(mut self, style: MetalReflectionStyle) -> Self {
        self.style = style;
        self
    }

    /// Raster density for the overlay bitmap.
    pub fn density(mut self, density: f32) -> Self {
        if density.is_finite() {
            self.density = density.clamp(1.0, 3.0);
        }
        self
    }
}

impl<'a, Message> From<MetalReflection<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(value: MetalReflection<'a, Message>) -> Self {
        Element::new(value)
    }
}

#[derive(Default)]
struct Inner {
    handle: Option<image::Handle>,
    pointer: Option<[f32; 2]>,
}

struct State {
    inner: RefCell<Inner>,
}

impl<Message> Widget<Message, Theme, Renderer> for MetalReflection<'_, Message> {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State {
            inner: RefCell::new(Inner::default()),
        })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.child)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.child));
    }

    fn size(&self) -> Size<Length> {
        self.child.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let child = self
            .child
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        let size = child.size();
        layout::Node::with_children(size, vec![child])
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.child.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout.children().next().expect("reflection child"),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        let state = tree.state.downcast_ref::<State>();
        let mut inner = state.inner.borrow_mut();
        match event {
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                inner.pointer = Some([position.x, position.y]);
                shell.request_redraw();
            }
            Event::Mouse(mouse::Event::CursorLeft) => {
                inner.pointer = None;
                shell.request_redraw();
            }
            Event::Window(window::Event::RedrawRequested(now))
                if reflection_anchor(&self.anchor_id).is_some() =>
            {
                // Match the web's ~15 fps reflection cadence.
                shell.request_redraw_at(*now + Duration::from_millis(66));
            }
            _ => {}
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let child_layout = layout.children().next().expect("reflection child");
        self.child.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            child_layout,
            cursor,
            viewport,
        );

        let Some(anchor) = reflection_anchor(&self.anchor_id) else {
            return;
        };
        let target = [bounds.x, bounds.y, bounds.width, bounds.height];
        let state = tree.state.downcast_ref::<State>();
        let mut inner = state.inner.borrow_mut();
        let pointer = inner
            .pointer
            .or_else(|| cursor.position().map(|p| [p.x, p.y]));
        let Some((w, h, rgba)) = paint_reflection(MetalReflectionPaint {
            anchor: &anchor,
            target,
            strength: self.strength,
            style: self.style,
            density: self.density,
            pointer,
            glyph_mask: None,
        }) else {
            inner.handle = None;
            return;
        };
        let handle = image::Handle::from_rgba(w, h, rgba);
        inner.handle = Some(handle.clone());
        renderer.draw_image(
            image::Image::new(handle).filter_method(image::FilterMethod::Linear),
            bounds,
            bounds,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.child.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().expect("reflection child"),
            cursor,
            viewport,
            renderer,
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
        self.child.as_widget_mut().operate(
            &mut tree.children[0],
            layout.children().next().expect("reflection child"),
            renderer,
            operation,
        );
    }
}
