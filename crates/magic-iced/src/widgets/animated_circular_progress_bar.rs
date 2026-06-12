use crate::primitives::{
    AnimatedValue, EffectCanvasProps, EffectProgram, MotionSpec, effect_canvas,
};
use iced::time::Instant;
use iced::widget::{canvas, container, stack, text};
use iced::widget::canvas::{LineCap, LineJoin};
use iced::{Color, Element, Length, Point, Radians, Rectangle, Renderer};
use iced_shadcn::Theme;
use std::f32::consts::PI;
use twill::prelude::{Easing, TransitionDuration};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimatedCircularProgressBarProps {
    pub min: f32,
    pub max: f32,
    pub value: f32,
    pub size: f32,
    pub stroke_width: f32,
    pub motion: MotionSpec,
    pub track_color: Option<Color>,
    pub fill_color: Option<Color>,
    pub show_label: bool,
    pub font_size: u16,
}

impl Default for AnimatedCircularProgressBarProps {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 100.0,
            value: 0.0,
            size: 160.0,
            stroke_width: 10.0,
            motion: MotionSpec::new()
                .duration(TransitionDuration::CustomMs(1000))
                .easing(Easing::Out),
            track_color: None,
            fill_color: None,
            show_label: true,
            font_size: 24,
        }
    }
}

impl AnimatedCircularProgressBarProps {
    pub fn new(value: f32) -> Self {
        Self {
            value,
            ..Self::default()
        }
    }

    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.min = min;
        self.max = max.max(min + f32::EPSILON);
        self
    }

    pub fn motion(mut self, motion: MotionSpec) -> Self {
        self.motion = motion;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ProgressFingerprint {
    min: f32,
    max: f32,
    value: f32,
}

#[derive(Clone, Debug)]
pub struct AnimatedCircularProgressBarState {
    progress: AnimatedValue,
    started: bool,
    fingerprint: ProgressFingerprint,
}

impl AnimatedCircularProgressBarState {
    pub fn new(props: &AnimatedCircularProgressBarProps) -> Self {
        Self {
            progress: AnimatedValue::with_spec(
                normalized_progress(props.min, props.max, props.min),
                props.motion,
            ),
            started: false,
            fingerprint: ProgressFingerprint {
                min: props.min,
                max: props.max,
                value: props.value,
            },
        }
    }

    pub fn tick(&mut self, now: Instant, props: &AnimatedCircularProgressBarProps) {
        let fingerprint = ProgressFingerprint {
            min: props.min,
            max: props.max,
            value: props.value,
        };
        let target = normalized_progress(props.min, props.max, props.value);

        if !self.started {
            self.started = true;
            self.progress = AnimatedValue::with_spec(
                normalized_progress(props.min, props.max, props.min),
                props.motion,
            );
            self.progress.transition(target, now);
            self.fingerprint = fingerprint;
            return;
        }

        self.progress.set_spec(props.motion);

        if self.fingerprint != fingerprint {
            let current = self.progress.value_at(now);
            self.progress = AnimatedValue::with_spec(current, props.motion);
            self.progress.transition(target, now);
            self.fingerprint = fingerprint;
        }
    }

    pub fn progress_at(&self, now: Instant) -> f32 {
        self.progress.value_at(now).clamp(0.0, 1.0)
    }
}

pub fn animated_circular_progress_bar<'a, Message: 'a>(
    state: &AnimatedCircularProgressBarState,
    now: Instant,
    props: &AnimatedCircularProgressBarProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let progress = state.progress_at(now);
    let track_color = props
        .track_color
        .or_else(|| theme.registry.color("magic.progress.ring.track"))
        .unwrap_or(Color {
            a: 0.16,
            ..theme.palette.foreground
        });
    let fill_color = props
        .fill_color
        .or_else(|| theme.registry.color("magic.progress.ring.fill"))
        .unwrap_or(theme.palette.primary);

    let canvas = effect_canvas::<_, Message>(
        CircularProgressProgram {
            progress,
            stroke_width: props.stroke_width,
            track_color,
            fill_color,
        },
        EffectCanvasProps::new()
            .width(Length::Fixed(props.size))
            .height(Length::Fixed(props.size)),
    );

    let mut layers: Vec<Element<'a, Message>> = vec![canvas.into()];

    if props.show_label {
        let label = container(
            text(format!("{}", (progress * 100.0).round() as i32)).size(u32::from(props.font_size)),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style({
            let color = theme.palette.foreground;
            move |_| iced::widget::container::Style {
                text_color: Some(color),
                ..iced::widget::container::Style::default()
            }
        });
        layers.push(
            container(label)
                .width(Length::Fixed(props.size))
                .height(Length::Fixed(props.size))
                .into(),
        );
    }

    container(stack(layers))
        .width(Length::Fixed(props.size))
        .height(Length::Fixed(props.size))
        .into()
}

struct CircularProgressProgram {
    progress: f32,
    stroke_width: f32,
    track_color: Color,
    fill_color: Color,
}

impl<Message> EffectProgram<Message> for CircularProgressProgram {
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
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let radius = (bounds.width.min(bounds.height) / 2.0 - self.stroke_width).max(1.0);

        frame.stroke(
            &canvas::Path::circle(center, radius),
            progress_stroke(self.stroke_width, self.track_color),
        );

        if self.progress > 0.0 {
            let mut builder = canvas::path::Builder::new();
            builder.arc(canvas::path::Arc {
                center,
                radius,
                start_angle: Radians(-PI / 2.0),
                end_angle: Radians(-PI / 2.0 + 2.0 * PI * self.progress),
            });
            let path = builder.build();

            frame.stroke(
                &path,
                progress_stroke(self.stroke_width, self.fill_color),
            );
        }

        vec![frame.into_geometry()]
    }
}

fn normalized_progress(min: f32, max: f32, value: f32) -> f32 {
    let span = (max - min).max(f32::EPSILON);
    ((value - min) / span).clamp(0.0, 1.0)
}

fn progress_stroke(width: f32, color: Color) -> canvas::Stroke<'static> {
    canvas::Stroke::default()
        .with_width(width)
        .with_line_cap(LineCap::Round)
        .with_line_join(LineJoin::Round)
        .with_color(color)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::time::Duration;

    #[test]
    fn normalized_progress_clamps_to_range() {
        assert_eq!(normalized_progress(0.0, 100.0, -20.0), 0.0);
        assert_eq!(normalized_progress(0.0, 100.0, 50.0), 0.5);
        assert_eq!(normalized_progress(0.0, 100.0, 200.0), 1.0);
    }

    #[test]
    fn circular_progress_animates_to_target() {
        let props = AnimatedCircularProgressBarProps::new(75.0).motion(
            MotionSpec::new()
                .duration(TransitionDuration::CustomMs(100))
                .easing(Easing::Linear),
        );
        let mut state = AnimatedCircularProgressBarState::new(&props);
        let start = Instant::now();

        state.tick(start, &props);

        let halfway = state.progress_at(start + Duration::from_millis(50));
        assert!(halfway > 0.35 && halfway < 0.40);
    }
}
