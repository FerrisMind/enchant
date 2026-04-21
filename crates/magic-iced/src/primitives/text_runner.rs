use iced::time::Instant;
use std::ops::Range;
use twill::backends::iced::to_duration;
use twill::prelude::TransitionDuration;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextSegmentMode {
    Text,
    #[default]
    Word,
    Character,
    Line,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextUnit {
    #[default]
    Grapheme,
    Word,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextLoopMode {
    #[default]
    Once,
    Loop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextRunSpec {
    pub unit: TextUnit,
    pub start_delay: TransitionDuration,
    pub typing_step: TransitionDuration,
    pub deleting_step: TransitionDuration,
    pub pause: TransitionDuration,
    pub loop_mode: TextLoopMode,
}

impl Default for TextRunSpec {
    fn default() -> Self {
        Self {
            unit: TextUnit::Grapheme,
            start_delay: TransitionDuration::Ms0,
            typing_step: TransitionDuration::Ms75,
            deleting_step: TransitionDuration::Ms75,
            pause: TransitionDuration::Ms300,
            loop_mode: TextLoopMode::Once,
        }
    }
}

impl TextRunSpec {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn unit(mut self, unit: TextUnit) -> Self {
        self.unit = unit;
        self
    }

    pub fn start_delay(mut self, delay: TransitionDuration) -> Self {
        self.start_delay = delay;
        self
    }

    pub fn typing_step(mut self, duration: TransitionDuration) -> Self {
        self.typing_step = duration;
        self
    }

    pub fn deleting_step(mut self, duration: TransitionDuration) -> Self {
        self.deleting_step = duration;
        self
    }

    pub fn pause(mut self, duration: TransitionDuration) -> Self {
        self.pause = duration;
        self
    }

    pub fn loop_mode(mut self, loop_mode: TextLoopMode) -> Self {
        self.loop_mode = loop_mode;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextRunPhase {
    Typing,
    Pause,
    Deleting,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TextRunAdvance {
    pub cycle_completed: bool,
    pub finished: bool,
}

#[derive(Clone, Debug)]
pub struct TextRunner {
    source: String,
    units: Box<[Range<usize>]>,
    spec: TextRunSpec,
    started_at: Option<Instant>,
    last_step_at: Option<Instant>,
    visible_units: usize,
    phase: TextRunPhase,
}

pub fn segment_text_ranges(source: &str, mode: TextSegmentMode) -> Vec<Range<usize>> {
    match mode {
        TextSegmentMode::Text => {
            if source.is_empty() {
                Vec::new()
            } else {
                std::iter::once(0..source.len()).collect()
            }
        }
        TextSegmentMode::Word => {
            let words = source.unicode_word_indices().collect::<Vec<_>>();
            if words.is_empty() {
                return if source.is_empty() {
                    Vec::new()
                } else {
                    std::iter::once(0..source.len()).collect()
                };
            }

            let mut ranges = Vec::with_capacity(words.len() * 2);
            let mut cursor = 0;

            for (start, word) in &words {
                if cursor < *start {
                    ranges.push(cursor..*start);
                }

                let end = *start + word.len();
                ranges.push(*start..end);
                cursor = end;
            }

            if cursor < source.len() {
                ranges.push(cursor..source.len());
            }

            ranges
        }
        TextSegmentMode::Character => source
            .grapheme_indices(true)
            .map(|(start, grapheme)| start..start + grapheme.len())
            .collect(),
        TextSegmentMode::Line => {
            if source.is_empty() {
                return Vec::new();
            }

            let mut ranges = Vec::new();
            let mut line_start = 0;

            for (index, ch) in source.char_indices() {
                if ch == '\n' {
                    if line_start < index {
                        ranges.push(line_start..index);
                    }
                    line_start = index + ch.len_utf8();
                }
            }

            if line_start < source.len() {
                ranges.push(line_start..source.len());
            }

            if ranges.is_empty() {
                ranges.push(0..source.len());
            }

            ranges
        }
    }
}

pub fn segment_text(source: &str, mode: TextSegmentMode) -> Vec<&str> {
    segment_text_ranges(source, mode)
        .into_iter()
        .map(|range| &source[range])
        .collect()
}

impl TextRunner {
    pub fn new(source: impl Into<String>, spec: TextRunSpec) -> Self {
        let source = source.into();
        let units = segment_ranges(&source, spec.unit).into_boxed_slice();

        Self {
            source,
            units,
            spec,
            started_at: None,
            last_step_at: None,
            visible_units: 0,
            phase: TextRunPhase::Typing,
        }
    }

    pub fn spec(&self) -> TextRunSpec {
        self.spec
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn phase(&self) -> TextRunPhase {
        self.phase
    }

    pub fn set_spec(&mut self, spec: TextRunSpec) {
        if self.spec.unit != spec.unit {
            self.source = self.source.clone();
            self.units = segment_ranges(&self.source, spec.unit).into_boxed_slice();
            self.visible_units = self.visible_units.min(self.units.len());
        }
        self.spec = spec;
    }

    pub fn unit_ranges(&self) -> &[Range<usize>] {
        &self.units
    }

    pub fn visible_units(&self) -> usize {
        self.visible_units
    }

    pub fn visible_text(&self) -> &str {
        match self.visible_units {
            0 => "",
            count => {
                let end = self
                    .units
                    .get(count - 1)
                    .map(|range| range.end)
                    .unwrap_or(self.source.len());
                &self.source[..end]
            }
        }
    }

    pub fn reset(&mut self) {
        self.started_at = None;
        self.last_step_at = None;
        self.visible_units = 0;
        self.phase = TextRunPhase::Typing;
    }

    pub fn set_source(&mut self, source: impl Into<String>) {
        self.source = source.into();
        self.units = segment_ranges(&self.source, self.spec.unit).into_boxed_slice();
        self.reset();
    }

    pub fn advance(&mut self, now: Instant) {
        let _ = self.advance_with_result(now);
    }

    pub fn advance_with_result(&mut self, now: Instant) -> TextRunAdvance {
        let mut result = TextRunAdvance::default();

        if self.units.is_empty() {
            self.phase = TextRunPhase::Done;
            result.finished = true;
            return result;
        }

        let started_at = *self.started_at.get_or_insert(now);
        let delay = to_duration(self.spec.start_delay);
        if now < started_at + delay {
            return result;
        }

        let mut cursor = self.last_step_at.unwrap_or(started_at + delay);

        loop {
            let step = match self.phase {
                TextRunPhase::Typing => to_duration(self.spec.typing_step),
                TextRunPhase::Pause => to_duration(self.spec.pause),
                TextRunPhase::Deleting => to_duration(self.spec.deleting_step),
                TextRunPhase::Done => break,
            };

            if now < cursor + step {
                break;
            }

            cursor += step;

            match self.phase {
                TextRunPhase::Typing => {
                    if self.visible_units < self.units.len() {
                        self.visible_units += 1;
                    }

                    if self.visible_units >= self.units.len() {
                        self.phase = match self.spec.loop_mode {
                            TextLoopMode::Once => {
                                result.finished = true;
                                TextRunPhase::Done
                            }
                            TextLoopMode::Loop => TextRunPhase::Pause,
                        };
                    }
                }
                TextRunPhase::Pause => {
                    self.phase = TextRunPhase::Deleting;
                }
                TextRunPhase::Deleting => {
                    if self.visible_units > 0 {
                        self.visible_units -= 1;
                    }

                    if self.visible_units == 0 {
                        result.cycle_completed = true;
                        self.phase = TextRunPhase::Typing;
                    }
                }
                TextRunPhase::Done => break,
            }
        }

        self.last_step_at = Some(cursor);
        result
    }
}

fn segment_ranges(source: &str, unit: TextUnit) -> Vec<Range<usize>> {
    let mut ranges = match unit {
        TextUnit::Grapheme => source
            .grapheme_indices(true)
            .map(|(start, grapheme)| start..start + grapheme.len())
            .collect::<Vec<_>>(),
        TextUnit::Word => source
            .unicode_word_indices()
            .map(|(start, word)| start..start + word.len())
            .collect::<Vec<_>>(),
    };

    if ranges.is_empty() && !source.is_empty() {
        ranges.push(0..source.len());
    }

    ranges
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::time::Duration;

    #[test]
    fn grapheme_segmentation_preserves_unicode_boundaries() {
        let runner = TextRunner::new("A👨‍👩‍👧‍👦B", TextRunSpec::new().unit(TextUnit::Grapheme));

        assert_eq!(runner.unit_ranges().len(), 3);
        assert_eq!(&runner.source[runner.unit_ranges()[1].clone()], "👨‍👩‍👧‍👦");
    }

    #[test]
    fn word_segmentation_keeps_prefix_spacing_in_visible_text() {
        let mut runner = TextRunner::new(
            "Hello  world",
            TextRunSpec::new()
                .unit(TextUnit::Word)
                .typing_step(TransitionDuration::Ms75),
        );
        let start = Instant::now();

        runner.advance(start);
        runner.advance(start + Duration::from_millis(80));
        assert_eq!(runner.visible_text(), "Hello");

        runner.advance(start + Duration::from_millis(160));
        assert_eq!(runner.visible_text(), "Hello  world");
    }

    #[test]
    fn looping_runner_types_pauses_deletes_and_restarts() {
        let mut runner = TextRunner::new(
            "Hi",
            TextRunSpec::new()
                .typing_step(TransitionDuration::CustomMs(50))
                .deleting_step(TransitionDuration::CustomMs(50))
                .pause(TransitionDuration::CustomMs(50))
                .loop_mode(TextLoopMode::Loop),
        );
        let start = Instant::now();

        runner.advance(start);
        runner.advance(start + Duration::from_millis(60));
        assert_eq!(runner.visible_text(), "H");

        runner.advance(start + Duration::from_millis(110));
        assert_eq!(runner.visible_text(), "Hi");

        runner.advance(start + Duration::from_millis(220));
        assert_eq!(runner.visible_text(), "H");

        runner.advance(start + Duration::from_millis(280));
        assert_eq!(runner.visible_text(), "");
    }

    #[test]
    fn set_source_replaces_text_and_preserves_spec() {
        let mut runner = TextRunner::new(
            "Hello",
            TextRunSpec::new()
                .typing_step(TransitionDuration::CustomMs(40))
                .deleting_step(TransitionDuration::CustomMs(30))
                .loop_mode(TextLoopMode::Loop),
        );
        let start = Instant::now();

        runner.advance(start);
        runner.advance(start + Duration::from_millis(80));
        assert_eq!(runner.visible_text(), "He");

        runner.set_source("World");

        assert_eq!(runner.source(), "World");
        assert_eq!(runner.visible_text(), "");
        assert_eq!(runner.visible_units(), 0);
        assert_eq!(runner.phase(), TextRunPhase::Typing);
        assert_eq!(runner.spec().typing_step, TransitionDuration::CustomMs(40));
        assert_eq!(
            runner.spec().deleting_step,
            TransitionDuration::CustomMs(30)
        );
        assert_eq!(runner.spec().loop_mode, TextLoopMode::Loop);
    }

    #[test]
    fn once_runner_exposes_done_phase_when_finished() {
        let mut runner = TextRunner::new(
            "Done",
            TextRunSpec::new()
                .typing_step(TransitionDuration::CustomMs(25))
                .loop_mode(TextLoopMode::Once),
        );
        let start = Instant::now();

        runner.advance(start);
        runner.advance(start + Duration::from_millis(125));

        assert_eq!(runner.visible_text(), "Done");
        assert_eq!(runner.phase(), TextRunPhase::Done);
    }

    #[test]
    fn advance_with_result_marks_completed_loop_cycle() {
        let mut runner = TextRunner::new(
            "Loop",
            TextRunSpec::new()
                .typing_step(TransitionDuration::CustomMs(20))
                .deleting_step(TransitionDuration::CustomMs(20))
                .pause(TransitionDuration::CustomMs(20))
                .loop_mode(TextLoopMode::Loop),
        );
        let start = Instant::now();

        runner.advance(start);
        let result = runner.advance_with_result(start + Duration::from_millis(180));

        assert!(result.cycle_completed);
        assert_eq!(runner.phase(), TextRunPhase::Typing);
        assert_eq!(runner.visible_units(), 0);
    }

    #[test]
    fn segment_text_word_mode_preserves_gaps_as_segments() {
        let segments = segment_text("Hello  world", TextSegmentMode::Word);

        assert_eq!(segments, vec!["Hello", "  ", "world"]);
    }

    #[test]
    fn segment_text_line_mode_splits_on_newlines_without_empty_tail() {
        let segments = segment_text("Alpha\nBeta\n", TextSegmentMode::Line);

        assert_eq!(segments, vec!["Alpha", "Beta"]);
    }
}
