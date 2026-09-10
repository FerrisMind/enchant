//! Builder-first circular magnifier lens (Magic UI `Lens`).
//!
//! Pointer + enter/exit visibility live in host-owned [`LensState`]. Draw path:
//! base content once; when visible, redraw magnified content about the cursor with a
//! circular clip approximated by horizontal chord strips (`with_layer` per strip).
//! Enter/exit scales the **mask diameter** via [`LensState::overlay_scale`]; content
//! zoom stays [`Lens::zoom_factor`]. Intermediate opacity is approximated by splitting
//! each strip between zoomed and base (unzoomed) passes.

use iced::advanced::Renderer as _;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{self, Tree, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::keyboard;
use iced::mouse;
use iced::time::Instant;
use iced::widget::{container, mouse_area};
use iced::{
    Background, Border, Color, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme,
    Transformation,
};
use std::fmt;

/// Cap on horizontal chord strips approximating the circular mask.
const LENS_STRIP_MAX: usize = 170;
/// Minimum strip height in logical pixels.
const LENS_STRIP_MIN_STEP: f32 = 1.0;
/// Maximum strip height in logical pixels.
const LENS_STRIP_MAX_STEP: f32 = 8.0;

const DEFAULT_ZOOM_FACTOR: f32 = 1.3;
const DEFAULT_LENS_SIZE: f32 = 170.0;
const DEFAULT_DURATION_SECS: f32 = 0.1;
const DEFAULT_BORDER_RADIUS: f32 = 12.0;
const ENTER_SCALE_FROM: f32 = 0.58;
const ENTER_SCALE_TO: f32 = 1.0;
const EXIT_SCALE_TO: f32 = 0.8;

/// Pointer / keyboard messages for the lens host.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LensAction {
    Move(Point),
    Enter,
    Exit,
    Dismiss,
}

/// Enter/exit presentation phase for the magnifier overlay.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LensVisibility {
    #[default]
    Hidden,
    Entering,
    Shown,
    Exiting,
}

/// Host-owned pointer + animation state for a lens instance.
#[derive(Clone, Debug)]
pub struct LensState {
    hovered: bool,
    /// Last pointer from [`LensAction::Move`] — retained after Exit/Dismiss so the
    /// exit overlay can keep drawing at that point while `hovered` is false.
    pointer: Option<Point>,
    dismissed: bool,
    visibility: LensVisibility,
    /// Progress within the current enter/exit phase, in `[0, 1]` (eased for presentation).
    anim_t: f32,
    phase_started: Option<Instant>,
    /// Opacity lerp endpoints for the active phase (`ease(anim_t)` blends between them).
    opacity_from: f32,
    opacity_to: f32,
    /// Scale lerp endpoints for the active phase.
    scale_from: f32,
    scale_to: f32,
}

impl Default for LensState {
    fn default() -> Self {
        Self {
            hovered: false,
            pointer: None,
            dismissed: false,
            visibility: LensVisibility::Hidden,
            anim_t: 0.0,
            phase_started: None,
            opacity_from: 0.0,
            opacity_to: 1.0,
            scale_from: ENTER_SCALE_FROM,
            scale_to: ENTER_SCALE_TO,
        }
    }
}

