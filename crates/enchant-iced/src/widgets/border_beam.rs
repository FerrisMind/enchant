//! Builder-first animated border beam (Magic UI `BorderBeam`).
//!
//! Travels along the perimeter of a rounded rectangle. Time is supplied by the
//! host via [`BorderBeam::elapsed`] or [`BorderBeam::into_element`].

use super::border_track::RoundedRectTrack;
use iced::Color;
use iced::Element;
use iced::Length;
use iced::Rectangle;
use iced::Renderer;
use iced::mouse;
use iced::widget::canvas::{self, LineCap, Stroke};
use std::fmt;
use std::time::Duration;

use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};

const BEAM_SEGMENTS: usize = 180;
/// Second gradient stop offset from Magic UI / `animated-beam` stop layout.
const BEAM_GRADIENT_STOP_MID: f32 = 0.325;

/// Easing applied to progress within each animation loop.
///
/// Maps Magic UI `transition.ease` without exposing `twill` in the public API.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum BorderBeamEase {
    /// Constant speed (Magic UI default).
    #[default]
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

/// Builder for an animated beam along a rounded-rect border.
///
/// # Examples
///
/// ```rust,no_run
/// use iced::Element;
/// use enchant_iced::BorderBeam;
/// use std::time::Duration;
///
/// fn overlay<'a, Message: 'a>(elapsed: Duration) -> Element<'a, Message> {
///     BorderBeam::new()
///         .size(100.0)
///         .duration_secs(8.0)
///         .radius(12.0)
///         .into_element(elapsed)
/// }
/// ```
#[must_use = "builders do nothing unless turned into an iced Element"]
#[derive(Clone, Copy)]
pub struct BorderBeam {
    size: f32,
    duration_secs: f32,
    delay_secs: f32,
    color_from: Color,
    color_to: Color,
    reverse: bool,
    /// Normalized start offset in `[0, 1)` (Magic UI accepts 0–100 as well).
    initial_offset: f32,
    border_width: f32,
    radius: f32,
    track_color: Option<Color>,
    beam_enabled: bool,
    ease: BorderBeamEase,
    width: Length,
    height: Length,
}

impl Default for BorderBeam {
    fn default() -> Self {
        Self {
            size: 50.0,
            duration_secs: 6.0,
            delay_secs: 0.0,
            color_from: Color::from_rgb8(0xFF, 0xAA, 0x40),
            color_to: Color::from_rgb8(0x9C, 0x40, 0xFF),
            reverse: false,
            initial_offset: 0.0,
            border_width: 1.0,
            radius: 20.0,
            track_color: None,
            beam_enabled: true,
            ease: BorderBeamEase::Linear,
            width: Length::Fill,
            height: Length::Fill,
        }
    }
}

impl fmt::Debug for BorderBeam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BorderBeam")
            .field("size", &self.size)
            .field("duration_secs", &self.duration_secs)
            .field("delay_secs", &self.delay_secs)
            .field("color_from", &self.color_from)
            .field("color_to", &self.color_to)
            .field("reverse", &self.reverse)
            .field("initial_offset", &self.initial_offset)
            .field("border_width", &self.border_width)
            .field("radius", &self.radius)
            .field("track_color", &self.track_color)
            .field("beam_enabled", &self.beam_enabled)
            .field("ease", &self.ease)
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

impl BorderBeam {
    /// Creates a beam with Magic UI defaults (`size` 50, `duration` 6s, …).
    pub fn new() -> Self {
        Self::default()
    }

