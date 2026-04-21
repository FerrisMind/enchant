use iced::widget::{row, text};
use iced::{Color, Element};
use std::time::Duration;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Debug, PartialEq)]
pub struct AuroraTextProps {
    pub colors: Vec<Color>,
    pub speed: f32,
    pub font_size: Option<u16>,
}

impl Default for AuroraTextProps {
    fn default() -> Self {
        Self {
            colors: vec![
                Color::from_rgb8(0xFF, 0x00, 0x80),
                Color::from_rgb8(0x79, 0x28, 0xCA),
                Color::from_rgb8(0x00, 0x70, 0xF3),
                Color::from_rgb8(0x38, 0xBD, 0xF8),
            ],
            speed: 1.0,
            font_size: None,
        }
    }
}

impl AuroraTextProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn aurora_text<'a, Message: 'a>(
    content: &'a str,
    elapsed: Duration,
    props: &AuroraTextProps,
) -> Element<'a, Message> {
    let size = u32::from(props.font_size.unwrap_or(18).max(1));
    let phase = (elapsed.as_secs_f32() * (props.speed.max(0.01) * 0.12)).rem_euclid(1.0);
    let stops = normalized_stops(props.colors.as_slice());
    let count = content.graphemes(true).count().max(1) as f32;

    row(content
        .graphemes(true)
        .enumerate()
        .map(|(index, grapheme)| {
            let offset = ((index as f32 / count) + phase).fract();
            text(grapheme)
                .size(size)
                .style({
                    let color = sample_stops(offset, &stops);
                    move |_| iced::widget::text::Style { color: Some(color) }
                })
                .into()
        }))
    .spacing(0)
    .into()
}

fn normalized_stops(colors: &[Color]) -> Vec<(f32, Color)> {
    let fallback = [Color::WHITE, Color::from_rgb8(0x99, 0x66, 0xFF)];
    let colors = if colors.len() >= 2 { colors } else { &fallback };
    let last = (colors.len() - 1).max(1) as f32;

    colors
        .iter()
        .enumerate()
        .map(|(index, color)| (index as f32 / last, *color))
        .collect()
}

fn sample_stops(progress: f32, stops: &[(f32, Color)]) -> Color {
    let progress = progress.clamp(0.0, 1.0);

    for window in stops.windows(2) {
        let (start_offset, start_color) = window[0];
        let (end_offset, end_color) = window[1];
        if progress <= end_offset {
            let local = ((progress - start_offset) / (end_offset - start_offset).max(f32::EPSILON))
                .clamp(0.0, 1.0);
            return mix_color(start_color, end_color, local);
        }
    }

    stops
        .last()
        .map(|(_, color)| *color)
        .unwrap_or(Color::WHITE)
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
    fn aurora_defaults_match_docs() {
        let props = AuroraTextProps::new();

        assert_eq!(props.colors.len(), 4);
        assert_eq!(props.speed, 1.0);
    }
}
