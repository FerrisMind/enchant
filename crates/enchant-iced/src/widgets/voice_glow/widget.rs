//! [`VoiceBeam`]: the iced widget that wraps one child in the voice glow.
//!
//! # Layering
//!
//! Upstream's layers are absolutely positioned children of the host at
//! `z-index` 1 to 4, so they paint **above** the host's plain (non-positioned)
//! children: inner light (1), edge stroke (2), bloom (3), band and epicentre
//! wash (4). [`GlowLayering::AboveContent`], the default, reproduces that:
//! the child is drawn first and the glow over it. Content that has to stay
//! crisp above the glow (the upstream demos give it `position: relative;
//! z-index: 5`) uses [`GlowLayering::BelowContent`], which draws the glow
//! first and the child over it — the child then needs a transparent
//! background to let the glow show through.

use std::cell::RefCell;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use iced::advanced::image::{self, Renderer as _};
use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{self, Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse, window};

use super::driver::VoiceGlowEngine;
use super::frame::{HostSize, VoiceGlowFrame};
use super::options::{Callback, ResolvedTheme, VoiceGlow};
use super::raster::VoiceGlowRaster;

/// Where the glow sits relative to the wrapped child.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::GlowLayering;
///
/// assert_eq!(GlowLayering::default(), GlowLayering::AboveContent);
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GlowLayering {
    /// The glow paints over the child, like upstream's z 1-4 layers over a
    /// plain child.
    #[default]
    AboveContent,
    /// The child paints over the glow (upstream's `z-index: 5` content).
    BelowContent,
}

/// What the engine's callbacks queue for the widget to publish.
#[derive(Clone, Copy, Debug)]
enum Note {
    Level(f64),
    Activate,
    Deactivate,
}

type Queue = Arc<Mutex<Vec<Note>>>;

struct Inner {
    glow: Option<VoiceGlow>,
    system: ResolvedTheme,
    engine: Option<VoiceGlowEngine>,
    last: Option<Instant>,
    frame: Option<VoiceGlowFrame>,
    raster: VoiceGlowRaster,
    cache: Option<(VoiceGlowFrame, u32, image::Handle)>,
}

/// Per-widget state that outlives `view` calls: the engine (envelope, flow
/// phase, clocks, fade), the raster buffers and the last frame's image.
struct BeamState {
    inner: RefCell<Inner>,
    queue: Queue,
}

impl BeamState {
    fn new() -> Self {
        Self {
            inner: RefCell::new(Inner {
                glow: None,
                system: ResolvedTheme::Dark,
                engine: None,
                last: None,
                frame: None,
                raster: VoiceGlowRaster::new(),
                cache: None,
            }),
            queue: Queue::default(),
        }
    }
}

/// Wraps one child element in the voice glow (`<VoiceBeam>`).
///
/// The widget owns an engine in its tree state, steps it with the real frame
/// deltas, paints [`VoiceGlowFrame`]s with the software [`VoiceGlowRaster`]
/// and asks for redraws only while the glow animates: not while paused,
/// inactive (after the fade-out) or off screen. The host is the child's
/// laid-out box; iced cannot read the child's own corner radius, so give it
/// with [`VoiceGlow::border_radius`] (default 16).
///
/// See the [module docs](self) for the layering.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::{VoiceBeam, VoiceGlow, VoiceGlowType};
/// use iced::widget::container;
/// use iced::{Element, Length};
///
/// #[derive(Clone)]
/// enum Message {
///     Level(f64),
/// }
///
/// fn view<'a>() -> Element<'a, Message> {
///     let glow = VoiceGlow::new().kind(VoiceGlowType::Pill).border_radius(22.0);
///     VoiceBeam::new(glow, container("Say something").width(Length::Fixed(320.0)).height(Length::Fixed(56.0)))
///         .on_level(Message::Level)
///         .into()
/// }
/// # let _ = view;
/// ```
#[must_use = "widgets do nothing unless turned into an iced Element"]
pub struct VoiceBeam<'a, Message> {
    glow: VoiceGlow,
    child: Element<'a, Message>,
    backdrop: Option<Element<'a, Message>>,
    layering: GlowLayering,
    pixel_density: f32,
    level_message: Option<Box<dyn Fn(f64) -> Message + 'a>>,
    activate_message: Option<Box<dyn Fn() -> Message + 'a>>,
    deactivate_message: Option<Box<dyn Fn() -> Message + 'a>>,
}

impl<Message> fmt::Debug for VoiceBeam<'_, Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VoiceBeam")
            .field("glow", &self.glow)
            .field("layering", &self.layering)
            .field("pixel_density", &self.pixel_density)
            .finish_non_exhaustive()
    }
}

impl<'a, Message> VoiceBeam<'a, Message> {
    /// Wraps `child` in the glow described by `glow`.
    pub fn new(glow: VoiceGlow, child: impl Into<Element<'a, Message>>) -> Self {
        Self {
            glow,
            child: child.into(),
            backdrop: None,
            layering: GlowLayering::default(),
            pixel_density: 1.0,
            level_message: None,
            activate_message: None,
            deactivate_message: None,
        }
    }

