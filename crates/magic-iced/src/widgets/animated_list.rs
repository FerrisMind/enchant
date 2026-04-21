use iced::time::Instant;
use iced::widget::{column, container, float};
use iced::{Element, Length};
use twill::prelude::{Easing, TransitionDuration};

use crate::primitives::{AnimatedValue, MotionSpec};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimatedListProps {
    pub delay_ms: u64,
    pub gap: f32,
    pub scale_from: f32,
    pub width: Length,
}

impl Default for AnimatedListProps {
    fn default() -> Self {
        Self {
            delay_ms: 1000,
            gap: 16.0,
            scale_from: 0.85,
            width: Length::Fill,
        }
    }
}

impl AnimatedListProps {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct AnimatedListFingerprint {
    delay_ms: u64,
    gap: f32,
    scale_from: f32,
}

impl From<&AnimatedListProps> for AnimatedListFingerprint {
    fn from(props: &AnimatedListProps) -> Self {
        Self {
            delay_ms: props.delay_ms,
            gap: props.gap,
            scale_from: props.scale_from,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AnimatedListState {
    started_at: Option<Instant>,
    visible_target: usize,
    reveal_progress: AnimatedValue,
    fingerprint: AnimatedListFingerprint,
}

impl AnimatedListState {
    pub fn new(props: &AnimatedListProps) -> Self {
        Self {
            started_at: None,
            visible_target: 0,
            reveal_progress: AnimatedValue::with_spec(0.0, reveal_motion(props)),
            fingerprint: AnimatedListFingerprint::from(props),
        }
    }

    pub fn tick(&mut self, now: Instant, props: &AnimatedListProps) {
        let fingerprint = AnimatedListFingerprint::from(props);
        if self.fingerprint != fingerprint {
            *self = Self::new(props);
        }

        let start = *self.started_at.get_or_insert(now);
        let elapsed_ms = now.saturating_duration_since(start).as_millis() as u64;
        let next_visible_target = 1 + (elapsed_ms / props.delay_ms.max(1)) as usize;

        if next_visible_target != self.visible_target {
            self.visible_target = next_visible_target;
            self.reveal_progress = AnimatedValue::with_spec(0.0, reveal_motion(props));
            self.reveal_progress.transition(1.0, now);
        }
    }

    pub fn visible_count(&self, now: Instant, total: usize, props: &AnimatedListProps) -> usize {
        if total == 0 {
            return 0;
        }

        if self.started_at.is_none() {
            let start = now;
            let elapsed_ms = now.saturating_duration_since(start).as_millis() as u64;
            return (1 + (elapsed_ms / props.delay_ms.max(1)) as usize).min(total);
        }

        self.visible_target.max(1).min(total)
    }

    pub fn newest_progress(&self, now: Instant, props: &AnimatedListProps) -> f32 {
        if self.started_at.is_none() {
            return 0.0;
        }

        let _ = props;
        self.reveal_progress.value_at(now).clamp(0.0, 1.0)
    }
}

pub fn animated_list<'a, Message: 'a>(
    items: Vec<Element<'a, Message>>,
    state: &AnimatedListState,
    now: Instant,
    props: &AnimatedListProps,
) -> Element<'a, Message> {
    let visible = state
        .visible_count(now, items.len(), props)
        .min(items.len());
    let newest_progress = state.newest_progress(now, props);
    let newest_scale = props.scale_from + (1.0 - props.scale_from) * newest_progress;

    let mut children = Vec::with_capacity(visible);
    for (display_index, item) in items.into_iter().take(visible).rev().enumerate() {
        let child: Element<'a, Message> = if display_index == 0 {
            float(container(item).width(props.width))
                .scale(newest_scale)
                .into()
        } else {
            container(item).width(props.width).into()
        };
        children.push(child);
    }

    column(children)
        .spacing(props.gap)
        .width(props.width)
        .into()
}

fn reveal_motion(props: &AnimatedListProps) -> MotionSpec {
    MotionSpec::new()
        .duration(TransitionDuration::CustomMs(
            ((props.delay_ms as f32 * 0.35).round() as u64).clamp(1, u16::MAX as u64) as u16,
        ))
        .easing(Easing::Out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::time::Duration;

    #[test]
    fn animated_list_defaults_match_docs() {
        let props = AnimatedListProps::new();

        assert_eq!(props.delay_ms, 1000);
    }

    #[test]
    fn animated_list_reveals_items_over_time() {
        let props = AnimatedListProps::new();
        let mut state = AnimatedListState::new(&props);
        let start = Instant::now();

        state.tick(start, &props);
        assert_eq!(state.visible_count(start, 5, &props), 1);
        assert_eq!(
            state.visible_count(start + Duration::from_millis(2200), 5, &props),
            1
        );

        state.tick(start + Duration::from_millis(2200), &props);
        assert_eq!(
            state.visible_count(start + Duration::from_millis(2200), 5, &props),
            3
        );
        assert!(state.newest_progress(start + Duration::from_millis(2600), &props) > 0.9);
    }
}
