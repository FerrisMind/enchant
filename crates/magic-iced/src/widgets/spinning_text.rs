use super::text_element::TextElementRole;
use crate::primitives::orbit_translation;
use iced::widget::{container, float, stack, text};
use iced::{Color, Element, Length, Vector};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpinningTextTransition {
    pub duration_secs: f32,
    pub delay_secs: f32,
}

impl Default for SpinningTextTransition {
    fn default() -> Self {
        Self {
            duration_secs: 10.0,
            delay_secs: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpinningTextVariantState {
    pub opacity: f32,
    pub scale: f32,
}

impl Default for SpinningTextVariantState {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            scale: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct SpinningTextVariants {
    pub container: Option<SpinningTextVariantState>,
    pub item: Option<SpinningTextVariantState>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpinningTextProps {
    pub duration_secs: f32,
    pub reverse: bool,
    pub radius: f32,
    pub transition: SpinningTextTransition,
    pub variants: SpinningTextVariants,
    pub role: TextElementRole,
    pub font_size: Option<u16>,
}

impl Default for SpinningTextProps {
    fn default() -> Self {
        Self {
            duration_secs: 10.0,
            reverse: false,
            radius: 5.0,
            transition: SpinningTextTransition::default(),
            variants: SpinningTextVariants::default(),
            role: TextElementRole::Div,
            font_size: None,
        }
    }
}

impl SpinningTextProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn spinning_text<'a, Message: 'a>(
    content: &str,
    elapsed: Duration,
    props: SpinningTextProps,
) -> Element<'a, Message> {
    let mut letters = content.chars().collect::<Vec<_>>();
    letters.push(' ');

    let rotation = (elapsed.as_secs_f32() - props.transition.delay_secs.max(0.0))
        / props.duration_secs.max(0.05)
        * 360.0
        * if props.reverse { -1.0 } else { 1.0 };
    let item_scale = props.variants.item.unwrap_or_default().scale.max(0.1);
    let item_opacity = props
        .variants
        .item
        .unwrap_or_default()
        .opacity
        .clamp(0.0, 1.0);
    let font_size = props
        .font_size
        .unwrap_or(props.role.default_font_size())
        .max(1);

    let mut layers = Vec::with_capacity(letters.len());
    for (index, letter) in letters.iter().enumerate() {
        let angle = rotation + (360.0 / letters.len().max(1) as f32) * index as f32 - 90.0;
        let translation = orbit_translation(props.radius * font_size as f32, angle);
        let color = Color {
            a: item_opacity,
            ..Color::WHITE
        };
        layers.push(
            float(
                container(
                    text(letter.to_string())
                        .size((font_size as f32 * item_scale) as u32)
                        .style(move |_| iced::widget::text::Style { color: Some(color) }),
                )
                .width(Length::Shrink)
                .height(Length::Shrink),
            )
            .translate(move |_bounds, _viewport| Vector::new(translation.x, translation.y))
            .into(),
        );
    }

    container(stack(layers))
        .width(Length::Fixed(font_size as f32 * props.radius * 3.0))
        .height(Length::Fixed(font_size as f32 * props.radius * 3.0))
        .center_x(Length::Shrink)
        .center_y(Length::Shrink)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spinning_text_defaults_match_docs() {
        let props = SpinningTextProps::new();

        assert_eq!(props.duration_secs, 10.0);
        assert!(!props.reverse);
        assert_eq!(props.radius, 5.0);
    }
}
