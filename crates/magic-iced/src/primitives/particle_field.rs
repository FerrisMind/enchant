use crate::primitives::StableRng;
use iced::{Point, Rectangle, Vector};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AmbientParticle {
    pub origin: Point,
    pub drift: Vector,
    pub size: f32,
    pub alpha: f32,
    pub magnetism: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SampledParticle {
    pub position: Point,
    pub size: f32,
    pub opacity: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParticleShape {
    Square,
    Circle,
    Star,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConfettiParticle {
    pub origin: Point,
    pub velocity: Vector,
    pub size: f32,
    pub rotation: f32,
    pub spin: f32,
    pub life_ms: u64,
    pub shape: ParticleShape,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SampledConfettiParticle {
    pub position: Point,
    pub size: f32,
    pub opacity: f32,
    pub rotation: f32,
    pub shape: ParticleShape,
}

pub fn ambient_particles(
    quantity: usize,
    bounds: Rectangle,
    seed: u64,
    base_size: f32,
) -> Vec<AmbientParticle> {
    let quantity = quantity.max(1);
    let mut rng = StableRng::new(seed);
    let mut particles = Vec::with_capacity(quantity);

    for _ in 0..quantity {
        particles.push(AmbientParticle {
            origin: Point::new(
                rng.next_f32() * bounds.width,
                rng.next_f32() * bounds.height,
            ),
            drift: Vector::new((rng.next_f32() - 0.5) * 18.0, (rng.next_f32() - 0.5) * 18.0),
            size: base_size.max(0.2) + rng.next_f32() * base_size.max(0.4),
            alpha: 0.16 + rng.next_f32() * 0.56,
            magnetism: 0.3 + rng.next_f32() * 3.5,
        });
    }

    particles
}

pub fn sample_ambient_particle(
    particle: AmbientParticle,
    elapsed_secs: f32,
    pointer: Option<Point>,
    staticity: f32,
    ease: f32,
    velocity: Vector,
    bounds: Rectangle,
) -> SampledParticle {
    let travel = Vector::new(
        particle.drift.x * elapsed_secs + velocity.x * elapsed_secs,
        particle.drift.y * elapsed_secs + velocity.y * elapsed_secs,
    );
    let wobble = Vector::new(
        (elapsed_secs * (0.7 + particle.magnetism * 0.12)).sin() * particle.size * 12.0,
        (elapsed_secs * (0.9 + particle.magnetism * 0.18)).cos() * particle.size * 12.0,
    );
    let pointer_push = pointer.map_or(Vector::default(), |pointer| {
        let dx = (pointer.x - bounds.width * 0.5) / staticity.max(1.0);
        let dy = (pointer.y - bounds.height * 0.5) / staticity.max(1.0);
        Vector::new(
            dx * particle.magnetism / ease.max(1.0),
            dy * particle.magnetism / ease.max(1.0),
        )
    });

    let position = Point::new(
        wrap_coordinate(
            bounds.width,
            particle.origin.x + travel.x + wobble.x + pointer_push.x,
        ),
        wrap_coordinate(
            bounds.height,
            particle.origin.y + travel.y + wobble.y + pointer_push.y,
        ),
    );

    SampledParticle {
        position,
        size: particle.size,
        opacity: particle.alpha.clamp(0.0, 1.0),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn confetti_particles(
    quantity: usize,
    origin: Point,
    seed: u64,
    angle_deg: f32,
    spread_deg: f32,
    start_velocity: f32,
    scalar: f32,
    shapes: &[ParticleShape],
) -> Vec<ConfettiParticle> {
    let quantity = quantity.max(1);
    let scalar = scalar.max(0.1);
    let mut rng = StableRng::new(seed);
    let mut particles = Vec::with_capacity(quantity);
    let shapes = if shapes.is_empty() {
        &[
            ParticleShape::Square,
            ParticleShape::Circle,
            ParticleShape::Star,
        ][..]
    } else {
        shapes
    };

    for index in 0..quantity {
        let direction = angle_deg - spread_deg * 0.5 + rng.next_f32() * spread_deg.max(0.0);
        let radians = direction.to_radians();
        let speed = start_velocity.max(1.0) * (0.65 + rng.next_f32() * 0.8);
        particles.push(ConfettiParticle {
            origin,
            velocity: Vector::new(radians.cos() * speed, -radians.sin() * speed),
            size: scalar * (4.0 + rng.next_f32() * 8.0),
            rotation: rng.next_f32() * std::f32::consts::TAU,
            spin: (rng.next_f32() - 0.5) * 8.0,
            life_ms: (1400.0 + rng.next_f32() * 1600.0) as u64,
            shape: shapes[index % shapes.len()],
        });
    }

    particles
}

pub fn sample_confetti_particle(
    particle: ConfettiParticle,
    elapsed_ms: u64,
    gravity: f32,
    decay: f32,
    drift: f32,
) -> Option<SampledConfettiParticle> {
    if elapsed_ms >= particle.life_ms {
        return None;
    }

    let t = elapsed_ms as f32 / 1000.0;
    let decay = decay.clamp(0.0, 1.0);
    let drag = 1.0 - decay.powf(t.max(0.0));
    let position = Point::new(
        particle.origin.x + particle.velocity.x * t + drift * t * t * 12.0,
        particle.origin.y + particle.velocity.y * t + gravity * 60.0 * t * t + drag * 12.0,
    );
    let life = 1.0 - elapsed_ms as f32 / particle.life_ms.max(1) as f32;

    Some(SampledConfettiParticle {
        position,
        size: particle.size,
        opacity: life.clamp(0.0, 1.0),
        rotation: particle.rotation + particle.spin * t,
        shape: particle.shape,
    })
}

fn wrap_coordinate(limit: f32, value: f32) -> f32 {
    if limit <= 0.0 {
        return value;
    }

    value.rem_euclid(limit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ambient_particles_are_repeatable() {
        let bounds = Rectangle::with_size(iced::Size::new(320.0, 180.0));
        let first = ambient_particles(8, bounds, 42, 0.5);
        let second = ambient_particles(8, bounds, 42, 0.5);

        assert_eq!(first, second);
    }

    #[test]
    fn confetti_particle_expires_after_life() {
        let particle = confetti_particles(
            1,
            Point::new(10.0, 20.0),
            7,
            90.0,
            10.0,
            45.0,
            1.0,
            &[ParticleShape::Square],
        )[0];

        assert!(sample_confetti_particle(particle, particle.life_ms + 1, 1.0, 0.9, 0.0).is_none());
    }
}
