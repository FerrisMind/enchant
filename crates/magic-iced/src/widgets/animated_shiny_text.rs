use iced::widget::{row, text};
use iced::{Color, Element};
use std::time::Duration;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimatedShinyTextProps {
    pub shimmer_width: f32,
    pub font_size: Option<u16>,
    pub base_color: Color,
    pub shine_color: Color,
    pub duration_secs: f32,
}

impl Default for AnimatedShinyTextProps {
    fn default() -> Self {
        Self {
            shimmer_width: 100.0,
            font_size: None,
            base_color: Color::from_rgba(1.0, 1.0, 1.0, 0.55),
            shine_color: Color::WHITE,
            duration_secs: 1.0,
        }
    }
}

impl AnimatedShinyTextProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn animated_shiny_text<'a, Message: 'a>(
    content: &'a str,
    elapsed: Duration,
    props: AnimatedShinyTextProps,
) -> Element<'a, Message> {
    let phase = (elapsed.as_secs_f32() / props.duration_secs.max(0.05)).rem_euclid(1.0);
    let size = u32::from(props.font_size.unwrap_or(18).max(1));
    let count = content.graphemes(true).count().max(1) as f32;
    let band = (props.shimmer_width / 100.0).clamp(0.05, 1.0);

    row(content
        .graphemes(true)
        .enumerate()
        .map(|(index, grapheme)| {
            let position = index as f32 / count;
            let delta = circular_distance(position, phase);
            let local = (1.0 - delta / band).clamp(0.0, 1.0);
            text(grapheme)
                .size(size)
                .style({
                    let color = mix_color(props.base_color, props.shine_color, local);
                    move |_| iced::widget::text::Style { color: Some(color) }
                })
                .into()
        }))
    .spacing(0)
    .into()
}

fn circular_distance(left: f32, right: f32) -> f32 {
    let direct = (left - right).abs();
    direct.min(1.0 - direct)
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
    fn animated_shiny_text_defaults_match_docs() {
        let props = AnimatedShinyTextProps::new();

        assert_eq!(props.shimmer_width, 100.0);
        assert_eq!(props.duration_secs, 1.0);
    }
}
