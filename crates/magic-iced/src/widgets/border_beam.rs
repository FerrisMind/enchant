use super::border_track::RoundedRectTrack;
use iced::Color;
use iced::Element;
use iced::Length;
use iced::Rectangle;
use iced::Renderer;
use iced::mouse;
use iced::widget::canvas::{self, Stroke};
use std::time::Duration;

use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};

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
        let head = beam_phase(self.elapsed, self.props);

        frame.stroke(
            &track.path(iced::Point::ORIGIN),
            Stroke::default()
                .with_width(self.props.border_width)
                .with_color(Color {
                    a: 0.16,
                    ..self.props.color_to
                }),
        );

        for index in (0..14).rev() {
            let factor = index as f32 / 13.0;
            let progress = if self.props.reverse {
                head + factor * 0.07
            } else {
                head - factor * 0.07
            };
            let sample = track.sample(progress);
            let color = mix_color(self.props.color_from, self.props.color_to, 1.0 - factor);
            let radius = 2.0 + (1.0 - factor) * (self.props.size * 0.08);
            frame.fill(
                &canvas::Path::circle(sample.point, radius),
                Color {
                    a: 0.12 + (1.0 - factor) * 0.6,
                    ..color
                },
            );
        }

        vec![frame.into_geometry()]
    }
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
}
