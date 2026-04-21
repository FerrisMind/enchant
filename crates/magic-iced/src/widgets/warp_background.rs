use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas, stable_noise};
use iced::widget::container;
use iced::{Color, Element, Length, Padding, Rectangle, Renderer};
use iced_shadcn::{DecorativeSurfaceProps, Theme, decorative_surface};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WarpBackgroundProps {
    pub perspective: f32,
    pub beams_per_side: usize,
    pub beam_size: f32,
    pub beam_delay_max: f32,
    pub beam_delay_min: f32,
    pub beam_duration: f32,
    pub grid_color: Option<Color>,
    pub seed: u64,
    pub padding: Padding,
    pub radius: f32,
    pub width: Length,
    pub height: Length,
    pub background: Option<Color>,
}

impl Default for WarpBackgroundProps {
    fn default() -> Self {
        Self {
            perspective: 100.0,
            beams_per_side: 3,
            beam_size: 5.0,
            beam_delay_max: 3.0,
            beam_delay_min: 0.0,
            beam_duration: 3.0,
            grid_color: None,
            seed: 19,
            padding: Padding::from(20.0),
            radius: 20.0,
            width: Length::Shrink,
            height: Length::Shrink,
            background: None,
        }
    }
}

impl WarpBackgroundProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn warp_background<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    elapsed: Duration,
    props: WarpBackgroundProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let grid_color = props
        .grid_color
        .or_else(|| theme.registry.color("magic.warp.grid"))
        .unwrap_or(theme.palette.border);
    let background = props.background.unwrap_or(theme.palette.card);

    decorative_surface(
        container(content.into())
            .padding(props.padding)
            .width(props.width)
            .height(props.height)
            .style(move |_| iced::widget::container::Style {
                background: Some(iced::Background::Color(background)),
                ..iced::widget::container::Style::default()
            }),
        vec![
            container(effect_canvas::<_, Message>(
                WarpBackgroundProgram {
                    elapsed,
                    props,
                    grid_color,
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
            .clip(true)
            .radius(props.radius)
            .border_color(theme.palette.border)
            .border_width(1.0),
        theme,
    )
}

struct WarpBackgroundProgram {
    elapsed: Duration,
    props: WarpBackgroundProps,
    grid_color: Color,
}

impl<Message> EffectProgram<Message> for WarpBackgroundProgram {
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
        let center = frame.center();
        let perspective = self.props.perspective.max(10.0);
        let beam_size = self.props.beam_size.max(1.0) / 100.0;
        let grid_alpha = (120.0 / perspective).clamp(0.06, 0.24);

        for plane in 0..4 {
            let plane_phase = plane as f32 * 0.25;
            for index in 0..8 {
                let t = index as f32 / 7.0;
                let inset = t * bounds.width.min(bounds.height) * 0.48;
                let alpha = grid_alpha * (1.0 - t * 0.75);
                frame.stroke(
                    &canvas::Path::rectangle(
                        iced::Point::new(inset, inset),
                        iced::Size::new(
                            (bounds.width - inset * 2.0).max(1.0),
                            (bounds.height - inset * 2.0).max(1.0),
                        ),
                    ),
                    canvas::Stroke::default().with_color(Color {
                        a: alpha,
                        ..self.grid_color
                    }),
                );

                let angle = plane_phase * std::f32::consts::TAU + t * 1.2;
                let far = iced::Point::new(
                    center.x + angle.cos() * (bounds.width * 0.55),
                    center.y + angle.sin() * (bounds.height * 0.55),
                );
                frame.stroke(
                    &canvas::Path::line(center, far),
                    canvas::Stroke::default().with_color(Color {
                        a: alpha * 0.7,
                        ..self.grid_color
                    }),
                );
            }
        }

        let time = self.elapsed.as_secs_f32();
        for side in 0..4 {
            for beam in 0..self.props.beams_per_side.max(1) {
                let delay = self.props.beam_delay_min
                    + (self.props.beam_delay_max - self.props.beam_delay_min)
                        * stable_noise(self.props.seed + side as u64, beam as u64 * 5);
                let duration = self.props.beam_duration.max(0.2);
                let cycle = duration + delay;
                let local = time.rem_euclid(cycle);
                if local < delay {
                    continue;
                }

                let progress = ((local - delay) / duration).clamp(0.0, 1.0);
                let lane = stable_noise(self.props.seed + 99 + side as u64, beam as u64 * 11);
                let color = hsla_to_color(
                    stable_noise(self.props.seed + 201, (side * 13 + beam) as u64) * 360.0,
                    0.8,
                    0.6,
                    0.7,
                );
                let half_width = bounds.width * beam_size * 0.5;

                let (from, to) = match side {
                    0 => (
                        iced::Point::new(lane * bounds.width, 0.0),
                        iced::Point::new(center.x, center.y),
                    ),
                    1 => (
                        iced::Point::new(bounds.width, lane * bounds.height),
                        iced::Point::new(center.x, center.y),
                    ),
                    2 => (
                        iced::Point::new(lane * bounds.width, bounds.height),
                        iced::Point::new(center.x, center.y),
                    ),
                    _ => (
                        iced::Point::new(0.0, lane * bounds.height),
                        iced::Point::new(center.x, center.y),
                    ),
                };

                let head = lerp_point(from, to, progress);
                let tail = lerp_point(from, to, (progress - 0.12).max(0.0));
                frame.stroke(
                    &canvas::Path::line(head, tail),
                    canvas::Stroke::default()
                        .with_width(half_width.max(1.0))
                        .with_color(Color {
                            a: 0.5 * (1.0 - progress * 0.2),
                            ..color
                        }),
                );
            }
        }

        vec![frame.into_geometry()]
    }
}

fn lerp_point(from: iced::Point, to: iced::Point, progress: f32) -> iced::Point {
    iced::Point::new(
        from.x + (to.x - from.x) * progress,
        from.y + (to.y - from.y) * progress,
    )
}

fn hsla_to_color(h: f32, s: f32, l: f32, a: f32) -> Color {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = l - c / 2.0;

    let (r1, g1, b1) = match h as i32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    Color::from_rgba(r1 + m, g1 + m, b1 + m, a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warp_background_defaults_match_docs() {
        let props = WarpBackgroundProps::new();

        assert_eq!(props.perspective, 100.0);
        assert_eq!(props.beams_per_side, 3);
        assert_eq!(props.beam_size, 5.0);
        assert_eq!(props.beam_duration, 3.0);
    }
}
