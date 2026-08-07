use crate::primitives::{AnimatedValue, MotionSpec};
use iced::time::Instant;
use iced::widget::{container, float, text};
use iced::{Color, Element, Length, Vector};
use iced_shadcn::Theme;
use twill::prelude::{Easing, TransitionDuration};

#[derive(Clone, Debug, PartialEq)]
pub struct WordRotateProps {
    pub words: Vec<String>,
    pub interval_ms: u64,
    pub motion: MotionSpec,
    pub font_size: u16,
    pub enter_offset: f32,
    pub exit_offset: f32,
}

impl Default for WordRotateProps {
    fn default() -> Self {
        Self {
            words: Vec::new(),
            interval_ms: 2500,
            motion: MotionSpec::new()
                .duration(TransitionDuration::CustomMs(250))
                .easing(Easing::Out),
            font_size: 36,
            enter_offset: -50.0,
            exit_offset: 50.0,
        }
    }
}

impl WordRotateProps {
    pub fn new<I, S>(words: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            words: words.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }

    pub fn words<I, S>(mut self, words: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.words = words.into_iter().map(Into::into).collect();
        self
    }

    pub fn interval_ms(mut self, interval_ms: u64) -> Self {
        self.interval_ms = interval_ms.max(1);
        self
    }

    pub fn motion(mut self, motion: MotionSpec) -> Self {
        self.motion = motion;
        self
    }

    pub fn font_size(mut self, font_size: u16) -> Self {
        self.font_size = font_size.max(1);
        self
    }

    pub fn enter_offset(mut self, enter_offset: f32) -> Self {
        self.enter_offset = enter_offset;
        self
    }

    pub fn exit_offset(mut self, exit_offset: f32) -> Self {
        self.exit_offset = exit_offset;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WordRotatePhase {
    Entering,
    Stable,
    Exiting,
}

#[derive(Clone, Debug, PartialEq)]
struct WordRotateFingerprint {
    words: Vec<String>,
    interval_ms: u64,
    motion: MotionSpec,
    enter_offset: f32,
    exit_offset: f32,
}

impl From<&WordRotateProps> for WordRotateFingerprint {
    fn from(props: &WordRotateProps) -> Self {
        Self {
            words: props.words.clone(),
            interval_ms: props.interval_ms,
            motion: props.motion,
            enter_offset: props.enter_offset,
            exit_offset: props.exit_offset,
        }
    }
}

#[derive(Clone, Debug)]
pub struct WordRotateState {
    opacity: AnimatedValue,
    offset: AnimatedValue,
    current_index: usize,
    pending_index: Option<usize>,
    phase: WordRotatePhase,
    started: bool,
    last_stable_at: Option<Instant>,
    fingerprint: WordRotateFingerprint,
}

impl WordRotateState {
    pub fn new(props: &WordRotateProps) -> Self {
        Self {
            opacity: AnimatedValue::with_spec(0.0, props.motion),
            offset: AnimatedValue::with_spec(props.enter_offset, props.motion),
            current_index: 0,
            pending_index: None,
            phase: WordRotatePhase::Entering,
            started: false,
            last_stable_at: None,
            fingerprint: WordRotateFingerprint::from(props),
        }
    }

    pub fn tick(&mut self, now: Instant, props: &WordRotateProps) {
        if props.words.is_empty() {
            self.started = true;
            self.phase = WordRotatePhase::Stable;
            self.pending_index = None;
            self.last_stable_at = Some(now);
            return;
        }

        let fingerprint = WordRotateFingerprint::from(props);
        if self.fingerprint != fingerprint {
            self.reset(now, props);
            return;
        }

        if !self.started {
            self.started = true;
            self.start_enter(now, props);
            return;
        }

        match self.phase {
            WordRotatePhase::Entering => {
                if !self.opacity.is_animating(now) && !self.offset.is_animating(now) {
                    self.phase = WordRotatePhase::Stable;
                    self.last_stable_at = Some(now);
                }
            }
            WordRotatePhase::Stable => {
                if props.words.len() <= 1 {
                    return;
                }

                let elapsed = self
                    .last_stable_at
                    .map(|stable_at| now.saturating_duration_since(stable_at).as_millis() as u64)
                    .unwrap_or(0);

                if elapsed >= props.interval_ms {
                    let next_index = (self.current_index + 1) % props.words.len();
                    self.pending_index = Some(next_index);
                    self.phase = WordRotatePhase::Exiting;
                    self.opacity = AnimatedValue::with_spec(1.0, props.motion);
                    self.opacity.transition(0.0, now);
                    self.offset = AnimatedValue::with_spec(0.0, props.motion);
                    self.offset.transition(props.exit_offset, now);
                }
            }
            WordRotatePhase::Exiting => {
                if !self.opacity.is_animating(now) && !self.offset.is_animating(now) {
                    self.current_index = self.pending_index.take().unwrap_or(self.current_index);
                    self.start_enter(now, props);
                }
            }
        }
    }

    pub fn current_word<'a>(&self, props: &'a WordRotateProps) -> &'a str {
        props
            .words
            .get(self.current_index.min(props.words.len().saturating_sub(1)))
            .map(String::as_str)
            .unwrap_or("")
    }