impl LensState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_action(&mut self, action: &LensAction) {
        match action {
            LensAction::Move(position) => {
                self.hovered = true;
                self.pointer = Some(*position);
                self.dismissed = false;
            }
            LensAction::Enter => {
                self.hovered = true;
                self.dismissed = false;
            }
            LensAction::Exit => {
                self.hovered = false;
            }
            LensAction::Dismiss => {
                self.dismissed = true;
                self.hovered = false;
            }
        }
    }

    /// Advance enter/exit animation — call every frame with a fresh [`Instant`].
    pub fn tick<Message>(&mut self, now: Instant, lens: &Lens<'_, Message>) {
        let want = lens_should_show(self, lens);

        match self.visibility {
            LensVisibility::Hidden => {
                if want {
                    self.begin_phase(
                        LensVisibility::Entering,
                        now,
                        0.0,
                        1.0,
                        ENTER_SCALE_FROM,
                        ENTER_SCALE_TO,
                    );
                }
            }
            LensVisibility::Entering => {
                if !want {
                    // Reverse into exit from current visual values (no opacity/scale pop).
                    let opacity = self.overlay_opacity();
                    let scale = self.overlay_scale();
                    self.begin_phase(
                        LensVisibility::Exiting,
                        now,
                        opacity,
                        0.0,
                        scale,
                        EXIT_SCALE_TO,
                    );
                } else {
                    self.advance_anim(now, lens.duration_secs);
                    if self.anim_t >= 1.0 {
                        self.visibility = LensVisibility::Shown;
                        self.anim_t = 1.0;
                        self.opacity_from = 1.0;
                        self.opacity_to = 1.0;
                        self.scale_from = ENTER_SCALE_TO;
                        self.scale_to = ENTER_SCALE_TO;
                    }
                }
            }
            LensVisibility::Shown => {
                if !want {
                    self.begin_phase(
                        LensVisibility::Exiting,
                        now,
                        1.0,
                        0.0,
                        ENTER_SCALE_TO,
                        EXIT_SCALE_TO,
                    );
                }
            }
            LensVisibility::Exiting => {
                if want {
                    let opacity = self.overlay_opacity();
                    let scale = self.overlay_scale();
                    self.begin_phase(
                        LensVisibility::Entering,
                        now,
                        opacity,
                        1.0,
                        scale,
                        ENTER_SCALE_TO,
                    );
                } else {
                    self.advance_anim(now, lens.duration_secs);
                    if self.anim_t >= 1.0 {
                        self.visibility = LensVisibility::Hidden;
                        self.anim_t = 0.0;
                        self.opacity_from = 0.0;
                        self.opacity_to = 1.0;
                        self.scale_from = ENTER_SCALE_FROM;
                        self.scale_to = ENTER_SCALE_TO;
                    }
                }
            }
        }
    }

    fn begin_phase(
        &mut self,
        phase: LensVisibility,
        now: Instant,
        opacity_from: f32,
        opacity_to: f32,
        scale_from: f32,
        scale_to: f32,
    ) {
        self.visibility = phase;
        self.anim_t = 0.0;
        self.phase_started = Some(now);
        self.opacity_from = opacity_from;
        self.opacity_to = opacity_to;
        self.scale_from = scale_from;
        self.scale_to = scale_to;
    }

    fn advance_anim(&mut self, now: Instant, duration_secs: f32) {
        let duration = duration_secs.max(0.01);
        let started = self.phase_started.unwrap_or(now);
        self.anim_t = (now.saturating_duration_since(started).as_secs_f32() / duration).clamp(0.0, 1.0);
    }

    pub fn visibility(&self) -> LensVisibility {
        self.visibility
    }

    pub fn anim_t(&self) -> f32 {
        self.anim_t
    }

    /// Overlay opacity for the current phase (`0` hidden → `1` fully shown).
    pub fn overlay_opacity(&self) -> f32 {
        match self.visibility {
            LensVisibility::Hidden => 0.0,
            LensVisibility::Shown => 1.0,
            LensVisibility::Entering | LensVisibility::Exiting => {
                lerp(self.opacity_from, self.opacity_to, ease(self.anim_t))
            }
        }
    }

    /// Overlay enter/exit scale (`0.58→1` enter, `1→0.8` exit; continuous on reverse).
    pub fn overlay_scale(&self) -> f32 {
        match self.visibility {
            LensVisibility::Hidden => ENTER_SCALE_FROM,
            LensVisibility::Shown => ENTER_SCALE_TO,
            LensVisibility::Entering | LensVisibility::Exiting => {
                lerp(self.scale_from, self.scale_to, ease(self.anim_t))
            }
        }
    }

    /// Whether the host should keep requesting frames (hover follow or animating).
    pub fn needs_redraw(&self) -> bool {
        matches!(
            self.visibility,
            LensVisibility::Entering | LensVisibility::Exiting
        ) || (self.hovered && !self.dismissed)
    }
}

/// Builder for a Magic UI circular magnifier over arbitrary child content.
///
/// # Examples
///
/// ```rust,no_run
/// use enchant_iced::{Lens, LensState};
/// use iced::Element;
/// use iced::widget::text;
///
/// fn view<'a, Message: Clone + 'a + 'static>(
///     state: &'a LensState,
/// ) -> Element<'a, Message> {
///     Lens::new(text("photo"))
///         .zoom_factor(2.0)
///         .lens_size(150.0)
///         .into_element(state, |_| unreachable!())
/// }
/// ```
#[must_use = "builders do nothing unless turned into an iced Element"]
pub struct Lens<'a, Message> {
    content: Element<'a, Message>,
    zoom_factor: f32,
    lens_size: f32,
    position: Option<Point>,
    default_position: Option<Point>,
    is_static: bool,
    duration_secs: f32,
    lens_color: Color,
    border_radius: f32,
    width: Length,
    height: Length,
}

impl<Message> fmt::Debug for Lens<'_, Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Lens")
            .field("content", &"<element>")
            .field("zoom_factor", &self.zoom_factor)
            .field("lens_size", &self.lens_size)
            .field("position", &self.position)
            .field("default_position", &self.default_position)
            .field("is_static", &self.is_static)
            .field("duration_secs", &self.duration_secs)
            .field("lens_color", &self.lens_color)
            .field("border_radius", &self.border_radius)
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

