use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};
use iced::time::Duration;
use iced::widget::{canvas, container};
use iced::{Color, Element, Length, Rectangle, Renderer};
use iced_shadcn::{ButtonProps, DecorativeSurfaceProps, Theme, button_content, decorative_surface};
use std::time::Duration as StdDuration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PulsatingButtonVariant {
    #[default]
    Pulse,
    Ripple,
}

#[derive(Clone, Copy, Debug)]
pub struct PulsatingButtonProps {
    pub button: ButtonProps,
    pub radius: f32,
    pub duration_secs: f32,
    pub distance_px: f32,
    pub pulse_color: Option<Color>,
    pub variant: PulsatingButtonVariant,
}

impl Default for PulsatingButtonProps {
    fn default() -> Self {
        Self {
            button: ButtonProps::new(),
            radius: 8.0,
            duration_secs: 1.5,
            distance_px: 8.0,
            pulse_color: None,
            variant: PulsatingButtonVariant::Pulse,
        }
    }
}

impl PulsatingButtonProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn button(mut self, button: ButtonProps) -> Self {
        self.button = button;
        self
    }
}

pub fn pulsating_button<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    on_press: Option<Message>,
    elapsed: StdDuration,
    props: PulsatingButtonProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let color = props
        .pulse_color
        .or_else(|| theme.registry.color("magic.button.pulse"))
        .unwrap_or(theme.palette.primary);

    decorative_surface(
        button_content(content, on_press, props.button, theme),
        Vec::new(),
        vec![
            container(effect_canvas::<_, Message>(
                PulsatingButtonProgram {
                    elapsed,
                    color,
                    distance_px: props.distance_px,
                    duration_secs: props.duration_secs,
                    variant: props.variant,
                },
                EffectCanvasProps::new()
                    .width(Length::Fill)
                    .height(Length::Fill),
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
        ],
        DecorativeSurfaceProps::new()
            .clip(true)
            .radius(props.radius),
        theme,
    )
}

struct PulsatingButtonProgram {
    elapsed: Duration,
    color: Color,
    distance_px: f32,
    duration_secs: f32,
    variant: PulsatingButtonVariant,
}

impl<Message> EffectProgram<Message> for PulsatingButtonProgram {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let duration = self.duration_secs.max(0.05);
        let phase = (self.elapsed.as_secs_f32() / duration).rem_euclid(1.0);
        let center = iced::Point::new(bounds.width * 0.5, bounds.height * 0.5);
        let base = bounds.width.max(bounds.height).max(1.0);
        let radius = base * 0.35 + self.distance_px;

        match self.variant {
            PulsatingButtonVariant::Pulse => {
                let scale = 0.8 + phase * 0.6;
                let alpha = (1.0 - phase).powf(1.8) * 0.22;
                frame.fill(
                    &canvas::Path::circle(center, radius * scale),
                    Color { a: alpha, ..self.color },
                );
            }
            PulsatingButtonVariant::Ripple => {
                for index in 0..4 {
                    let local = (phase + index as f32 * 0.18).fract();
                    let alpha = (1.0 - local).powf(1.7) * 0.22;
                    frame.stroke(
                        &canvas::Path::circle(center, radius * (0.7 + local * 1.2)),
                        canvas::Stroke::default()
                            .with_width(2.0)
                            .with_color(Color { a: alpha, ..self.color }),
                    );
                }
            }
        }

        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulsating_button_defaults_are_stable() {
        let props = PulsatingButtonProps::new();

        assert_eq!(props.duration_secs, 1.5);
        assert_eq!(props.distance_px, 8.0);
        assert!(matches!(props.variant, PulsatingButtonVariant::Pulse));
    }
}
