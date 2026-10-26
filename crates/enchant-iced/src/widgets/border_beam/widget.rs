//! The [`BorderBeam`] iced widget: fade lifecycle, animation clock and drawing.

use std::cell::RefCell;
use std::fmt;
use std::time::{Duration, Instant};

use iced::advanced::image::{self, Renderer as _};
use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{self, Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse, window};

use super::frame::BorderBeamFrame;
use super::options::{BorderBeamOptions, FADE_IN, FADE_OUT};
use super::palette::{
    BeamHost, BorderBeamPalette, BorderBeamSize, BorderBeamTheme, ResolvedBeamTheme,
};
use super::raster::{BeamBitmap, BorderBeamRaster};

/// Wraps a child in a Libraries.dev-style animated border beam.
///
/// The widget owns the animation clock in its tree state, steps it only while
/// visible and active, rasterizes one frame per redraw and asks for another
/// redraw only while the effect is on. Fades follow the reference: 0.6s in,
/// 0.5s out, with `ease` timing. With [`reduced_motion`](Self::reduced_motion)
/// they snap instead and the frame freezes at a representative phase.
///
/// iced cannot inspect the child's corner radius, so pass it with
/// [`radius`](Self::radius) (the size preset's default is used otherwise).
///
/// # Examples
///
/// ```rust
/// use enchant_iced::{
///     BorderBeam, BorderBeamPalette, BorderBeamSize, BorderBeamTheme,
/// };
/// use iced::widget::container;
/// use iced::{Element, Length};
///
/// #[derive(Clone)]
/// enum Message {
///     Lit,
///     Dark,
/// }
///
/// fn view<'a>() -> Element<'a, Message> {
///     BorderBeam::new(
///         container("Deploy")
///             .width(Length::Fixed(348.0))
///             .height(Length::Fixed(122.0)),
///     )
///     .size(BorderBeamSize::PulseOutside)
///     .palette(BorderBeamPalette::Ocean)
///     .theme(BorderBeamTheme::Auto)
///     .radius(20.0)
///     .strength(0.85)
///     .on_activate(|| Message::Lit)
///     .on_deactivate(|| Message::Dark)
///     .into()
/// }
/// # let _ = view;
/// ```
#[must_use = "widgets do nothing unless turned into an iced Element"]
pub struct BorderBeam<'a, Message> {
    options: BorderBeamOptions,
    child: Element<'a, Message>,
    pixel_density: f32,
    activate_message: Option<Box<dyn Fn() -> Message + 'a>>,
    deactivate_message: Option<Box<dyn Fn() -> Message + 'a>>,
}

impl<Message> fmt::Debug for BorderBeam<'_, Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BorderBeam")
            .field("options", &self.options)
            .field("pixel_density", &self.pixel_density)
            .finish_non_exhaustive()
    }
}

impl<'a, Message> BorderBeam<'a, Message> {
    /// Wraps `child`.
    pub fn new(child: impl Into<Element<'a, Message>>) -> Self {
        Self {
            options: BorderBeamOptions::new(),
            child: child.into(),
            pixel_density: 1.0,
            activate_message: None,
            deactivate_message: None,
        }
    }

    /// Sets the size preset.
    pub fn size(mut self, size: BorderBeamSize) -> Self {
        self.options = self.options.size(size);
        self
    }

    /// Sets the palette (reference `colorVariant`).
    pub fn palette(mut self, palette: BorderBeamPalette) -> Self {
        self.options = self.options.palette(palette);
        self
    }

    /// Alias for [`Self::palette`].
    pub fn color_variant(self, palette: BorderBeamPalette) -> Self {
        self.palette(palette)
    }

    /// Sets the light/dark preset.
    pub fn theme(mut self, theme: BorderBeamTheme) -> Self {
        self.options = self.options.theme(theme);
        self
    }

    /// Freezes hue rotation.
    pub fn static_colors(mut self, static_colors: bool) -> Self {
        self.options = self.options.static_colors(static_colors);
        self
    }

    /// Sets the loop duration.
    pub fn duration(mut self, duration: Duration) -> Self {
        self.options = self.options.duration(duration);
        self
    }

    /// Restores the size-dependent default duration.
    pub fn reset_duration(mut self) -> Self {
        self.options = self.options.reset_duration();
        self
    }

