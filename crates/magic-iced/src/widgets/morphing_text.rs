use iced::widget::{container, float, stack, text};
use iced::{Color, Element, Length, Vector};
use iced_shadcn::Theme;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub struct MorphingTextProps {
    pub texts: Vec<String>,
    pub width: Length,
    pub height: Length,
    pub font_size: u16,
    pub morph_time_secs: f32,
    pub cooldown_time_secs: f32,
    pub color: Option<Color>,
}

impl Default for MorphingTextProps {
    fn default() -> Self {
        Self {
            texts: Vec::new(),
            width: Length::Fill,
            height: Length::Fixed(112.0),
            font_size: 64,
            morph_time_secs: 1.5,
            cooldown_time_secs: 0.5,
            color: None,
        }
    }
}

impl MorphingTextProps {
    pub fn new<I, S>(texts: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            texts: texts.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }
}

pub fn morphing_text<'a, Message: 'a>(
    elapsed: Duration,
    props: &MorphingTextProps,
    theme: &Theme,
) -> Element<'a, Message> {
    if props.texts.is_empty() {
        return container(iced::widget::Space::new())
            .width(props.width)
            .height(props.height)
            .into();
    }

    let cycle = (props.morph_time_secs + props.cooldown_time_secs).max(0.05);
    let phase = elapsed.as_secs_f32().rem_euclid(cycle);
    let index = ((elapsed.as_secs_f32() / cycle).floor() as usize) % props.texts.len();
    let next = (index + 1) % props.texts.len();
    let fraction = if phase <= props.cooldown_time_secs {
        0.0
    } else {
        ((phase - props.cooldown_time_secs) / props.morph_time_secs.max(0.05)).clamp(0.0, 1.0)
    };
    let base = props.color.unwrap_or(theme.palette.foreground);
    let current_color = Color {
        a: (1.0 - fraction).powf(0.4),
        ..base
    };
    let next_color = Color {
        a: fraction.powf(0.4),
        ..base
    };

    let current: Element<'a, Message> = float(
        container(
            text(props.texts[index].clone())
                .size(u32::from(props.font_size))
                .style(move |_| iced::widget::text::Style {
                    color: Some(current_color),
                }),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill),
    )
    .scale(1.0 + fraction * 0.12)
    .into();

    let incoming: Element<'a, Message> = float(
        container(
            text(props.texts[next].clone())
                .size(u32::from(props.font_size))
                .style(move |_| iced::widget::text::Style {
                    color: Some(next_color),
                }),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill),
    )
    .translate(move |_bounds, _viewport| Vector::new(0.0, (1.0 - fraction) * 10.0))
    .scale(0.88 + fraction * 0.12)
    .into();

    container(stack(vec![current, incoming]))
        .width(props.width)
        .height(props.height)
        .center_x(props.width)
        .center_y(props.height)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn morphing_text_defaults_match_docs() {
        let props = MorphingTextProps::new(["Hello", "World"]);

        assert_eq!(props.font_size, 64);
        assert_eq!(props.morph_time_secs, 1.5);
        assert_eq!(props.cooldown_time_secs, 0.5);
        assert_eq!(props.texts.len(), 2);
    }
}
