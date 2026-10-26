//! Liquid gooey layout: a port of Libraries.dev's `liquid-gooey` package.
//!
//! A [`Liquid`] is a container whose children are rendered as pieces of one
//! merged liquid surface. The children stay crisp, interactive widgets; the
//! liquid silhouette underneath them is produced by a software reproduction
//! of the reference's SVG filter graph (gaussian goo blur + alpha contrast,
//! `box-shadow` morphology, waviness displacement, dissolve/melt turbulence).
//!
//! The port keeps every public capability of the reference:
//!
//! | here | upstream |
//! |------|----------|
//! | [`Liquid`] | `<Liquid>` group |
//! | [`LiquidItem`] | `<LiquidItem>` |
//! | [`Morph`], [`EvolveOptions`], [`EVOLVE_DEFAULTS`] | `MorphTuning`, `EvolveOptions`, `EVOLVE_DEFAULTS` |
//! | [`Move`], [`MoveOptions`], [`MOVE_DEFAULTS`] | `MoveTuning`, `MoveOptions`, `MOVE_DEFAULTS` |
//! | [`Bend`] | `BendTuning` |
//! | [`DissolveOptions`] | `DissolveOptions` |
//! | [`ImageMeltOptions`], [`IMAGE_MELT_DEFAULTS`] | `ImageMeltOptions`, `IMAGE_MELT_DEFAULTS` |
//! | [`Shadow`], [`ShadowLayer`] | `parseShadow`, `box-shadow` string |
//! | [`Transition`], [`TransitionPreset`], [`SpringConfig`] | `Transition`, spring presets |
//! | [`CornerRadii`] | `CornerRadii`, `border-radius` |
//! | [`LiquidRaster`] | the rendered filter result |
//!
//! # Behavior
//!
//! - Group: blur, contrast, fill, `box-shadow` stack (outer and inset, with
//!   spread), filter padding, waviness and its frequency.
//! - Item: `morph` (merge + optional jelly shape springs), `move` (spring-lag
//!   surface + trailing droplet tongue + velocity stretch), `bend` (vertical
//!   bow and horizontal cap deformation), `melt` (image-to-image melt), and
//!   `dissolve` (turbulence-warped imagery + edge holes where two liquids
//!   meet). Items also support component-driven position/scale with spring or
//!   timed transitions, per-item delay, radius override, blob inset,
//!   bridge grow, observation mode, and built-in dragging. A
//!   [content](LiquidItem::content) item is laid out and drawn inside the
//!   group without joining the silhouette (the reference's plain children),
//!   and [behind](LiquidItem::behind) puts an item's element below the
//!   liquid surface.
//!
//! # API style
//!
//! Builder-first: private fields, fluent `#[must_use]` setters that validate
//! and clamp their input, `#[non_exhaustive]` public enums, and a terminal
//! [`into_element`](Liquid::into_element) / [`view`](Liquid::view).
//!
//! # Image effects
//!
//! iced has no readback of arbitrary widget layers. The pixel effects
//! (`dissolve`, `melt`, content cross-blur) therefore apply to items created
//! with [`LiquidItem::image`] — the rasterizer owns those pixels and can warp,
//! blur and mask them. Element content is always drawn crisp, while its
//! silhouette still morphs, moves, bends and merges.
//!
//! # Example
//!
//! ```rust,no_run
//! use enchant_iced::widgets::gooey::{Liquid, LiquidItem, Morph, Move};
//! use iced::widget::{button, text};
//!
//! # fn demo<'a>() -> iced::Element<'a, ()> {
//! Liquid::row(
//!     vec![
//!         LiquidItem::new(button("Plan")).radius(20.0),
//!         LiquidItem::new(button("Debug"))
//!             .radius(20.0)
//!             .move_effect(Move::new().springiness(0.7)),
//!         LiquidItem::new(button("Ask"))
//!             .radius(20.0)
//!             .morph(Morph::new().shape(true)),
//!     ],
//!     6.0,
//! )
//! .blur(3.5)
//! .contrast(22.0)
//! .shadow("0 2px 6px rgba(0, 0, 0, 0.08)")
//! .into_element()
//! # }
//! ```

