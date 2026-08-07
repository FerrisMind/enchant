use iced::Color;
use iced::Element;
use iced::widget::row;
use iced::widget::text;
#[cfg(test)]
use iced::widget::text::Span;
use std::time::Duration;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimatedGradientTextProps {
    pub speed: f32,
    pub color_from: Color,
    pub color_to: Color,
    pub color_via: Option<Color>,
    pub font_size: u16,
}

impl Default for AnimatedGradientTextProps {
    fn default() -> Self {
        Self {
            speed: 0.35,
            color_from: Color::from_rgb8(0xFF, 0xAA, 0x40),
            color_to: Color::from_rgb8(0x9C, 0x40, 0xFF),
            color_via: None,
            font_size: 18,
        }
    }
}

impl AnimatedGradientTextProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed.max(0.01);
        self
    }

    pub fn color_from(mut self, color_from: Color) -> Self {
        self.color_from = color_from;
        self
    }

    pub fn color_to(mut self, color_to: Color) -> Self {
        self.color_to = color_to;
        self
    }

    pub fn color_via(mut self, color_via: Color) -> Self {
        self.color_via = Some(color_via);
        self
    }

    pub fn font_size(mut self, font_size: u16) -> Self {
        self.font_size = font_size.max(1);
        self
    }
}

pub fn animated_gradient_text<'a, Message: 'a>(
    content: &'a str,
    elapsed: Duration,
    props: AnimatedGradientTextProps,
) -> Element<'a, Message> {
    let phase = gradient_phase(elapsed, props.speed);
    let count = content.graphemes(true).count().max(1) as f32;

    row(content
        .graphemes(true)
        .enumerate()
        .map(|(index, grapheme)| {
            let progress = ((index as f32 / count) + phase).fract();
            text(grapheme)
                .size(u32::from(props.font_size))
                .style({
                    let color = sample_gradient(progress, props);
                    move |_| iced::widget::text::Style { color: Some(color) }
                })
                .into()
        }))
    .spacing(0)
    .into()
}

#[cfg(test)]
fn gradient_spans<'a>(
    content: &'a str,
    elapsed: Duration,
    props: AnimatedGradientTextProps,
) -> Vec<Span<'a>> {
    let graphemes = content.graphemes(true).collect::<Vec<_>>();
    let count = graphemes.len().max(1) as f32;
    let phase = gradient_phase(elapsed, props.speed);

    graphemes
        .into_iter()
        .enumerate()
        .map(|(index, grapheme)| {
            let progress = ((index as f32 / count) + phase).fract();
            Span::new(grapheme).color(sample_gradient(progress, props))
        })
        .collect()
}

fn gradient_phase(elapsed: Duration, speed: f32) -> f32 {
    (elapsed.as_secs_f32() * speed).rem_euclid(1.0)
}

fn sample_gradient(progress: f32, props: AnimatedGradientTextProps) -> Color {
    let progress = progress.clamp(0.0, 1.0);

    match props.color_via {
        Some(via) if progress < 0.5 => mix_color(props.color_from, via, progress * 2.0),
        Some(via) => mix_color(via, props.color_to, (progress - 0.5) * 2.0),
        None if progress < 0.5 => mix_color(props.color_from, props.color_to, progress * 2.0),
        None => mix_color(props.color_to, props.color_from, (progress - 0.5) * 2.0),
    }
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
    fn gradient_spans_preserve_grapheme_boundaries() {
        let spans = gradient_spans(
            "A👨‍👩‍👧‍👦B",
            Duration::from_secs(0),
            AnimatedGradientTextProps::new(),
        );

        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].text.as_ref(), "A");
        assert_eq!(spans[1].text.as_ref(), "👨‍👩‍👧‍👦");
        assert_eq!(spans[2].text.as_ref(), "B");
    }

    #[test]
    fn sample_gradient_moves_across_text() {
        let props = AnimatedGradientTextProps::new()
            .color_from(Color::from_rgb8(0x00, 0x00, 0x00))
            .color_to(Color::from_rgb8(0xFF, 0x00, 0x00));

        let start = sample_gradient(0.0, props);
        let middle = sample_gradient(0.25, props);
        let end = sample_gradient(0.5, props);

        assert!(middle.r > start.r);
        assert!(end.r >= middle.r);
    }
}
