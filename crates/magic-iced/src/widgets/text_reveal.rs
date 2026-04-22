use crate::primitives::{ScrollSample, segment_progress};
use iced::Color;
use iced::Element;
use iced::widget::rich_text;
use iced::widget::text::Span;
use iced_shadcn::Theme;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextRevealProps {
    pub progress: f32,
    pub scroll_sample: Option<ScrollSample>,
    pub segment_start: f32,
    pub segment_end: f32,
    pub font_size: u16,
    pub base_opacity: f32,
}

impl Default for TextRevealProps {
    fn default() -> Self {
        Self {
            progress: 0.0,
            scroll_sample: None,
            segment_start: 0.0,
            segment_end: 1.0,
            font_size: 24,
            base_opacity: 0.24,
        }
    }
}

impl TextRevealProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn progress(mut self, progress: f32) -> Self {
        self.progress = progress;
        self
    }

    pub fn scroll_sample(mut self, scroll_sample: ScrollSample) -> Self {
        self.scroll_sample = Some(scroll_sample);
        self
    }

    pub fn segment(mut self, start: f32, end: f32) -> Self {
        self.segment_start = start;
        self.segment_end = end;
        self
    }

    pub fn font_size(mut self, font_size: u16) -> Self {
        self.font_size = font_size.max(1);
        self
    }

    pub fn base_opacity(mut self, base_opacity: f32) -> Self {
        self.base_opacity = base_opacity.clamp(0.0, 1.0);
        self
    }
}

pub fn text_reveal<'a, Message: 'a>(
    content: &'a str,
    props: TextRevealProps,
    theme: &Theme,
) -> Element<'a, Message> {
    rich_text(word_spans(content, props, theme))
        .size(u32::from(props.font_size))
        .into()
}

fn word_spans<'a>(content: &'a str, props: TextRevealProps, theme: &Theme) -> Vec<Span<'a>> {
    let chunks = word_chunks(content);
    let count = chunks.len().max(1) as f32;
    let base = apply_alpha(theme.palette.foreground, props.base_opacity);
    let reveal = theme.palette.foreground;
    let reveal_progress = props
        .scroll_sample
        .map(|sample| sample.segment_progress(props.segment_start, props.segment_end))
        .unwrap_or(props.progress);

    chunks
        .into_iter()
        .enumerate()
        .map(|(index, chunk)| {
            let start = index as f32 / count;
            let end = (index + 1) as f32 / count;
            let local = segment_progress(reveal_progress, start, end);
            Span::new(chunk).color(mix_color(base, reveal, local))
        })
        .collect()
}

fn word_chunks(content: &str) -> Vec<&str> {
    content.split(' ').collect()
}

fn apply_alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

fn mix_color(from: Color, to: Color, progress: f32) -> Color {
    let progress = progress.clamp(0.0, 1.0);

    Color {
        r: from.r + (to.r - from.r) * progress,
        g: from.g + (to.g - from.g) * progress,
        b: from.b + (to.b - from.b) * progress,
        a: from.a + (to.a - from.a) * progress,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_chunks_match_literal_space_split() {
        let chunks = word_chunks("Hello  world from FerrisMind");

        assert_eq!(chunks, vec!["Hello", "", "world", "from", "FerrisMind"]);
    }

    #[test]
    fn text_reveal_progress_clamps_to_word_range() {
        assert_eq!(segment_progress(-1.0, 0.0, 1.0), 0.0);
        assert_eq!(segment_progress(2.0, 0.0, 1.0), 1.0);
    }

    #[test]
    fn text_reveal_can_drive_progress_from_scroll_sample() {
        let sample = ScrollSample::new(
            iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(100.0, 100.0)),
            iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(100.0, 400.0)),
            iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: 50.0 },
            iced::widget::scrollable::RelativeOffset { x: 0.0, y: 0.5 },
        );
        let props = TextRevealProps::new()
            .scroll_sample(sample)
            .segment(0.25, 0.75);

        assert_eq!(
            props
                .scroll_sample
                .expect("scroll sample should be stored")
                .segment_progress(props.segment_start, props.segment_end),
            0.5
        );
    }
}
