use crate::primitives::{TextLoopMode, TextRunPhase, TextRunSpec, TextRunner};
use iced::time::Instant;
use iced::widget::{row, text};
use iced::{Element, Length};
use iced_shadcn::Theme;
use twill::prelude::TransitionDuration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TypingAnimationCursorStyle {
    #[default]
    Line,
    Block,
    Underscore,
}

impl TypingAnimationCursorStyle {
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Line => "|",
            Self::Block => "▌",
            Self::Underscore => "_",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypingAnimationProps {
    pub words: Vec<String>,
    pub typing_step: TransitionDuration,
    pub deleting_step: TransitionDuration,
    pub start_delay: TransitionDuration,
    pub pause_delay: TransitionDuration,
    pub loop_enabled: bool,
    pub show_cursor: bool,
    pub blink_cursor: bool,
    pub cursor_style: TypingAnimationCursorStyle,
    pub font_size: u16,
}

impl Default for TypingAnimationProps {
    fn default() -> Self {
        Self {
            words: Vec::new(),
            typing_step: TransitionDuration::Ms100,
            deleting_step: TransitionDuration::CustomMs(50),
            start_delay: TransitionDuration::Ms0,
            pause_delay: TransitionDuration::Ms1000,
            loop_enabled: false,
            show_cursor: true,
            blink_cursor: true,
            cursor_style: TypingAnimationCursorStyle::Line,
            font_size: 24,
        }
    }
}

impl TypingAnimationProps {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            words: vec![text.into()],
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

    pub fn typing_step(mut self, typing_step: TransitionDuration) -> Self {
        self.typing_step = typing_step;
        self
    }

    pub fn deleting_step(mut self, deleting_step: TransitionDuration) -> Self {
        self.deleting_step = deleting_step;
        self
    }

    pub fn start_delay(mut self, start_delay: TransitionDuration) -> Self {
        self.start_delay = start_delay;
        self
    }

    pub fn pause_delay(mut self, pause_delay: TransitionDuration) -> Self {
        self.pause_delay = pause_delay;
        self
    }

    pub fn loop_enabled(mut self, loop_enabled: bool) -> Self {
        self.loop_enabled = loop_enabled;
        self
    }

    pub fn show_cursor(mut self, show_cursor: bool) -> Self {
        self.show_cursor = show_cursor;
        self
    }

    pub fn blink_cursor(mut self, blink_cursor: bool) -> Self {
        self.blink_cursor = blink_cursor;
        self
    }

    pub fn cursor_style(mut self, cursor_style: TypingAnimationCursorStyle) -> Self {
        self.cursor_style = cursor_style;
        self
    }

