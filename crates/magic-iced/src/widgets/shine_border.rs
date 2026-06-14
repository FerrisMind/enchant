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
const BACKGROUND_SCALE: f32 = 3.0;

#[derive(Clone, Debug, PartialEq)]
pub struct ShineBorderProps {
    pub border_width: f32,
    pub duration_secs: f32,
    pub shine_colors: Vec<Color>,
    pub radius: f32,
    /// When false, animated shine is hidden (use during theme transitions).
    pub shine_enabled: bool,
}

impl Default for ShineBorderProps {
    fn default() -> Self {
        Self {
            border_width: 2.0,
            duration_secs: 14.0,
            shine_colors: vec![
                Color::from_rgb8(0xA0, 0x7C, 0xFE),
                Color::from_rgb8(0xFE, 0x8F, 0xB5),
                Color::from_rgb8(0xFF, 0xBE, 0x7B),
            ],
            radius: 12.0,
            shine_enabled: true,
        }
    }
}

impl ShineBorderProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn shine_enabled(mut self, enabled: bool) -> Self {
        self.shine_enabled = enabled;
        self
    }
}

pub fn shine_border<'a, Message: 'a>(
    elapsed: Duration,
    props: &ShineBorderProps,
) -> Element<'a, Message> {
    effect_canvas::<_, Message>(
        ShineBorderProgram {
            elapsed,
            props: props.clone(),
        },
        EffectCanvasProps::new()
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .into()
}

struct ShineBorderProgram {
    elapsed: Duration,
    props: ShineBorderProps,
}

impl<Message> EffectProgram<Message> for ShineBorderProgram {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<canvas::Geometry> {
        if !self.props.shine_enabled || self.props.shine_colors.is_empty() {
            return Vec::new();
        }

        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let track = RoundedRectTrack::new(bounds.size(), self.props.radius);
        let size = bounds.size();
        let width = size.width.max(1.0);
        let height = size.height.max(1.0);
        let (pos_x, pos_y) = shine_background_position(self.elapsed, self.props.duration_secs);
        let stops = shine_radial_stops(&self.props.shine_colors);
        let last = BEAM_SEGMENTS.saturating_sub(1);

        for index in 0..BEAM_SEGMENTS {
            let t0 = index as f32 / BEAM_SEGMENTS as f32;
            let t1 = (index + 1) as f32 / BEAM_SEGMENTS as f32;
            let p0 = track.sample(t0).point;
            let p1 = track.sample(t1).point;
            let mid = track.sample((t0 + t1) * 0.5);
            let (u, v) =
                element_to_gradient_uv(mid.point.x / width, mid.point.y / height, pos_x, pos_y);
            let color = sample_shine_radial(radial_distance(u, v), &stops);

            if color.a <= f32::EPSILON {
                continue;
            }

            let segment = canvas::Path::line(p0, p1);
            let line_cap = if index == 0 || index == last {
                LineCap::Round
            } else {
                LineCap::Butt
            };

            frame.stroke(
                &segment,
                Stroke::default()
                    .with_width(self.props.border_width)
                    .with_line_cap(line_cap)
                    .with_color(color),
            );
        }

        vec![frame.into_geometry()]
    }
}

/// Magic UI `@keyframes shine`: 0% → 50% → 100% background-position sweep.
fn shine_background_position(elapsed: Duration, duration_secs: f32) -> (f32, f32) {
    let duration = duration_secs.max(0.01);
    let progress = (elapsed.as_secs_f32() / duration).rem_euclid(1.0);

    if progress <= 0.5 {
        let t = progress / 0.5;
        (t, t)
    } else {
        let t = (progress - 0.5) / 0.5;
        (1.0 - t, 1.0 - t)
    }
}

