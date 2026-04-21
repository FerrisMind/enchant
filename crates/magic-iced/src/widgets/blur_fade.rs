use crate::primitives::{AnimatedValue, MotionSpec};
use iced::time::Instant;
use iced::widget::{float, opaque};
use iced::{Color, Element, Length, Vector};
use iced_shadcn::{DecorativeSurfaceProps, Theme, decorative_surface};
use twill::prelude::{Easing, TransitionDuration};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BlurFadeDirection {
    Up,
    #[default]
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlurFadeProps {
    pub motion: MotionSpec,
    pub offset: f32,
    pub direction: BlurFadeDirection,
    pub veil_opacity: f32,
    pub veil_color: Option<Color>,
}

impl Default for BlurFadeProps {
    fn default() -> Self {
        Self {
            motion: MotionSpec::new()
                .duration(TransitionDuration::CustomMs(400))
                .delay(TransitionDuration::CustomMs(40))
                .easing(Easing::Out),
            offset: 6.0,
            direction: BlurFadeDirection::Down,
            veil_opacity: 0.52,
            veil_color: None,
        }
    }
}

impl BlurFadeProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn motion(mut self, motion: MotionSpec) -> Self {
        self.motion = motion;
        self
    }

    pub fn offset(mut self, offset: f32) -> Self {
        self.offset = offset.max(0.0);
        self
    }

    pub fn direction(mut self, direction: BlurFadeDirection) -> Self {
        self.direction = direction;
        self
    }

    pub fn veil_opacity(mut self, veil_opacity: f32) -> Self {
        self.veil_opacity = veil_opacity.clamp(0.0, 1.0);
        self
    }

    pub fn veil_color(mut self, veil_color: Color) -> Self {
        self.veil_color = Some(veil_color);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct BlurFadeFingerprint {
    motion: MotionSpec,
    offset: f32,
    direction: BlurFadeDirection,
    veil_opacity: f32,
}

impl From<&BlurFadeProps> for BlurFadeFingerprint {
    fn from(props: &BlurFadeProps) -> Self {
        Self {
            motion: props.motion,
            offset: props.offset,
            direction: props.direction,
            veil_opacity: props.veil_opacity,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BlurFadeState {
    progress: AnimatedValue,
    started: bool,
    fingerprint: BlurFadeFingerprint,
}

impl BlurFadeState {
    pub fn new(props: &BlurFadeProps) -> Self {
        Self {
            progress: AnimatedValue::with_spec(0.0, props.motion),
            started: false,
            fingerprint: BlurFadeFingerprint::from(props),
        }
    }

    pub fn tick(&mut self, now: Instant, props: &BlurFadeProps) {
        let fingerprint = BlurFadeFingerprint::from(props);
        if self.fingerprint != fingerprint {
            self.progress = AnimatedValue::with_spec(0.0, props.motion);
            self.started = false;
            self.fingerprint = fingerprint;
        }

        self.progress.set_spec(props.motion);

        if !self.started {
            self.started = true;
            self.progress.transition(1.0, now);
        }
    }

    pub fn progress_at(&self, now: Instant) -> f32 {
        self.progress.value_at(now).clamp(0.0, 1.0)
    }
}

pub fn blur_fade<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    state: &BlurFadeState,
    now: Instant,
    props: &BlurFadeProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let progress = state.progress_at(now);
    let translation = translation_vector(props.direction, props.offset, progress);
    let veil_opacity = props.veil_opacity;

    let content: Element<'a, Message> = if translation != Vector::ZERO {
        float(content.into())
            .translate(move |_bounds, _viewport| translation)
            .into()
    } else {
        content.into()
    };

    let veil_base = props
        .veil_color
        .or_else(|| theme.registry.color("magic.blur.veil"))
        .unwrap_or(theme.palette.background);
    let veil = opaque(
        iced::widget::container(iced::widget::Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| iced::widget::container::Style {
                background: Some(iced::Background::Color(Color {
                    a: veil_opacity * (1.0 - progress),
                    ..veil_base
                })),
                ..iced::widget::container::Style::default()
            }),
    );

    decorative_surface(
        content,
        Vec::new(),
        vec![veil],
        DecorativeSurfaceProps::new().clip(true),
        theme,
    )
}

fn translation_vector(direction: BlurFadeDirection, offset: f32, progress: f32) -> Vector {
    let delta = (1.0 - progress.clamp(0.0, 1.0)) * offset.max(0.0);

    match direction {
        BlurFadeDirection::Down => Vector::new(0.0, -delta),
        BlurFadeDirection::Up => Vector::new(0.0, delta),
        BlurFadeDirection::Left => Vector::new(delta, 0.0),
        BlurFadeDirection::Right => Vector::new(-delta, 0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::time::Duration;

    #[test]
    fn blur_fade_progress_runs_once_and_clamps() {
        let props = BlurFadeProps::new().motion(
            MotionSpec::new()
                .duration(TransitionDuration::CustomMs(100))
                .easing(Easing::Linear),
        );
        let mut state = BlurFadeState::new(&props);
        let start = Instant::now();

        state.tick(start, &props);

        let halfway = state.progress_at(start + Duration::from_millis(50));
        let end = state.progress_at(start + Duration::from_millis(200));

        assert!(halfway > 0.45 && halfway < 0.55);
        assert_eq!(end, 1.0);
    }

    #[test]
    fn translation_vector_matches_direction_contract() {
        assert_eq!(
            translation_vector(BlurFadeDirection::Down, 10.0, 0.0),
            Vector::new(0.0, -10.0)
        );
        assert_eq!(
            translation_vector(BlurFadeDirection::Up, 10.0, 0.0),
            Vector::new(0.0, 10.0)
        );
        assert_eq!(
            translation_vector(BlurFadeDirection::Left, 10.0, 0.0),
            Vector::new(10.0, 0.0)
        );
        assert_eq!(
            translation_vector(BlurFadeDirection::Right, 10.0, 0.0),
            Vector::new(-10.0, 0.0)
        );
    }
}