use std::time::Duration;

use iced::advanced::image::Handle;
use iced::{Element, Point, Size, Vector, advanced::widget::tree};

mod engine;
mod geometry;
pub mod options;
mod raster;
mod shadow;
mod spring;
mod widget;

pub use geometry::CornerRadii;
pub use options::{
    Bend, DissolveOptions, EVOLVE_DEFAULTS, EvolveOptions, IMAGE_MELT_DEFAULTS, ImageMeltOptions,
    ItemBox, ItemOptions, LiquidLayout, MOVE_DEFAULTS, Morph, Move, MoveOptions, Shadow,
    ShadowLayer, preset, spring, transition,
};
pub use raster::LiquidRaster;
pub use shadow::parse_color;
pub use spring::{Easing, SpringConfig, Transition, TransitionPreset};

use options::{
    DEFAULT_BLUR, DEFAULT_CONTRAST, DEFAULT_FILL, DEFAULT_FILTER_PADDING, DEFAULT_WAVINESS_FREQ,
};

/// One child of a [`Liquid`].
///
/// Build it from a regular iced element ([`LiquidItem::new`]) or from an image
/// ([`LiquidItem::image`]). Image items participate in the dissolve/melt pixel
/// effects; element items stay crisp.
pub struct LiquidItem<'a, Message> {
    element: Option<Element<'a, Message>>,
    options: ItemOptions,
    on_drag: Option<Box<dyn Fn(usize, Vector) -> Message + 'a>>,
    on_drag_end: Option<Box<dyn Fn(usize, Vector) -> Message + 'a>>,
}

