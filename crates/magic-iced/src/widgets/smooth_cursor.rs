use super::pointer::default_pointer_overlay;
use crate::primitives::{
    AnimatedValue, MotionSpec, POINTER_SPEED_THRESHOLD_PX_PER_SEC, PointerMotion,
};
use iced::mouse;
use iced::time::Instant;
use iced::widget::{container, mouse_area, stack};
use iced::{Color, Element, Length, Point, Vector};
use std::time::Duration;
use twill::prelude::{Easing, TransitionDuration};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SmoothCursorAction {
    Move(Point),
    Exit,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmoothCursorSpringConfig {
    pub damping: f32,
    pub stiffness: f32,
    pub mass: f32,
    pub rest_delta: f32,
}

impl Default for SmoothCursorSpringConfig {
    fn default() -> Self {
        Self {
            damping: 45.0,
            stiffness: 400.0,
            mass: 1.0,
            rest_delta: 0.001,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SmoothCursorState {
    motion: PointerMotion,
    x: AnimatedValue,
    y: AnimatedValue,
    rotation: AnimatedValue,
    scale: AnimatedValue,
    visible: bool,
    initialized: bool,
    last_move_at: Option<Instant>,
}

impl Default for SmoothCursorState {
    fn default() -> Self {
        Self {
            motion: PointerMotion::new(),
            x: AnimatedValue::new(0.0),
            y: AnimatedValue::new(0.0),
            rotation: AnimatedValue::new(0.0),
            scale: AnimatedValue::new(1.0),
            visible: false,
            initialized: false,
            last_move_at: None,
        }
    }
}

impl SmoothCursorState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_action(
        &mut self,
        action: &SmoothCursorAction,
        now: Instant,
        props: &SmoothCursorProps<'_, ()>,
    ) {
        match action {
            SmoothCursorAction::Move(position) => {
                let sample = self.motion.update(*position, now);
                let spec = motion_spec(props.spring_config);
                self.x.set_spec(spec);
                self.y.set_spec(spec);
                self.rotation.set_spec(spec);
                self.scale.set_spec(spec);

                if !self.initialized {
                    self.x = AnimatedValue::with_spec(position.x, spec);
                    self.y = AnimatedValue::with_spec(position.y, spec);
                    self.rotation = AnimatedValue::with_spec(sample.angle_deg, spec);
                    self.scale = AnimatedValue::with_spec(1.0, spec);
                    self.initialized = true;
                }

                self.x.transition(position.x, now);
                self.y.transition(position.y, now);
                self.rotation.transition(sample.angle_deg, now);
                self.scale.transition(
                    if sample.speed > POINTER_SPEED_THRESHOLD_PX_PER_SEC {
                        0.95
                    } else {
                        1.0
                    },
                    now,
                );
                self.visible = true;
                self.last_move_at = Some(now);
            }
            SmoothCursorAction::Exit => {
                self.motion.reset();
                self.visible = false;
            }
        }
    }

    pub fn tick(&mut self, now: Instant, props: &SmoothCursorProps<'_, ()>) {
        let spec = motion_spec(props.spring_config);
        self.x.set_spec(spec);
        self.y.set_spec(spec);
        self.rotation.set_spec(spec);
        self.scale.set_spec(spec);

        if self
            .last_move_at
            .is_some_and(|last| now.saturating_duration_since(last) > Duration::from_millis(150))
        {
            self.scale.transition(1.0, now);
        }
    }

    pub fn position_at(&self, now: Instant) -> Option<Point> {
        if !self.visible {
            return None;
        }

        Some(Point::new(self.x.value_at(now), self.y.value_at(now)))
    }

    pub fn rotation_at(&self, now: Instant) -> f32 {
        self.rotation.value_at(now)
    }

    pub fn scale_at(&self, now: Instant) -> f32 {
        self.scale.value_at(now)
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

pub struct SmoothCursorProps<'a, Message> {
    pub cursor: Option<Element<'a, Message>>,
    pub spring_config: SmoothCursorSpringConfig,
    pub width: Length,
    pub height: Length,
    pub enabled: bool,
    pub color: Option<Color>,
    pub size: f32,
}

impl<Message> Default for SmoothCursorProps<'_, Message> {
    fn default() -> Self {
        Self {
            cursor: None,
            spring_config: SmoothCursorSpringConfig::default(),
            width: Length::Fill,
            height: Length::Fill,
            enabled: true,
            color: None,
            size: 28.0,
        }
    }
}

impl<'a, Message> SmoothCursorProps<'a, Message> {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn smooth_cursor<'a, Message: Clone + 'a>(
    state: &SmoothCursorState,
    now: Instant,
    props: SmoothCursorProps<'a, Message>,
    on_action: impl Fn(SmoothCursorAction) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    if !props.enabled {
        return iced::widget::Space::new().into();
    }

    let (position, overlay_child): (Point, Element<'a, Message>) =
        if let Some(position) = state.position_at(now) {
            (
                position,
                props.cursor.unwrap_or_else(|| {
                    default_pointer_overlay(props.size, props.color.unwrap_or(Color::BLACK))
                }),
            )
        } else {
            (Point::ORIGIN, iced::widget::Space::new().into())
        };

    let overlay: Element<'a, Message> = iced::widget::float(
        container(overlay_child)
            .width(Length::Shrink)
            .height(Length::Shrink),
    )
    .translate(move |_bounds, _viewport| Vector::new(position.x, position.y))
    .scale(state.scale_at(now))
    .into();

    mouse_area(stack(vec![
        container(iced::widget::Space::new())
            .width(props.width)
            .height(props.height)
            .into(),
        overlay,
    ]))
    .on_move({
        let on_action = on_action.clone();
        move |position| on_action(SmoothCursorAction::Move(position))
    })
    .on_exit(on_action(SmoothCursorAction::Exit))
    .interaction(if state.is_visible() {
        mouse::Interaction::Hidden
    } else {
        mouse::Interaction::None
    })
    .into()
}

fn motion_spec(config: SmoothCursorSpringConfig) -> MotionSpec {
    let duration_ms =
        ((config.mass * 220.0) + (config.damping * 2.0) + (4000.0 / config.stiffness.max(1.0)))
            .round()
            .clamp(60.0, 260.0) as u16;

    MotionSpec::new()
        .duration(TransitionDuration::CustomMs(duration_ms))
        .easing(Easing::Out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn props() -> SmoothCursorProps<'static, ()> {
        SmoothCursorProps::new()
    }

    #[test]
    fn smooth_cursor_defaults_match_docs() {
        let props = props();

        assert_eq!(props.spring_config.damping, 45.0);
        assert_eq!(props.spring_config.stiffness, 400.0);
        assert_eq!(props.spring_config.mass, 1.0);
        assert_eq!(props.spring_config.rest_delta, 0.001);
    }

    #[test]
    fn smooth_cursor_state_tracks_visibility() {
        let mut state = SmoothCursorState::new();
        let now = Instant::now();
        let props = props();

        state.apply_action(
            &SmoothCursorAction::Move(Point::new(10.0, 20.0)),
            now,
            &props,
        );
        assert!(state.is_visible());
        assert_eq!(state.position_at(now), Some(Point::new(10.0, 20.0)));

        state.apply_action(&SmoothCursorAction::Exit, now, &props);
        assert!(!state.is_visible());
    }

    #[test]
    fn smooth_cursor_scale_responds_only_above_pointer_speed_threshold() {
        let props = props();
        let start = Instant::now();

        let mut slow = SmoothCursorState::new();
        slow.apply_action(
            &SmoothCursorAction::Move(Point::new(0.0, 0.0)),
            start,
            &props,
        );
        slow.apply_action(
            &SmoothCursorAction::Move(Point::new(10.0, 0.0)),
            start + Duration::from_millis(100),
            &props,
        );
        let slow_scale = slow.scale_at(start + Duration::from_millis(200));

        let mut fast = SmoothCursorState::new();
        fast.apply_action(
            &SmoothCursorAction::Move(Point::new(0.0, 0.0)),
            start,
            &props,
        );
        fast.apply_action(
            &SmoothCursorAction::Move(Point::new(20.0, 0.0)),
            start + Duration::from_millis(100),
            &props,
        );
        let fast_scale = fast.scale_at(start + Duration::from_millis(200));

        assert_eq!(slow_scale, 1.0);
        assert!(fast_scale < slow_scale);
    }
}
