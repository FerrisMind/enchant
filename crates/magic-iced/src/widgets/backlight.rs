use super::border_track::RoundedRectTrack;
use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};
use iced::widget::container;
use iced::{Color, Element, Length, Padding, Rectangle, Renderer};
use iced_shadcn::{DecorativeSurfaceProps, Theme, decorative_surface};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BacklightProps {
    pub blur: f32,
    pub padding: Padding,
    pub radius: Option<f32>,
    pub intensity: f32,
    pub color: Option<Color>,
}

impl Default for BacklightProps {
    fn default() -> Self {
        Self {
            blur: 20.0,
            padding: Padding::ZERO,
            radius: None,
            intensity: 0.38,
            color: None,
        }
    }
}

impl BacklightProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn blur(mut self, blur: f32) -> Self {
        self.blur = blur.max(0.0);
        self
    }
}

pub fn backlight<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    props: BacklightProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let radius = props.radius.unwrap_or(theme.radius.lg);
    let glow = props
        .color
        .or_else(|| theme.registry.color("magic.backlight.color"))
        .unwrap_or(theme.palette.primary);

    decorative_surface(
        container(content.into())
            .padding(props.padding)
            .width(Length::Shrink)
            .height(Length::Shrink),
        vec![
            container(effect_canvas::<_, Message>(
                BacklightProgram {
                    blur: props.blur,
                    radius,
                    intensity: props.intensity.clamp(0.0, 1.0),
                    color: glow,
                },
                EffectCanvasProps::new()
                    .width(Length::Fill)
                    .height(Length::Fill),
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
        ],
        Vec::new(),
        DecorativeSurfaceProps::new().clip(false).radius(radius),
        theme,
    )
}

struct BacklightProgram {
    blur: f32,
    radius: f32,
    intensity: f32,
    color: Color,
}

impl<Message> EffectProgram<Message> for BacklightProgram {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<iced::widget::canvas::Geometry> {
        use iced::widget::canvas;

        let mut frame = canvas::Frame::new(renderer, bounds.size());
        for index in (0..12).rev() {
            let factor = index as f32 / 11.0;
            let inflate = self.blur * (0.2 + factor * 0.8);
            let color = Color {
                a: self.intensity * (1.0 - factor).powi(2),
                ..self.color
            };
            let origin = iced::Point::new(-inflate, -inflate);
            let path = RoundedRectTrack::new(
                iced::Size::new(bounds.width + inflate * 2.0, bounds.height + inflate * 2.0),
                self.radius + inflate * 0.25,
            )
            .path(origin);
            frame.fill(&path, color);
        }

        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backlight_defaults_match_docs() {
        let props = BacklightProps::new();

        assert_eq!(props.blur, 20.0);
        assert_eq!(props.intensity, 0.38);
    }
}
