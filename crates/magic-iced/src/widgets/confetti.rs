use crate::primitives::{
    ConfettiParticle, EffectCanvasProps, EffectProgram, ParticleShape, confetti_particles,
    effect_canvas, sample_confetti_particle,
};
use iced::border;
use iced::mouse;
use iced::time::Instant;
use iced::widget::{canvas, container, mouse_area};
use iced::{Color, Element, Length, Point, Rectangle, Renderer};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConfettiAction {
    Fire(Option<Point>),
    Clear,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConfettiBurst {
    started_at: Instant,
    origin: Point,
    particles: Vec<ConfettiParticle>,
}

#[derive(Clone, Debug, Default)]
pub struct ConfettiState {
    bursts: Vec<ConfettiBurst>,
    nonce: u64,
}

impl ConfettiState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_action(&mut self, action: &ConfettiAction, now: Instant, props: &ConfettiProps) {
        match action {
            ConfettiAction::Fire(origin_override) => {
                let origin = origin_override.unwrap_or(props.origin);
                self.bursts.push(ConfettiBurst {
                    started_at: now,
                    origin,
                    particles: confetti_particles(
                        props.particle_count,
                        Point::new(0.0, 0.0),
                        props.seed.wrapping_add(self.nonce),
                        props.angle,
                        props.spread,
                        props.start_velocity,
                        props.scalar,
                        &props.shapes,
                    ),
                });
                self.nonce = self.nonce.wrapping_add(1);
            }
            ConfettiAction::Clear => self.bursts.clear(),
        }
    }

    pub fn tick(&mut self, now: Instant, _props: &ConfettiProps) {
        self.bursts.retain(|burst| {
            burst.particles.iter().any(|particle| {
                (now.saturating_duration_since(burst.started_at).as_millis() as u64)
                    < particle.life_ms
            })
        });
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConfettiProps {
    pub width: Length,
    pub height: Length,
    pub particle_count: usize,
    pub angle: f32,
    pub spread: f32,
    pub start_velocity: f32,
    pub decay: f32,
    pub gravity: f32,
    pub drift: f32,
    pub flat: bool,
    pub ticks: u64,
    pub origin: Point,
    pub colors: Vec<Color>,
    pub shapes: Vec<ParticleShape>,
    pub scalar: f32,
    pub seed: u64,
}

impl Default for ConfettiProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            height: Length::Fill,
            particle_count: 50,
            angle: 90.0,
            spread: 45.0,
            start_velocity: 45.0,
            decay: 0.9,
            gravity: 1.0,
            drift: 0.0,
            flat: false,
            ticks: 200,
            origin: Point::new(0.5, 0.5),
            colors: vec![
                Color::from_rgb8(0x26, 0xCC, 0xFF),
                Color::from_rgb8(0xA2, 0x5A, 0xFD),
                Color::from_rgb8(0xFF, 0x5E, 0x7E),
                Color::from_rgb8(0x88, 0xFF, 0x5A),
                Color::from_rgb8(0xFC, 0xFF, 0x42),
                Color::from_rgb8(0xFF, 0xA6, 0x2D),
                Color::from_rgb8(0xFF, 0x36, 0xFF),
            ],
            shapes: vec![
                ParticleShape::Square,
                ParticleShape::Circle,
                ParticleShape::Star,
            ],
            scalar: 1.0,
            seed: 117,
        }
    }
}

impl ConfettiProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn confetti<'a, Message: 'a>(
    state: &'a ConfettiState,
    now: Instant,
    props: &'a ConfettiProps,
) -> Element<'a, Message> {
    effect_canvas::<_, Message>(
        ConfettiProgram { state, now, props },
        EffectCanvasProps::new()
            .width(props.width)
            .height(props.height),
    )
    .into()
}

