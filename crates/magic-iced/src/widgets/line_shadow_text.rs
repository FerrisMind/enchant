use super::text_element::TextElementRole;
use iced::widget::rich_text;
use iced::widget::text::Span;
use iced::{Border, Color, Element};
use std::time::Duration;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineShadowTextProps {
    pub shadow_color: Color,
    pub role: TextElementRole,
    pub font_size: Option<u16>,
    pub speed: f32,
}

impl Default for LineShadowTextProps {
    fn default() -> Self {
        Self {
            shadow_color: Color::BLACK,
            role: TextElementRole::Span,
            font_size: None,
            speed: 15.0,
        }
    }
}

impl LineShadowTextProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn line_shadow_text<'a, Message: 'a>(
    content: &'a str,
    elapsed: Duration,
    props: LineShadowTextProps,
) -> Element<'a, Message> {
    let size = u32::from(
        props
            .font_size
            .unwrap_or(props.role.default_font_size())
            .max(1),
    );
    let phase = (elapsed.as_secs_f32() / props.speed.max(0.05)).rem_euclid(1.0);
    let count = content.graphemes(true).count().max(1) as f32;

    let spans: Vec<Span<'a>> = content
        .graphemes(true)
        .enumerate()
        .map(|(index, grapheme)| {
            let progress = ((index as f32 / count) + phase).fract();
            let intensity = ((progress * std::f32::consts::TAU).sin() + 1.0) * 0.5;
            Span::new(grapheme)
                .size(size as f32)
                .color(Color::WHITE)
                .border(Border {
                    color: Color {
                        a: 0.18 + intensity * 0.3,
                        ..props.shadow_color
                    },
                    width: 1.0,
                    radius: 0.0.into(),
                })
        })
        .collect();

    rich_text(spans).size(size).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_shadow_defaults_match_docs() {
        let props = LineShadowTextProps::new();

        assert_eq!(props.shadow_color, Color::BLACK);
        assert_eq!(props.role, TextElementRole::Span);
    }
}
