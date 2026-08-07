use crate::primitives::{EffectCanvasProps, EffectProgram, PointerMotion, effect_canvas};
use iced::mouse;
use iced::time::Instant;
use iced::widget::{mouse_area, stack};
use iced::{Color, Element, Length, Point, Rectangle, Renderer};
use std::borrow::Cow;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Default)]
pub enum CoolParticleVisual {
    #[default]
    Circle,
    Text(Cow<'static, str>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CoolModeAction {
    Move(Point),
    Press,
    Release,
    Exit,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CoolParticle {
    position: Point,
    velocity: iced::Vector,
    life: f32,
    size: f32,
}

#[derive(Clone, Debug)]
pub struct CoolModeState {
    particles: Vec<CoolParticle>,
    pressed: bool,
    pointer: Option<Point>,
    last_emit_at: Option<Instant>,
    last_tick_at: Option<Instant>,
    motion: PointerMotion,
}

impl Default for CoolModeState {
    fn default() -> Self {
        Self {
            particles: Vec::new(),
            pressed: false,
            pointer: None,
            last_emit_at: None,
            last_tick_at: None,
            motion: PointerMotion::new(),
        }
    }
}

impl CoolModeState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_action(&mut self, action: &CoolModeAction, now: Instant, props: &CoolModeProps) {
        match action {
            CoolModeAction::Move(position) => {
                self.pointer = Some(*position);
                self.motion.update(*position, now);
                if self.pressed {
                    self.emit(now, *position, props, 1);
                }
            }
            CoolModeAction::Press => {
                self.pressed = true;
                let position = self.pointer.unwrap_or(Point::new(0.0, 0.0));
                self.motion.update(position, now);
                self.emit(now, position, props, props.particle_count.min(8));
            }
            CoolModeAction::Release | CoolModeAction::Exit => {
                self.pressed = false;
                self.pointer = None;
            }
        }
    }

    pub fn tick(&mut self, now: Instant, props: &CoolModeProps) {
        let dt = self
            .last_tick_at
            .map(|last| now.saturating_duration_since(last).as_secs_f32())
            .unwrap_or(0.0);
        self.last_tick_at = Some(now);

        for particle in &mut self.particles {
            particle.position.x += particle.velocity.x * dt;
            particle.position.y += particle.velocity.y * dt;
            particle.velocity.y -= props.speed_up * 0.8 * dt;
            particle.life -= dt;
        }
        self.particles.retain(|particle| particle.life > 0.0);
    }

    fn emit(&mut self, now: Instant, position: Point, props: &CoolModeProps, burst: usize) {
        if self
            .last_emit_at
            .is_some_and(|last| now.saturating_duration_since(last) < Duration::from_millis(24))
        {
            return;
        }
        self.last_emit_at = Some(now);

        let base = self.motion.velocity();
        for index in 0..burst {
            let direction = if index % 2 == 0 { -1.0 } else { 1.0 };
            let spread = 0.5 + index as f32 * 0.18;
            self.particles.push(CoolParticle {
                position,
                velocity: iced::Vector::new(
                    direction * props.speed_horz * spread + base.x * 0.04,
                    -(props.speed_up * (0.6 + spread * 0.25)) + base.y * 0.02,
                ),
                life: 0.9,
                size: props.size.max(4.0),
            });
        }

        if self.particles.len() > props.particle_count.max(1) * 6 {
            let drain = self.particles.len() - props.particle_count.max(1) * 6;
            self.particles.drain(0..drain);
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoolModeProps {
    pub visual: CoolParticleVisual,
    pub size: f32,
    pub particle_count: usize,
    pub speed_horz: f32,
    pub speed_up: f32,
}

impl Default for CoolModeProps {
    fn default() -> Self {
        Self {
            visual: CoolParticleVisual::Circle,
            size: 20.0,
            particle_count: 12,
            speed_horz: 10.0,
            speed_up: 25.0,
        }
    }
}

impl CoolModeProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn cool_mode<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    state: &CoolModeState,
    props: &CoolModeProps,
    on_action: impl Fn(CoolModeAction) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let overlay: Element<'a, Message> = effect_canvas::<_, Message>(
        CoolModeProgram {
            particles: state.particles.clone(),
            visual: props.visual.clone(),
            size: props.size,
        },
        EffectCanvasProps::new()
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .into();

    mouse_area(stack(vec![content.into(), overlay]))
        .on_move({
            let on_action = on_action.clone();
            move |position| on_action(CoolModeAction::Move(position))
        })
        .on_press(on_action(CoolModeAction::Press))
        .on_release(on_action(CoolModeAction::Release))
        .on_exit(on_action(CoolModeAction::Exit))
        .interaction(mouse::Interaction::Pointer)
        .into()
}

struct CoolModeProgram {
    particles: Vec<CoolParticle>,
    visual: CoolParticleVisual,
    size: f32,
}

impl<Message> EffectProgram<Message> for CoolModeProgram {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<iced::widget::canvas::Geometry> {
        let mut frame = iced::widget::canvas::Frame::new(renderer, bounds.size());
        for particle in &self.particles {
            match &self.visual {
                CoolParticleVisual::Circle => {
                    frame.fill(
                        &iced::widget::canvas::Path::circle(particle.position, particle.size * 0.5),
                        Color {
                            a: particle.life.clamp(0.0, 1.0),
                            ..Color::from_rgb8(255, 96, 112)
                        },
                    );
                }
                CoolParticleVisual::Text(label) => {
                    frame.fill_text(iced::widget::canvas::Text {
                        content: label.to_string(),
                        position: particle.position,
                        color: Color {
                            a: particle.life.clamp(0.0, 1.0),
                            ..Color::WHITE
                        },
                        size: iced::Pixels(self.size.max(8.0)),
                        align_x: iced::alignment::Horizontal::Center.into(),
                        align_y: iced::alignment::Vertical::Center,
                        ..iced::widget::canvas::Text::default()
                    });
                }
            }
        }

        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cool_mode_defaults_match_docs() {
        let props = CoolModeProps::new();

        assert_eq!(props.particle_count, 12);
        assert_eq!(props.speed_horz, 10.0);
        assert_eq!(props.speed_up, 25.0);
    }

    #[test]
    fn cool_mode_press_spawns_particles() {
        let props = CoolModeProps::new();
        let mut state = CoolModeState::new();
        let now = Instant::now();

        state.apply_action(&CoolModeAction::Move(Point::new(20.0, 20.0)), now, &props);
        state.apply_action(&CoolModeAction::Press, now, &props);
        assert!(!state.particles.is_empty());
    }
}
