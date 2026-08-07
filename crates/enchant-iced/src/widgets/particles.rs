use crate::primitives::{
    AmbientParticle, EffectCanvasProps, EffectProgram, ambient_particle_out_of_bounds,
    ambient_particles, effect_canvas, sample_ambient_particle,
};
use iced::mouse;
use iced::widget::{canvas, mouse_area};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Vector};
use std::cell::RefCell;
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
            ParticlesAction::Exit => {}
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

#[derive(Default)]
struct ParticlesRuntimeState {
    runtime: RefCell<ParticlesRuntime>,
}

#[derive(Default)]
struct ParticlesRuntime {
    particles: Vec<AmbientParticle>,
    seed: u64,
    quantity: usize,
    size: f32,
    refresh: bool,
    bounds: Option<(f32, f32)>,
    last_elapsed_secs: f32,
    respawn_nonce: u64,
}

impl ParticlesRuntime {
    fn ensure_initialized(&mut self, seed: u64, bounds: Rectangle, props: &ParticlesProps) -> bool {
        let bounds_key = (bounds.width, bounds.height);
        let needs_reset = self.particles.len() != props.quantity
            || self.seed != seed
            || self.quantity != props.quantity
            || (self.size - props.size).abs() > f32::EPSILON
            || self.refresh != props.refresh
            || self.bounds != Some(bounds_key);

        if needs_reset {
            self.particles = ambient_particles(props.quantity, bounds, seed, props.size);
            self.seed = seed;
            self.quantity = props.quantity;
            self.size = props.size;
            self.refresh = props.refresh;
            self.bounds = Some(bounds_key);
            self.last_elapsed_secs = 0.0;
            self.respawn_nonce = 0;
        }

        needs_reset
    }
}

impl<Message> EffectProgram<Message> for ParticlesProgram {
    type State = ParticlesRuntimeState;

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<canvas::Geometry> {
        let mut runtime = state.runtime.borrow_mut();
        let current_elapsed_secs = self.elapsed.as_secs_f32();
        let reset = runtime.ensure_initialized(self.seed, bounds, &self.props);
        if reset {
            runtime.last_elapsed_secs = current_elapsed_secs;
        }

        let frame_factor =
            ((current_elapsed_secs - runtime.last_elapsed_secs).max(0.0) * 60.0).clamp(0.0, 3.0);
        runtime.last_elapsed_secs = current_elapsed_secs;

        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let velocity = Vector::new(self.props.vx, self.props.vy);
        let mut respawn_nonce = runtime.respawn_nonce;

        for (index, particle) in runtime.particles.iter_mut().enumerate() {
            let sampled = sample_ambient_particle(
                particle,
                frame_factor,
                self.pointer,
                self.props.staticity,
                self.props.ease,
                velocity,
                bounds,
            );

            frame.fill(
                &canvas::Path::circle(sampled.position, sampled.size.max(0.0)),
                Color {
                    a: sampled.opacity.clamp(0.0, 1.0),
                    ..self.props.color
                },
            );

            if ambient_particle_out_of_bounds(particle, bounds) {
                let respawn_seed = self.seed ^ respawn_nonce.wrapping_add(index as u64 + 1);
                *particle = ambient_particles(1, bounds, respawn_seed, self.props.size)
                    .into_iter()
                    .next()
                    .expect("ambient_particles(1, ..) must yield one particle");
                respawn_nonce = respawn_nonce.wrapping_add(1);
            }
        }

        runtime.respawn_nonce = respawn_nonce;

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

    #[test]
    fn particles_state_exit_keeps_last_pointer() {
        let mut state = ParticlesState::new();
        state.apply_action(&ParticlesAction::Move(Point::new(12.0, 24.0)));
        state.apply_action(&ParticlesAction::Exit);

        assert_eq!(state.pointer, Some(Point::new(12.0, 24.0)));
    }
}