impl<'a, Message> Lens<'a, Message> {
    /// Creates a lens wrapping `content` with Magic UI defaults.
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            zoom_factor: DEFAULT_ZOOM_FACTOR,
            lens_size: DEFAULT_LENS_SIZE,
            position: None,
            default_position: None,
            is_static: false,
            duration_secs: DEFAULT_DURATION_SECS,
            lens_color: Color::BLACK,
            border_radius: DEFAULT_BORDER_RADIUS,
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    /// Magnification scale (default `1.3`, clamped to `≥ 1.01`).
    pub fn zoom_factor(mut self, zoom_factor: f32) -> Self {
        self.zoom_factor = zoom_factor.max(1.01);
        self
    }

    /// Diameter of the circular mask in logical pixels (default `170`, clamped to `≥ 1`).
    pub fn lens_size(mut self, lens_size: f32) -> Self {
        self.lens_size = lens_size.max(1.0);
        self
    }

    /// Controlled / static lens point (relative to the widget origin).
    pub fn position(mut self, position: Point) -> Self {
        self.position = Some(position);
        self
    }

    /// Clears a previously set [`Self::position`].
    pub fn clear_position(mut self) -> Self {
        self.position = None;
        self
    }

    /// Resting lens point when not hovering (also keeps the lens present).
    pub fn default_position(mut self, position: Point) -> Self {
        self.default_position = Some(position);
        self
    }

    /// Clears a previously set [`Self::default_position`].
    pub fn clear_default_position(mut self) -> Self {
        self.default_position = None;
        self
    }

    /// When `true`, the lens stays visible at [`Self::position`] (or center).
    pub fn is_static(mut self, is_static: bool) -> Self {
        self.is_static = is_static;
        self
    }

    /// Enter/exit animation duration in seconds (default `0.1`, clamped to `≥ 0.01`).
    pub fn duration_secs(mut self, duration_secs: f32) -> Self {
        self.duration_secs = duration_secs.max(0.01);
        self
    }

    /// Mask strength color — alpha is mask opacity (default opaque black).
    pub fn lens_color(mut self, lens_color: Color) -> Self {
        self.lens_color = lens_color;
        self
    }

    /// Container corner radius (default `12`, Magic UI `rounded-xl`).
    pub fn border_radius(mut self, border_radius: f32) -> Self {
        self.border_radius = border_radius.max(0.0);
        self
    }

    /// Outer width (default [`Length::Shrink`]).
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Outer height (default [`Length::Shrink`]).
    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    /// Read-only config snapshot (setters reclaim C-BUILDER short names).
    pub fn values(&self) -> LensValues {
        LensValues {
            zoom_factor: self.zoom_factor,
            lens_size: self.lens_size,
            position: self.position,
            default_position: self.default_position,
            is_static: self.is_static,
            duration_secs: self.duration_secs,
            lens_color: self.lens_color,
            border_radius: self.border_radius,
            width: self.width,
            height: self.height,
        }
    }

    /// Builds the lens element for the given host state and action mapper.
    pub fn into_element(
        self,
        state: &'a LensState,
        on_action: impl Fn(LensAction) -> Message + Clone + 'a + 'static,
    ) -> Element<'a, Message>
    where
        Message: Clone + 'a + 'static,
    {
        let radius = self.border_radius;
        let width = self.width;
        let height = self.height;
        let active = lens_present(state);

        let widget = LensWidget {
            content: self.content,
            state,
            zoom_factor: self.zoom_factor,
            lens_size: self.lens_size,
            position: self.position,
            default_position: self.default_position,
            is_static: self.is_static,
            lens_color: self.lens_color,
            border_radius: self.border_radius,
            overlay_opacity: state.overlay_opacity(),
            overlay_scale: state.overlay_scale(),
            on_action: on_action.clone(),
        };

        let body = container(widget)
            .width(width)
            .height(height)
            .clip(true)
            .style(move |_theme: &Theme| container::Style {
                background: Some(Background::Color(Color::TRANSPARENT)),
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: radius.into(),
                },
                ..container::Style::default()
            });

        mouse_area(body)
            .on_move({
                let on_action = on_action.clone();
                move |position| on_action(LensAction::Move(position))
            })
            .on_enter(on_action(LensAction::Enter))
            .on_exit(on_action(LensAction::Exit))
            .interaction(if active {
                mouse::Interaction::ZoomIn
            } else {
                mouse::Interaction::Pointer
            })
            .into()
    }

    /// Alias for [`Self::into_element`].
    pub fn view(
        self,
        state: &'a LensState,
        on_action: impl Fn(LensAction) -> Message + Clone + 'a + 'static,
    ) -> Element<'a, Message>
    where
        Message: Clone + 'a + 'static,
    {
        self.into_element(state, on_action)
    }
}

/// Read-only copy of [`Lens`] configuration (see [`Lens::values`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LensValues {
    pub zoom_factor: f32,
    pub lens_size: f32,
    pub position: Option<Point>,
    pub default_position: Option<Point>,
    pub is_static: bool,
    pub duration_secs: f32,
    pub lens_color: Color,
    pub border_radius: f32,
    pub width: Length,
    pub height: Length,
}

