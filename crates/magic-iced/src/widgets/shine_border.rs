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

#[derive(Clone, Debug, PartialEq)]
pub struct ShineBorderProps {
    pub border_width: f32,
    pub duration_secs: f32,
    pub shine_colors: Vec<Color>,
    pub radius: f32,
}

impl Default for ShineBorderProps {
    fn default() -> Self {
        Self {
            border_width: 1.0,
            duration_secs: 14.0,
            shine_colors: vec![
                Color::from_rgb8(0xA0, 0x7C, 0xFE),
                Color::from_rgb8(0xFE, 0x8F, 0xB5),
                Color::from_rgb8(0xFF, 0xBE, 0x7B),
            ],
            radius: 20.0,
        }
    }
}

impl ShineBorderProps {
    pub fn new() -> Self {
        Self::default()
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
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let track = RoundedRectTrack::new(bounds.size(), self.props.radius);
        let path = track.path(iced::Point::ORIGIN);

        frame.stroke(
            &path,
            Stroke::default()
                .with_width(self.props.border_width)
                .with_color(Color {
                    a: 0.18,
                    ..self
                        .props
                        .shine_colors
                        .first()
                        .copied()
                        .unwrap_or(Color::WHITE)
                }),
        );

        for (index, color) in self.props.shine_colors.iter().copied().enumerate() {
            let phase = shine_phase(
                self.elapsed,
                self.props.duration_secs,
                index,
                self.props.shine_colors.len(),
            );
            for tail in 0..10 {
                let factor = tail as f32 / 9.0;
                let sample = track.sample(phase - factor * 0.05);
                frame.fill(
                    &canvas::Path::circle(sample.point, 1.5 + (1.0 - factor) * 2.2),
                    Color {
                        a: 0.06 + (1.0 - factor) * 0.35,
                        ..color
                    },
                );
            }
        }

        vec![frame.into_geometry()]
    }
}

fn shine_phase(elapsed: Duration, duration_secs: f32, index: usize, count: usize) -> f32 {
    let duration = duration_secs.max(0.01);
    let base = elapsed.as_secs_f32() / duration;
    let stagger = if count == 0 {
        0.0
    } else {
        index as f32 / count as f32
    };
    (base + stagger).rem_euclid(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shine_border_defaults_include_three_colors() {
        let props = ShineBorderProps::new();
        assert_eq!(props.border_width, 1.0);
        assert_eq!(props.duration_secs, 14.0);
        assert_eq!(props.shine_colors.len(), 3);
    }

    #[test]
    fn shine_phase_staggers_multiple_colors() {
        let first = shine_phase(Duration::from_secs(0), 10.0, 0, 3);
        let second = shine_phase(Duration::from_secs(0), 10.0, 1, 3);

        assert!(first < second);
        assert!(second < 1.0);
    }
}