impl<'a, Message> LiquidItem<'a, Message> {
    /// Wraps a regular iced element.
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            element: Some(content.into()),
            options: ItemOptions::default(),
            on_drag: None,
            on_drag_end: None,
        }
    }

    /// An image item. The image is drawn by the liquid rasterizer, so content
    /// blur, dissolve and melt can process its pixels.
    ///
    /// Give it an explicit size with [`size`](Self::size); the default is
    /// `40 x 40` logical pixels.
    #[must_use]
    pub fn image(handle: impl Into<Handle>) -> Self {
        let options = ItemOptions {
            image: Some(handle.into()),
            size: Some(Size::new(40.0, 40.0)),
            ..ItemOptions::default()
        };
        Self {
            element: None,
            options,
            on_drag: None,
            on_drag_end: None,
        }
    }

    /// Lays out an element inside the group without adding its geometry to
    /// the merged liquid silhouette.
    ///
    /// This is the counterpart of a plain child of the reference
    /// `Liquid` component: tracks, labels and backdrops that the liquid may
    /// flow around but that must not become a blob themselves.
    #[must_use]
    pub fn content(content: impl Into<Element<'a, Message>>) -> Self {
        Self::new(content).silhouette(false)
    }

    /// Draws an element on top of an image item (labels over photos).
    #[must_use]
    pub fn overlay(mut self, content: impl Into<Element<'a, Message>>) -> Self {
        self.element = Some(content.into());
        self
    }

    /// Overrides the measured radius of the liquid body.
    ///
    /// Accepts `f32`, `[f32; 4]`, `(f32, f32, f32, f32)`, a `u16`/`i32`, or
    /// an [`iced::border::Radius`].
    #[must_use]
    pub fn radius(mut self, radius: impl Into<CornerRadii>) -> Self {
        self.options.radius = Some(radius.into());
        self
    }

    /// The item's layout size (image items only; element items measure their
    /// content).
    #[must_use]
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.options.size = Some(Size::new(non_negative(width), non_negative(height)));
        self
    }

    /// The item's layout size.
    #[must_use]
    pub fn sized(mut self, size: impl Into<Size>) -> Self {
        let size = size.into();
        self.options.size = Some(Size::new(
            non_negative(size.width),
            non_negative(size.height),
        ));
        self
    }

    /// Places a free-layout item at `(x, y)` inside the group.
    #[must_use]
    pub fn position(mut self, x: f32, y: f32) -> Self {
        self.options.position = Some(Point::new(x, y));
        self
    }

    /// Places a free-layout item.
    #[must_use]
    pub fn at(mut self, position: impl Into<Point>) -> Self {
        self.options.position = Some(position.into());
        self
    }

    /// A translation on top of the layout position.
    #[must_use]
    pub fn offset(mut self, offset: impl Into<Vector>) -> Self {
        self.options.offset = offset.into();
        self
    }

    /// A translation on top of the layout position.
    #[must_use]
    pub fn translate(mut self, x: f32, y: f32) -> Self {
        self.options.offset = Vector::new(x, y);
        self
    }

    /// The component-driven scale of the item and its liquid.
    #[must_use]
    pub fn scale(mut self, scale: f32) -> Self {
        self.options.scale = finite(scale, 1.0).clamp(0.0, 4.0);
        self
    }

    /// How the liquid follows component-driven position/scale changes.
    #[must_use]
    pub fn transition(mut self, transition: impl Into<Transition>) -> Self {
        self.options.transition = transition.into();
        self
    }

    /// Delay before the transition starts (stagger).
    #[must_use]
    pub fn delay(mut self, delay: Duration) -> Self {
        self.options.delay = Some(delay);
        self
    }

    /// Makes the liquid follow the child's measured rect even when the child
    /// animates itself. Implied by `morph.shape`, `dissolve` and `move`.
    #[must_use]
    pub fn observe(mut self, observe: bool) -> Self {
        self.options.observe = observe;
        self
    }

    /// Whether this item's geometry is merged into the liquid silhouette.
    ///
    /// Defaults to `true`. Set to `false` for children that should be laid out
    /// and drawn inside the group but stay crisp outside the goo, such as a
    /// slider track. Image items always draw their pixels either way.
    #[must_use]
    pub fn silhouette(mut self, silhouette: bool) -> Self {
        self.options.silhouette = silhouette;
        self
    }

    /// Draws this item's element below the liquid surface.
    ///
    /// Defaults to `false` (content sits above the liquid). Useful for parts
    /// that the liquid should cover, like the track of a slider thumb.
    #[must_use]
    pub fn behind(mut self, behind: bool) -> Self {
        self.options.behind = behind;
        self
    }

    /// Enables the morph effect. Use [`Morph::shape`] for the shape springs.
    #[must_use]
    pub fn morph(mut self, morph: Morph) -> Self {
        self.options.morph = Some(morph);
        self
    }

    /// Enables the jelly shape physics with the tuned defaults.
    #[must_use]
    pub fn morph_shape(mut self, shape: bool) -> Self {
        let mut morph = self.options.morph.unwrap_or_default();
        morph.shape = shape;
        self.options.morph = Some(morph);
        self
    }

    /// Shrinks the blob by `px` per side so opaque content covers it.
    #[must_use]
    pub fn blob_inset(mut self, inset: f32) -> Self {
        let mut morph = self.options.morph.unwrap_or_default();
        morph.blob_inset = non_negative(inset);
        self.options.morph = Some(morph);
        self
    }

    /// Lets the blob swell back out near a neighbour.
    #[must_use]
    pub fn bridge_grow(mut self, grow: f32) -> Self {
        let mut morph = self.options.morph.unwrap_or_default();
        morph.bridge_grow = non_negative(grow);
        self.options.morph = Some(morph);
        self
    }

    /// Enables the `move` effect: a lagging liquid surface with a droplet tail.
    #[must_use]
    pub fn move_effect(mut self, tuning: Move) -> Self {
        self.options.move_effect = Some(tuning);
        self
    }

    /// Enables the `bend` effect: the body deforms with velocity.
    #[must_use]
    pub fn bend(mut self, tuning: Bend) -> Self {
        self.options.bend = Some(tuning);
        self
    }

    /// Enables the image-to-image melt with the tuned defaults.
    #[must_use]
    pub fn melt(mut self, options: ImageMeltOptions) -> Self {
        self.options.melt = Some(options);
        self
    }

    /// Enables the melt with [`IMAGE_MELT_DEFAULTS`].
    #[must_use]
    pub fn melt_default(mut self) -> Self {
        self.options.melt = Some(IMAGE_MELT_DEFAULTS);
        self
    }

    /// Enables the dissolve modifier.
    ///
    /// `true` uses the tuned defaults; a `f32` scales the strength (`0..=1`);
    /// [`DissolveOptions`] gives full control.
    #[must_use]
    pub fn dissolve(mut self, dissolve: impl Into<DissolveOptions>) -> Self {
        self.options.dissolve = Some(dissolve.into());
        self
    }

    /// Makes the item draggable by the pointer.
    #[must_use]
    pub fn draggable(mut self, draggable: bool) -> Self {
        self.options.draggable = draggable;
        self
    }

    /// Restricts the drag translation: `origin` is the minimum offset and
    /// `origin + size` the maximum.
    #[must_use]
    pub fn drag_bounds(mut self, bounds: ItemBox) -> Self {
        self.options.drag_bounds = Some(bounds);
        self
    }

    /// Publishes a message while the item is dragged, with the item index and
    /// the item's absolute target offset (its base offset plus the pointer
    /// delta, clamped by [`drag_bounds`](Self::drag_bounds)).
    ///
    /// When this callback is present the application owns the position: write
    /// the reported offset back into the item (for example through
    /// [`offset`](Self::offset) or your own model). Without it the widget keeps
    /// the pointer delta itself and the item stays where it was dropped.
    #[must_use]
    pub fn on_drag(mut self, f: impl Fn(usize, Vector) -> Message + 'a) -> Self {
        self.on_drag = Some(Box::new(f));
        self
    }

    /// Publishes a message when a drag ends, with the item index and final
    /// offset. The final offset is absolute when [`on_drag`](Self::on_drag) is
    /// set, and the retained pointer delta otherwise.
    #[must_use]
    pub fn on_drag_end(mut self, f: impl Fn(usize, Vector) -> Message + 'a) -> Self {
        self.on_drag_end = Some(Box::new(f));
        self
    }

    /// The item's tuning.
    #[must_use]
    pub fn options(&self) -> &ItemOptions {
        &self.options
    }

    /// The item's iced content, if any.
    pub(crate) fn element(&self) -> Option<&Element<'a, Message>> {
        self.element.as_ref()
    }

    pub(crate) fn element_mut(&mut self) -> Option<&mut Element<'a, Message>> {
        self.element.as_mut()
    }

    pub(crate) fn is_draggable(&self) -> bool {
        self.options.draggable
    }

    pub(crate) fn drag_message(&self, index: usize, offset: Vector) -> Option<Message> {
        self.on_drag_end.as_ref().map(|f| f(index, offset))
    }

    pub(crate) fn drag_live_message(&self, index: usize, offset: Vector) -> Option<Message> {
        self.on_drag.as_ref().map(|f| f(index, offset))
    }

    pub(crate) fn layout_size(&self) -> Size {
        self.options.size.unwrap_or_else(|| Size::new(40.0, 40.0))
    }
}

