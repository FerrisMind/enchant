use iced::mouse;
use iced::time::Instant;
use iced::widget::{canvas, mouse_area};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Vector};
use std::time::Duration;
use unicode_segmentation::UnicodeSegmentation;

use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Text3DFlipAction {
    Trigger,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Text3DFlipDirection {
    Top,
    #[default]
    Right,
    Bottom,
    Left,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum Text3DFlipStaggerFrom {
    #[default]
    First,
    Last,
    Center,
    Index(usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Text3DFlipProps {
    pub stagger_duration_secs: f32,
    pub duration_secs: f32,
    pub rotate_direction: Text3DFlipDirection,
    pub stagger_from: Text3DFlipStaggerFrom,
    pub font_size: f32,
    pub color: Color,
}

impl Default for Text3DFlipProps {
    fn default() -> Self {
        Self {
            stagger_duration_secs: 0.05,
            duration_secs: 0.45,
            rotate_direction: Text3DFlipDirection::Right,
            stagger_from: Text3DFlipStaggerFrom::First,
            font_size: 56.0,
            color: Color::BLACK,
        }
    }
}

impl Text3DFlipProps {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Text3DFlipState {
    started_at: Option<Instant>,
}

impl Text3DFlipState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_action(&mut self, action: &Text3DFlipAction, now: Instant) {
        match action {
            Text3DFlipAction::Trigger => self.started_at = Some(now),
        }
    }

    pub fn tick(&mut self, now: Instant, content: &str, props: &Text3DFlipProps) {
        let Some(started_at) = self.started_at else {
            return;
        };
        let total = total_duration(content, props);
        if now.saturating_duration_since(started_at).as_secs_f32() > total {
            self.started_at = None;
        }
    }
}

pub fn text_3d_flip<'a, Message: Clone + 'a>(
    content: &'a str,
    state: &Text3DFlipState,
    now: Instant,
    props: Text3DFlipProps,
    on_action: impl Fn(Text3DFlipAction) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let graphemes = split_graphemes(content);
    let width = (graphemes.len().max(1) as f32 * props.font_size * 0.68).max(props.font_size);
    let height = props.font_size * 1.5;

    let canvas: Element<'a, Message> = effect_canvas::<_, Message>(
        Text3DFlipProgram {
            content: graphemes,
            started_at: state.started_at,
            now,
            props,
        },
        EffectCanvasProps::new()
            .width(Length::Fixed(width))
            .height(Length::Fixed(height)),
    )
    .into();

    mouse_area(canvas)
        .on_enter(on_action(Text3DFlipAction::Trigger))
        .interaction(mouse::Interaction::Pointer)
        .into()
}

struct Text3DFlipProgram {
    content: Vec<String>,
    started_at: Option<Instant>,
    now: Instant,
    props: Text3DFlipProps,
}

impl<Message> EffectProgram<Message> for Text3DFlipProgram {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let advance = self.props.font_size * 0.62;
        let elapsed = self
            .started_at
            .map(|started_at| self.now.saturating_duration_since(started_at))
            .unwrap_or(Duration::ZERO);

        for (index, grapheme) in self.content.iter().enumerate() {
            let delay = stagger_delay(index, self.content.len(), self.props.stagger_from);
            let progress = ((elapsed.as_secs_f32() - delay) / self.props.duration_secs.max(0.05))
                .clamp(0.0, 1.0);
            let center = Point::new(advance * index as f32 + advance * 0.5, bounds.height * 0.55);
            let angle = progress * 90.0 * direction_sign(self.props.rotate_direction);
            let axis_scale = angle.to_radians().cos().abs().max(0.08);
            let rotation_hint = angle.to_radians() * 0.08;

            frame.with_save(|frame| {
                frame.translate(Vector::new(center.x, center.y));
                frame.rotate(rotation_hint);
                match self.props.rotate_direction {
                    Text3DFlipDirection::Top | Text3DFlipDirection::Bottom => {
                        frame.scale_nonuniform(Vector::new(1.0, axis_scale));
                    }
                    Text3DFlipDirection::Left | Text3DFlipDirection::Right => {
                        frame.scale_nonuniform(Vector::new(axis_scale, 1.0));
                    }
                }
                frame.fill_text(canvas::Text {
                    content: grapheme.clone(),
                    position: Point::new(0.0, 0.0),
                    color: self.props.color,
                    size: iced::Pixels(self.props.font_size),
                    align_x: iced::alignment::Horizontal::Center.into(),
                    align_y: iced::alignment::Vertical::Center,
                    ..canvas::Text::default()
                });
            });
        }

        vec![frame.into_geometry()]
    }
}

fn split_graphemes(content: &str) -> Vec<String> {
    UnicodeSegmentation::graphemes(content, true)
        .map(ToOwned::to_owned)
        .collect()
}

fn direction_sign(direction: Text3DFlipDirection) -> f32 {
    match direction {
        Text3DFlipDirection::Top | Text3DFlipDirection::Right => 1.0,
        Text3DFlipDirection::Bottom | Text3DFlipDirection::Left => -1.0,
    }
}

fn stagger_delay(index: usize, total: usize, mode: Text3DFlipStaggerFrom) -> f32 {
    let total = total.max(1);
    let base = match mode {
        Text3DFlipStaggerFrom::First => index,
        Text3DFlipStaggerFrom::Last => total.saturating_sub(index + 1),
        Text3DFlipStaggerFrom::Center => {
            let center = total / 2;
            index.abs_diff(center)
        }
        Text3DFlipStaggerFrom::Index(target) => index.abs_diff(target),
    };
    base as f32 * 0.05
}

fn total_duration(content: &str, props: &Text3DFlipProps) -> f32 {
    let total = UnicodeSegmentation::graphemes(content, true).count().max(1);
    stagger_delay(total.saturating_sub(1), total, props.stagger_from) + props.duration_secs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_3d_flip_defaults_match_docs() {
        let props = Text3DFlipProps::new();

        assert_eq!(props.stagger_duration_secs, 0.05);
        assert_eq!(props.rotate_direction, Text3DFlipDirection::Right);
    }

    #[test]
    fn stagger_delay_supports_center_ordering() {
        let total = 5;
        assert_eq!(stagger_delay(2, total, Text3DFlipStaggerFrom::Center), 0.0);
        assert!(stagger_delay(0, total, Text3DFlipStaggerFrom::Center) > 0.0);
    }
}
