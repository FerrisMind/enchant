//! The iced `Widget` behind [`Liquid`]: layout, the shared clock, drag input,
//! and the three-layer draw pass.
//!
//! The widget is a container: it measures every child once per layout, lays
//! them out in flow or free mode, and then draws:
//!
//! 1. the rasterizer's `behind` layer (shadows + goo fill),
//! 2. the rasterizer's `content` layer (image items with pixel effects),
//! 3. the actual child widgets, positioned at the component-driven target,
//! 4. the rasterizer's `above` layer (dissolve/melt copies).
//!
//! Only the liquid silhouette lags behind with springs; widget content always
//! sits at its layout target, exactly like the reference DOM.

use std::cell::{Cell, RefCell};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Instant;

use iced::advanced::Renderer as _;
use iced::advanced::image::{self, Renderer as _};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, overlay, renderer};
use iced::{
    Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Transformation, Vector, mouse,
    window,
};

use super::engine::{self, ItemSpec, ItemState};
use super::options::ItemBox;
use super::raster::{self, GroupStyle, RasterItem};
use super::Liquid;

struct CachedRaster {
    signature: u64,
    behind: image::Handle,
    content: image::Handle,
    above: image::Handle,
    bounds: Rectangle,
}

/// The widget's persistent state.
pub(super) struct LiquidState {
    states: RefCell<Vec<ItemState>>,
    cache: RefCell<Option<CachedRaster>>,
    origin: Instant,
    last: Cell<Option<Instant>>,
    active: Cell<bool>,
    drag: Cell<Option<(usize, Point, Vector, Vector)>>,
    boxes: RefCell<Vec<ItemBox>>,
}