/// A gooey liquid group.
///
/// Items are laid out in a row, a column, or at explicit positions, and their
/// silhouettes are merged by the group's goo filter. See the module docs for
/// the full capability table.
pub struct Liquid<'a, Message> {
    items: Vec<LiquidItem<'a, Message>>,
    layout: LiquidLayout,
    padding: f32,
    width: Option<f32>,
    height: Option<f32>,
    blur: f32,
    contrast: f32,
    fill: iced::Color,
    shadows: Shadow,
    waviness: f32,
    waviness_freq: f32,
    filter_padding: f32,
    density: f32,
    reduced_motion: bool,
    revision: u64,
}

impl<'a, Message: 'a> Liquid<'a, Message> {
    /// A row of items with no gap.
    pub fn new(items: impl IntoIterator<Item = LiquidItem<'a, Message>>) -> Self {
        let mut liquid = Self {
            items: Vec::new(),
            layout: LiquidLayout::Row { spacing: 0.0 },
            padding: 0.0,
            width: None,
            height: None,
            blur: DEFAULT_BLUR,
            contrast: DEFAULT_CONTRAST,
            fill: DEFAULT_FILL,
            shadows: Shadow::none(),
            waviness: 0.0,
            waviness_freq: DEFAULT_WAVINESS_FREQ,
            filter_padding: DEFAULT_FILTER_PADDING,
            density: 2.0,
            reduced_motion: false,
            revision: 0,
        };
        for item in items {
            liquid = liquid.push(item);
        }
        liquid
    }

    /// An empty row with `spacing` between items.
    #[must_use]
    pub fn row(spacing: f32) -> Self {
        Self::new([]).layout(LiquidLayout::Row {
            spacing: non_negative(spacing),
        })
    }

    /// An empty column with `spacing` between items.
    #[must_use]
    pub fn column(spacing: f32) -> Self {
        Self::new([]).layout(LiquidLayout::Column {
            spacing: non_negative(spacing),
        })
    }

    /// An empty free-layout group; items use [`LiquidItem::position`].
    #[must_use]
    pub fn free() -> Self {
        Self::new([]).layout(LiquidLayout::Free)
    }

    /// Adds an item.
    #[must_use]
    pub fn push(mut self, item: LiquidItem<'a, Message>) -> Self {
        let mut item = item;
        item.options.index = self.items.len();
        self.items.push(item);
        self.revision = self.revision.wrapping_add(1);
        self
    }

    /// Adds several items.
    #[must_use]
    pub fn extend(mut self, items: impl IntoIterator<Item = LiquidItem<'a, Message>>) -> Self {
        for item in items {
            self = self.push(item);
        }
        self
    }

    /// Sets the layout mode.
    #[must_use]
    pub fn layout(mut self, layout: LiquidLayout) -> Self {
        self.layout = layout;
        self.revision = self.revision.wrapping_add(1);
        self
    }

    /// Space around the items.
    #[must_use]
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = non_negative(padding);
        self.revision = self.revision.wrapping_add(1);
        self
    }

    /// Forces the group's width.
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(non_negative(width));
        self.revision = self.revision.wrapping_add(1);
        self
    }

    /// Forces the group's height.
    #[must_use]
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(non_negative(height));
        self.revision = self.revision.wrapping_add(1);
        self
    }

    /// Forces the group's size.
    #[must_use]
    pub fn size(mut self, size: impl Into<Size>) -> Self {
        let size = size.into();
        self.width = Some(non_negative(size.width));
        self.height = Some(non_negative(size.height));
        self.revision = self.revision.wrapping_add(1);
        self
    }

    /// The goo blur sigma (upstream default `6`).
    #[must_use]
    pub fn blur(mut self, blur: f32) -> Self {
        self.blur = non_negative(blur);
        self.revision = self.revision.wrapping_add(1);
        self
    }

    /// The alpha-contrast slope (upstream default `18`).
    #[must_use]
    pub fn contrast(mut self, contrast: f32) -> Self {
        self.contrast = finite(contrast, DEFAULT_CONTRAST).clamp(0.0, 100.0);
        self.revision = self.revision.wrapping_add(1);
        self
    }

    /// The silhouette fill colour.
    #[must_use]
    pub fn fill(mut self, fill: impl Into<iced::Color>) -> Self {
        self.fill = fill.into();
        self.revision = self.revision.wrapping_add(1);
        self
    }

    /// The `box-shadow` stack.
    #[must_use]
    pub fn shadow(mut self, shadow: impl Into<Shadow>) -> Self {
        self.shadows = shadow.into();
        self.revision = self.revision.wrapping_add(1);
        self
    }

    /// The liquid edge undulation in pixels (`0` = calm).
    #[must_use]
    pub fn waviness(mut self, waviness: f32) -> Self {
        self.waviness = non_negative(waviness);
        self.revision = self.revision.wrapping_add(1);
        self
    }

    /// The undulation frequency (default `0.018`; lower = lazier waves).
    #[must_use]
    pub fn waviness_freq(mut self, frequency: f32) -> Self {
        self.waviness_freq = finite(frequency, DEFAULT_WAVINESS_FREQ).clamp(0.001, 1.0);
        self.revision = self.revision.wrapping_add(1);
        self
    }

    /// Transparent margin added around the filter region.
    #[must_use]
    pub fn filter_padding(mut self, padding: f32) -> Self {
        self.filter_padding = non_negative(padding);
        self.revision = self.revision.wrapping_add(1);
        self
    }

    /// Raster pixels per logical point, `0.5..=3`. Default `2`.
    #[must_use]
    pub fn density(mut self, density: f32) -> Self {
        self.density = finite(density, 2.0).clamp(0.5, 3.0);
        self.revision = self.revision.wrapping_add(1);
        self
    }

    /// Disables all spring motion (the liquid still renders).
    #[must_use]
    pub fn reduced_motion(mut self, reduced: bool) -> Self {
        self.reduced_motion = reduced;
        self.revision = self.revision.wrapping_add(1);
        self
    }

    /// The terminal conversion into an [`Element`].
    #[must_use]
    pub fn into_element(self) -> Element<'a, Message> {
        Element::new(self)
    }

    /// The terminal conversion into an [`Element`] (alias of
    /// [`into_element`](Self::into_element)).
    #[must_use]
    pub fn view(self) -> Element<'a, Message> {
        self.into_element()
    }

    /// The items of the group.
    #[must_use]
    pub fn items(&self) -> &[LiquidItem<'a, Message>] {
        &self.items
    }

    /// The layout mode.
    #[must_use]
    pub fn layout_mode(&self) -> LiquidLayout {
        self.layout
    }

    /// The fill colour.
    #[must_use]
    pub fn fill_color(&self) -> iced::Color {
        self.fill
    }

    pub(crate) fn items_mut(&mut self) -> &mut [LiquidItem<'a, Message>] {
        &mut self.items
    }

    /// Maps a group item index to the tree child index (element-only).
    pub(crate) fn element_index(&self, item_index: usize) -> Option<usize> {
        let mut child = 0;
        for (index, item) in self.items.iter().enumerate() {
            if item.element.is_some() {
                if index == item_index {
                    return Some(child);
                }
                child += 1;
            }
        }
        None
    }

    /// Lays every child out and returns positioned nodes plus the group size.
    pub(crate) fn layout_children(
        &mut self,
        tree: &mut tree::Tree,
        renderer: &iced::Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> (Vec<iced::advanced::layout::Node>, Size, Vec<ItemBox>) {
        let child_limit = iced::advanced::layout::Limits::new(Size::ZERO, limits.max());
        let mut element_nodes = Vec::with_capacity(self.items.len());
        let mut sizes = Vec::with_capacity(self.items.len());
        for index in 0..self.items.len() {
            if let Some((child_index, element)) = self
                .element_index(index)
                .zip(self.items[index].element_mut())
            {
                let node = element.as_widget_mut().layout(
                    &mut tree.children[child_index],
                    renderer,
                    &child_limit,
                );
                sizes.push(node.size());
                element_nodes.push((index, node));
                continue;
            }
            sizes.push(self.items[index].layout_size());
        }
        let positions: Vec<Option<Point>> = self.items.iter().map(|i| i.options.position).collect();
        let offsets: Vec<Vector> = self.items.iter().map(|i| i.options.offset).collect();
        let no_offsets = vec![Vector::ZERO; self.items.len()];
        let ones = vec![1.0_f32; self.items.len()];
        // Widget content sits at the component-driven target immediately;
        // only the liquid silhouette is spring-lagged.
        let content_boxes = engine::layout_targets(
            self.layout,
            self.padding,
            &sizes,
            &positions,
            &offsets,
            &ones,
        );
        let nodes: Vec<_> = element_nodes
            .into_iter()
            .map(|(index, node)| node.move_to(content_boxes[index].origin))
            .collect();
        // The engine applies each item's offset and scale itself; hand it the
        // unoffset layout box so those properties are not applied twice.
        let base_boxes = engine::layout_targets(
            self.layout,
            self.padding,
            &sizes,
            &positions,
            &no_offsets,
            &ones,
        );
        let scales: Vec<f32> = self.items.iter().map(|i| i.options.scale).collect();
        let targets = engine::layout_targets(
            self.layout,
            self.padding,
            &sizes,
            &positions,
            &offsets,
            &scales,
        );
        let total = match self.layout {
            LiquidLayout::Row { spacing } => Size::new(
                sizes.iter().map(|s| s.width).sum::<f32>()
                    + spacing * (sizes.len().saturating_sub(1)) as f32
                    + self.padding * 2.0,
                sizes.iter().map(|s| s.height).fold(0.0, f32::max) + self.padding * 2.0,
            ),
            LiquidLayout::Column { spacing } => Size::new(
                sizes.iter().map(|s| s.width).fold(0.0, f32::max) + self.padding * 2.0,
                sizes.iter().map(|s| s.height).sum::<f32>()
                    + spacing * (sizes.len().saturating_sub(1)) as f32
                    + self.padding * 2.0,
            ),
            LiquidLayout::Free => Size::new(
                targets.iter().map(|t| t.right()).fold(0.0, f32::max) + self.padding,
                targets.iter().map(|t| t.bottom()).fold(0.0, f32::max) + self.padding,
            ),
        };
        let size = Size::new(
            self.width.unwrap_or(total.width),
            self.height.unwrap_or(total.height),
        );
        (nodes, size, base_boxes)
    }
}

