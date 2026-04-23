use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};
use iced::time::Duration;
use iced::widget::{canvas, container};
use iced::{Color, Element, Length, Rectangle, Renderer};
use iced_shadcn::{ButtonProps, DecorativeSurfaceProps, Theme, button_content, decorative_surface};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RainbowButtonVariant {
    #[default]
    Default,
    Outline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RainbowButtonSize {
    Sm,
    #[default]
    Default,
    Lg,
    Icon,
}

#[derive(Clone, Copy, Debug)]
pub struct RainbowButtonProps {
    pub button: ButtonProps,
    pub radius: f32,
    pub variant: RainbowButtonVariant,
    pub size: RainbowButtonSize,
    pub duration_secs: f32,
}

impl Default for RainbowButtonProps {
    fn default() -> Self {
        Self {
            button: ButtonProps::new(),
            radius: 6.0,
            variant: RainbowButtonVariant::Default,
            size: RainbowButtonSize::Default,
            duration_secs: 2.0,
        }
    }
}

impl RainbowButtonProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn button(mut self, button: ButtonProps) -> Self {
        self.button = button;
        self
    }
}

pub fn rainbow_button<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    on_press: Option<Message>,
    elapsed: Duration,
    props: RainbowButtonProps,
    theme: &Theme,
) -> Element<'a, Message> {
    decorative_surface(
        button_content(content, on_press, props.button, theme),
        Vec::new(),
        vec![
            container(effect_canvas::<_, Message>(
                RainbowButtonProgram {
                    elapsed,
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

struct RainbowButtonProgram {
    elapsed: Duration,
    duration_secs: f32,
    variant: RainbowButtonVariant,
}

impl<Message> EffectProgram<Message> for RainbowButtonProgram {
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
        let stroke = bounds.height.min(bounds.width).max(1.0) * 0.06;
        let colors = rainbow_colors(phase);
        let rect = canvas::Path::rounded_rectangle(
            iced::Point::new(0.0, 0.0),
            bounds.size(),
            6.0.into(),
        );

        match self.variant {
            RainbowButtonVariant::Default => {
                frame.stroke(
                    &rect,
                    canvas::Stroke::default().with_width(stroke).with_color(colors[0]),
                );
                frame.fill_rectangle(
                    iced::Point::new(0.0, 0.0),
                    bounds.size(),
                    Color {
                        a: 0.02,
                        ..colors[1]
                    },
                );
            }
            RainbowButtonVariant::Outline => {
                frame.stroke(
                    &rect,
                    canvas::Stroke::default()
                        .with_width(stroke)
                        .with_color(colors[1]),
                );
            }
        }

        vec![frame.into_geometry()]
    }
}

fn rainbow_colors(phase: f32) -> [Color; 2] {
    let palette = [
        Color::from_rgb8(0xFF, 0x5F, 0x57),
        Color::from_rgb8(0xFF, 0xC0, 0x00),
        Color::from_rgb8(0x34, 0xD3, 0xFF),
        Color::from_rgb8(0x8B, 0x5C, 0xFF),
        Color::from_rgb8(0xFF, 0x4E, 0xB8),
    ];
    let index = (phase * palette.len() as f32).floor() as usize % palette.len();
    [palette[index], palette[(index + 1) % palette.len()]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rainbow_button_defaults_are_stable() {
        let props = RainbowButtonProps::new();

        assert_eq!(props.radius, 6.0);
        assert_eq!(props.duration_secs, 2.0);
        assert!(matches!(props.variant, RainbowButtonVariant::Default));
    }
}
