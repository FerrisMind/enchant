use super::border_track::RoundedRectTrack;
use iced::Color;
use iced::Element;
use iced::Length;
use iced::Rectangle;
use iced::Renderer;
use iced::mouse;
use iced::widget::canvas::{self, LineCap, Stroke};
use std::time::Duration;

use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};

const BEAM_SEGMENTS: usize = 180;
/// Second gradient stop offset from `animated-beam.tsx` / Magic UI border beam.
const BEAM_GRADIENT_STOP_MID: f32 = 0.325;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderBeamProps {
    pub size: f32,
    pub duration_secs: f32,
    pub delay_secs: f32,
    pub color_from: Color,
    pub color_to: Color,
    pub reverse: bool,
    pub initial_offset: f32,
    pub border_width: f32,
    pub radius: f32,
    /// Optional static border stroke (e.g. theme border). When `None`, only the beam is drawn.
    pub track_color: Option<Color>,
    /// When false, animated beam is hidden (use during theme transitions).
    pub beam_enabled: bool,
}

impl Default for BorderBeamProps {
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
        }
    }
}

impl BorderBeamProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    pub fn initial_offset(mut self, initial_offset: f32) -> Self {
        self.initial_offset = initial_offset;
        self
    }

    pub fn track_color(mut self, color: Color) -> Self {
        self.track_color = Some(color);
        self
    }

    pub fn beam_enabled(mut self, enabled: bool) -> Self {
        self.beam_enabled = enabled;
        self
    }
}

pub fn border_beam<'a, Message: 'a>(
    elapsed: Duration,
    props: BorderBeamProps,
) -> Element<'a, Message> {
    effect_canvas::<_, Message>(
        BorderBeamProgram { elapsed, props },
        EffectCanvasProps::new()
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .into()
}

struct BorderBeamProgram {
    elapsed: Duration,
    props: BorderBeamProps,
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
        let track = RoundedRectTrack::new(bounds.size(), self.props.radius);

        if let Some(track_color) = self.props.track_color {
            frame.stroke(
                &track.path(iced::Point::ORIGIN),
                Stroke::default()
                    .with_width(self.props.border_width)
                    .with_color(track_color),
            );
        }

        if self.props.beam_enabled {
            draw_gradient_beam(&mut frame, &track, self.elapsed, self.props);
        }

        vec![frame.into_geometry()]
    }
}

fn draw_gradient_beam(
    frame: &mut canvas::Frame,
    track: &RoundedRectTrack,
    elapsed: Duration,
    props: BorderBeamProps,
) {
    let head = beam_phase(elapsed, props);
    let perimeter = track.perimeter().max(1.0);
    let beam_span = (props.size / perimeter).clamp(0.005, 1.0);
    let last = BEAM_SEGMENTS.saturating_sub(1);

    for index in 0..BEAM_SEGMENTS {
        let t0 = index as f32 / BEAM_SEGMENTS as f32;
        let t1 = (index + 1) as f32 / BEAM_SEGMENTS as f32;
        let mid_t = (t0 + t1) * 0.5;
        let dist = distance_behind_head(mid_t, head, props.reverse);

        if dist > beam_span {
            continue;
        }

        let along_beam = 1.0 - dist / beam_span;
        let color = sample_border_beam_gradient(
            along_beam,
            props.color_from,
            props.color_to,
        );

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
                .with_width(props.border_width)
                .with_line_cap(line_cap)
                .with_color(color),
        );
    }
}

/// Distance along the track behind the beam head, in progress units `[0, 1)`.
fn distance_behind_head(progress: f32, head: f32, reverse: bool) -> f32 {
    if reverse {
        (head - progress).rem_euclid(1.0)
    } else {
        (progress - head).rem_euclid(1.0)
    }
}

/// Gradient along the beam window — same stop layout as `animated_beam`.
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

fn beam_phase(elapsed: Duration, props: BorderBeamProps) -> f32 {
    let duration = props.duration_secs.max(0.01);
    let offset = normalize_offset(props.initial_offset);
    let shifted = (elapsed.as_secs_f32() + props.delay_secs) / duration;
    if props.reverse {
        (1.0 - offset - shifted).rem_euclid(1.0)
    } else {
        (offset + shifted).rem_euclid(1.0)
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
    fn border_beam_defaults_are_stable() {
        let props = BorderBeamProps::new();
        assert_eq!(props.size, 50.0);
        assert_eq!(props.duration_secs, 6.0);
        assert_eq!(props.border_width, 1.0);
        assert_eq!(props.radius, 20.0);
        assert!(props.beam_enabled);
        assert!(props.track_color.is_none());
    }

    #[test]
    fn beam_phase_supports_reverse_and_percent_offsets() {
        let normal = beam_phase(Duration::from_secs(3), BorderBeamProps::new());
        let reversed = beam_phase(Duration::from_secs(3), BorderBeamProps::new().reverse(true));
        let offset = beam_phase(
            Duration::from_secs(0),
            BorderBeamProps::new().initial_offset(25.0),
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
}
