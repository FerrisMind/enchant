use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};
use iced::widget::canvas;
use iced::{Color, Element, Length, Rectangle, Renderer};
use iced_shadcn::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ProgressiveBlurPosition {
    Top,
    #[default]
    Bottom,
    Both,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProgressiveBlurProps {
    pub width: Length,
    pub height: Length,
    pub position: ProgressiveBlurPosition,
    pub blur_levels: Vec<f32>,
    pub veil_color: Option<Color>,
}

impl Default for ProgressiveBlurProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            height: Length::Fill,
            position: ProgressiveBlurPosition::Bottom,
            blur_levels: vec![0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0],
            veil_color: None,
        }
    }
}

impl ProgressiveBlurProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn progressive_blur<'a, Message: 'a>(
    props: ProgressiveBlurProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let width = props.width;
    let height = props.height;
    let veil = props
        .veil_color
        .or_else(|| theme.registry.color("magic.blur.veil"))
        .unwrap_or(theme.palette.background);

    effect_canvas::<_, Message>(
        ProgressiveBlurProgram {
            props,
            veil,
            max_opacity: theme
                .registry
                .number("magic.blur.veil.opacity")
                .unwrap_or(0.52),
        },
        EffectCanvasProps::new().width(width).height(height),
    )
    .into()
}

struct ProgressiveBlurProgram {
    props: ProgressiveBlurProps,
    veil: Color,
    max_opacity: f32,
}

impl<Message> EffectProgram<Message> for ProgressiveBlurProgram {
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
        let levels = if self.props.blur_levels.is_empty() {
            &[1.0][..]
        } else {
            self.props.blur_levels.as_slice()
        };

        draw_layers(
            &mut frame,
            bounds,
            levels,
            self.max_opacity,
            self.veil,
            self.props.position,
        );

        vec![frame.into_geometry()]
    }
}

fn draw_layers(
    frame: &mut canvas::Frame,
    bounds: Rectangle,
    levels: &[f32],
    max_opacity: f32,
    veil: Color,
    position: ProgressiveBlurPosition,
) {
    let count = levels.len().max(1) as f32;
    let height = bounds.height;

    match position {
        ProgressiveBlurPosition::Top => {
            for (index, _) in levels.iter().enumerate() {
                let start = index as f32 / count;
                let end = (index + 1) as f32 / count;
                frame.fill_rectangle(
                    iced::Point::new(0.0, start * height),
                    iced::Size::new(bounds.width, (end - start) * height),
                    Color {
                        a: max_opacity * (1.0 - start),
                        ..veil
                    },
                );
            }
        }
        ProgressiveBlurPosition::Bottom => {
            for (index, _) in levels.iter().enumerate() {
                let start = index as f32 / count;
                let end = (index + 1) as f32 / count;
                frame.fill_rectangle(
                    iced::Point::new(0.0, height - end * height),
                    iced::Size::new(bounds.width, (end - start) * height),
                    Color {
                        a: max_opacity * (1.0 - start),
                        ..veil
                    },
                );
            }
        }
        ProgressiveBlurPosition::Both => {
            draw_layers(
                frame,
                bounds,
                levels,
                max_opacity * 0.8,
                veil,
                ProgressiveBlurPosition::Top,
            );
            draw_layers(
                frame,
                bounds,
                levels,
                max_opacity * 0.8,
                veil,
                ProgressiveBlurPosition::Bottom,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progressive_blur_defaults_match_docs() {
        let props = ProgressiveBlurProps::new();

        assert_eq!(props.position, ProgressiveBlurPosition::Bottom);
        assert_eq!(props.blur_levels.len(), 8);
        assert_eq!(props.blur_levels[0], 0.5);
        assert_eq!(props.blur_levels[7], 64.0);
    }
}
