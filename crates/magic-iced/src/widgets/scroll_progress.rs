use crate::primitives::ScrollSample;
use iced::gradient;
use iced::widget::{container, stack};
use iced::{Background, Color, Element, Length, Radians};
use iced_shadcn::Theme;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollProgressProps {
    pub progress: f32,
    pub scroll_sample: Option<ScrollSample>,
    pub segment_start: f32,
    pub segment_end: f32,
    pub width: Length,
    pub height: f32,
    pub background: Option<Color>,
    pub color_from: Option<Color>,
    pub color_via: Option<Color>,
    pub color_to: Option<Color>,
}

impl Default for ScrollProgressProps {
    fn default() -> Self {
        Self {
            progress: 0.0,
            scroll_sample: None,
            segment_start: 0.0,
            segment_end: 1.0,
            width: Length::Fill,
            height: 2.0,
            background: None,
            color_from: None,
            color_via: None,
            color_to: None,
        }
    }
}

impl ScrollProgressProps {
    pub fn new(progress: f32) -> Self {
        Self {
            progress,
            ..Self::default()
        }
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
}

pub fn scroll_progress<'a, Message: 'a>(
    props: ScrollProgressProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let progress = props
        .scroll_sample
        .map(|sample| sample.segment_progress(props.segment_start, props.segment_end))
        .unwrap_or(props.progress)
        .clamp(0.0, 1.0);
    let background = props.background.unwrap_or(Color {
        a: 0.14,
        ..theme.palette.foreground
    });
    let from = props
        .color_from
        .or_else(|| theme.registry.color("magic.gradient.from"))
        .unwrap_or(theme.palette.primary);
    let via = props
        .color_via
        .or_else(|| theme.registry.color("magic.gradient.via"))
        .unwrap_or(theme.palette.accent);
    let to = props
        .color_to
        .or_else(|| theme.registry.color("magic.gradient.to"))
        .unwrap_or(theme.palette.chart_4);

    let track: Element<'a, Message> = container(iced::widget::Space::new())
        .width(props.width)
        .height(Length::Fixed(props.height.max(1.0)))
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(background)),
            ..iced::widget::container::Style::default()
        })
        .into();

    let fill: Element<'a, Message> = container(iced::widget::Space::new())
        .width(Length::FillPortion((progress * 10_000.0).round() as u16))
        .height(Length::Fixed(props.height.max(1.0)))
        .style(move |_| {
            gradient::Linear::new(Radians(0.0))
                .add_stop(0.0, from)
                .add_stop(0.5, via)
                .add_stop(1.0, to)
                .into()
        })
        .into();

    let remainder: Element<'a, Message> = container(iced::widget::Space::new())
        .width(Length::FillPortion(
            ((1.0 - progress) * 10_000.0).round().max(0.0) as u16,
        ))
        .height(Length::Fixed(props.height.max(1.0)))
        .into();

    let fill_row: Element<'a, Message> = iced::widget::row![fill, remainder]
        .width(props.width)
        .height(Length::Fixed(props.height.max(1.0)))
        .into();

    stack(vec![track, fill_row]).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_progress_defaults_match_docs() {
        let props = ScrollProgressProps::new(1.7);

        assert!(props.progress > 1.0);
        assert_eq!(props.height, 2.0);
    }

    #[test]
    fn scroll_progress_can_use_scroll_sample_segment() {
        let sample = ScrollSample::new(
            iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(100.0, 100.0)),
            iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(100.0, 400.0)),
            iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: 50.0 },
            iced::widget::scrollable::RelativeOffset { x: 0.0, y: 0.5 },
        );
        let props = ScrollProgressProps::new(0.0)
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
