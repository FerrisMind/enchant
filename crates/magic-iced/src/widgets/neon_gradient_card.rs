use super::border_track::RoundedRectTrack;
use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};
use iced::widget::container;
use iced::{Color, Element, Length, Padding, Rectangle, Renderer};
use iced_shadcn::{DecorativeSurfaceProps, Theme, decorative_surface};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NeonColors {
    pub first_color: Color,
    pub second_color: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NeonGradientCardProps {
    pub border_size: f32,
    pub border_radius: f32,
    pub neon_colors: Option<NeonColors>,
    pub padding: Padding,
    pub background: Option<Color>,
    pub glow_opacity: f32,
    pub width: Length,
    pub height: Length,
}

impl Default for NeonGradientCardProps {
    fn default() -> Self {
        Self {
            border_size: 2.0,
            border_radius: 20.0,
            neon_colors: None,
            padding: Padding::from(24.0),
            background: None,
            glow_opacity: 0.28,
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }
}

impl NeonGradientCardProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn neon_gradient_card<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    elapsed: Duration,
    props: NeonGradientCardProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let colors = props.neon_colors.unwrap_or(NeonColors {
        first_color: theme
            .registry
            .color("magic.neon.from")
            .unwrap_or(Color::from_rgb8(0xFF, 0x00, 0xAA)),
        second_color: theme
            .registry
            .color("magic.neon.to")
            .unwrap_or(Color::from_rgb8(0x00, 0xFF, 0xF1)),
    });

    let background = props.background.unwrap_or(theme.palette.card);
    let radius = props.border_radius.max(0.0);

    decorative_surface(
        container(content.into())
            .padding(props.padding)
            .width(props.width)
            .height(props.height)
            .style(move |_| iced::widget::container::Style {
                background: Some(iced::Background::Color(background)),
                border: iced::Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: (radius - props.border_size).max(0.0).into(),
                },
                ..iced::widget::container::Style::default()
            }),
        vec![
            container(effect_canvas::<_, Message>(
                NeonGradientCardProgram {
                    elapsed,
                    border_size: props.border_size.max(1.0),
                    radius,
                    colors,
                    glow_opacity: props.glow_opacity.clamp(0.0, 1.0),
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
        DecorativeSurfaceProps::new()
            .width(props.width)
            .height(props.height)
            .clip(false)
            .radius(radius),
        theme,
    )
}

struct NeonGradientCardProgram {
    elapsed: Duration,
    border_size: f32,
    radius: f32,
    colors: NeonColors,
    glow_opacity: f32,
}

impl<Message> EffectProgram<Message> for NeonGradientCardProgram {
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
        let track = RoundedRectTrack::new(bounds.size(), self.radius);
        let phase = (self.elapsed.as_secs_f32() / 3.0).rem_euclid(1.0);

        for index in (0..18).rev() {
            let factor = index as f32 / 17.0;
            let sample = track.sample((phase + factor * 0.05).fract());
            let color = mix_color(self.colors.first_color, self.colors.second_color, factor);

            frame.fill(
                &canvas::Path::circle(sample.point, self.border_size * (1.0 + factor)),
                Color {
                    a: self.glow_opacity * (1.0 - factor) * 0.9,
                    ..color
                },
            );
        }

        frame.stroke(
            &track.path(iced::Point::ORIGIN),
            canvas::Stroke::default()
                .with_width(self.border_size)
                .with_color(Color {
                    a: 0.35,
                    ..self.colors.first_color
                }),
        );

        vec![frame.into_geometry()]
    }
}

fn mix_color(from: Color, to: Color, progress: f32) -> Color {
    let progress = progress.clamp(0.0, 1.0);

    Color {
        r: from.r + (to.r - from.r) * progress,
        g: from.g + (to.g - from.g) * progress,
        b: from.b + (to.b - from.b) * progress,
        a: from.a + (to.a - from.a) * progress,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neon_gradient_card_defaults_match_docs() {
        let props = NeonGradientCardProps::new();

        assert_eq!(props.border_size, 2.0);
        assert_eq!(props.border_radius, 20.0);
        assert_eq!(props.glow_opacity, 0.28);
    }
}