    /// The host's own background (its fill, border and shadow): drawn first,
    /// below the glow, sized to the child's box. Upstream's host paints its
    /// background under the glow layers while its content sits above them;
    /// with one child element that split needs the background separate. It is
    /// decorative: it receives no events.
    pub fn backdrop(mut self, backdrop: impl Into<Element<'a, Message>>) -> Self {
        self.backdrop = Some(backdrop.into());
        self
    }

    /// Where the glow sits relative to the child (default
    /// [`GlowLayering::AboveContent`]).
    pub fn layering(mut self, layering: GlowLayering) -> Self {
        self.layering = layering;
        self
    }

    /// Device pixels per logical px of the raster (default 1; `2` on a 2x
    /// display for hairline-crisp edges).
    pub fn pixel_density(mut self, density: f32) -> Self {
        self.pixel_density = if density.is_finite() {
            density.clamp(0.5, 4.0)
        } else {
            1.0
        };
        self
    }

    /// Publishes the smoothed level (0-1) every frame the engine reads the
    /// source (`onLevel`).
    pub fn on_level(mut self, f: impl Fn(f64) -> Message + 'a) -> Self {
        self.level_message = Some(Box::new(f));
        self
    }

    /// Publishes a message when the fade-in has finished (`onActivate`).
    pub fn on_activate(mut self, f: impl Fn() -> Message + 'a) -> Self {
        self.activate_message = Some(Box::new(f));
        self
    }

    /// Publishes a message when the fade-out has finished (`onDeactivate`).
    pub fn on_deactivate(mut self, f: impl Fn() -> Message + 'a) -> Self {
        self.deactivate_message = Some(Box::new(f));
        self
    }
}

impl VoiceGlow {
    /// Wraps `child` in this glow: shorthand for [`VoiceBeam::new`].
    pub fn wrap<'a, Message>(
        self,
        child: impl Into<Element<'a, Message>>,
    ) -> VoiceBeam<'a, Message> {
        VoiceBeam::new(self, child)
    }
}

impl<'a, Message: 'a> From<VoiceBeam<'a, Message>> for Element<'a, Message> {
    fn from(beam: VoiceBeam<'a, Message>) -> Self {
        Element::new(beam)
    }
}

fn system_theme(theme: &Theme) -> ResolvedTheme {
    if theme.extended_palette().is_dark {
        ResolvedTheme::Dark
    } else {
        ResolvedTheme::Light
    }
}

impl<Message> VoiceBeam<'_, Message> {
    /// The glow with its callbacks teed into the widget's queue, for the
    /// message-based props.
    fn engine_glow(&self, queue: &Queue) -> VoiceGlow {
        let mut glow = self.glow.clone();
        if self.level_message.is_some() {
            let user = glow.on_level.clone();
            let queue = Arc::clone(queue);
            glow.on_level = Some(Callback::new(move |level: f64| {
                if let Some(user) = &user {
                    user.call(level);
                }
                queue
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(Note::Level(level));
            }));
        }
        if self.activate_message.is_some() {
            let user = glow.on_activate.clone();
            let queue = Arc::clone(queue);
            glow.on_activate = Some(Callback::new(move |()| {
                if let Some(user) = &user {
                    user.call(());
                }
                queue
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(Note::Activate);
            }));
        }
        if self.deactivate_message.is_some() {
            let user = glow.on_deactivate.clone();
            let queue = Arc::clone(queue);
            glow.on_deactivate = Some(Callback::new(move |()| {
                if let Some(user) = &user {
                    user.call(());
                }
                queue
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(Note::Deactivate);
            }));
        }
        glow
    }

    /// Brings the engine in line with the current props and theme.
    fn sync(&self, inner: &mut Inner, queue: &Queue, system: ResolvedTheme) {
        let changed = inner.glow.as_ref() != Some(&self.glow) || inner.system != system;
        if !changed && inner.engine.is_some() {
            return;
        }
        let glow = self.engine_glow(queue);
        match &mut inner.engine {
            Some(engine) => engine.reconfigure(&glow, system),
            None => inner.engine = Some(VoiceGlowEngine::new(&glow, system)),
        }
        inner.glow = Some(self.glow.clone());
        inner.system = system;
    }

    fn child_layout<'l>(layout: Layout<'l>) -> Layout<'l> {
        layout.children().next().expect("voice_glow child layout")
    }
}