pub fn confetti_button<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    _state: &'a ConfettiState,
    _now: Instant,
    _props: &'a ConfettiProps,
    on_action: impl Fn(ConfettiAction) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let button_content = container(content.into())
        .padding([8.0, 16.0])
        .style(|theme: &iced::Theme| {
            let palette = theme.palette();
            iced::widget::container::Style {
                background: Some(iced::Background::Color(palette.primary)),
                text_color: Some(palette.background),
                border: border::Border {
                    radius: 6.0.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                ..Default::default()
            }
        });

    mouse_area(button_content)
        .on_press(on_action(ConfettiAction::Fire(Some(Point::new(0.5, 0.65)))))
        .interaction(mouse::Interaction::Pointer)
        .into()
}

struct ConfettiProgram<'a> {
    state: &'a ConfettiState,
    now: Instant,
    props: &'a ConfettiProps,
}

impl<Message> EffectProgram<Message> for ConfettiProgram<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        for burst in &self.state.bursts {
            let origin = Point::new(
                bounds.width * burst.origin.x.clamp(0.0, 1.0),
                bounds.height * burst.origin.y.clamp(0.0, 1.0),
            );
            let elapsed_ms = self
                .now
                .saturating_duration_since(burst.started_at)
                .as_millis() as u64;

            for (index, particle) in burst.particles.iter().enumerate() {
                let Some(sample) = sample_confetti_particle(
                    *particle,
                    elapsed_ms.min(self.props.ticks.saturating_mul(10)),
                    self.props.gravity,
                    self.props.decay,
                    self.props.drift,
                ) else {
                    continue;
                };

                let color = self.props.colors[index % self.props.colors.len()];
                let position =
                    Point::new(origin.x + sample.position.x, origin.y + sample.position.y);
                frame.with_save(|frame| {
                    frame.translate(iced::Vector::new(position.x, position.y));
                    frame.rotate(sample.rotation);
                    match sample.shape {
                        ParticleShape::Square => {
                            let rect = canvas::Path::rounded_rectangle(
                                Point::new(-sample.size * 0.5, -sample.size * 0.5),
                                iced::Size::new(
                                    sample.size,
                                    sample.size * if self.props.flat { 0.5 } else { 1.0 },
                                ),
                                border::Radius::from(sample.size * 0.24),
                            );
                            frame.fill(
                                &rect,
                                Color {
                                    a: sample.opacity,
                                    ..color
                                },
                            );
                        }
                        ParticleShape::Circle => {
                            frame.fill(
                                &canvas::Path::circle(Point::ORIGIN, sample.size * 0.5),
                                Color {
                                    a: sample.opacity,
                                    ..color
                                },
                            );
                        }
                        ParticleShape::Star => {
                            frame.fill(
                                &star_path(sample.size * 0.58),
                                Color {
                                    a: sample.opacity,
                                    ..color
                                },
                            );
                        }
                    }
                });
            }
        }

        vec![frame.into_geometry()]
    }
}

fn star_path(radius: f32) -> canvas::Path {
    canvas::Path::new(|builder| {
        let inner = radius * 0.42;
        for index in 0..10 {
            let angle = -std::f32::consts::FRAC_PI_2 + index as f32 * std::f32::consts::PI / 5.0;
            let length = if index % 2 == 0 { radius } else { inner };
            let point = Point::new(angle.cos() * length, angle.sin() * length);
            if index == 0 {
                builder.move_to(point);
            } else {
                builder.line_to(point);
            }
        }
        builder.close();
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confetti_defaults_match_docs() {
        let props = ConfettiProps::new();

        assert_eq!(props.particle_count, 50);
        assert_eq!(props.angle, 90.0);
        assert_eq!(props.spread, 45.0);
        assert_eq!(props.start_velocity, 45.0);
    }

    #[test]
    fn firing_confetti_adds_burst() {
        let mut state = ConfettiState::new();
        let props = ConfettiProps::new();
        let now = Instant::now();

        state.apply_action(&ConfettiAction::Fire(None), now, &props);

        assert_eq!(state.bursts.len(), 1);
    }
}
