use crate::primitives::{
    EffectCanvasProps, EffectProgram, ambient_particles, effect_canvas, sample_ambient_particle,
};
use iced::mouse;
use iced::widget::{canvas, mouse_area};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Vector};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParticlesAction {
    Move(Point),
    Exit,
    Refresh,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct ParticlesState {
    pointer: Option<Point>,
    generation: u64,
}

impl ParticlesState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_action(&mut self, action: &ParticlesAction) {
        match action {
            ParticlesAction::Move(position) => self.pointer = Some(*position),
            ParticlesAction::Exit => self.pointer = None,
            ParticlesAction::Refresh => self.generation = self.generation.wrapping_add(1),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticlesProps {
    pub width: Length,
    pub height: Length,
    pub quantity: usize,
    pub staticity: f32,
    pub ease: f32,
    pub size: f32,
    pub refresh: bool,
    pub color: Color,
    pub vx: f32,
    pub vy: f32,
    pub seed: u64,
}

impl Default for ParticlesProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            height: Length::Fixed(320.0),
            quantity: 100,
            staticity: 50.0,
            ease: 50.0,
            size: 0.4,
            refresh: false,
            color: Color::WHITE,
            vx: 0.0,
            vy: 0.0,
            seed: 91,
        }
    }
}

impl ParticlesProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn particles<'a, Message: Clone + 'a>(
    state: &ParticlesState,
    elapsed: Duration,
    props: ParticlesProps,
    on_action: impl Fn(ParticlesAction) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let width = props.width;
    let height = props.height;
    let canvas: Element<'a, Message> = effect_canvas::<_, Message>(
        ParticlesProgram {
            elapsed,
            pointer: state.pointer,
            seed: props.seed.wrapping_add(state.generation),
            props,
        },
        EffectCanvasProps::new().width(width).height(height),
    )
    .into();

    mouse_area(canvas)
        .on_move({
            let on_action = on_action.clone();
            move |position| on_action(ParticlesAction::Move(position))
        })
        .on_exit(on_action(ParticlesAction::Exit))
        .interaction(mouse::Interaction::None)
        .into()
}

struct ParticlesProgram {
    elapsed: Duration,
    pointer: Option<Point>,
    seed: u64,
    props: ParticlesProps,
}

impl<Message> EffectProgram<Message> for ParticlesProgram {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<canvas::Geometry> {
        let particles = ambient_particles(self.props.quantity, bounds, self.seed, self.props.size);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let elapsed_secs = self.elapsed.as_secs_f32();

        for particle in particles {
            let sampled = sample_ambient_particle(
                particle,
                elapsed_secs,
                self.pointer,
                self.props.staticity,
                self.props.ease,
                Vector::new(self.props.vx, self.props.vy),
                bounds,
            );

            frame.fill(
                &canvas::Path::circle(sampled.position, sampled.size.max(0.2)),
                Color {
                    a: sampled.opacity.clamp(0.0, 1.0),
                    ..self.props.color
                },
            );
        }

        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particles_defaults_match_docs() {
        let props = ParticlesProps::new();

        assert_eq!(props.quantity, 100);
        assert_eq!(props.staticity, 50.0);
        assert_eq!(props.ease, 50.0);
        assert_eq!(props.size, 0.4);
    }

    #[test]
    fn particles_state_refresh_increments_generation() {
        let mut state = ParticlesState::new();
        state.apply_action(&ParticlesAction::Refresh);

        assert_eq!(state.generation, 1);
    }
}
