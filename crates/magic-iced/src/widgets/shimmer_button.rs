use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};
use iced::widget::{canvas, container};
use iced::{Color, Element, Length, Rectangle, Renderer};
use iced_shadcn::{ButtonProps, DecorativeSurfaceProps, Theme, button_content, decorative_surface};
use std::time::Duration;

#[derive(Clone, Copy, Debug)]
pub struct ShimmerButtonProps {
    pub button: ButtonProps,
    pub radius: f32,
    pub duration_secs: f32,
    pub shimmer_width: f32,
    pub overlay_opacity: f32,
    pub shimmer_color: Option<Color>,
}

impl Default for ShimmerButtonProps {
    fn default() -> Self {
        Self {
            button: ButtonProps::new(),
            radius: 999.0,
            duration_secs: 3.0,
            shimmer_width: 28.0,
            overlay_opacity: 0.85,
            shimmer_color: None,
        }
    }
}

impl ShimmerButtonProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn button(mut self, button: ButtonProps) -> Self {
        self.button = button;
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius.max(0.0);
        self
    }
}

pub fn shimmer_button<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    on_press: Option<Message>,
    elapsed: Duration,
    props: ShimmerButtonProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let shimmer_color = props
        .shimmer_color
        .or_else(|| theme.registry.color("magic.button.shimmer"))
        .unwrap_or(theme.palette.foreground);

    decorative_surface(
        button_content(content, on_press, props.button, theme),
        Vec::new(),
        vec![
            container(effect_canvas::<_, Message>(
                ShimmerButtonProgram {
                    elapsed,
                    color: shimmer_color,
                    duration_secs: props.duration_secs,
                    shimmer_width: props.shimmer_width,
                    overlay_opacity: props.overlay_opacity,
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

struct ShimmerButtonProgram {
    elapsed: Duration,
    color: Color,
    duration_secs: f32,
    shimmer_width: f32,
    overlay_opacity: f32,
}

impl<Message> EffectProgram<Message> for ShimmerButtonProgram {
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
        let x = shimmer_phase(self.elapsed, self.duration_secs)
            * (bounds.width + self.shimmer_width * 2.0)
            - self.shimmer_width;

        for index in 0..8 {
            let factor = index as f32 / 7.0;
            let band_x = x - self.shimmer_width * 0.5 + factor * self.shimmer_width;
            let path = canvas::Path::line(
                iced::Point::new(band_x - bounds.height * 0.18, 0.0),
                iced::Point::new(band_x + bounds.height * 0.18, bounds.height),
            );
            frame.stroke(
                &path,
                canvas::Stroke::default()
                    .with_width(2.0 + (1.0 - factor) * 3.0)
                    .with_color(Color {
                        a: self.overlay_opacity * (1.0 - (factor - 0.5).abs() * 1.4).max(0.0),
                        ..self.color
                    }),
            );
        }

        vec![frame.into_geometry()]
    }
}

fn shimmer_phase(elapsed: Duration, duration_secs: f32) -> f32 {
    let duration = duration_secs.max(0.01);
    (elapsed.as_secs_f32() / duration).rem_euclid(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shimmer_button_defaults_are_stable() {
        let props = ShimmerButtonProps::new();

        assert_eq!(props.duration_secs, 3.0);
        assert_eq!(props.radius, 999.0);
        assert_eq!(props.overlay_opacity, 0.85);
    }

    #[test]
    fn shimmer_phase_wraps_cleanly() {
        let first = shimmer_phase(Duration::from_secs(0), 3.0);
        let wrapped = shimmer_phase(Duration::from_secs(3), 3.0);

        assert_eq!(first, 0.0);
        assert!((wrapped - 0.0).abs() < 0.001);
    }
}
