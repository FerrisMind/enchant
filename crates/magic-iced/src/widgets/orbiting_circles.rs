use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas, orbit_translation};
use iced::widget::{container, float, stack};
use iced::{Color, Element, Length, Rectangle, Renderer, Vector};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrbitingCirclesProps {
    pub reverse: bool,
    pub duration_secs: f32,
    pub delay_secs: f32,
    pub radius: f32,
    pub path: bool,
    pub icon_size: f32,
    pub speed: f32,
    pub stroke_color: Option<Color>,
    pub width: Length,
    pub height: Length,
}

impl Default for OrbitingCirclesProps {
    fn default() -> Self {
        Self {
            reverse: false,
            duration_secs: 20.0,
            delay_secs: 0.0,
            radius: 160.0,
            path: true,
            icon_size: 30.0,
            speed: 1.0,
            stroke_color: None,
            width: Length::Fill,
            height: Length::Fixed(320.0),
        }
    }
}

impl OrbitingCirclesProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn orbiting_circles<'a, Message: 'a>(
    children: Vec<Element<'a, Message>>,
    elapsed: Duration,
    props: OrbitingCirclesProps,
) -> Element<'a, Message> {
    let mut layers: Vec<Element<'a, Message>> = Vec::with_capacity(children.len() + 1);

    if props.path {
        layers.push(
            effect_canvas::<_, Message>(
                OrbitPathProgram {
                    radius: props.radius.max(1.0),
                    color: props
                        .stroke_color
                        .unwrap_or(Color::from_rgba(1.0, 1.0, 1.0, 0.12)),
                },
                EffectCanvasProps::new()
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .into(),
        );
    }

    let total = children.len().max(1) as f32;
    let direction = if props.reverse { -1.0 } else { 1.0 };
    let base_rotation = (elapsed.as_secs_f32() - props.delay_secs.max(0.0))
        / (props.duration_secs / props.speed.max(0.01)).max(0.05)
        * 360.0
        * direction;

    for (index, child) in children.into_iter().enumerate() {
        let angle = base_rotation + (360.0 / total) * index as f32 - 90.0;
        let translation = orbit_translation(props.radius, angle);
        layers.push(
            container(
                float(
                    container(child)
                        .width(Length::Fixed(props.icon_size))
                        .height(Length::Fixed(props.icon_size))
                        .center_x(Length::Fill)
                        .center_y(Length::Fill),
                )
                .translate(move |_bounds, _viewport| Vector::new(translation.x, translation.y)),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
        );
    }

    container(stack(layers))
        .width(props.width)
        .height(props.height)
        .center_x(props.width)
        .center_y(props.height)
        .into()
}

struct OrbitPathProgram {
    radius: f32,
    color: Color,
}

impl<Message> EffectProgram<Message> for OrbitPathProgram {
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
        frame.stroke(
            &canvas::Path::circle(frame.center(), self.radius),
            canvas::Stroke::default().with_color(self.color),
        );
        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orbiting_circles_defaults_match_docs() {
        let props = OrbitingCirclesProps::new();

        assert!(props.path);
        assert_eq!(props.radius, 160.0);
        assert_eq!(props.icon_size, 30.0);
        assert_eq!(props.speed, 1.0);
    }
}
