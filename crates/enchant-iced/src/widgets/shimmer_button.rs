//! Builder-first shimmer button — canon: `svelte-animations/.../ShimmerButton.svelte`.
//!
//! CSS layers (inside the real `<button>`):
//! 1. `--bg` fill + `border-white/10` + radius + `overflow:hidden`
//! 2. spark `-z-30` `blur-[2px]`: sliding square + `spin-around` conic (`--spread: 90°`)
//! 3. slot (label)
//! 4. highlight: `inset 0 -8px 10px #ffffff1f` (hover/active alphas)
//! 5. backdrop `-z-20`: `inset: var(--cut)` + `--bg` → thin perimeter ring of the conic
//!
//! iced: paint `--bg` on the outer container; spark as a soft perimeter stroke
//! (conic ∩ cut) on an overlay so it stays visible; highlight is a short soft
//! bottom wash (iced has no inset `Shadow`).

use super::border_track::RoundedRectTrack;
use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};
use iced::border::Radius;
use iced::mouse;
use iced::widget::button;
use iced::widget::canvas::{self, Action, Fill, LineCap, Stroke};
use iced::widget::{container, stack, text};
use iced::{
    Background, Border, Color, Element, Event, Length, Padding, Point, Rectangle, Renderer, Shadow,
    Size,
};
use std::fmt;
use std::time::Duration;

/// svelte `--spread` default.
const DEFAULT_SPREAD_DEG: f32 = 90.0;
/// Soft stroke passes ≈ CSS `blur-[2px]`.
const SPARK_BLUR_PASSES: usize = 3;
const SPARK_SEGMENTS: usize = 160;
/// svelte `--cut` / `shimmerSize` = `0.05em` ≈ 0.8px @ 16px.
const DEFAULT_CUT_PX: f32 = 0.8;
/// CSS `blur-[2px]` — widens the visible ring slightly (no filter blur in iced).
const BLUR_PX: f32 = 2.0;

/// svelte inset shadow alphas / offsets.
const HIGHLIGHT_REST_OFFSET: f32 = 8.0;
const HIGHLIGHT_REST_ALPHA: f32 = 0x1f as f32 / 255.0;
const HIGHLIGHT_HOVER_OFFSET: f32 = 6.0;
const HIGHLIGHT_HOVER_ALPHA: f32 = 0x3f as f32 / 255.0;
const HIGHLIGHT_ACTIVE_OFFSET: f32 = 10.0;
const HIGHLIGHT_ACTIVE_ALPHA: f32 = 0x3f as f32 / 255.0;
const HIGHLIGHT_BLUR: f32 = 10.0;

/// Builder for the svelte-animations shimmer button.
#[must_use = "builders do nothing unless turned into an iced Element"]
pub struct ShimmerButton<'a, Message> {
    content: Element<'a, Message>,
    on_press: Option<Message>,
    shimmer_color: Color,
    cut: f32,
    border_radius: f32,
    duration_secs: f32,
    background: Color,
    spread_deg: f32,
    padding: Padding,
    width: Length,
    height: Length,
    border_color: Color,
}

impl<'a, Message> fmt::Debug for ShimmerButton<'a, Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ShimmerButton")
            .field("on_press", &self.on_press.is_some())
            .field("shimmer_color", &self.shimmer_color)
            .field("cut", &self.cut)
            .field("border_radius", &self.border_radius)
            .field("duration_secs", &self.duration_secs)
            .field("background", &self.background)
            .field("spread_deg", &self.spread_deg)
            .field("padding", &self.padding)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("border_color", &self.border_color)
            .finish_non_exhaustive()
    }
}

impl<'a, Message> ShimmerButton<'a, Message> {
    /// Creates a text button with svelte defaults (black fill, white shimmer, 3s).
    pub fn new(label: impl Into<String>) -> Self
    where
        Message: 'a,
    {
        let label = label.into();
        let content: Element<'a, Message> = text(label).size(16).color(Color::WHITE).into();
        Self::from_content(content)
    }