impl LiquidState {
    fn new() -> Self {
        Self {
            states: RefCell::new(Vec::new()),
            cache: RefCell::new(None),
            origin: Instant::now(),
            last: Cell::new(None),
            active: Cell::new(true),
            drag: Cell::new(None),
            boxes: RefCell::new(Vec::new()),
        }
    }
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for Liquid<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<LiquidState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(LiquidState::new())
    }

    fn children(&self) -> Vec<Tree> {
        self.items()
            .iter()
            .filter_map(|item| item.element().map(Tree::new))
            .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        let children: Vec<&Element<'_, Message>> =
            self.items().iter().filter_map(|item| item.element()).collect();
        tree.diff_children(&children);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let (nodes, total, boxes) = self.layout_children(tree, renderer, limits);
        *tree.state.downcast_mut::<LiquidState>().boxes.borrow_mut() = boxes;
        layout::Node::with_children(total, nodes)
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
        let bounds = layout.bounds();
        let state = tree.state.downcast_ref::<LiquidState>();

        // Forward the event to every live child at the position it is drawn.
        for index in 0..self.items().len() {
            let Some(child_index) = self.element_index(index) else {
                continue;
            };
            let Some(child_layout) = layout.children().nth(child_index) else {
                continue;
            };
            let Some(item) = self.items().get(index) else {
                continue;
            };
            let scale = item.options().scale;
            let drag = state
                .states
                .borrow()
                .get(index)
                .map_or(Vector::ZERO, |s| s.drag);
            let center = child_layout.bounds().center();
            let adjusted = adjust_cursor(cursor, center, scale, drag);
            let Some(element) = self.items_mut()[index].element_mut() else {
                continue;
            };
            element.as_widget_mut().update(
                &mut tree.children[child_index],
                event,
                child_layout,
                adjusted,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(position) = cursor.position_over(bounds) else {
                    return;
                };
                let local = position - Vector::new(bounds.x, bounds.y);
                let hit = {
                    let states = state.states.borrow();
                    (0..self.items().len()).rev().find_map(|index| {
                        if !self.items()[index].is_draggable() {
                            return None;
                        }
                        let rendered = states.get(index).and_then(|s| s.rendered)?;
                        contains(rendered, local).then(|| {
                            let start = states.get(index).map_or(Vector::ZERO, |s| s.drag);
                            (index, start)
                        })
                    })
                };
                if let Some((index, start)) = hit {
                    let base = self.items()[index].options().offset;
                    state.drag.set(Some((index, position, start, base)));
                    if let Some(s) = state.states.borrow_mut().get_mut(index) {
                        s.dragging = true;
                    }
                    state.active.set(true);
                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let Some((index, press, start, base)) = state.drag.get() else {
                    return;
                };
                let Some(position) = cursor.position() else {
                    return;
                };
                let mut delta = start + (position - press);
                if let Some(limit) = self.items().get(index).and_then(|item| item.options.drag_bounds) {
                    delta.x = delta.x.clamp(limit.origin.x, limit.origin.x + limit.size.width);
                    delta.y = delta.y.clamp(limit.origin.y, limit.origin.y + limit.size.height);
                }
                let item = &self.items()[index];
                // With a live callback the application owns the position: the
                // callback receives the absolute target and the widget keeps no
                // extra offset. Without one the delta itself is the position.
                let live = if item.on_drag.is_some() {
                    let mut target = base + delta;
                    if let Some(limit) = item.options.drag_bounds {
                        target.x = target.x.clamp(limit.origin.x, limit.origin.x + limit.size.width);
                        target.y = target.y.clamp(limit.origin.y, limit.origin.y + limit.size.height);
                    }
                    Some(target)
                } else {
                    None
                };
                if let Some(s) = state.states.borrow_mut().get_mut(index) {
                    s.drag = live.map_or(delta, |_| Vector::ZERO);
                    s.dragging = true;
                }
                if let Some(message) = item.drag_live_message(index, live.unwrap_or(delta)) {
                    shell.publish(message);
                }
                state.active.set(true);
                shell.request_redraw();
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let Some((index, _, _, base)) = state.drag.take() else {
                    return;
                };
                let item = &self.items()[index];
                let stored = state
                    .states
                    .borrow()
                    .get(index)
                    .map_or(Vector::ZERO, |s| s.drag);
                let final_offset = if item.on_drag.is_some() {
                    item.options().offset
                } else {
                    base + stored
                };
                if let Some(s) = state.states.borrow_mut().get_mut(index) {
                    s.dragging = false;
                }
                if let Some(message) = self.items()[index].drag_message(index, final_offset) {
                    shell.publish(message);
                }
                state.active.set(true);
                shell.request_redraw();
                shell.capture_event();
            }
            Event::Window(window::Event::RedrawRequested(_)) => {
                if state.active.get() || state.drag.get().is_some() {
                    shell.request_redraw();
                }
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
        if bounds.width < 1.0 || bounds.height < 1.0 {
            return;
        }
        let state = tree.state.downcast_ref::<LiquidState>();
        let now = Instant::now();
        let elapsed = state
            .last
            .get()
            .map_or(0.0, |last| now.saturating_duration_since(last).as_secs_f64());
        state.last.set(Some(now));
        let time = now.saturating_duration_since(state.origin).as_secs_f64();

        // Engine coordinates are local to the group so the raster canvas and
        // the layout node share one origin.
        let boxes = state.boxes.borrow().clone();
        let mut specs = Vec::with_capacity(self.items().len());
        for (index, base) in boxes.iter().enumerate() {
            let Some(item) = self.items().get(index) else {
                break;
            };
            let radii = item.options().resolve_radius(base.size);
            specs.push(ItemSpec {
                base: *base,
                radii,
                options: item.options().clone(),
            });
        }

        let scene = {
            let mut states = state.states.borrow_mut();
            states.resize_with(specs.len(), ItemState::default);
            let before: Vec<Option<ItemBox>> = states.iter().map(|s| s.rendered).collect();
            let mut scene = engine::step(
                &mut states,
                &specs,
                time,
                elapsed,
                self.blur,
                self.reduced_motion,
            );
            for (index, item) in scene.iter_mut().enumerate() {
                if let Some(spec) = specs.get(index) {
                    item.silhouette = spec.options.silhouette;
                    item.image = spec.options.image.as_ref().and_then(raster::decode);
                }
            }
            let mut active = state.drag.get().is_some();
            for (index, item_state) in states.iter().enumerate() {
                active |= item_state.is_active(before.get(index).copied().flatten());
            }
            state.active.set(active);
            scene
        };

        let group = GroupStyle {
            blur: self.blur,
            contrast: self.contrast,
            fill: self.fill,
            shadows: self.shadows.layers().to_vec(),
            waviness: self.waviness,
            waviness_freq: self.waviness_freq,
            filter_padding: self.filter_padding,
        };
        let signature = signature(self.revision, &scene, &group, bounds);
        let needs_raster = state
            .cache
            .borrow()
            .as_ref()
            .is_none_or(|cache| cache.signature != signature || cache.bounds != bounds);
        if needs_raster {
            let raster = raster::render(bounds.size(), self.density, &group, &scene);
            #[cfg(debug_assertions)]
            if std::env::var_os("GOOEY_DUMP").is_some() {
                use std::sync::atomic::{AtomicUsize, Ordering};
                static FRAME: AtomicUsize = AtomicUsize::new(0);
                let frame = FRAME.fetch_add(1, Ordering::Relaxed);
                if frame < 400
                    && scene.iter().any(|item| item.image.is_some())
                    && bounds.width > 200.0
                {
                    raster.dump(
                        std::path::Path::new("target/gooey-dump"),
                        &format!("frame{frame}-{}-{}", bounds.width as u32, bounds.height as u32),
                    );
                }
            }
            let behind =
                image::Handle::from_rgba(raster.width, raster.height, raster.behind.clone());
            let content =
                image::Handle::from_rgba(raster.width, raster.height, raster.content.clone());
            let above = image::Handle::from_rgba(raster.width, raster.height, raster.above.clone());
            *state.cache.borrow_mut() = Some(CachedRaster {
                signature,
                behind,
                content,
                above,
                bounds,
            });
        }

        let cache = state.cache.borrow();
        let Some(cache) = cache.as_ref() else {
            return;
        };

        let draw_children = |renderer: &mut Renderer, below: bool| {
            // Children sit at their component-driven target: canvas content
            // does not lag, only the liquid silhouette does.
            let states = state.states.borrow();
            for index in 0..self.items().len() {
                let Some(item) = self.items().get(index) else {
                    continue;
                };
                if item.options().behind != below {
                    continue;
                }
                let Some(child_index) = self.element_index(index) else {
                    continue;
                };
                let Some(child_layout) = layout.children().nth(child_index) else {
                    continue;
                };
                let Some(element) = item.element() else {
                    continue;
                };
                let scale = item.options().scale;
                let drag = states.get(index).map_or(Vector::ZERO, |s| s.drag);
                let base = child_layout.bounds();
                let center = base.center();
                let transform = Transformation::translate(drag.x, drag.y)
                    * Transformation::translate(-center.x, -center.y)
                    * Transformation::scale(scale)
                    * Transformation::translate(center.x, center.y);
                renderer.with_layer(bounds, |renderer| {
                    renderer.with_transformation(transform, |renderer| {
                        element.as_widget().draw(
                            &tree.children[child_index],
                            renderer,
                            theme,
                            style,
                            child_layout,
                            adjust_cursor(cursor, center, scale, drag),
                            viewport,
                        );
                    });
                });
            }
        };

        draw_children(renderer, true);
        draw_layer(renderer, &cache.behind, bounds);
        draw_layer(renderer, &cache.content, bounds);
        draw_children(renderer, false);
        draw_layer(renderer, &cache.above, bounds);
        drop(cache);
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let bounds = layout.bounds();
        let state = tree.state.downcast_ref::<LiquidState>();
        if state.drag.get().is_some() {
            return mouse::Interaction::Grabbing;
        }
        let local = cursor
            .position()
            .map(|p| p - Vector::new(bounds.x, bounds.y));
        let states = state.states.borrow();
        for index in (0..self.items().len()).rev() {
            if !self.items()[index].is_draggable() {
                continue;
            }
            if local.is_some_and(|p| {
                states
                    .get(index)
                    .and_then(|s| s.rendered)
                    .is_some_and(|rendered| contains(rendered, p))
            }) {
                return mouse::Interaction::Grab;
            }
        }
        drop(states);
        for index in 0..self.items().len() {
            let Some(child_index) = self.element_index(index) else {
                continue;
            };
            let Some(child_layout) = layout.children().nth(child_index) else {
                continue;
            };
            let Some(item) = self.items().get(index) else {
                continue;
            };
            let Some(element) = item.element() else {
                continue;
            };
            let scale = item.options().scale;
            let drag = state
                .states
                .borrow()
                .get(index)
                .map_or(Vector::ZERO, |s| s.drag);
            let center = child_layout.bounds().center();
            let interaction = element.as_widget().mouse_interaction(
                &tree.children[child_index],
                child_layout,
                adjust_cursor(cursor, center, scale, drag),
                viewport,
                renderer,
            );
            if interaction != mouse::Interaction::None {
                return interaction;
            }
        }
        mouse::Interaction::None
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        for index in 0..self.items().len() {
            let Some(child_index) = self.element_index(index) else {
                continue;
            };
            let Some(child_layout) = layout.children().nth(child_index) else {
                continue;
            };
            let Some(item) = self.items.get_mut(index) else {
                continue;
            };
            let Some(element) = item.element_mut() else {
                continue;
            };
            element
                .as_widget_mut()
                .operate(&mut tree.children[child_index], child_layout, renderer, operation);
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let mut overlays = Vec::new();
        let mut states = tree.children.iter_mut();
        let mut layouts = layout.children();
        for item in self.items.iter_mut() {
            let Some(element) = item.element_mut() else {
                continue;
            };
            let (Some(state), Some(child_layout)) = (states.next(), layouts.next()) else {
                break;
            };
            if let Some(overlay) = element.as_widget_mut().overlay(
                state,
                child_layout,
                renderer,
                viewport,
                translation,
            ) {
                overlays.push(overlay);
            }
        }
        if overlays.is_empty() {
            None
        } else {
            Some(overlay::Group::with_children(overlays).overlay())
        }
    }
}

fn draw_layer(renderer: &mut Renderer, handle: &image::Handle, bounds: Rectangle) {
    renderer.draw_image(
        image::Image::new(handle.clone())
            .filter_method(image::FilterMethod::Linear)
            .snap(false),
        bounds,
        bounds,
    );
}

fn adjust_cursor(
    cursor: mouse::Cursor,
    center: Point,
    scale: f32,
    drag: Vector,
) -> mouse::Cursor {
    let scale = scale.max(0.05);
    match cursor {
        mouse::Cursor::Available(p) => mouse::Cursor::Available(child_point(p, center, scale, drag)),
        mouse::Cursor::Levitating(p) => {
            mouse::Cursor::Levitating(child_point(p, center, scale, drag))
        }
        mouse::Cursor::Unavailable => mouse::Cursor::Unavailable,
    }
}

fn child_point(point: Point, center: Point, scale: f32, drag: Vector) -> Point {
    let local = point - drag - Vector::new(center.x, center.y);
    Point::new(
        center.x + local.x / scale,
        center.y + local.y / scale,
    )
}

fn contains(rect: ItemBox, point: Point) -> bool {
    point.x >= rect.origin.x
        && point.x <= rect.right()
        && point.y >= rect.origin.y
        && point.y <= rect.bottom()
}

fn signature(revision: u64, scene: &[RasterItem], style: &GroupStyle, bounds: Rectangle) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    revision.hash(&mut hasher);
    scene.len().hash(&mut hasher);
    bounds.x.to_bits().hash(&mut hasher);
    bounds.y.to_bits().hash(&mut hasher);
    bounds.width.to_bits().hash(&mut hasher);
    bounds.height.to_bits().hash(&mut hasher);
    style.blur.to_bits().hash(&mut hasher);
    style.contrast.to_bits().hash(&mut hasher);
    style.fill.r.to_bits().hash(&mut hasher);
    style.fill.g.to_bits().hash(&mut hasher);
    style.fill.b.to_bits().hash(&mut hasher);
    style.fill.a.to_bits().hash(&mut hasher);
    style.waviness.to_bits().hash(&mut hasher);
    style.waviness_freq.to_bits().hash(&mut hasher);
    style.filter_padding.to_bits().hash(&mut hasher);
    for layer in &style.shadows {
        layer.x.to_bits().hash(&mut hasher);
        layer.y.to_bits().hash(&mut hasher);
        layer.blur.to_bits().hash(&mut hasher);
        layer.spread.to_bits().hash(&mut hasher);
        layer.color.r.to_bits().hash(&mut hasher);
        layer.color.g.to_bits().hash(&mut hasher);
        layer.color.b.to_bits().hash(&mut hasher);
        layer.color.a.to_bits().hash(&mut hasher);
        layer.inset.hash(&mut hasher);
    }
    for item in scene {
        item.bounds.origin.x.to_bits().hash(&mut hasher);
        item.bounds.origin.y.to_bits().hash(&mut hasher);
        item.bounds.size.width.to_bits().hash(&mut hasher);
        item.bounds.size.height.to_bits().hash(&mut hasher);
        item.radii.top_left.to_bits().hash(&mut hasher);
        item.radii.top_right.to_bits().hash(&mut hasher);
        item.radii.bottom_right.to_bits().hash(&mut hasher);
        item.radii.bottom_left.to_bits().hash(&mut hasher);
        item.content_blur.to_bits().hash(&mut hasher);
        item.silhouette.hash(&mut hasher);
        item.image
            .as_ref()
            .map(|image| Arc::as_ptr(image) as usize)
            .hash(&mut hasher);
        if let Some((y, x)) = item.bend {
            y.to_bits().hash(&mut hasher);
            x.to_bits().hash(&mut hasher);
        }
        if let Some(tail) = item.tail {
            tail.tail.0.x.to_bits().hash(&mut hasher);
            tail.tail.0.y.to_bits().hash(&mut hasher);
            tail.tail.1.to_bits().hash(&mut hasher);
            tail.mid_a.0.x.to_bits().hash(&mut hasher);
            tail.mid_a.0.y.to_bits().hash(&mut hasher);
            tail.mid_a.1.to_bits().hash(&mut hasher);
        }
        if let Some(dissolve) = &item.dissolve {
            dissolve.strength.to_bits().hash(&mut hasher);
            dissolve.opacity.to_bits().hash(&mut hasher);
            dissolve.seam.x.to_bits().hash(&mut hasher);
            dissolve.seam.y.to_bits().hash(&mut hasher);
            dissolve.d.to_bits().hash(&mut hasher);
            dissolve.phase.to_bits().hash(&mut hasher);
            dissolve.hole.is_some().hash(&mut hasher);
        }
        if let Some(melt) = &item.melt {
            melt.strength.to_bits().hash(&mut hasher);
            melt.seam.x.to_bits().hash(&mut hasher);
            melt.seam.y.to_bits().hash(&mut hasher);
            melt.phase.to_bits().hash(&mut hasher);
        }
    }
    hasher.finish()
}