struct LensWidget<'a, Message, F>
where
    F: Fn(LensAction) -> Message + Clone + 'a,
{
    content: Element<'a, Message>,
    state: &'a LensState,
    zoom_factor: f32,
    lens_size: f32,
    position: Option<Point>,
    default_position: Option<Point>,
    is_static: bool,
    lens_color: Color,
    border_radius: f32,
    overlay_opacity: f32,
    overlay_scale: f32,
    on_action: F,
}

impl<Message, F> Widget<Message, Theme, Renderer> for LensWidget<'_, Message, F>
where
    Message: 'static,
    F: Fn(LensAction) -> Message + Clone + 'static,
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
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
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
        if let Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event
            && *key == keyboard::Key::Named(keyboard::key::Named::Escape)
            && lens_present(self.state)
        {
            shell.publish((self.on_action)(LensAction::Dismiss));
            shell.capture_event();
        }

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
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
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let content_tree = &tree.children[0];
        self.content.as_widget().draw(
            content_tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );

        let combined_opacity = self.overlay_opacity * self.lens_color.a;
        if combined_opacity <= 0.01 || !lens_present(self.state) {
            return;
        }

        let bounds = layout.bounds();
        let Some(rel) = current_position(
            self.state,
            self.is_static,
            self.position,
            self.default_position,
            bounds,
        ) else {
            return;
        };

        let origin = Point::new(bounds.x + rel.x, bounds.y + rel.y);
        // Overlay scale grows/shrinks the circular mask; zoom stays zoom_factor.
        let radius = effective_mask_radius(self.lens_size, self.overlay_scale);
        let zoom = self.zoom_factor;
        let Some(clip) = bounds.intersection(viewport) else {
            return;
        };

        // Scale about the lens origin so the cursor point stays fixed.
        let transform = Transformation::translate(origin.x, origin.y)
            * Transformation::scale(zoom)
            * Transformation::translate(-origin.x, -origin.y);

        let zoomed_only = combined_opacity >= 0.99;
        let border_radius = self.border_radius;

        let draw_zoomed = |renderer: &mut Renderer, strip: Rectangle| {
            renderer.with_layer(strip, |renderer| {
                renderer.with_transformation(transform, |renderer| {
                    self.content.as_widget().draw(
                        content_tree,
                        renderer,
                        theme,
                        style,
                        layout,
                        mouse::Cursor::Unavailable,
                        viewport,
                    );
                });
            });
        };

        for_each_chord_strip(origin, radius, clip, |strip| {
            // Match CSS `overflow: hidden` + border-radius: AABB clip alone leaves
            // square corner caps. Mid-y alone can leak: a strip spanning a corner
            // band is wider at mid than at the extreme edge. Intersect spans at
            // both strip ends so the full strip height stays inside the round.
            let Some((x0, x1)) =
                rounded_rect_x_span_covering_strip(bounds, border_radius, strip)
            else {
                return;
            };
            let Some(strip) = intersect_strip_x_span(strip, x0, x1) else {
                return;
            };

            if zoomed_only {
                draw_zoomed(renderer, strip);
                return;
            }

            // Coverage dither: fraction `combined_opacity` of strip height = zoomed;
            // remainder redraws base (unzoomed) so the lens cross-fades without layer alpha.
            let zoomed_h = (strip.height * combined_opacity).max(0.0);
            let base_h = (strip.height - zoomed_h).max(0.0);

            if zoomed_h > 0.0 {
                let zoomed_strip = Rectangle {
                    x: strip.x,
                    y: strip.y,
                    width: strip.width,
                    height: zoomed_h,
                };
                draw_zoomed(renderer, zoomed_strip);
            }

            if base_h > 0.0 {
                let base_strip = Rectangle {
                    x: strip.x,
                    y: strip.y + zoomed_h,
                    width: strip.width,
                    height: base_h,
                };
                renderer.with_layer(base_strip, |renderer| {
                    self.content.as_widget().draw(
                        content_tree,
                        renderer,
                        theme,
                        style,
                        layout,
                        mouse::Cursor::Unavailable,
                        viewport,
                    );
                });
            }
        });
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) && lens_present(self.state) {
            mouse::Interaction::ZoomIn
        } else {
            self.content.as_widget().mouse_interaction(
                &tree.children[0],
                layout,
                cursor,
                viewport,
                renderer,
            )
        }
    }
}

impl<'a, Message, F> From<LensWidget<'a, Message, F>> for Element<'a, Message>
where
    Message: 'a + 'static,
    F: Fn(LensAction) -> Message + Clone + 'a + 'static,
{
    fn from(widget: LensWidget<'a, Message, F>) -> Self {
        Element::new(widget)
    }
}

fn lens_should_show<Message>(state: &LensState, lens: &Lens<'_, Message>) -> bool {
    if state.dismissed {
        return false;
    }
    if lens.is_static || lens.default_position.is_some() {
        return true;
    }
    state.hovered
}

