//! The iced `Widget` behind [`ThinkingOrb`]: layout, drawing, redraw pacing
//! and the gravity overlay.

use std::cell::Cell;
use std::sync::OnceLock;
use std::time::Instant;

use iced::advanced::Renderer as _;
use iced::advanced::image::{self, Renderer as _};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout};
use iced::{Event, Length, Rectangle, Renderer, Size, Theme, Vector, keyboard, mouse, window};

use super::gravity;
use super::soft;
use super::{MotionPreference, OrbTheme, ThinkingOrb};

/// The shared clock origin: every orb reads the same time, so they stay in
/// phase (the reference's `performance.now()`).
fn clock_origin() -> Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    *START.get_or_init(Instant::now)
}

fn clock_seconds(now: Instant) -> f64 {
    now.saturating_duration_since(clock_origin()).as_secs_f64()
}

/// Per-widget state that outlives `view` calls.
struct OrbTreeState {
    id: u64,
    /// The clock reading a paused orb is frozen on.
    frozen: Cell<Option<f64>>,
    registered: bool,
}

impl OrbTreeState {
    fn new() -> Self {
        // make sure the origin is pinned no later than the first orb
        let _ = clock_origin();
        Self {
            id: gravity::next_id(),
            frozen: Cell::new(None),
            registered: false,
        }
    }
}

impl Drop for OrbTreeState {
    fn drop(&mut self) {
        if self.registered {
            gravity::unregister(self.id);
        }
    }
}

impl ThinkingOrb {
    fn px(&self) -> f32 {
        f32::from(self.size.px())
    }

    fn gravity_enabled(&self) -> bool {
        self.gravity.is_some() && self.motion == MotionPreference::Full
    }

    /// Whether the orb moves on its own clock and so needs redraws.
    fn animating(&self) -> bool {
        !self.paused && self.motion == MotionPreference::Full && self.elapsed.is_none()
    }

    fn is_dark(&self, theme: &Theme) -> bool {
        match self.theme {
            OrbTheme::Auto => theme.extended_palette().is_dark,
            OrbTheme::Dark => true,
            OrbTheme::Light => false,
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for ThinkingOrb {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<OrbTreeState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(OrbTreeState::new())
    }

    fn size(&self) -> Size<Length> {
        let px = self.px();
        Size::new(Length::Fixed(px), Length::Fixed(px))
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let px = Length::Fixed(self.px());
        layout::atomic(limits, px, px)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<OrbTreeState>();
        let bounds = layout.bounds();

        match event {
            Event::Window(window::Event::RedrawRequested(now)) => {
                match self.gravity {
                    Some(opts) if self.gravity_enabled() => {
                        gravity::register(state.id, bounds, opts);
                        state.registered = true;
                        gravity::tick(clock_seconds(*now), cursor.position());
                    }
                    _ if state.registered => {
                        gravity::unregister(state.id);
                        state.registered = false;
                    }
                    _ => {}
                }

                // Offscreen orbs idle, like the reference's IntersectionObserver.
                let animates = self.animating() && bounds.intersects(viewport);
                if animates || (state.registered && gravity::wants_frames()) {
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::CursorLeft)
                if self.gravity_enabled() =>
            {
                gravity::pointer_moved();
                shell.request_redraw();
            }
            Event::Keyboard(keyboard::Event::KeyPressed { .. }) if self.gravity_enabled() => {
                gravity::key_pressed();
                shell.request_redraw();
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<OrbTreeState>();
        if state.registered && gravity::hides_pointer() {
            // `Hidden` is the weakest interaction: a button's hand still wins.
            mouse::Interaction::Hidden
        } else {
            mouse::Interaction::None
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if bounds.width < 1.0 || bounds.height < 1.0 || !bounds.intersects(viewport) {
            return;
        }
        let state = tree.state.downcast_ref::<OrbTreeState>();

        let now = self
            .elapsed
            .map_or_else(|| clock_seconds(Instant::now()), |e| e.as_secs_f64());
        let clock = if self.paused && self.motion == MotionPreference::Full {
            match state.frozen.get() {
                Some(frozen) => frozen,
                None => {
                    state.frozen.set(Some(now));
                    now
                }
            }
        } else {
            state.frozen.set(None);
            now
        };

        let frame = self.frame_at(self.engine_time(clock));
        let bitmap = soft::render(
            &frame,
            self.px(),
            self.pixel_density,
            self.is_dark(theme),
            self.color,
        );
        let filter = if (self.pixel_density - 1.0).abs() < f32::EPSILON {
            image::FilterMethod::Nearest
        } else {
            image::FilterMethod::Linear
        };
        let handle = image::Handle::from_rgba(bitmap.side, bitmap.side, bitmap.rgba);
        renderer.draw_image(
            image::Image::new(handle).filter_method(filter).snap(true),
            Rectangle::new(bounds.position(), Size::new(self.px(), self.px())),
            bounds,
        );
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        _layout: Layout<'a>,
        _renderer: &Renderer,
        _viewport: &Rectangle,
        _translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, Renderer>> {
        if !self.gravity_enabled() {
            return None;
        }
        let id = tree.state.downcast_ref::<OrbTreeState>().id;
        Some(overlay::Element::new(Box::new(SpriteOverlay { id })))
    }
}

/// Draws the bent pointer above everything else, when this orb owns the pull.
struct SpriteOverlay {
    id: u64,
}

impl<Message> overlay::Overlay<Message, Theme, Renderer> for SpriteOverlay {
    fn layout(&mut self, _renderer: &Renderer, bounds: Size) -> layout::Node {
        layout::Node::new(bounds)
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
    ) {
        let Some(frame) = gravity::frame_for(self.id) else {
            return;
        };
        let clip = layout.bounds();
        renderer.with_layer(clip, |renderer| {
            renderer.draw_image(
                image::Image::new(frame.handle).snap(true),
                frame.bounds,
                clip,
            );
        });
    }
}