    /// Turns the effect on or off (animated fade).
    pub fn active(mut self, active: bool) -> Self {
        self.options = self.options.active(active);
        self
    }

    /// Sets the child's corner radius.
    pub fn radius(mut self, radius: f32) -> Self {
        self.options = self.options.radius(radius);
        self
    }

    /// Alias for [`Self::radius`], matching the reference `borderRadius`.
    pub fn border_radius(self, radius: f32) -> Self {
        self.radius(radius)
    }

    /// Restores the size-dependent default radius.
    pub fn reset_radius(mut self) -> Self {
        self.options = self.options.reset_radius();
        self
    }

    /// Sets the ring thickness (`pulse-outside` always uses 1px).
    pub fn border_width(mut self, width: f32) -> Self {
        self.options = self.options.border_width(width);
        self
    }

    /// Restores the default 1px ring.
    pub fn reset_border_width(mut self) -> Self {
        self.options = self.options.reset_border_width();
        self
    }

    /// Overrides the theme brightness.
    pub fn brightness(mut self, brightness: f32) -> Self {
        self.options = self.options.brightness(brightness);
        self
    }

    /// Restores the theme brightness.
    pub fn reset_brightness(mut self) -> Self {
        self.options = self.options.reset_brightness();
        self
    }

    /// Overrides the theme saturation.
    pub fn saturation(mut self, saturation: f32) -> Self {
        self.options = self.options.saturation(saturation);
        self
    }

    /// Restores the theme saturation.
    pub fn reset_saturation(mut self) -> Self {
        self.options = self.options.reset_saturation();
        self
    }

    /// Hue ping-pong range in degrees (line is capped at 13).
    pub fn hue_range(mut self, degrees: f32) -> Self {
        self.options = self.options.hue_range(degrees);
        self
    }

    /// Multiplies glow blur radii.
    pub fn glow_size(mut self, scale: f32) -> Self {
        self.options = self.options.glow_size(scale);
        self
    }

    /// Layer opacity multiplier in `0..=1`.
    pub fn strength(mut self, strength: f32) -> Self {
        self.options = self.options.strength(strength);
        self
    }

    /// Freezes the animation at a representative phase.
    pub fn reduced_motion(mut self, reduced: bool) -> Self {
        self.options = self.options.reduced_motion(reduced);
        self
    }

    /// Device pixels per logical pixel of the raster (default 1; use 2 on a
    /// 2x display for crisp 1px rings).
    pub fn pixel_density(mut self, density: f32) -> Self {
        self.pixel_density = clean_density(density);
        self
    }

    /// Publishes a message after the 0.6s fade-in completes.
    pub fn on_activate(mut self, f: impl Fn() -> Message + 'a) -> Self {
        self.activate_message = Some(Box::new(f));
        self
    }

    /// Publishes a message after the 0.5s fade-out completes.
    pub fn on_deactivate(mut self, f: impl Fn() -> Message + 'a) -> Self {
        self.deactivate_message = Some(Box::new(f));
        self
    }

    /// The current options (useful for headless rendering of the same style).
    ///
    /// Call [`BorderBeamOptions::spec`] to read the resolved light/dark values.
    pub fn options(&self) -> &BorderBeamOptions {
        &self.options
    }
}

impl<'a, Message: 'a> From<BorderBeam<'a, Message>> for Element<'a, Message> {
    fn from(beam: BorderBeam<'a, Message>) -> Self {
        Element::new(beam)
    }
}

