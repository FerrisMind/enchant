use iced::time::Instant;
use iced::widget::{container, stack, text};
use iced::{Color, Element, Length, Vector};
use std::borrow::Cow;

use crate::primitives::{AnimatedValue, MotionSpec, TextSegmentMode, segment_text};
use twill::prelude::{Easing, TransitionDuration};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ComicTextProps {
    pub font_size: f32,
    pub entrance_duration_ms: u64,
    pub dot_color: Color,
    pub fill_color: Color,
    pub outline_color: Color,
    pub shadow_color: Color,
}

impl Default for ComicTextProps {
    fn default() -> Self {
        Self {
            font_size: 5.0,
            entrance_duration_ms: 600,
            dot_color: Color::from_rgb8(0xEF, 0x44, 0x44),
            fill_color: Color::from_rgb8(0xFA, 0xCC, 0x15),
            outline_color: Color::BLACK,
            shadow_color: Color::from_rgb8(0xD9, 0x1C, 0x55),
        }
    }
}

impl ComicTextProps {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug)]
pub struct ComicTextState {
    progress: AnimatedValue,
    started: bool,
}

impl ComicTextState {
    pub fn new(props: &ComicTextProps) -> Self {
        Self {
            progress: AnimatedValue::with_spec(0.0, motion_spec(props.entrance_duration_ms)),
            started: false,
        }
    }

    pub fn tick(&mut self, now: Instant, props: &ComicTextProps) {
        self.progress
            .set_spec(motion_spec(props.entrance_duration_ms));
        if !self.started {
            self.started = true;
            self.progress.transition(1.0, now);
        }
    }

    pub fn progress_at(&self, now: Instant) -> f32 {
        self.progress.value_at(now).clamp(0.0, 1.0)
    }
}

pub fn comic_text<'a, Message: 'a>(
    content: impl Into<Cow<'a, str>>,
    state: &ComicTextState,
    now: Instant,
    props: &ComicTextProps,
) -> Element<'a, Message> {
    let label = segment_text(content.into().as_ref(), TextSegmentMode::Character)
        .into_iter()
        .map(str::to_uppercase)
        .collect::<String>();
    let progress = state.progress_at(now);
    let font_px = (props.font_size.max(0.5) * 16.0).round() as u16;
    let opacity = progress;

    let shadow_primary = layered_text(
        label.clone(),
        font_px,
        Color {
            a: opacity * 0.9,
            ..props.outline_color
        },
        Vector::new(5.0, 5.0),
        1.0,
    );
    let shadow_secondary = layered_text(
        label.clone(),
        font_px,
        Color {
            a: opacity * 0.8,
            ..props.shadow_color
        },
        Vector::new(3.0, 3.0),
        1.0,
    );
    let fill = layered_text(
        label,
        font_px,
        Color {
            a: opacity,
            ..props.fill_color
        },
        Vector::ZERO,
        0.82 + progress * 0.18,
    );

    container(stack(vec![shadow_primary, shadow_secondary, fill]))
        .width(Length::Shrink)
        .height(Length::Shrink)
        .into()
}

fn layered_text<'a, Message: 'a>(
    label: String,
    font_px: u16,
    color: Color,
    offset: Vector,
    scale: f32,
) -> Element<'a, Message> {
    iced::widget::float(
        container(
            text(label)
                .size(u32::from(font_px))
                .style(move |_| iced::widget::text::Style { color: Some(color) }),
        )
        .width(Length::Shrink)
        .height(Length::Shrink),
    )
    .translate(move |_bounds, _viewport| offset)
    .scale(scale)
    .into()
}

fn motion_spec(duration_ms: u64) -> MotionSpec {
    MotionSpec::new()
        .duration(TransitionDuration::CustomMs(
            duration_ms.clamp(1, u16::MAX as u64) as u16,
        ))
        .easing(Easing::Out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn comic_text_defaults_match_docs() {
        let props = ComicTextProps::new();

        assert_eq!(props.font_size, 5.0);
        assert_eq!(props.entrance_duration_ms, 600);
    }

    #[test]
    fn comic_text_progress_runs_once() {
        let props = ComicTextProps::new();
        let mut state = ComicTextState::new(&props);
        let start = Instant::now();

        state.tick(start, &props);
        assert!(state.progress_at(start + Duration::from_millis(700)) > 0.9);
    }
}