impl<'a, Message: 'a> From<Liquid<'a, Message>> for Element<'a, Message> {
    fn from(liquid: Liquid<'a, Message>) -> Self {
        Element::new(liquid)
    }
}

impl<'a, Message: 'a> Default for Liquid<'a, Message> {
    fn default() -> Self {
        Self::new([])
    }
}

fn finite(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

fn non_negative(value: f32) -> f32 {
    finite(value, 0.0).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_upstream() {
        let liquid: Liquid<'_, ()> = Liquid::new([]);
        assert_eq!(liquid.blur, 6.0);
        assert_eq!(liquid.contrast, 18.0);
        assert_eq!(liquid.waviness_freq, 0.018);
        assert!(liquid.shadows.layers().is_empty());
    }

    #[test]
    fn shadow_string_parses_into_layers() {
        let liquid: Liquid<'_, ()> =
            Liquid::new([]).shadow("0 0 0 1px rgba(0,0,0,.06) inset, 0 4px 8px #0003");
        assert_eq!(liquid.shadows.layers().len(), 2);
        assert!(liquid.shadows.layers()[0].inset);
    }

    #[test]
    fn setters_clamp() {
        let liquid: Liquid<'_, ()> = Liquid::new([])
            .blur(-4.0)
            .contrast(f32::NAN)
            .density(99.0)
            .waviness(-1.0);
        assert_eq!(liquid.blur, 0.0);
        assert_eq!(liquid.contrast, 18.0);
        assert_eq!(liquid.density, 3.0);
        assert_eq!(liquid.waviness, 0.0);
    }

    #[test]
    fn item_defaults_to_a_pill() {
        let item: LiquidItem<'_, ()> = LiquidItem::new(iced::widget::text("x"));
        assert!(item.options().radius.is_none());
        assert_eq!(item.options().scale, 1.0);
        assert!(item.options().silhouette);
        assert!(!item.options().behind);
    }

    #[test]
    fn content_items_stay_out_of_the_silhouette() {
        let item: LiquidItem<'_, ()> = LiquidItem::content(iced::widget::text("x"));
        assert!(!item.options().silhouette);
        let item = item.silhouette(true).behind(true);
        assert!(item.options().silhouette);
        assert!(item.options().behind);
    }
}