    pub fn font_size(mut self, font_size: u16) -> Self {
        self.font_size = font_size.max(1);
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
struct TypingAnimationFingerprint {
    words: Vec<String>,
    typing_step: TransitionDuration,
    deleting_step: TransitionDuration,
    start_delay: TransitionDuration,
    pause_delay: TransitionDuration,
    loop_enabled: bool,
}

impl From<&TypingAnimationProps> for TypingAnimationFingerprint {
    fn from(props: &TypingAnimationProps) -> Self {
        Self {
            words: props.words.clone(),
            typing_step: props.typing_step,
            deleting_step: props.deleting_step,
            start_delay: props.start_delay,
            pause_delay: props.pause_delay,
            loop_enabled: props.loop_enabled,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TypingAnimationState {
    runner: TextRunner,
    current_word_index: usize,
    previous_phase: TextRunPhase,
    started: bool,
    blink_origin: Option<Instant>,
    fingerprint: TypingAnimationFingerprint,
}

impl TypingAnimationState {
    pub fn new(props: &TypingAnimationProps) -> Self {
        let word = props.words.first().cloned().unwrap_or_default();
        let runner = TextRunner::new(word, runner_spec(props, 0));

        Self {
            previous_phase: runner.phase(),
            runner,
            current_word_index: 0,
            started: false,
            blink_origin: None,
            fingerprint: TypingAnimationFingerprint::from(props),
        }
    }

    pub fn tick(&mut self, now: Instant, props: &TypingAnimationProps) {
        let fingerprint = TypingAnimationFingerprint::from(props);
        if self.fingerprint != fingerprint {
            *self = Self::new(props);
        }

        if props.words.is_empty() {
            self.started = true;
            self.previous_phase = TextRunPhase::Done;
            return;
        }

        if !self.started {
            self.started = true;
            self.blink_origin = Some(now);
        }

        self.runner
            .set_spec(runner_spec(props, self.current_word_index));
        let before = self.runner.phase();
        let advance = self.runner.advance_with_result(now);
        let after = self.runner.phase();

        if (before == TextRunPhase::Deleting || advance.cycle_completed)
            && after == TextRunPhase::Typing
            && self.runner.visible_units() == 0
        {
            let next_index = if self.current_word_index + 1 < props.words.len() {
                Some(self.current_word_index + 1)
            } else if props.loop_enabled {
                Some(0)
            } else {
                None
            };

            if let Some(next_index) = next_index {
                self.current_word_index = next_index;
                self.runner
                    .set_spec(runner_spec(props, self.current_word_index));
                self.runner.set_source(props.words[next_index].clone());
                self.runner.advance(now);
            }
        }

        self.previous_phase = self.runner.phase();
    }

    pub fn display_text(&self) -> &str {
        self.runner.visible_text()
    }

    pub fn current_word_index(&self) -> usize {
        self.current_word_index
    }

    pub fn is_complete(&self, props: &TypingAnimationProps) -> bool {
        !props.loop_enabled
            && !props.words.is_empty()
            && self.current_word_index == props.words.len().saturating_sub(1)
            && self.runner.phase() == TextRunPhase::Done
    }

    pub fn cursor_visible(&self, now: Instant, props: &TypingAnimationProps) -> bool {
        if !props.show_cursor || self.is_complete(props) {
            return false;
        }

        if !props.blink_cursor {
            return true;
        }

        let origin = self.blink_origin.unwrap_or(now);
        let frame = now.saturating_duration_since(origin).as_millis() / 500;
        frame.is_multiple_of(2)
    }
}

pub fn typing_animation<'a, Message: 'a>(
    state: &TypingAnimationState,
    now: Instant,
    props: &TypingAnimationProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let mut children: Vec<Element<'a, Message>> = vec![
        text(state.display_text().to_owned())
            .size(u32::from(props.font_size))
            .style({
                let color = theme.palette.foreground;
                move |_| iced::widget::text::Style { color: Some(color) }
            })
            .into(),
    ];

    if state.cursor_visible(now, props) {
        children.push(
            text(props.cursor_style.glyph())
                .size(u32::from(props.font_size))
                .style({
                    let color = theme
                        .registry
                        .color("magic.typing.cursor")
                        .unwrap_or(theme.palette.foreground);
                    move |_| iced::widget::text::Style { color: Some(color) }
                })
                .into(),
        );
    }

    row(children).spacing(2).width(Length::Shrink).into()
}

fn runner_spec(props: &TypingAnimationProps, current_word_index: usize) -> TextRunSpec {
    let should_continue = props.loop_enabled || current_word_index + 1 < props.words.len();

    TextRunSpec::new()
        .start_delay(props.start_delay)
        .typing_step(props.typing_step)
        .deleting_step(props.deleting_step)
        .pause(props.pause_delay)
        .loop_mode(if should_continue {
            TextLoopMode::Loop
        } else {
            TextLoopMode::Once
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::time::Duration;

    #[test]
    fn typing_animation_switches_to_next_word_after_delete_cycle() {
        let props = TypingAnimationProps::new("First")
            .words(["First", "Second"])
            .typing_step(TransitionDuration::CustomMs(40))
            .deleting_step(TransitionDuration::CustomMs(40))
            .pause_delay(TransitionDuration::CustomMs(40));
        let mut state = TypingAnimationState::new(&props);
        let start = Instant::now();

        state.tick(start, &props);
        state.tick(start + Duration::from_millis(250), &props);
        state.tick(start + Duration::from_millis(320), &props);
        state.tick(start + Duration::from_millis(450), &props);

        assert_eq!(state.current_word_index(), 1);
    }

    #[test]
    fn typing_animation_stops_on_last_word_when_loop_disabled() {
        let props = TypingAnimationProps::new("First")
            .words(["First", "Second"])
            .typing_step(TransitionDuration::CustomMs(20))
            .deleting_step(TransitionDuration::CustomMs(20))
            .pause_delay(TransitionDuration::CustomMs(20));
        let mut state = TypingAnimationState::new(&props);
        let start = Instant::now();

        state.tick(start, &props);
        state.tick(start + Duration::from_millis(220), &props);
        state.tick(start + Duration::from_millis(320), &props);
        state.tick(start + Duration::from_millis(420), &props);
        state.tick(start + Duration::from_millis(520), &props);

        assert_eq!(state.current_word_index(), 1);
        assert!(state.is_complete(&props));
    }

    #[test]
    fn typing_animation_cursor_style_maps_to_expected_glyph() {
        assert_eq!(TypingAnimationCursorStyle::Line.glyph(), "|");
        assert_eq!(TypingAnimationCursorStyle::Block.glyph(), "▌");
        assert_eq!(TypingAnimationCursorStyle::Underscore.glyph(), "_");
    }
}