impl<Message> Widget<Message, Theme, Renderer> for VoiceBeam<'_, Message> {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<BeamState>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(BeamState::new())
    }

    fn children(&self) -> Vec<Tree> {
        let mut children = vec![Tree::new(&self.child)];
        children.extend(self.backdrop.as_ref().map(Tree::new));
        children
    }

    fn diff(&self, tree: &mut Tree) {
        let mut elements = vec![&self.child];
        elements.extend(self.backdrop.as_ref());
        tree.diff_children(&elements);
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
        let node = self
            .child
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        let size = node.size();
        let mut nodes = vec![node];
        if let Some(backdrop) = self.backdrop.as_mut() {
            let fixed = layout::Limits::new(size, size);
            nodes.push(
                backdrop
                    .as_widget_mut()
                    .layout(&mut tree.children[1], renderer, &fixed),
            );
        }
        layout::Node::with_children(size, nodes)
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
            Self::child_layout(layout),
            renderer,
            operation,
        );
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
            Self::child_layout(layout),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let Event::Window(window::Event::RedrawRequested(now)) = event else {
            return;
        };
        let bounds = layout.bounds();
        let state = tree.state.downcast_ref::<BeamState>();
        let mut inner = state.inner.borrow_mut();
        let system = inner.system;
        self.sync(&mut inner, &state.queue, system);

        // Off screen the glow rests, like upstream's IntersectionObserver.
        let visible = bounds.intersects(viewport);
        if visible {
            let dt = inner.last.map_or(0.0, |last| {
                now.saturating_duration_since(last).as_secs_f64()
            });
            inner.last = Some(*now);
            let host = HostSize::new(f64::from(bounds.width), f64::from(bounds.height));
            let Some(engine) = inner.engine.as_mut() else {
                return;
            };
            let frame = engine.step(dt, host);
            let keep_going = engine.is_running() && !engine.config().paused;
            inner.frame = Some(frame);
            if keep_going {
                shell.request_redraw();
            } else {
                inner.last = None;
            }
        } else {
            inner.last = None;
        }
        drop(inner);

        let notes: Vec<Note> =
            std::mem::take(&mut *state.queue.lock().unwrap_or_else(PoisonError::into_inner));
        for note in notes {
            match note {
                Note::Level(level) => {
                    if let Some(f) = &self.level_message {
                        shell.publish(f(level));
                    }
                }
                Note::Activate => {
                    if let Some(f) = &self.activate_message {
                        shell.publish(f());
                    }
                }
                Note::Deactivate => {
                    if let Some(f) = &self.deactivate_message {
                        shell.publish(f());
                    }
                }
            }
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
        if let Some(backdrop) = &self.backdrop
            && let Some(backdrop_layout) = layout.children().nth(1)
        {
            backdrop.as_widget().draw(
                &tree.children[1],
                renderer,
                theme,
                style,
                backdrop_layout,
                cursor,
                viewport,
            );
        }
        let draw_child = |renderer: &mut Renderer| {
            self.child.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                Self::child_layout(layout),
                cursor,
                viewport,
            );
        };
        if bounds.width < 1.0 || bounds.height < 1.0 || !bounds.intersects(viewport) {
            draw_child(renderer);
            return;
        }

        let state = tree.state.downcast_ref::<BeamState>();
        let handle = {
            let mut inner = state.inner.borrow_mut();
            self.sync(&mut inner, &state.queue, system_theme(theme));
            let host = HostSize::new(f64::from(bounds.width), f64::from(bounds.height));
            if inner.frame.as_ref().is_none_or(|f| f.host != host) {
                let Some(engine) = inner.engine.as_mut() else {
                    draw_child(renderer);
                    return;
                };
                let frame = engine.step(0.0, host);
                inner.frame = Some(frame);
            }
            self.glow_image(&mut inner)
        };

        let paint = |renderer: &mut Renderer| {
            if let Some(handle) = handle.clone() {
                let filter = if (self.pixel_density - 1.0).abs() < f32::EPSILON {
                    image::FilterMethod::Nearest
                } else {
                    image::FilterMethod::Linear
                };
                renderer.draw_image(
                    image::Image::new(handle).filter_method(filter).snap(true),
                    bounds,
                    bounds,
                );
            }
        };
        match self.layering {
            GlowLayering::AboveContent => {
                draw_child(renderer);
                paint(renderer);
            }
            GlowLayering::BelowContent => {
                paint(renderer);
                draw_child(renderer);
            }
        }
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
            Self::child_layout(layout),
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.child.as_widget_mut().overlay(
            &mut tree.children[0],
            Self::child_layout(layout),
            renderer,
            viewport,
            translation,
        )
    }
}

impl<Message> VoiceBeam<'_, Message> {
    /// The image of the latest frame, rastered once per distinct frame.
    fn glow_image(&self, inner: &mut Inner) -> Option<image::Handle> {
        let frame = inner.frame.clone()?;
        let density_key = self.pixel_density.to_bits();
        if let Some((cached, key, handle)) = &inner.cache
            && *key == density_key
            && *cached == frame
        {
            return Some(handle.clone());
        }
        let config = inner.engine.as_ref()?.config().clone();
        let bitmap = inner.raster.render(&frame, &config, self.pixel_density);
        if bitmap.width == 0 || bitmap.height == 0 {
            return None;
        }
        let handle = image::Handle::from_rgba(bitmap.width, bitmap.height, bitmap.rgba.clone());
        inner.cache = Some((frame, density_key, handle.clone()));
        Some(handle)
    }
}
