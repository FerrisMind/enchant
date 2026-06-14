use crate::primitives::{AnimatedValue, MotionSpec};
use iced::time::Instant;
use iced::widget::{container, float, text};
use iced::{Color, Element, Length, Vector};
use iced_shadcn::{ButtonProps, ButtonVariant, Theme, button_content};
use twill::prelude::{Easing, TransitionDuration};

const ENTER_OFFSET_Y: f32 = 50.0;
const EXIT_OFFSET_X: f32 = 50.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Idle,
    ExitUnsubscribeLabel,
    EnterSubscribeLabel,
    FadeUnsubscribeIn,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct AnimatedSubscribeFingerprint {
    subscribe_status: Option<bool>,
    subscribe_label: &'static str,
    subscribed_label: &'static str,
}

impl From<&AnimatedSubscribeButtonProps> for AnimatedSubscribeFingerprint {
    fn from(props: &AnimatedSubscribeButtonProps) -> Self {
        Self {
            subscribe_status: props.subscribe_status,
            subscribe_label: props.subscribe_label,
            subscribed_label: props.subscribed_label,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AnimatedSubscribeButtonProps {
    pub subscribe_label: &'static str,
    pub subscribed_label: &'static str,
    /// When `Some`, the button is controlled and does not toggle on its own.
    pub subscribe_status: Option<bool>,
    pub height: f32,
    pub radius: f32,
    pub padding_h: f32,
    pub exit_motion: MotionSpec,
    pub enter_motion: MotionSpec,
    pub fade_motion: MotionSpec,
    pub button: ButtonProps,
}

impl Default for AnimatedSubscribeButtonProps {
    fn default() -> Self {
        Self {
            subscribe_label: "Subscribe",
            subscribed_label: "Subscribed",
            subscribe_status: None,
            height: 40.0,
            radius: 8.0,
            padding_h: 24.0,
            exit_motion: MotionSpec::new()
                .duration(TransitionDuration::CustomMs(100))
                .easing(Easing::Linear),
            enter_motion: MotionSpec::new()
                .duration(TransitionDuration::Ms150)
                .easing(Easing::Out),
            fade_motion: MotionSpec::new()
                .duration(TransitionDuration::Ms150)
                .easing(Easing::InOut),
            button: ButtonProps::new().variant(ButtonVariant::Default),
        }
    }
}

impl AnimatedSubscribeButtonProps {
    pub fn new(subscribe_label: &'static str, subscribed_label: &'static str) -> Self {
        Self {
            subscribe_label,
            subscribed_label,
            ..Self::default()
        }
    }

    pub fn subscribe_status(mut self, subscribe_status: bool) -> Self {
        self.subscribe_status = Some(subscribe_status);
        self
    }

    pub fn button(mut self, button: ButtonProps) -> Self {
        self.button = button;
        self
    }
}

#[derive(Clone, Debug)]
pub struct AnimatedSubscribeButtonState {
    is_subscribed: bool,
    phase: Phase,
    label_opacity: AnimatedValue,
    label_offset_x: AnimatedValue,
    label_offset_y: AnimatedValue,
    shell_opacity: AnimatedValue,
    fingerprint: AnimatedSubscribeFingerprint,
}

impl AnimatedSubscribeButtonState {
    pub fn new(props: &AnimatedSubscribeButtonProps) -> Self {
        let is_subscribed = props.subscribe_status.unwrap_or(false);
        Self {
            is_subscribed,
            phase: Phase::Idle,
            label_opacity: AnimatedValue::with_spec(1.0, props.fade_motion),
            label_offset_x: AnimatedValue::with_spec(0.0, props.exit_motion),
            label_offset_y: AnimatedValue::with_spec(0.0, props.enter_motion),
            shell_opacity: AnimatedValue::with_spec(1.0, props.fade_motion),
            fingerprint: AnimatedSubscribeFingerprint::from(props),
        }
    }

    pub fn is_subscribed(&self) -> bool {
        self.is_subscribed
    }

    pub fn is_animating(&self, now: Instant) -> bool {
        self.phase != Phase::Idle
            || self.label_opacity.is_animating(now)
            || self.label_offset_x.is_animating(now)
            || self.label_offset_y.is_animating(now)
            || self.shell_opacity.is_animating(now)
    }

    pub fn press(&mut self, now: Instant, props: &AnimatedSubscribeButtonProps) {
        if props.subscribe_status.is_some() || self.phase != Phase::Idle {
            return;
        }

        if self.is_subscribed {
            self.begin_unsubscribe(now, props);
        } else {
            self.begin_subscribe(now, props);
        }
    }

    pub fn set_subscribe_status(
        &mut self,
        subscribe_status: bool,
        now: Instant,
        props: &AnimatedSubscribeButtonProps,
    ) {
        if self.phase != Phase::Idle {
            return;
        }

        if subscribe_status == self.is_subscribed {
            return;
        }

        if subscribe_status {
            self.begin_subscribe(now, props);
        } else {
            self.begin_unsubscribe(now, props);
        }
    }

    pub fn tick(&mut self, now: Instant, props: &AnimatedSubscribeButtonProps) {
        let fingerprint = AnimatedSubscribeFingerprint::from(props);
        if self.fingerprint != fingerprint {
            *self = Self::new(props);
        }

        if let Some(status) = props.subscribe_status {
            self.set_subscribe_status(status, now, props);
        }

        match self.phase {
            Phase::ExitUnsubscribeLabel => {
                if !self.label_offset_x.is_animating(now) {
                    self.start_enter_subscribed(now, props);
                }
            }
            Phase::EnterSubscribeLabel => {
                if !self.label_opacity.is_animating(now) && !self.label_offset_y.is_animating(now) {
                    self.is_subscribed = true;
                    self.phase = Phase::Idle;
                }
            }
            Phase::FadeUnsubscribeIn => {
                if !self.shell_opacity.is_animating(now) {
                    self.is_subscribed = false;
                    self.phase = Phase::Idle;
                }
            }
            Phase::Idle => {}
        }
    }

    fn begin_subscribe(&mut self, now: Instant, props: &AnimatedSubscribeButtonProps) {
        self.phase = Phase::ExitUnsubscribeLabel;
        self.label_opacity = AnimatedValue::with_spec(1.0, props.fade_motion);
        self.label_offset_x = AnimatedValue::with_spec(0.0, props.exit_motion);
        self.label_offset_x.transition(EXIT_OFFSET_X, now);
    }

    fn start_enter_subscribed(&mut self, now: Instant, props: &AnimatedSubscribeButtonProps) {
        self.phase = Phase::EnterSubscribeLabel;
        self.shell_opacity = AnimatedValue::with_spec(0.0, props.fade_motion);
        self.shell_opacity.transition(1.0, now);
        self.label_opacity = AnimatedValue::with_spec(0.0, props.fade_motion);
        self.label_opacity.transition(1.0, now);
        self.label_offset_y = AnimatedValue::with_spec(-ENTER_OFFSET_Y, props.enter_motion);
        self.label_offset_y.transition(0.0, now);
        self.label_offset_x = AnimatedValue::with_spec(0.0, props.exit_motion);
    }

    fn begin_unsubscribe(&mut self, now: Instant, props: &AnimatedSubscribeButtonProps) {
        self.phase = Phase::FadeUnsubscribeIn;
        self.shell_opacity = AnimatedValue::with_spec(0.0, props.fade_motion);
        self.shell_opacity.transition(1.0, now);
        self.label_opacity = AnimatedValue::with_spec(1.0, props.fade_motion);
        self.label_offset_x = AnimatedValue::with_spec(0.0, props.exit_motion);
        self.label_offset_y = AnimatedValue::with_spec(0.0, props.enter_motion);
    }

    fn display_subscribed(&self) -> bool {
        match self.phase {
            Phase::EnterSubscribeLabel => true,
            Phase::ExitUnsubscribeLabel | Phase::FadeUnsubscribeIn => false,
            Phase::Idle => self.is_subscribed,
        }
    }

    fn shell_opacity_at(&self, now: Instant) -> f32 {
        self.shell_opacity.value_at(now).clamp(0.0, 1.0)
    }

    fn label_opacity_at(&self, now: Instant) -> f32 {
        self.label_opacity.value_at(now).clamp(0.0, 1.0)
    }

    fn label_offset_at(&self, now: Instant) -> Vector {
        Vector::new(
            self.label_offset_x.value_at(now),
            self.label_offset_y.value_at(now),
        )
    }
}

pub fn animated_subscribe_button<'a, Message: Clone + 'a>(
    state: &AnimatedSubscribeButtonState,
    now: Instant,
    props: &AnimatedSubscribeButtonProps,
    on_press: Option<Message>,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let label = if state.display_subscribed() {
        props.subscribed_label
    } else {
        props.subscribe_label
    };

    let shell_alpha = state.shell_opacity_at(now);
    let label_alpha = state.label_opacity_at(now);
    let offset = state.label_offset_at(now);
    let foreground = Color {
        a: label_alpha,
        ..theme.palette.primary_foreground
    };

    let label_element: Element<'a, Message> = text(label)
        .size(14)
        .style(move |_| iced::widget::text::Style {
            color: Some(foreground),
        })
        .into();

    let label_element = if offset.x.abs() > f32::EPSILON || offset.y.abs() > f32::EPSILON {
        float(label_element)
            .translate(move |_bounds, _viewport| offset)
            .into()
    } else {
        label_element
    };

    let shell = button_content(
        container(label_element)
            .height(Length::Fill)
            .center_y(Length::Fill)
            .center_x(Length::Fill),
        on_press,
        props.button,
        theme,
    );

    container(shell)
        .height(Length::Fixed(props.height))
        .padding([0.0, props.padding_h])
        .style(move |_| iced::widget::container::Style {
            text_color: Some(Color {
                a: shell_alpha,
                ..theme.palette.primary_foreground
            }),
            ..Default::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::time::Duration;

    #[test]
    fn animated_subscribe_button_defaults_match_reference_labels() {
        let props = AnimatedSubscribeButtonProps::new("Subscribe", "Subscribed");
        assert_eq!(props.subscribe_label, "Subscribe");
        assert_eq!(props.subscribed_label, "Subscribed");
        assert_eq!(props.height, 40.0);
        assert_eq!(props.radius, 8.0);
        assert_eq!(
            props.exit_motion.duration,
            TransitionDuration::CustomMs(100)
        );
    }

    #[test]
    fn uncontrolled_press_toggles_subscribed_state() {
        let props = AnimatedSubscribeButtonProps::new("Subscribe", "Subscribed");
        let mut state = AnimatedSubscribeButtonState::new(&props);
        let start = Instant::now();

        state.press(start, &props);
        state.tick(start + Duration::from_millis(120), &props);
        state.tick(start + Duration::from_millis(320), &props);
        assert!(state.is_subscribed());

        state.press(start + Duration::from_millis(400), &props);
        state.tick(start + Duration::from_millis(560), &props);
        assert!(!state.is_subscribed());
    }

    #[test]
    fn controlled_mode_follows_subscribe_status() {
        let props =
            AnimatedSubscribeButtonProps::new("Subscribe", "Subscribed").subscribe_status(false);
        let mut state = AnimatedSubscribeButtonState::new(&props);
        let start = Instant::now();

        let controlled =
            AnimatedSubscribeButtonProps::new("Subscribe", "Subscribed").subscribe_status(true);
        state.tick(start, &controlled);
        state.tick(start + Duration::from_millis(320), &controlled);
        assert!(state.is_subscribed());
    }
}
