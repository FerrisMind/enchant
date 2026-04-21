use iced::animation::Animation;
use iced::time::Instant;
use twill::backends::iced::{to_duration, to_easing};
use twill::prelude::{Easing, TransitionDuration};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RepeatMode {
    #[default]
    Never,
    Count(u32),
    Forever,
    AutoReverseCount(u32),
    AutoReverseForever,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MotionSpec {
    pub duration: TransitionDuration,
    pub delay: TransitionDuration,
    pub easing: Easing,
    pub repeat: RepeatMode,
}

impl Default for MotionSpec {
    fn default() -> Self {
        Self {
            duration: TransitionDuration::Ms300,
            delay: TransitionDuration::Ms0,
            easing: Easing::InOut,
            repeat: RepeatMode::Never,
        }
    }
}

impl MotionSpec {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn duration(mut self, duration: TransitionDuration) -> Self {
        self.duration = duration;
        self
    }

    pub fn delay(mut self, delay: TransitionDuration) -> Self {
        self.delay = delay;
        self
    }

    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    pub fn repeat(mut self, repeat: RepeatMode) -> Self {
        self.repeat = repeat;
        self
    }
}

#[derive(Clone, Debug)]
pub struct AnimatedValue {
    spec: MotionSpec,
    animation: Animation<f32>,
}

impl AnimatedValue {
    pub fn new(initial: f32) -> Self {
        Self {
            spec: MotionSpec::default(),
            animation: Animation::new(initial),
        }
    }

    pub fn with_spec(initial: f32, spec: MotionSpec) -> Self {
        Self {
            spec,
            animation: configured_animation(initial, spec),
        }
    }

    pub fn spec(&self) -> MotionSpec {
        self.spec
    }

    pub fn set_spec(&mut self, spec: MotionSpec) {
        self.spec = spec;
    }

    pub fn target(&self) -> f32 {
        self.animation.value()
    }

    pub fn transition(&mut self, target: f32, at: Instant) {
        let current = self.value_at(at);
        let mut animation = configured_animation(current, self.spec);
        animation.go_mut(target, at);
        self.animation = animation;
    }

    pub fn value_at(&self, at: Instant) -> f32 {
        self.animation.interpolate_with(|value| value, at)
    }

    pub fn is_animating(&self, at: Instant) -> bool {
        self.animation.is_animating(at)
    }
}

fn configured_animation(initial: f32, spec: MotionSpec) -> Animation<f32> {
    let animation = Animation::new(initial)
        .duration(to_duration(spec.duration))
        .delay(to_duration(spec.delay))
        .easing(to_easing(spec.easing));

    match spec.repeat {
        RepeatMode::Never => animation,
        RepeatMode::Count(count) => animation.repeat(count),
        RepeatMode::Forever => animation.repeat_forever(),
        RepeatMode::AutoReverseCount(count) => animation.repeat(count).auto_reverse(),
        RepeatMode::AutoReverseForever => animation.repeat_forever().auto_reverse(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::time::Duration;

    fn approx_eq(left: f32, right: f32) {
        assert!(
            (left - right).abs() < 0.2,
            "expected {left} ~= {right} (delta {})",
            (left - right).abs()
        );
    }

    #[test]
    fn animated_value_respects_delay_before_starting() {
        let start = Instant::now();
        let mut animated = AnimatedValue::with_spec(
            0.0,
            MotionSpec::new()
                .duration(TransitionDuration::Ms100)
                .delay(TransitionDuration::Ms100)
                .easing(Easing::Linear),
        );

        animated.transition(10.0, start);

        approx_eq(animated.value_at(start + Duration::from_millis(50)), 0.0);
        approx_eq(animated.value_at(start + Duration::from_millis(150)), 5.0);
    }

    #[test]
    fn animated_value_repeat_auto_reverse_traverses_back_during_reverse_leg() {
        let start = Instant::now();
        let mut animated = AnimatedValue::with_spec(
            0.0,
            MotionSpec::new()
                .duration(TransitionDuration::Ms100)
                .easing(Easing::Linear)
                .repeat(RepeatMode::AutoReverseCount(1)),
        );

        animated.transition(10.0, start);

        approx_eq(animated.value_at(start + Duration::from_millis(50)), 5.0);
        approx_eq(animated.value_at(start + Duration::from_millis(150)), 5.0);
        approx_eq(animated.value_at(start + Duration::from_millis(200)), 0.0);
        assert!(animated.is_animating(start + Duration::from_millis(250)));
    }

    #[test]
    fn animated_value_uses_twill_easing_mapping() {
        let start = Instant::now();
        let mut animated = AnimatedValue::with_spec(
            0.0,
            MotionSpec::new()
                .duration(TransitionDuration::Ms100)
                .easing(Easing::In),
        );

        animated.transition(10.0, start);

        let eased_half = animated.value_at(start + Duration::from_millis(50));
        assert!(
            eased_half < 5.0,
            "expected ease-in midpoint below linear midpoint"
        );
    }
}
