use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};
use iced::widget::{canvas, container};
use iced::{Color, Element, Length, Rectangle, Renderer};
use iced_shadcn::{ButtonProps, DecorativeSurfaceProps, Theme, button_content, decorative_surface};
use std::time::Duration;

#[derive(Clone, Copy, Debug)]
pub struct ShinyButtonProps {
    pub button: ButtonProps,
    pub radius: f32,
    pub duration_secs: f32,
    pub sweep_width: f32,
    pub sweep_opacity: f32,
    pub shine_color: Option<Color>,
}

impl Default for ShinyButtonProps {
    fn default() -> Self {
        Self {
            button: ButtonProps::new(),
            radius: 12.0,
            duration_secs: 2.4,
            sweep_width: 20.0,
            sweep_opacity: 0.55,
            shine_color: None,
        }
    }
}

impl ShinyButtonProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn button(mut self, button: ButtonProps) -> Self {
        self.button = button;
        self
    }
}

pub fn shiny_button<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    on_press: Option<Message>,
    elapsed: Duration,
    props: ShinyButtonProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let shine_color = props
        .shine_color
        .or_else(|| theme.registry.color("magic.button.shine"))
        .unwrap_or(theme.palette.primary);

    decorative_surface(
        button_content(content, on_press, props.button, theme),
        Vec::new(),
        vec![
            container(effect_canvas::<_, Message>(
                ShinyButtonProgram {
                    elapsed,
                    color: shine_color,
                    duration_secs: props.duration_secs,
                    sweep_width: props.sweep_width,
                    sweep_opacity: props.sweep_opacity,
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

struct ShinyButtonProgram {
    elapsed: Duration,
    color: Color,
    duration_secs: f32,
    sweep_width: f32,
    sweep_opacity: f32,
}

impl<Message> EffectProgram<Message> for ShinyButtonProgram {
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
        let x = shiny_phase(self.elapsed, self.duration_secs)
            * (bounds.width + self.sweep_width * 2.0)
            - self.sweep_width;
        let path = canvas::Path::line(
            iced::Point::new(x - bounds.height * 0.1, 0.0),
            iced::Point::new(x + bounds.height * 0.1, bounds.height),
        );

        frame.stroke(
            &path,
            canvas::Stroke::default()
                .with_width(self.sweep_width)
                .with_color(Color {
                    a: self.sweep_opacity,
                    ..self.color
                }),
        );

        vec![frame.into_geometry()]
    }
}

fn shiny_phase(elapsed: Duration, duration_secs: f32) -> f32 {
    let duration = duration_secs.max(0.01);
    (elapsed.as_secs_f32() / duration).rem_euclid(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shiny_button_defaults_are_stable() {
        let props = ShinyButtonProps::new();

        assert_eq!(props.duration_secs, 2.4);
        assert_eq!(props.radius, 12.0);
        assert_eq!(props.sweep_opacity, 0.55);
    }

    #[test]
    fn shiny_phase_wraps_cleanly() {
        let first = shiny_phase(Duration::from_secs(0), 2.4);
        let wrapped = shiny_phase(Duration::from_secs_f32(2.4), 2.4);

        assert_eq!(first, 0.0);
        assert!((wrapped - 0.0).abs() < 0.001);
    }
}