    /// Length of the lit segment along the perimeter, in logical pixels.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size.max(1.0);
        self
    }

    /// Full-loop duration in seconds (Magic UI `duration`).
    pub fn duration_secs(mut self, duration_secs: f32) -> Self {
        self.duration_secs = duration_secs.max(0.01);
        self
    }

    /// Positive delay advances the phase (Magic UI `transition.delay: -delay`).
    pub fn delay_secs(mut self, delay_secs: f32) -> Self {
        self.delay_secs = delay_secs.max(0.0);
        self
    }

    /// Gradient start color (`colorFrom`, default `#ffaa40`).
    pub fn color_from(mut self, color: Color) -> Self {
        self.color_from = color;
        self
    }

    /// Gradient mid/end color (`colorTo`, default `#9c40ff`).
    pub fn color_to(mut self, color: Color) -> Self {
        self.color_to = color;
        self
    }

    /// Reverse travel direction.
    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    /// Initial offset. Values `> 1` are treated as percents (0–100), matching Magic UI.
    pub fn initial_offset(mut self, initial_offset: f32) -> Self {
        self.initial_offset = normalize_offset(initial_offset);
        self
    }

    /// Stroke width of the beam (and optional track).
    pub fn border_width(mut self, border_width: f32) -> Self {
        self.border_width = border_width.max(0.0);
        self
    }

    /// Corner radius of the path (parent `rounded-[inherit]` analogue).
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius.max(0.0);
        self
    }

    /// Optional static border under the beam (theme chrome).
    pub fn track_color(mut self, color: Color) -> Self {
        self.track_color = Some(color);
        self
    }

    /// Clears a previously set track color.
    pub fn clear_track_color(mut self) -> Self {
        self.track_color = None;
        self
    }

    /// When `false`, only the optional track is drawn (e.g. during theme transitions).
    pub fn beam_enabled(mut self, enabled: bool) -> Self {
        self.beam_enabled = enabled;
        self
    }

    /// Per-loop easing (Magic UI `transition.ease`).
    pub fn ease(mut self, ease: BorderBeamEase) -> Self {
        self.ease = ease;
        self
    }

    /// Widget width (default [`Length::Fill`]).
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Widget height (default [`Length::Fill`]).
    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    /// Builds the canvas element for the given animation clock.
    pub fn into_element<'a, Message: 'a>(self, elapsed: Duration) -> Element<'a, Message> {
        effect_canvas::<_, Message>(
            BorderBeamProgram {
                elapsed,
                config: self,
            },
            EffectCanvasProps::new()
                .width(self.width)
                .height(self.height),
        )
        .into()
    }

    /// Alias for [`Self::into_element`].
    pub fn view<'a, Message: 'a>(self, elapsed: Duration) -> Element<'a, Message> {
        self.into_element(elapsed)
    }
}

struct BorderBeamProgram {
    elapsed: Duration,
    config: BorderBeam,
}

impl<Message> EffectProgram<Message> for BorderBeamProgram {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let track = RoundedRectTrack::new(bounds.size(), self.config.radius);

        if let Some(track_color) = self.config.track_color {
            frame.stroke(
                &track.path(iced::Point::ORIGIN),
                Stroke::default()
                    .with_width(self.config.border_width)
                    .with_color(track_color),
            );
        }

        if self.config.beam_enabled {
            draw_gradient_beam(&mut frame, &track, self.elapsed, self.config);
        }

        vec![frame.into_geometry()]
    }
}

fn draw_gradient_beam(
    frame: &mut canvas::Frame,
    track: &RoundedRectTrack,
    elapsed: Duration,
    config: BorderBeam,
) {
    let head = beam_phase(elapsed, config);
    let perimeter = track.perimeter().max(1.0);
    let beam_span = (config.size / perimeter).clamp(0.005, 1.0);
    let last = BEAM_SEGMENTS.saturating_sub(1);

    for index in 0..BEAM_SEGMENTS {
        let t0 = index as f32 / BEAM_SEGMENTS as f32;
        let t1 = (index + 1) as f32 / BEAM_SEGMENTS as f32;
        let mid_t = (t0 + t1) * 0.5;
        let dist = distance_behind_head(mid_t, head, config.reverse);

        if dist > beam_span {
            continue;
        }

        let along_beam = 1.0 - dist / beam_span;
        let color =
            sample_border_beam_gradient(along_beam, config.color_from, config.color_to);

        if color.a <= 0.01 {
            continue;
        }

        let p0 = track.sample(t0).point;
        let p1 = track.sample(t1).point;
        let segment = canvas::Path::line(p0, p1);
        let line_cap = if index == 0 || index == last {
            LineCap::Round
        } else {
            LineCap::Butt
        };

        frame.stroke(
            &segment,
            Stroke::default()
                .with_width(config.border_width)
                .with_line_cap(line_cap)
                .with_color(color),
        );
    }
}

fn distance_behind_head(progress: f32, head: f32, reverse: bool) -> f32 {
    if reverse {
        (head - progress).rem_euclid(1.0)
    } else {
        (progress - head).rem_euclid(1.0)
    }
}

fn sample_border_beam_gradient(t: f32, from: Color, to: Color) -> Color {
    if t < 0.0 {
        return Color::TRANSPARENT;
    }

    if t <= BEAM_GRADIENT_STOP_MID {
        let local = (t / BEAM_GRADIENT_STOP_MID).clamp(0.0, 1.0);
        return mix_color(from, to, local);
    }

    if t < 1.0 {
        let fade = 1.0 - ((t - BEAM_GRADIENT_STOP_MID) / (1.0 - BEAM_GRADIENT_STOP_MID));
        return Color {
            a: fade * to.a.max(from.a),
            ..to
        };
    }

    Color::TRANSPARENT
}