/// Map element coordinates to the 300% background image (CSS `background-size: 300% 300%`).
fn element_to_gradient_uv(nx: f32, ny: f32, pos_x: f32, pos_y: f32) -> (f32, f32) {
    let u = (nx + (BACKGROUND_SCALE - 1.0) * pos_x) / BACKGROUND_SCALE;
    let v = (ny + (BACKGROUND_SCALE - 1.0) * pos_y) / BACKGROUND_SCALE;
    (u.clamp(0.0, 1.0), v.clamp(0.0, 1.0))
}

fn radial_distance(u: f32, v: f32) -> f32 {
    let dx = u - 0.5;
    let dy = v - 0.5;
    (dx * dx + dy * dy).sqrt() * 2.0
}

/// `radial-gradient(transparent, transparent, …colors…, transparent, transparent)`.
fn shine_radial_stops(colors: &[Color]) -> Vec<(f32, Color)> {
    let slot_count = colors.len() + 4;
    let denom = (slot_count - 1).max(1) as f32;

    (0..slot_count)
        .map(|index| {
            let position = index as f32 / denom;
            let color = if (0..2).contains(&index) || index >= slot_count - 2 {
                Color::TRANSPARENT
            } else {
                colors[index - 2]
            };
            (position, color)
        })
        .collect()
}

fn sample_shine_radial(distance: f32, stops: &[(f32, Color)]) -> Color {
    if stops.is_empty() {
        return Color::TRANSPARENT;
    }

    if distance <= stops[0].0 {
        return stops[0].1;
    }

    for window in stops.windows(2) {
        let (left_pos, left_color) = window[0];
        let (right_pos, right_color) = window[1];

        if distance <= right_pos {
            let span = (right_pos - left_pos).max(f32::EPSILON);
            let factor = ((distance - left_pos) / span).clamp(0.0, 1.0);
            return blend_colors(left_color, right_color, factor);
        }
    }

    stops
        .last()
        .map(|(_, color)| *color)
        .unwrap_or(Color::TRANSPARENT)
}

fn blend_colors(from: Color, to: Color, progress: f32) -> Color {
    let t = progress.clamp(0.0, 1.0);
    Color {
        r: from.r + (to.r - from.r) * t,
        g: from.g + (to.g - from.g) * t,
        b: from.b + (to.b - from.b) * t,
        a: from.a + (to.a - from.a) * t,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shine_border_defaults_include_three_colors() {
        let props = ShineBorderProps::new();
        assert_eq!(props.border_width, 2.0);
        assert_eq!(props.duration_secs, 14.0);
        assert_eq!(props.shine_colors.len(), 3);
        assert_eq!(props.radius, 12.0);
        assert!(props.shine_enabled);
    }

    #[test]
    fn shine_background_position_ping_pongs_diagonally() {
        let start = shine_background_position(Duration::from_secs(0), 14.0);
        let mid = shine_background_position(Duration::from_secs(7), 14.0);
        let end = shine_background_position(Duration::from_secs(14), 14.0);

        assert!((start.0 - 0.0).abs() < 0.001 && (start.1 - 0.0).abs() < 0.001);
        assert!((mid.0 - 1.0).abs() < 0.001 && (mid.1 - 1.0).abs() < 0.001);
        assert!((end.0 - 0.0).abs() < 0.001 && (end.1 - 0.0).abs() < 0.001);
    }

    #[test]
    fn shine_radial_stops_pad_colors_with_transparent_edges() {
        let stops = shine_radial_stops(&[Color::WHITE]);
        assert_eq!(stops.len(), 5);
        assert_eq!(stops[0].1.a, 0.0);
        assert_eq!(stops[2].1.a, 1.0);
    }

    #[test]
    fn sample_shine_radial_peaks_in_the_color_band() {
        let stops = shine_radial_stops(&[
            Color::from_rgb8(0xA0, 0x7C, 0xFE),
            Color::from_rgb8(0xFE, 0x8F, 0xB5),
        ]);
        let edge = sample_shine_radial(0.0, &stops);
        let center = sample_shine_radial(0.5, &stops);

        assert!(edge.a < 0.01);
        assert!(center.a > 0.5);
    }
}
