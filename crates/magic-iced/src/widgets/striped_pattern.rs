use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};
use iced::widget::canvas::{self, Stroke};
use iced::{Color, Element, Length, Rectangle, Renderer};
use iced_shadcn::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum StripedPatternDirection {
    #[default]
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StripedPatternProps {
    pub width: Length,
    pub height: Length,
    pub stripe_width: f32,
    pub stripe_height: f32,
    pub direction: StripedPatternDirection,
    pub stroke_width: f32,
    pub stroke_color: Option<Color>,
    pub stroke_dasharray: Option<f32>,
}

impl Default for StripedPatternProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            height: Length::Fixed(220.0),
            stripe_width: 10.0,
            stripe_height: 10.0,
            direction: StripedPatternDirection::Left,
            stroke_width: 0.5,
            stroke_color: None,
            stroke_dasharray: None,
        }
    }
}

impl StripedPatternProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn striped_pattern<'a, Message: 'a>(
    props: StripedPatternProps,
    theme: &Theme,
) -> Element<'a, Message> {
    effect_canvas::<_, Message>(
        StripedPatternProgram {
            props,
            color: props
                .stroke_color
                .or_else(|| theme.registry.color("magic.pattern.stripe"))
                .unwrap_or(Color::from_rgba(1.0, 1.0, 1.0, 0.16)),
        },
        EffectCanvasProps::new()
            .width(props.width)
            .height(props.height),
    )
    .into()
}

struct StripedPatternProgram {
    props: StripedPatternProps,
    color: Color,
}

impl<Message> EffectProgram<Message> for StripedPatternProgram {
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
        let stride = self.props.stripe_width.max(1.0);
        let extent = bounds.width + bounds.height + stride * 2.0;

        let dash_segment = self.props.stroke_dasharray.unwrap_or_default().max(1.0);
        let dash_segments = [dash_segment, dash_segment];
        let dash = self.props.stroke_dasharray.map(|_| canvas::LineDash {
            offset: 0,
            segments: &dash_segments,
        });
        let stroke = Stroke {
            width: self.props.stroke_width.max(0.25),
            style: canvas::stroke::Style::Solid(self.color),
            line_dash: dash.unwrap_or_default(),
            ..Stroke::default()
        };

        let mut offset = -bounds.height - stride;
        while offset <= bounds.width + stride {
            let (from, to) = match self.props.direction {
                StripedPatternDirection::Left => (
                    iced::Point::new(offset, bounds.height),
                    iced::Point::new(offset + extent, -extent + bounds.height),
                ),
                StripedPatternDirection::Right => (
                    iced::Point::new(offset, 0.0),
                    iced::Point::new(offset + extent, extent),
                ),
            };
            frame.stroke(&canvas::Path::line(from, to), stroke);
            offset += stride;
        }

        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn striped_pattern_defaults_match_docs() {
        let props = StripedPatternProps::new();

        assert_eq!(props.stripe_width, 10.0);
        assert_eq!(props.stripe_height, 10.0);
        assert_eq!(props.direction, StripedPatternDirection::Left);
    }
}