fn beam_phase(elapsed: Duration, config: BorderBeam) -> f32 {
    let duration = config.duration_secs.max(0.01);
    let offset = config.initial_offset;
    // Magic UI uses `delay: -delay`, which advances phase by `delay` seconds.
    let linear = ((elapsed.as_secs_f32() + config.delay_secs) / duration).rem_euclid(1.0);
    let eased = apply_ease(linear, config.ease);

    if config.reverse {
        (1.0 - offset - eased).rem_euclid(1.0)
    } else {
        (offset + eased).rem_euclid(1.0)
    }
}

fn apply_ease(t: f32, ease: BorderBeamEase) -> f32 {
    let t = t.clamp(0.0, 1.0);
    match ease {
        BorderBeamEase::Linear => t,
        BorderBeamEase::EaseIn => t * t,
        BorderBeamEase::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
        BorderBeamEase::EaseInOut => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
            }
        }
    }
}

fn normalize_offset(initial_offset: f32) -> f32 {
    if initial_offset.abs() > 1.0 {
        (initial_offset / 100.0).rem_euclid(1.0)
    } else {
        initial_offset.rem_euclid(1.0)
    }
}

fn mix_color(from: Color, to: Color, progress: f32) -> Color {
    let progress = progress.clamp(0.0, 1.0);
    Color {
        r: from.r + (to.r - from.r) * progress,
        g: from.g + (to.g - from.g) * progress,
        b: from.b + (to.b - from.b) * progress,
        a: from.a + (to.a - from.a) * progress,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn border_beam_defaults_match_magic_ui() {
        let beam = BorderBeam::new();
        assert_eq!(beam.size, 50.0);
        assert_eq!(beam.duration_secs, 6.0);
        assert_eq!(beam.border_width, 1.0);
        assert_eq!(beam.radius, 20.0);
        assert!(beam.beam_enabled);
        assert!(beam.track_color.is_none());
        assert_eq!(beam.ease, BorderBeamEase::Linear);
    }

    #[test]
    fn setters_validate_ranges() {
        let beam = BorderBeam::new()
            .size(-10.0)
            .duration_secs(0.0)
            .delay_secs(-1.0)
            .border_width(-2.0)
            .initial_offset(25.0);

        assert_eq!(beam.size, 1.0);
        assert_eq!(beam.duration_secs, 0.01);
        assert_eq!(beam.delay_secs, 0.0);
        assert_eq!(beam.border_width, 0.0);
        assert!((beam.initial_offset - 0.25).abs() < 0.001);
    }

    #[test]
    fn beam_phase_supports_reverse_and_percent_offsets() {
        let normal = beam_phase(Duration::from_secs(3), BorderBeam::new());
        let reversed = beam_phase(Duration::from_secs(3), BorderBeam::new().reverse(true));
        let offset = beam_phase(
            Duration::from_secs(0),
            BorderBeam::new().initial_offset(25.0),
        );

        assert!((0.0..1.0).contains(&normal));
        assert!((0.0..1.0).contains(&reversed));
        assert!((offset - 0.25).abs() < 0.001);
    }

    #[test]
    fn gradient_matches_animated_beam_stop_layout() {
        let from = Color::from_rgb8(0xFF, 0xAA, 0x40);
        let to = Color::from_rgb8(0x9C, 0x40, 0xFF);

        assert_eq!(sample_border_beam_gradient(0.0, from, to), from);
        assert_eq!(sample_border_beam_gradient(0.325, from, to), to);
        assert!(sample_border_beam_gradient(1.0, from, to).a < 0.01);
    }

    #[test]
    fn distance_behind_head_wraps_clockwise() {
        let behind = distance_behind_head(0.1, 0.0, false);
        assert!((behind - 0.1).abs() < 0.001);

        let wrapped = distance_behind_head(0.05, 0.95, false);
        assert!((wrapped - 0.1).abs() < 0.001);
    }

    #[test]
    fn ease_in_out_is_symmetric() {
        let a = apply_ease(0.25, BorderBeamEase::EaseInOut);
        let b = apply_ease(0.75, BorderBeamEase::EaseInOut);
        assert!((a + b - 1.0).abs() < 0.001);
    }
}