fn lens_present(state: &LensState) -> bool {
    !matches!(state.visibility, LensVisibility::Hidden)
}

/// Circular mask radius in logical pixels: half of `lens_size`, scaled by enter/exit overlay.
#[inline]
fn effective_mask_radius(lens_size: f32, overlay_scale: f32) -> f32 {
    lens_size * 0.5 * overlay_scale
}

fn current_position(
    state: &LensState,
    is_static: bool,
    position: Option<Point>,
    default_position: Option<Point>,
    bounds: Rectangle,
) -> Option<Point> {
    let fallback = Point::new(bounds.width * 0.5, bounds.height * 0.5);

    if is_static {
        return Some(position.or(default_position).unwrap_or(fallback));
    }

    // While the overlay paints (including Exiting after hover ends), keep the last
    // pointer so hover-only exit motion is not dropped by `hovered == false`.
    if state.hovered || lens_present(state) {
        return state
            .pointer
            .or(position)
            .or(default_position)
            .or(Some(fallback));
    }

    default_position.or(position)
}

fn ease(t: f32) -> f32 {
    // Smoothstep — lightweight stand-in for CSS `ease` without allocating.
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Half-width of a horizontal chord at vertical offset `dy` from circle center.
fn chord_half_width(radius: f32, dy: f32) -> f32 {
    let r2 = radius * radius;
    let dy2 = dy * dy;
    if dy2 >= r2 {
        0.0
    } else {
        (r2 - dy2).sqrt()
    }
}

/// Inclusive horizontal span `[x0, x1]` inside a rounded rect at absolute `y`.
///
/// Mid-band rows use the full width. In a corner band, left/right edges follow the
/// corner-circle chords (CSS `overflow: hidden` + `border-radius`).
fn rounded_rect_x_span(bounds: Rectangle, radius: f32, y: f32) -> Option<(f32, f32)> {
    if y < bounds.y || y > bounds.y + bounds.height {
        return None;
    }
    if bounds.width <= 0.0 || bounds.height <= 0.0 {
        return None;
    }

    let r = radius
        .max(0.0)
        .min(bounds.width * 0.5)
        .min(bounds.height * 0.5);
    let left = bounds.x;
    let right = bounds.x + bounds.width;

    if r <= 0.0 {
        return Some((left, right));
    }

    let mut x0 = left;
    let mut x1 = right;

    // Top corners
    if y < bounds.y + r {
        let dy = (bounds.y + r) - y;
        let half = chord_half_width(r, dy);
        x0 = x0.max((bounds.x + r) - half);
        x1 = x1.min((bounds.x + bounds.width - r) + half);
    }

    // Bottom corners
    if y > bounds.y + bounds.height - r {
        let dy = y - (bounds.y + bounds.height - r);
        let half = chord_half_width(r, dy);
        x0 = x0.max((bounds.x + r) - half);
        x1 = x1.min((bounds.x + bounds.width - r) + half);
    }

    if x1 <= x0 {
        None
    } else {
        Some((x0, x1))
    }
}

/// Horizontal span that keeps the **entire** strip height inside the rounded rect.
///
/// Samples the chord at the strip top and bottom (and mid) and takes the
/// intersection of those spans. Mid-y alone is too wide near corners: paint at
/// the extreme edge would leak into the square AABB corner caps.
fn rounded_rect_x_span_covering_strip(
    bounds: Rectangle,
    radius: f32,
    strip: Rectangle,
) -> Option<(f32, f32)> {
    if strip.height <= 0.0 {
        return None;
    }
    let y_lo = bounds.y;
    let y_hi = bounds.y + bounds.height;
    let y_top = strip.y.clamp(y_lo, y_hi);
    let y_bot = (strip.y + strip.height).clamp(y_lo, y_hi);
    let y_mid = (strip.y + strip.height * 0.5).clamp(y_lo, y_hi);

    let (t0, t1) = rounded_rect_x_span(bounds, radius, y_top)?;
    let (b0, b1) = rounded_rect_x_span(bounds, radius, y_bot)?;
    let (m0, m1) = rounded_rect_x_span(bounds, radius, y_mid)?;

    let x0 = t0.max(b0).max(m0);
    let x1 = t1.min(b1).min(m1);
    if x1 <= x0 {
        None
    } else {
        Some((x0, x1))
    }
}

/// Clip `strip` horizontally to inclusive span `[x0, x1]`.
fn intersect_strip_x_span(strip: Rectangle, x0: f32, x1: f32) -> Option<Rectangle> {
    let left = strip.x.max(x0);
    let right = (strip.x + strip.width).min(x1);
    if right <= left || strip.height <= 0.0 {
        return None;
    }
    Some(Rectangle {
        x: left,
        y: strip.y,
        width: right - left,
        height: strip.height,
    })
}

fn strip_step(radius: f32) -> f32 {
    let diameter = (radius * 2.0).max(1.0);
    (diameter / LENS_STRIP_MAX as f32).clamp(LENS_STRIP_MIN_STEP, LENS_STRIP_MAX_STEP)
}

fn for_each_chord_strip(center: Point, radius: f32, clip: Rectangle, mut emit: impl FnMut(Rectangle)) {
    let step = strip_step(radius);
    let y_start = center.y - radius;
    let y_end = center.y + radius;
    let mut y = y_start;
    let mut strips = 0usize;

    while y < y_end && strips < LENS_STRIP_MAX {
        let height = step.min(y_end - y);
        let mid_y = y + height * 0.5;
        let half_w = chord_half_width(radius, mid_y - center.y);
        if half_w > 0.0 {
            let strip = Rectangle {
                x: center.x - half_w,
                y,
                width: half_w * 2.0,
                height,
            };
            if let Some(clipped) = strip.intersection(&clip)
                && clipped.width > 0.0
                && clipped.height > 0.0
            {
                emit(clipped);
            }
        }
        y += step;
        strips += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::text;
    use std::time::Duration;

    #[test]
    fn lens_defaults_match_svelte() {
        let lens = Lens::<()>::new(text("x"));
        let v = lens.values();

        assert!((v.zoom_factor - 1.3).abs() < f32::EPSILON);
        assert!((v.lens_size - 170.0).abs() < f32::EPSILON);
        assert!(!v.is_static);
        assert!(v.position.is_none());
        assert!(v.default_position.is_none());
        assert!((v.duration_secs - 0.1).abs() < f32::EPSILON);
        assert_eq!(v.lens_color, Color::BLACK);
        assert!((v.border_radius - 12.0).abs() < f32::EPSILON);
    }

    #[test]
    fn lens_setters_clamp_ranges() {
        let lens = Lens::<()>::new(text("x"))
            .zoom_factor(0.5)
            .lens_size(0.0)
            .duration_secs(0.0)
            .border_radius(-3.0);

        let v = lens.values();
        assert!((v.zoom_factor - 1.01).abs() < f32::EPSILON);
        assert!((v.lens_size - 1.0).abs() < f32::EPSILON);
        assert!((v.duration_secs - 0.01).abs() < f32::EPSILON);
        assert!((v.border_radius - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn static_lens_stays_visible_without_hover() {
        let mut state = LensState::new();
        let lens = Lens::<()>::new(text("x")).is_static(true);
        let now = Instant::now();

        state.tick(now, &lens);
        assert_eq!(state.visibility(), LensVisibility::Entering);

        state.tick(now + Duration::from_millis(200), &lens);
        assert_eq!(state.visibility(), LensVisibility::Shown);
        assert!(state.overlay_opacity() > 0.99);
    }

    #[test]
    fn hover_only_hides_when_not_hovered() {
        let mut state = LensState::new();
        let lens = Lens::<()>::new(text("x"));
        let now = Instant::now();

        state.tick(now, &lens);
        assert_eq!(state.visibility(), LensVisibility::Hidden);

        state.apply_action(&LensAction::Enter);
        state.apply_action(&LensAction::Move(Point::new(10.0, 12.0)));
        state.tick(now, &lens);
        assert_eq!(state.visibility(), LensVisibility::Entering);

        state.apply_action(&LensAction::Exit);
        state.tick(now + Duration::from_millis(5), &lens);
        assert_eq!(state.visibility(), LensVisibility::Exiting);

        state.tick(now + Duration::from_millis(200), &lens);
        assert_eq!(state.visibility(), LensVisibility::Hidden);
    }

    #[test]
    fn default_position_keeps_lens_present_when_idle() {
        let mut state = LensState::new();
        let lens = Lens::<()>::new(text("x")).default_position(Point::new(40.0, 50.0));
        let now = Instant::now();

        state.tick(now, &lens);
        assert!(matches!(
            state.visibility(),
            LensVisibility::Entering | LensVisibility::Shown
        ));
    }

    #[test]
    fn dismiss_hides_until_next_move() {
        let mut state = LensState::new();
        let lens = Lens::<()>::new(text("x"));
        let t0 = Instant::now();

        state.apply_action(&LensAction::Enter);
        state.apply_action(&LensAction::Move(Point::new(50.0, 40.0)));
        state.tick(t0, &lens);
        state.tick(t0 + Duration::from_millis(200), &lens);
        assert_eq!(state.visibility(), LensVisibility::Shown);

        state.apply_action(&LensAction::Dismiss);
        state.tick(t0 + Duration::from_millis(210), &lens);
        assert_eq!(state.visibility(), LensVisibility::Exiting);

        state.tick(t0 + Duration::from_millis(400), &lens);
        assert_eq!(state.visibility(), LensVisibility::Hidden);

        state.apply_action(&LensAction::Move(Point::new(60.0, 44.0)));
        state.tick(t0 + Duration::from_millis(410), &lens);
        assert_eq!(state.visibility(), LensVisibility::Entering);
    }

    #[test]
    fn enter_exit_progress_endpoints() {
        let mut state = LensState::new();
        let lens = Lens::<()>::new(text("x")).duration_secs(0.1);
        let t0 = Instant::now();

        state.apply_action(&LensAction::Enter);
        state.tick(t0, &lens);
        assert_eq!(state.visibility(), LensVisibility::Entering);
        assert!((state.anim_t() - 0.0).abs() < 1e-5);
        assert!((state.overlay_scale() - ENTER_SCALE_FROM).abs() < 1e-3);
        assert!(state.overlay_opacity() < 0.05);

        state.tick(t0 + Duration::from_millis(100), &lens);
        assert_eq!(state.visibility(), LensVisibility::Shown);
        assert!((state.overlay_scale() - 1.0).abs() < 1e-3);
        assert!((state.overlay_opacity() - 1.0).abs() < 1e-3);

        state.apply_action(&LensAction::Exit);
        state.tick(t0 + Duration::from_millis(101), &lens);
        assert_eq!(state.visibility(), LensVisibility::Exiting);
        assert!(state.anim_t() < 0.05);
        assert!((state.overlay_scale() - 1.0).abs() < 0.05);

        state.tick(t0 + Duration::from_millis(201), &lens);
        assert_eq!(state.visibility(), LensVisibility::Hidden);
        assert!((state.overlay_scale() - ENTER_SCALE_FROM).abs() < 1e-3);
        assert!(state.overlay_opacity() < 0.05);
    }

    #[test]
    fn chord_half_width_at_center_equals_radius() {
        let radius = 85.0;
        assert!((chord_half_width(radius, 0.0) - radius).abs() < 1e-5);
        assert!(chord_half_width(radius, radius).abs() < 1e-5);
        assert!(chord_half_width(radius, radius + 1.0).abs() < 1e-5);

        let half = chord_half_width(radius, radius * 0.5);
        assert!(half > 0.0 && half < radius);
    }

    #[test]
    fn rounded_rect_x_span_mid_y_is_full_width() {
        let bounds = Rectangle {
            x: 10.0,
            y: 20.0,
            width: 200.0,
            height: 160.0,
        };
        let radius = 12.0;
        let mid_y = bounds.y + bounds.height * 0.5;
        let (x0, x1) = rounded_rect_x_span(bounds, radius, mid_y).expect("mid span");
        assert!((x0 - bounds.x).abs() < 1e-5);
        assert!((x1 - (bounds.x + bounds.width)).abs() < 1e-5);
    }

    #[test]
    fn rounded_rect_x_span_near_top_left_is_narrower() {
        let bounds = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 160.0,
        };
        let radius = 12.0;
        // One pixel below the top edge — deep in the top-left corner band.
        let y = bounds.y + 1.0;
        let (x0, x1) = rounded_rect_x_span(bounds, radius, y).expect("corner span");
        assert!(
            x0 > bounds.x,
            "left edge should inset in corner band: x0={x0}"
        );
        assert!(
            x1 < bounds.x + bounds.width,
            "right edge should inset in corner band: x1={x1}"
        );
        assert!(
            (x1 - x0) < bounds.width,
            "corner span must be narrower than full width"
        );
    }

    #[test]
    fn covering_strip_span_tighter_than_mid_near_corner() {
        let bounds = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 160.0,
        };
        let radius = 12.0;
        // Tall strip in the top corner band: mid chord is wider than the top edge.
        let strip = Rectangle {
            x: 0.0,
            y: 1.0,
            width: 200.0,
            height: 8.0,
        };
        let mid_y = strip.y + strip.height * 0.5;
        let (mid_x0, mid_x1) = rounded_rect_x_span(bounds, radius, mid_y).expect("mid");
        let (cov_x0, cov_x1) =
            rounded_rect_x_span_covering_strip(bounds, radius, strip).expect("cover");
        let (top_x0, top_x1) =
            rounded_rect_x_span(bounds, radius, strip.y).expect("top");

        assert!(
            cov_x0 > mid_x0,
            "covering left inset must exceed mid-only: cov={cov_x0} mid={mid_x0}"
        );
        assert!(
            cov_x1 < mid_x1,
            "covering right inset must exceed mid-only: cov={cov_x1} mid={mid_x1}"
        );
        assert!(
            (cov_x0 - top_x0).abs() < 1e-5 && (cov_x1 - top_x1).abs() < 1e-5,
            "intersection should match the narrower top-edge span"
        );
    }

    #[test]
    fn strip_step_respects_cap_bounds() {
        let step = strip_step(85.0); // diameter 170 → ~1px
        assert!(step >= LENS_STRIP_MIN_STEP && step <= LENS_STRIP_MAX_STEP);

        let big = strip_step(2000.0);
        assert!((big - LENS_STRIP_MAX_STEP).abs() < f32::EPSILON);
    }

    #[test]
    fn overlay_scale_scales_mask_radius_not_zoom_factor() {
        let lens_size = 170.0;
        let zoom_factor = DEFAULT_ZOOM_FACTOR;
        let overlay_scale = ENTER_SCALE_FROM;

        let radius = effective_mask_radius(lens_size, overlay_scale);
        assert!((radius - (lens_size * 0.5 * overlay_scale)).abs() < 1e-5);
        assert!((radius - 49.3).abs() < 1e-3);

        // Draw contract: content transform zoom is exactly `zoom_factor`.
        let content_zoom = zoom_factor;
        assert!((content_zoom - 1.3).abs() < f32::EPSILON);

        // Old bug multiplied overlay into zoom (~0.75) while leaving mask at full radius.
        let buggy_zoom = zoom_factor * overlay_scale;
        assert!(buggy_zoom < 1.0);
        assert!((radius - lens_size * 0.5).abs() > 1.0);

        let full = effective_mask_radius(lens_size, ENTER_SCALE_TO);
        assert!((full - lens_size * 0.5).abs() < 1e-5);

        let exit_mid = effective_mask_radius(lens_size, EXIT_SCALE_TO);
        assert!((exit_mid - lens_size * 0.5 * EXIT_SCALE_TO).abs() < 1e-5);
    }

    #[test]
    fn combined_opacity_split_heights_sum_to_strip() {
        let strip_h = 8.0;
        for &opacity in &[0.25_f32, 0.5, 0.75] {
            let zoomed_h = strip_h * opacity;
            let base_h = strip_h - zoomed_h;
            assert!((zoomed_h + base_h - strip_h).abs() < 1e-5);
            assert!(zoomed_h > 0.0 && base_h > 0.0);
        }
        assert!((1.0_f32 * strip_h) >= strip_h * 0.99);
    }

    #[test]
    fn exit_keeps_last_pointer_for_current_position() {
        let mut state = LensState::new();
        let lens = Lens::<()>::new(text("x")).duration_secs(0.1);
        let t0 = Instant::now();
        let pointer = Point::new(42.0, 77.0);
        let bounds = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 160.0,
        };

        state.apply_action(&LensAction::Enter);
        state.apply_action(&LensAction::Move(pointer));
        state.tick(t0, &lens);
        state.tick(t0 + Duration::from_millis(100), &lens);
        assert_eq!(state.visibility(), LensVisibility::Shown);

        state.apply_action(&LensAction::Exit);
        state.tick(t0 + Duration::from_millis(105), &lens);
        assert_eq!(state.visibility(), LensVisibility::Exiting);
        assert!(state.anim_t() < 1.0);

        let rel = current_position(&state, false, None, None, bounds);
        assert_eq!(rel, Some(pointer));

        // Still resolving mid-exit (before Hidden).
        state.tick(t0 + Duration::from_millis(140), &lens);
        assert_eq!(state.visibility(), LensVisibility::Exiting);
        assert!(state.anim_t() < 1.0);
        assert_eq!(
            current_position(&state, false, None, None, bounds),
            Some(pointer)
        );
    }

    #[test]
    fn mid_enter_reverse_to_exit_keeps_opacity_continuous() {
        let mut state = LensState::new();
        let lens = Lens::<()>::new(text("x")).duration_secs(0.1);
        let t0 = Instant::now();

        state.apply_action(&LensAction::Enter);
        state.apply_action(&LensAction::Move(Point::new(10.0, 10.0)));
        state.tick(t0, &lens);
        assert_eq!(state.visibility(), LensVisibility::Entering);

        // Mid-enter (~50% linear time → smoothstep ~0.5 opacity / mid scale).
        state.tick(t0 + Duration::from_millis(50), &lens);
        assert_eq!(state.visibility(), LensVisibility::Entering);
        assert!((state.anim_t() - 0.5).abs() < 0.05);

        let opacity_before = state.overlay_opacity();
        let scale_before = state.overlay_scale();
        assert!(opacity_before > 0.2 && opacity_before < 0.9);
        assert!(scale_before > ENTER_SCALE_FROM && scale_before < ENTER_SCALE_TO);

        state.apply_action(&LensAction::Exit);
        let reverse_at = t0 + Duration::from_millis(51);
        state.tick(reverse_at, &lens);
        assert_eq!(state.visibility(), LensVisibility::Exiting);
        assert!((state.anim_t() - 0.0).abs() < 1e-5);

        let opacity_after = state.overlay_opacity();
        let scale_after = state.overlay_scale();
        assert!(
            (opacity_after - opacity_before).abs() < 1e-4,
            "opacity jumped: {opacity_before} → {opacity_after}"
        );
        assert!(
            (scale_after - scale_before).abs() < 1e-4,
            "scale jumped: {scale_before} → {scale_after}"
        );
    }
}
