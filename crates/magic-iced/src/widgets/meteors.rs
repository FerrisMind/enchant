use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas, stable_noise};
use iced::widget::canvas;
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Vector};
use iced_shadcn::Theme;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeteorsProps {
    pub width: Length,
    pub height: Length,
    pub number: usize,
    pub min_delay_secs: f32,
    pub max_delay_secs: f32,
    pub min_duration_secs: f32,
    pub max_duration_secs: f32,
    pub angle_deg: f32,
    pub seed: u64,
    pub head_color: Option<Color>,
    pub tail_color: Option<Color>,
    pub tail_length: f32,
}

impl Default for MeteorsProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            height: Length::Fixed(220.0),
            number: 20,
            min_delay_secs: 0.2,
            max_delay_secs: 1.2,
            min_duration_secs: 2.0,
            max_duration_secs: 10.0,
            angle_deg: 215.0,
            seed: 11,
            head_color: None,
            tail_color: None,
            tail_length: 50.0,
        }
    }
}

impl MeteorsProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn meteors<'a, Message: 'a>(
    elapsed: Duration,
    props: MeteorsProps,
    theme: &Theme,
) -> Element<'a, Message> {
    effect_canvas::<_, Message>(
        MeteorsProgram {
            elapsed,
            props,
            head_color: props
                .head_color
                .unwrap_or(Color::from_rgba(1.0, 1.0, 1.0, 0.8)),
            tail_color: props.tail_color.unwrap_or(Color {
                a: 0.38,
                ..theme.palette.muted_foreground
            }),
        },
        EffectCanvasProps::new()
            .width(props.width)
            .height(props.height),
    )
    .into()
}

struct MeteorsProgram {
    elapsed: Duration,
    props: MeteorsProps,
    head_color: Color,
    tail_color: Color,
}

impl<Message> EffectProgram<Message> for MeteorsProgram {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let time = self.elapsed.as_secs_f32();
        let angle = self.props.angle_deg.to_radians();
        let direction = Vector::new(angle.cos(), angle.sin());
        let travel = bounds.width.max(bounds.height) * 1.5 + self.props.tail_length;

        for index in 0..self.props.number {
            let delay = lerp(
                self.props.min_delay_secs,
                self.props.max_delay_secs,
                stable_noise(self.props.seed, index as u64 * 3),
            );
            let duration = lerp(
                self.props.min_duration_secs,
                self.props.max_duration_secs,
                stable_noise(self.props.seed + 1, index as u64 * 7),
            )
            .max(0.1);
            let cycle = delay + duration;
            let local = time.rem_euclid(cycle);
            if local < delay {
                continue;
            }

            let progress = ((local - delay) / duration).clamp(0.0, 1.0);
            let start_x = stable_noise(self.props.seed + 2, index as u64 * 11) * bounds.width * 1.2
                - bounds.width * 0.1;
            let start_y = -bounds.height * 0.1
                + stable_noise(self.props.seed + 3, index as u64 * 13) * bounds.height * 0.2;

            let head = Point::new(
                start_x + direction.x * travel * progress,
                start_y + direction.y * travel * progress,
            );
            let tail = Point::new(
                head.x - direction.x * self.props.tail_length,
                head.y - direction.y * self.props.tail_length,
            );

            frame.stroke(
                &canvas::Path::line(head, tail),
                canvas::Stroke::default()
                    .with_width(1.2)
                    .with_color(self.tail_color),
            );
            frame.fill(&canvas::Path::circle(head, 1.5), self.head_color);
        }

        vec![frame.into_geometry()]
    }
}

fn lerp(min: f32, max: f32, progress: f32) -> f32 {
    min + (max - min) * progress.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meteors_defaults_match_docs() {
        let props = MeteorsProps::new();

        assert_eq!(props.number, 20);
        assert_eq!(props.min_delay_secs, 0.2);
        assert_eq!(props.max_delay_secs, 1.2);
        assert_eq!(props.angle_deg, 215.0);
    }
}