    pub fn from_content(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            on_press: None,
            shimmer_color: Color::WHITE,
            cut: DEFAULT_CUT_PX,
            border_radius: 100.0,
            duration_secs: 3.0,
            background: Color::BLACK,
            spread_deg: DEFAULT_SPREAD_DEG,
            padding: Padding::from([12, 24]),
            width: Length::Shrink,
            height: Length::Shrink,
            border_color: Color::from_rgba(1.0, 1.0, 1.0, 0.10),
        }
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    pub fn clear_on_press(mut self) -> Self {
        self.on_press = None;
        self
    }

    pub fn shimmer_color(mut self, color: Color) -> Self {
        self.shimmer_color = color;
        self
    }

    pub fn cut(mut self, cut: f32) -> Self {
        self.cut = cut.max(0.4);
        self
    }

    pub fn shimmer_size(self, cut: f32) -> Self {
        self.cut(cut)
    }

    pub fn border_radius(mut self, radius: f32) -> Self {
        self.border_radius = radius.max(0.0);
        self
    }

    pub fn radius(self, radius: f32) -> Self {
        self.border_radius(radius)
    }

    pub fn duration_secs(mut self, duration_secs: f32) -> Self {
        self.duration_secs = duration_secs.max(0.01);
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = color;
        self
    }

    pub fn spread_deg(mut self, spread_deg: f32) -> Self {
        self.spread_deg = spread_deg.clamp(1.0, 360.0);
        self
    }

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    /// Builds the interactive element for the given animation clock.
    pub fn into_element(self, elapsed: Duration) -> Element<'a, Message>
    where
        Message: Clone + 'a,
    {
        let radius = self.border_radius;
        let background = self.background;
        let border_color = self.border_color;
        let padding = self.padding;
        let width = self.width;
        let height = self.height;

        let spark = effect_canvas::<_, Message>(
            ShimmerSparkProgram {
                elapsed,
                duration_secs: self.duration_secs,
                shimmer_color: self.shimmer_color,
                cut: self.cut,
                radius,
                spread_deg: self.spread_deg,
            },
            EffectCanvasProps::new()
                .width(Length::Fill)
                .height(Length::Fill),
        );

        let highlight = effect_canvas::<_, Message>(
            ShimmerHighlightProgram { radius },
            EffectCanvasProps::new()
                .width(Length::Fill)
                .height(Length::Fill),
        );

        let mut control = button(self.content)
            .padding(padding)
            .style(move |_theme, status| shimmer_button_style(status));

        if let Some(on_press) = self.on_press {
            control = control.on_press(on_press);
        }

        let spark_layer: Element<'a, Message> = container(spark)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
        let highlight_layer: Element<'a, Message> = container(highlight)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        // Base = button (sizing + clicks). Spark + highlight overlays (Interaction::None)
        // so the button keeps Pointer and presses; spark stays visible above `--bg`.
        let layered = stack(vec![control.into()])
            .push(spark_layer)
            .push(highlight_layer)
            .clip(true);

        container(layered)
            .width(width)
            .height(height)
            .clip(true)
            .style(move |_theme| container::Style {
                background: Some(Background::Color(background)),
                text_color: None,
                border: Border {
                    color: border_color,
                    width: 1.0,
                    radius: Radius::from(radius),
                },
                shadow: Shadow::default(),
                snap: false,
            })
            .into()
    }

    pub fn view(self, elapsed: Duration) -> Element<'a, Message>
    where
        Message: Clone + 'a,
    {
        self.into_element(elapsed)
    }
}