fn clean_density(density: f32) -> f32 {
    if density.is_finite() {
        density.clamp(0.5, 4.0)
    } else {
        1.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Note {
    Activate,
    Deactivate,
}

#[derive(Clone)]
pub(crate) struct LayerImage {
    handle: image::Handle,
    width: u32,
    height: u32,
    left: f32,
    top: f32,
    density: f32,
}

impl LayerImage {
    pub(crate) fn new(bitmap: BeamBitmap) -> Self {
        let (width, height) = (bitmap.width(), bitmap.height());
        let (left, top, density) = (bitmap.left(), bitmap.top(), bitmap.density());
        Self {
            handle: image::Handle::from_rgba(width, height, bitmap.into_rgba()),
            width,
            height,
            left,
            top,
            density,
        }
    }

    pub(crate) fn destination(&self, bounds: Rectangle) -> Rectangle {
        self.destination_at(bounds.position())
    }

    pub(crate) fn destination_at(&self, origin: iced::Point) -> Rectangle {
        Rectangle {
            x: origin.x + self.left,
            y: origin.y + self.top,
            width: self.width as f32 / self.density,
            height: self.height as f32 / self.density,
        }
    }

    pub(crate) fn handle(&self) -> &image::Handle {
        &self.handle
    }
}

struct Cache {
    frame: BorderBeamFrame,
    opacity_bits: u32,
    density_bits: u32,
    above: Option<LayerImage>,
    behind: Option<LayerImage>,
}

struct Inner {
    options: Option<BorderBeamOptions>,
    system: ResolvedBeamTheme,
    clock: f64,
    last: Option<Instant>,
    active: bool,
    fading: bool,
    fade_start: f64,
    fade_value: f32,
    fade_announced: bool,
    frame: Option<BorderBeamFrame>,
    raster: BorderBeamRaster,
    cache: Option<Cache>,
}

impl Inner {
    fn new() -> Self {
        Self {
            options: None,
            system: ResolvedBeamTheme::Dark,
            clock: 0.0,
            last: None,
            active: false,
            fading: false,
            fade_start: 0.0,
            fade_value: 0.0,
            fade_announced: false,
            frame: None,
            raster: BorderBeamRaster::new(),
            cache: None,
        }
    }

    fn invalidate(&mut self) {
        self.frame = None;
        self.cache = None;
    }
}

/// Per-widget state that outlives `view` calls.
struct BeamState {
    inner: RefCell<Inner>,
}

fn intersects_expanded(bounds: Rectangle, viewport: &Rectangle, margin: f32) -> bool {
    bounds.intersects(&Rectangle {
        x: viewport.x - margin,
        y: viewport.y - margin,
        width: viewport.width + 2.0 * margin,
        height: viewport.height + 2.0 * margin,
    })
}

fn system_theme(theme: &Theme) -> ResolvedBeamTheme {
    if theme.extended_palette().is_dark {
        ResolvedBeamTheme::Dark
    } else {
        ResolvedBeamTheme::Light
    }
}

impl<Message> BorderBeam<'_, Message> {
    /// Brings the fade state machine in line with the current props.
    fn sync_options(&self, inner: &mut Inner) {
        if inner.options.as_ref() == Some(&self.options) {
            return;
        }
        let first = inner.options.is_none();
        let target_active = self.options.active;
        inner.options = Some(self.options.clone());
        inner.invalidate();

        if self.options.reduced_motion {
            let was_active = inner.active && !inner.fading;
            if first || inner.fading || was_active != target_active {
                inner.active = target_active;
                inner.fading = false;
                inner.fade_value = if target_active { 1.0 } else { 0.0 };
                inner.fade_announced = first && !target_active;
            }
            if first {
                inner.fade_start = 0.0;
            }
            return;
        }

        if first {
            inner.active = target_active;
            inner.fading = false;
            inner.fade_start = 0.0;
            inner.fade_value = 0.0;
            inner.fade_announced = false;
            return;
        }

        if target_active {
            if !inner.active && !inner.fading {
                inner.active = true;
                inner.fade_start = inner.clock;
                inner.fade_value = 0.0;
                inner.fade_announced = false;
            }
        } else if inner.active && !inner.fading {
            inner.fading = true;
            inner.fade_start = inner.clock;
            inner.fade_value = 1.0;
        }
    }

    fn sync_system(&self, inner: &mut Inner, system: ResolvedBeamTheme) {
        if inner.system != system {
            inner.system = system;
            inner.invalidate();
        }
    }

    /// Advances the fade state; returns a callback to publish exactly once.
    fn advance_fade(&self, inner: &mut Inner) -> Option<Note> {
        if self.options.reduced_motion {
            if inner.active {
                inner.fade_value = 1.0;
                if !inner.fade_announced {
                    inner.fade_announced = true;
                    return Some(Note::Activate);
                }
            } else {
                inner.fade_value = 0.0;
                if !inner.fade_announced {
                    inner.fade_announced = true;
                    return Some(Note::Deactivate);
                }
            }
            return None;
        }
        if inner.fading {
            let progress = ((inner.clock - inner.fade_start) / FADE_OUT.as_secs_f64()) as f32;
            if progress >= 1.0 {
                if self.options.active {
                    // The fade-out completed, so report it (the reference
                    // fires `onDeactivate` before starting the new fade-in)
                    // and immediately hand over to a fade-in.
                    inner.active = true;
                    inner.fading = false;
                    inner.fade_start = inner.clock;
                    inner.fade_value = 0.0;
                    inner.fade_announced = false;
                    inner.invalidate();
                    return Some(Note::Deactivate);
                }
                inner.fading = false;
                inner.active = false;
                inner.fade_value = 0.0;
                inner.clock = 0.0;
                inner.last = None;
                inner.fade_announced = false;
                inner.invalidate();
                return Some(Note::Deactivate);
            }
            inner.fade_value = 1.0 - super::frame::css_ease(progress.clamp(0.0, 1.0));
            return None;
        }
        if inner.active {
            let progress = ((inner.clock - inner.fade_start) / FADE_IN.as_secs_f64()) as f32;
            if progress >= 1.0 {
                inner.fade_value = 1.0;
                if !inner.fade_announced {
                    inner.fade_announced = true;
                    return Some(Note::Activate);
                }
            } else {
                inner.fade_value = super::frame::css_ease(progress.clamp(0.0, 1.0));
            }
            return None;
        }
        inner.fade_value = 0.0;
        None
    }

    fn rasterize(
        &self,
        inner: &mut Inner,
        host: BeamHost,
    ) -> (Option<LayerImage>, Option<LayerImage>) {
        if inner.fade_value <= 0.0 {
            return (None, None);
        }
        let Some(options) = inner.options.as_ref() else {
            return (None, None);
        };
        let frame = options.frame_at(
            Duration::from_secs_f64(inner.clock.max(0.0)),
            host,
            inner.system,
        );
        let opacity_bits = inner.fade_value.to_bits();
        let density_bits = self.pixel_density.to_bits();
        if let Some(cache) = &inner.cache
            && cache.frame == frame
            && cache.opacity_bits == opacity_bits
            && cache.density_bits == density_bits
        {
            return (cache.behind.clone(), cache.above.clone());
        }
        let layers = inner
            .raster
            .render(&frame, inner.fade_value, self.pixel_density);
        let (above, behind) = layers.into_parts();
        let above = above.map(LayerImage::new);
        let behind = behind.map(LayerImage::new);
        inner.cache = Some(Cache {
            frame,
            opacity_bits,
            density_bits,
            above: above.clone(),
            behind: behind.clone(),
        });
        (behind, above)
    }

    fn child_layout(layout: Layout<'_>) -> Layout<'_> {
        layout.children().next().expect("border beam child layout")
    }
}

impl<Message> Widget<Message, Theme, Renderer> for BorderBeam<'_, Message> {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<BeamState>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(BeamState {
            inner: RefCell::new(Inner::new()),
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
        let node = self
            .child
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        let size = node.size();
        layout::Node::with_children(size, vec![node])
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
        self.sync_options(&mut inner);

        let visible = bounds.intersects(viewport);
        // The reference starts animating 256px before the element enters the
        // viewport (IntersectionObserver rootMargin), so the first visible
        // frame is not phase zero.
        let nearby = intersects_expanded(bounds, viewport, 256.0);
        let running = inner.active || inner.fading;
        // A fade-out started onscreen must finish even if the widget scrolls
        // away, otherwise `on_deactivate` never fires.
        if (nearby || inner.fading) && running {
            let dt = inner.last.map_or(0.0, |last| {
                now.saturating_duration_since(last).as_secs_f64()
            });
            inner.last = Some(*now);
            inner.clock += dt;
        } else {
            inner.last = None;
        }
        let note = self.advance_fade(&mut inner);
        if visible && (inner.active || inner.fading) {
            let host = BeamHost::new(bounds.width, bounds.height);
            let _ = self.rasterize(&mut inner, host);
        }
        let fading = inner.fading;
        let steady_pulse = self.options.size.is_pulse()
            && inner.active
            && !inner.fading
            && inner.fade_value >= 1.0;
        drop(inner);

        match note {
            Some(Note::Activate) => {
                if let Some(f) = &self.activate_message {
                    shell.publish(f());
                }
            }
            Some(Note::Deactivate) => {
                if let Some(f) = &self.deactivate_message {
                    shell.publish(f());
                }
            }
            None => {}
        }
        if (nearby || fading) && running && !self.options.reduced_motion {
            // The reference throttles the pulse driver to ~30 fps once the
            // fade has settled; fades themselves stay at the display rate.
            if steady_pulse {
                shell.request_redraw_at(*now + Duration::from_millis(33));
            } else {
                shell.request_redraw();
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
        if bounds.width < 1.0 || bounds.height < 1.0 {
            draw_child(renderer);
            return;
        }

        let state = tree.state.downcast_ref::<BeamState>();
        let (behind, above) = {
            let mut inner = state.inner.borrow_mut();
            self.sync_system(&mut inner, system_theme(theme));
            self.sync_options(&mut inner);
            if bounds.intersects(viewport) {
                let host = BeamHost::new(bounds.width, bounds.height);
                self.rasterize(&mut inner, host)
            } else {
                (None, None)
            }
        };

        if let Some(layer) = behind {
            draw_image(renderer, &layer, bounds);
        }
        draw_child(renderer);
        if let Some(layer) = above {
            draw_image(renderer, &layer, bounds);
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

fn draw_image(renderer: &mut Renderer, layer: &LayerImage, bounds: Rectangle) {
    let filter = if (layer.density - 1.0).abs() < f32::EPSILON {
        image::FilterMethod::Nearest
    } else {
        image::FilterMethod::Linear
    };
    // `destination` is the full bitmap: the child bounds for the above layer,
    // the expanded halo for `pulse-outside`'s behind layer. Clipping to the
    // child bounds would cut that halo off.
    let destination = layer.destination(bounds);
    renderer.draw_image(
        image::Image::new(layer.handle.clone())
            .filter_method(filter)
            .snap(true),
        destination,
        destination,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::container;

    fn beam(active: bool, reduced: bool) -> BorderBeam<'static, ()> {
        BorderBeam::new(container("child"))
            .active(active)
            .reduced_motion(reduced)
    }

    #[test]
    fn reduced_motion_snaps_in_and_out_without_animating() {
        let mut inner = Inner::new();
        let on = beam(true, true);
        on.sync_options(&mut inner);
        assert!(inner.active && !inner.fading);
        assert_eq!(inner.fade_value, 1.0);
        assert_eq!(on.advance_fade(&mut inner), Some(Note::Activate));
        assert_eq!(on.advance_fade(&mut inner), None);

        let off = beam(false, true);
        off.sync_options(&mut inner);
        assert!(!inner.active && !inner.fading);
        assert_eq!(inner.fade_value, 0.0);
        assert_eq!(off.advance_fade(&mut inner), Some(Note::Deactivate));
        assert_eq!(off.advance_fade(&mut inner), None);
    }

    #[test]
    fn reactivating_during_fade_out_restarts_fade_in() {
        let mut inner = Inner::new();
        let on = beam(true, false);
        on.sync_options(&mut inner);
        inner.clock = 1.0;
        assert_eq!(on.advance_fade(&mut inner), Some(Note::Activate));

        let off = beam(false, false);
        off.sync_options(&mut inner);
        inner.clock = 1.25;
        assert_eq!(off.advance_fade(&mut inner), None);
        let mid = inner.fade_value;
        assert!(mid > 0.0 && mid < 1.0, "mid fade {mid}");

        on.sync_options(&mut inner);
        inner.clock = 1.6;
        assert_eq!(on.advance_fade(&mut inner), Some(Note::Deactivate));
        assert!(inner.active && !inner.fading);
        assert_eq!(inner.fade_value, 0.0);

        inner.clock = 2.2;
        assert_eq!(on.advance_fade(&mut inner), Some(Note::Activate));
        assert_eq!(inner.fade_value, 1.0);
    }

    #[test]
    fn reduced_motion_snaps_a_running_fade_out() {
        let mut inner = Inner::new();
        let off = beam(false, false);
        off.sync_options(&mut inner);
        inner.fading = true;
        inner.active = true;
        inner.fade_value = 1.0;

        let reduced = beam(false, true);
        reduced.sync_options(&mut inner);
        assert!(!inner.active && !inner.fading);
        assert_eq!(inner.fade_value, 0.0);
        assert_eq!(reduced.advance_fade(&mut inner), Some(Note::Deactivate));
    }
}