    pub fn current_index(&self) -> usize {
        self.current_index
    }

    pub fn opacity_at(&self, now: Instant) -> f32 {
        self.opacity.value_at(now).clamp(0.0, 1.0)
    }

    pub fn offset_at(&self, now: Instant) -> f32 {
        self.offset.value_at(now)
    }

    fn reset(&mut self, now: Instant, props: &WordRotateProps) {
        self.opacity = AnimatedValue::with_spec(0.0, props.motion);
        self.offset = AnimatedValue::with_spec(props.enter_offset, props.motion);
        self.current_index = 0;
        self.pending_index = None;
        self.phase = WordRotatePhase::Entering;
        self.started = true;
        self.last_stable_at = None;
        self.fingerprint = WordRotateFingerprint::from(props);
        self.start_enter(now, props);
    }

    fn start_enter(&mut self, now: Instant, props: &WordRotateProps) {
        self.phase = WordRotatePhase::Entering;
        self.opacity = AnimatedValue::with_spec(0.0, props.motion);
        self.opacity.transition(1.0, now);
        self.offset = AnimatedValue::with_spec(props.enter_offset, props.motion);
        self.offset.transition(0.0, now);
        self.last_stable_at = None;
    }
}

pub fn word_rotate<'a, Message: 'a>(
    state: &WordRotateState,
    now: Instant,
    props: &WordRotateProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let word = state.current_word(props).to_owned();
    if word.is_empty() {
        return iced::widget::Space::new().into();
    }

    let alpha = state.opacity_at(now);
    let color = Color {
        a: alpha,
        ..theme.palette.foreground
    };
    let y_offset = state.offset_at(now);
    let content: Element<'a, Message> = text(word)
        .size(u32::from(props.font_size))
        .style(move |_| iced::widget::text::Style { color: Some(color) })
        .into();

    let content = if y_offset.abs() > f32::EPSILON {
        float(content)
            .translate(move |_bounds, _viewport| Vector::new(0.0, y_offset))
            .into()
    } else {
        content
    };

    container(content)
        .height(Length::Fixed(f32::from(props.font_size) * 1.6))
        .clip(true)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::time::Duration;

    #[test]
    fn word_rotate_enters_and_cycles_to_next_word() {
        let props = WordRotateProps::new(["Word", "Rotate"])
            .interval_ms(100)
            .motion(
                MotionSpec::new()
                    .duration(TransitionDuration::CustomMs(40))
                    .easing(Easing::Linear),
            );
        let mut state = WordRotateState::new(&props);
        let start = Instant::now();

        state.tick(start, &props);
        state.tick(start + Duration::from_millis(50), &props);
        state.tick(start + Duration::from_millis(160), &props);
        state.tick(start + Duration::from_millis(210), &props);
        state.tick(start + Duration::from_millis(260), &props);

        assert_eq!(state.current_index(), 1);
        assert_eq!(state.current_word(&props), "Rotate");
    }

    #[test]
    fn word_rotate_resets_when_words_change() {
        let props = WordRotateProps::new(["One", "Two"]);
        let mut state = WordRotateState::new(&props);
        let start = Instant::now();

        state.tick(start, &props);
        state.tick(start + Duration::from_millis(3000), &props);

        let updated = WordRotateProps::new(["Alpha", "Beta"]).interval_ms(200);
        state.tick(start + Duration::from_millis(3010), &updated);

        assert_eq!(state.current_index(), 0);
        assert_eq!(state.current_word(&updated), "Alpha");
    }
}