fn shimmer_button_style(status: button::Status) -> button::Style {
    let mut style = button::Style {
        background: None,
        text_color: Color::WHITE,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: Radius::from(100.0),
        },
        shadow: Shadow::default(),
        snap: false,
    };

    match status {
        button::Status::Pressed => {
            // svelte `active:translate-y-[1px]` — no translate without AABB bleed.
            style.text_color = Color::from_rgba(1.0, 1.0, 1.0, 0.92);
        }
        button::Status::Hovered | button::Status::Active | button::Status::Disabled => {}
    }

    style
}

// --- spark: solid 90° perimeter arc (conic ∩ cut), soft edges ≈ blur-[2px] ----

struct ShimmerSparkProgram {
    elapsed: Duration,
    duration_secs: f32,
    shimmer_color: Color,
    cut: f32,
    radius: f32,
    spread_deg: f32,
}

impl<Message> EffectProgram<Message> for ShimmerSparkProgram {
    type State = ();

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> mouse::Interaction {
        mouse::Interaction::None
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let size = bounds.size();
        if size.width < 2.0 || size.height < 2.0 {
            return vec![frame.into_geometry()];
        }

        let radius = self.radius.min(size.width.min(size.height) * 0.5);
        let speed = self.duration_secs.max(0.01);
        let spin_t = (self.elapsed.as_secs_f32() / (speed * 2.0)).rem_euclid(1.0);
        let spin_deg = spin_around_degrees(spin_t);
        let slide = shimmer_slide_t(self.elapsed, speed);

        // Visible ring ≈ cut + soft blur (CSS punch leaves `--cut`, blur softens).
        let ring = (self.cut + BLUR_PX * 0.75).clamp(1.6, 3.5);
        let inset = (ring * 0.5).min(size.width.min(size.height) * 0.2);
        let track_size = Size::new(
            (size.width - inset * 2.0).max(1.0),
            (size.height - inset * 2.0).max(1.0),
        );
        let track_radius = (radius - inset).max(0.0);
        let track = RoundedRectTrack::new(track_size, track_radius);
        let origin = Point::new(inset, inset);

        let perimeter = track.perimeter().max(1.0);
        let top_len = (track_size.width - 2.0 * track_radius).max(0.0);
        // spin 0 → CSS conic mid at 270° (top center).
        let top_mid = (top_len * 0.5) / perimeter;
        let slide_bias = (slide - 0.5) * (top_len / perimeter);
        let head = (spin_deg / 360.0 + top_mid + slide_bias).rem_euclid(1.0);
        let beam_span = (self.spread_deg / 360.0).clamp(0.05, 1.0);

        for pass in 0..SPARK_BLUR_PASSES {
            let pass_t = pass as f32 / (SPARK_BLUR_PASSES - 1).max(1) as f32;
            let width = ring * (1.0 + pass_t * 0.45);
            let span = beam_span * (1.0 + pass_t * 0.06);
            // Core pass nearly opaque; outer passes softer (blur).
            let base_a = self.shimmer_color.a * (1.0 - pass_t * 0.55).max(0.25);

            for index in 0..SPARK_SEGMENTS {
                let t0 = index as f32 / SPARK_SEGMENTS as f32;
                let t1 = (index + 1) as f32 / SPARK_SEGMENTS as f32;
                let mid_t = (t0 + t1) * 0.5;
                // Distance behind the head along the track (clockwise).
                let dist = (mid_t - head).rem_euclid(1.0);
                if dist > span {
                    continue;
                }

                // CSS conic is *solid* across `--spread`; only the angular edges soften.
                let u = dist / span;
                let edge = 0.10;
                let falloff = if u < edge {
                    u / edge
                } else if u > 1.0 - edge {
                    (1.0 - u) / edge
                } else {
                    1.0
                };
                let alpha = base_a * falloff;
                if alpha < 0.03 {
                    continue;
                }

                let p0 = track.sample(t0).point;
                let p1 = track.sample(t1).point;
                let segment = canvas::Path::line(
                    Point::new(origin.x + p0.x, origin.y + p0.y),
                    Point::new(origin.x + p1.x, origin.y + p1.y),
                );

                frame.stroke(
                    &segment,
                    Stroke::default()
                        .with_width(width)
                        .with_line_cap(LineCap::Round)
                        .with_color(Color {
                            a: alpha,
                            ..self.shimmer_color
                        }),
                );
            }
        }

        vec![frame.into_geometry()]
    }
}

// --- highlight: short soft bottom wash (CSS inset shadow) --------------------

#[derive(Clone, Copy, Debug, Default)]
struct HighlightState {
    hovered: bool,
    pressed: bool,
}

struct ShimmerHighlightProgram {
    radius: f32,
}

impl<Message> EffectProgram<Message> for ShimmerHighlightProgram {
    type State = HighlightState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Option<Action<Message>> {
        let mut dirty = false;
        let over = cursor.is_over(bounds);

        if state.hovered != over {
            state.hovered = over;
            dirty = true;
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(iced::touch::Event::FingerPressed { .. })
                if over =>
            {
                if !state.pressed {
                    state.pressed = true;
                    dirty = true;
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(iced::touch::Event::FingerLifted { .. }) => {
                if state.pressed {
                    state.pressed = false;
                    dirty = true;
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. })
            | Event::Mouse(mouse::Event::CursorEntered)
            | Event::Mouse(mouse::Event::CursorLeft) => {
                dirty = true;
            }
            _ => {}
        }

        dirty.then(Action::request_redraw)
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> mouse::Interaction {
        mouse::Interaction::None
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let size = bounds.size();
        let radius = self.radius.min(size.width.min(size.height) * 0.5);

        let (offset_y, alpha) = if state.pressed {
            (HIGHLIGHT_ACTIVE_OFFSET, HIGHLIGHT_ACTIVE_ALPHA)
        } else if state.hovered {
            (HIGHLIGHT_HOVER_OFFSET, HIGHLIGHT_HOVER_ALPHA)
        } else {
            (HIGHLIGHT_REST_OFFSET, HIGHLIGHT_REST_ALPHA)
        };

        paint_inset_bottom_glow(&mut frame, size, radius, offset_y, alpha);
        vec![frame.into_geometry()]
    }
}

/// Soft bottom wash approximating `inset 0 -<offset>px 10px #ffffffNN`.
/// Kept short and low-alpha so it never reads as a gray slab over the face.
fn paint_inset_bottom_glow(
    frame: &mut canvas::Frame,
    size: Size,
    radius: f32,
    offset_y: f32,
    alpha: f32,
) {
    // Only the bottom ~blur band; peak near `offset_y` from the edge.
    let extent = (offset_y * 0.35 + HIGHLIGHT_BLUR * 0.85).min(size.height * 0.22);
    let steps = 12;
    let step = (extent / steps as f32).max(0.75);
    let peak = (offset_y * 0.25 / extent).clamp(0.05, 0.4);

    for i in 0..steps {
        let y_from_bottom = (i as f32 + 0.5) * step;
        if y_from_bottom > extent {
            break;
        }
        let t = (y_from_bottom / extent).clamp(0.0, 1.0);
        // Strongest near the bottom edge, fades upward (inset shadow look).
        let fall = (1.0 - (t - peak).abs() / (1.0 - peak).max(0.3)).clamp(0.0, 1.0);
        let a = alpha * fall.powi(2) * 0.14;
        if a < 0.003 {
            continue;
        }

        let y = size.height - y_from_bottom;
        let edge = (y_from_bottom / radius.max(1.0)).clamp(0.0, 1.0);
        let side_pad = 2.0 + radius * (1.0 - edge).powi(2) * 0.4;
        let w = (size.width - side_pad * 2.0).max(1.0);

        frame.fill(
            &canvas::Path::rectangle(Point::new(side_pad, y - step * 0.5), Size::new(w, step)),
            Fill::from(Color::from_rgba(1.0, 1.0, 1.0, a)),
        );
    }
}

// --- svelte keyframes --------------------------------------------------------

fn shimmer_slide_t(elapsed: Duration, speed: f32) -> f32 {
    let phase = (elapsed.as_secs_f32() / speed).rem_euclid(2.0);
    if phase <= 1.0 {
        ease_in_out(phase)
    } else {
        ease_in_out(2.0 - phase)
    }
}

fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// `@keyframes spin-around` — period `2 * --speed`.
fn spin_around_degrees(t: f32) -> f32 {
    const KEYS: [(f32, f32); 6] = [
        (0.00, 0.0),
        (0.15, 90.0),
        (0.35, 90.0),
        (0.65, 270.0),
        (0.85, 270.0),
        (1.00, 360.0),
    ];

    for window in KEYS.windows(2) {
        let (t0, a0) = window[0];
        let (t1, a1) = window[1];
        if t <= t1 {
            let local = if (t1 - t0).abs() < f32::EPSILON {
                0.0
            } else {
                ((t - t0) / (t1 - t0)).clamp(0.0, 1.0)
            };
            return a0 + (a1 - a0) * local;
        }
    }
    360.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shimmer_button_defaults_match_svelte() {
        let button = ShimmerButton::<()>::new("Shimmer Button");
        assert_eq!(button.duration_secs, 3.0);
        assert_eq!(button.border_radius, 100.0);
        assert_eq!(button.spread_deg, 90.0);
        assert_eq!(button.background, Color::BLACK);
        assert_eq!(button.shimmer_color, Color::WHITE);
        assert!((button.cut - DEFAULT_CUT_PX).abs() < f32::EPSILON);
    }

    #[test]
    fn setters_validate_ranges() {
        let button = ShimmerButton::<()>::new("x")
            .cut(0.1)
            .duration_secs(0.0)
            .spread_deg(999.0)
            .border_radius(-4.0);

        assert_eq!(button.cut, 0.4);
        assert_eq!(button.duration_secs, 0.01);
        assert_eq!(button.spread_deg, 360.0);
        assert_eq!(button.border_radius, 0.0);
    }

    #[test]
    fn spin_around_hits_keyframe_plateaus() {
        assert!((spin_around_degrees(0.0) - 0.0).abs() < 0.01);
        assert!((spin_around_degrees(0.25) - 90.0).abs() < 0.01);
        assert!((spin_around_degrees(0.75) - 270.0).abs() < 0.01);
        assert!((spin_around_degrees(1.0) - 360.0).abs() < 0.01);
    }

    #[test]
    fn cut_matches_svelte_em_default() {
        assert!((DEFAULT_CUT_PX - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn highlight_alphas_match_svelte_hex() {
        assert!((HIGHLIGHT_REST_ALPHA - 31.0 / 255.0).abs() < f32::EPSILON);
        assert!((HIGHLIGHT_HOVER_ALPHA - 63.0 / 255.0).abs() < f32::EPSILON);
        assert!((HIGHLIGHT_ACTIVE_ALPHA - 63.0 / 255.0).abs() < f32::EPSILON);
    }

    #[test]
    fn conic_falloff_is_solid_in_middle() {
        // Solid white across most of --spread; soft only at edges (not sin that zeros the head).
        let edge = 0.10;
        let mid_u = 0.5;
        let falloff = if mid_u < edge {
            mid_u / edge
        } else if mid_u > 1.0 - edge {
            (1.0 - mid_u) / edge
        } else {
            1.0
        };
        assert!((falloff - 1.0_f32).abs() < f32::EPSILON);
    }

    #[test]
    fn slide_t_stays_unit_interval() {
        for secs in [0.0_f32, 0.5, 1.5, 3.0, 4.5] {
            let t = shimmer_slide_t(Duration::from_secs_f32(secs), 3.0);
            assert!((0.0..=1.0).contains(&t), "slide_t={t}");
        }
    }
}
