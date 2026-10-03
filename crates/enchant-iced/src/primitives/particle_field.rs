use crate::primitives::StableRng;
use iced::{Point, Rectangle, Vector};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AmbientParticle {
    pub position: Point,
    pub translate: Vector,
    pub size: f32,
    pub alpha: f32,
    pub target_alpha: f32,
    pub dx: f32,
    pub dy: f32,
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
    if quantity == 0 {
        return Vec::new();
    }

    let mut rng = StableRng::new(seed);
    let mut particles = Vec::with_capacity(quantity);
    let base_size = base_size.max(0.0);

    for _ in 0..quantity {
        particles.push(AmbientParticle {
            position: Point::new(
                (rng.next_f32() * bounds.width.max(0.0)).floor(),
                (rng.next_f32() * bounds.height.max(0.0)).floor(),
            ),
            translate: Vector::ZERO,
            size: (rng.next_f32() * 2.0).floor() + base_size,
            alpha: 0.0,
            target_alpha: ((rng.next_f32() * 0.6 + 0.1) * 10.0).round() / 10.0,
            dx: (rng.next_f32() - 0.5) * 0.1,
            dy: (rng.next_f32() - 0.5) * 0.1,
            magnetism: 0.1 + rng.next_f32() * 4.0,
        });
    }

    particles
}

pub fn sample_ambient_particle(
    particle: &mut AmbientParticle,
    frame_factor: f32,
    pointer: Option<Point>,
    staticity: f32,
    ease: f32,
    velocity: Vector,
    bounds: Rectangle,
) -> SampledParticle {
    let frame_factor = frame_factor.max(0.0);
    let pointer_push = pointer.map_or(Vector::ZERO, |pointer| {
        let centered = Point::new(
            pointer.x - bounds.width * 0.5,
            pointer.y - bounds.height * 0.5,
        );
        let staticity = staticity.max(1.0);
        let ease = ease.max(1.0);
        Vector::new(
            (centered.x / (staticity / particle.magnetism) - particle.translate.x) / ease,
            (centered.y / (staticity / particle.magnetism) - particle.translate.y) / ease,
        )
    });

    particle.position.x += (particle.dx + velocity.x) * frame_factor;
    particle.position.y += (particle.dy + velocity.y) * frame_factor;
    particle.translate.x += pointer_push.x * frame_factor;
    particle.translate.y += pointer_push.y * frame_factor;

    let closest_edge = [
        particle.position.x + particle.translate.x - particle.size,
        bounds.width - particle.position.x - particle.translate.x - particle.size,
        particle.position.y + particle.translate.y - particle.size,
        bounds.height - particle.position.y - particle.translate.y - particle.size,
    ]
    .into_iter()
    .fold(f32::INFINITY, f32::min);

    let edge_alpha = remap_value(closest_edge, 0.0, 20.0, 0.0, 1.0);
    if edge_alpha > 1.0 {
        particle.alpha = (particle.alpha + 0.02 * frame_factor).min(particle.target_alpha);
    } else {
        particle.alpha = particle.target_alpha * edge_alpha;
    }

    SampledParticle {
        position: Point::new(
            particle.position.x + particle.translate.x,
            particle.position.y + particle.translate.y,
        ),
        size: particle.size,
        opacity: particle.alpha.clamp(0.0, 1.0),
    }
}

pub(crate) fn ambient_particle_out_of_bounds(
    particle: &AmbientParticle,
    bounds: Rectangle,
) -> bool {
    let x = particle.position.x + particle.translate.x;
    let y = particle.position.y + particle.translate.y;

    x < -particle.size
        || x > bounds.width + particle.size
        || y < -particle.size
        || y > bounds.height + particle.size
}

/// canvas-confetti particle extent base (`10 * scalar` wobble offset).
pub const CONFETTI_SIZE_FACTOR: f32 = 10.0;
/// canvas-confetti circle ellipse scale (`ovalScalar`).
pub const CONFETTI_OVAL_SCALAR: f32 = 0.6;
/// canvas-confetti star outer radius factor (`8 * scalar`).
pub const CONFETTI_STAR_OUTER_FACTOR: f32 = 8.0;
/// canvas-confetti star inner radius factor (`4 * scalar`).
pub const CONFETTI_STAR_INNER_FACTOR: f32 = 4.0;
/// canvas-confetti applies `opts.gravity * 3` per tick.
pub const CONFETTI_GRAVITY_SCALE: f32 = 3.0;
/// Approximate canvas-confetti tick length at ~60 fps (ms).
pub const CONFETTI_MS_PER_TICK: f32 = 16.0;

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
    if quantity == 0 {
        return Vec::new();
    }

    let scalar = scalar.max(0.0);
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

    // canvas-confetti `randomPhysics`:
    //   velocity = startVelocity*0.5 + random()*startVelocity
    //   angle2D  = -radAngle + (0.5*radSpread - random()*radSpread)
    // Screen-space velocity uses cos(angle2D)/sin(angle2D); equivalent to
    // sampling θ = angle ± spread/2 then (cos θ, -sin θ).
    for index in 0..quantity {
        let direction = angle_deg - spread_deg * 0.5 + rng.next_f32() * spread_deg.max(0.0);
        let radians = direction.to_radians();
        let start_velocity = start_velocity.max(0.0);
        let speed = start_velocity * 0.5 + rng.next_f32() * start_velocity;
        particles.push(ConfettiParticle {
            origin,
            velocity: Vector::new(radians.cos() * speed, -radians.sin() * speed),
            // Deterministic visual extent matching wobble span `10 * scalar`.
            size: CONFETTI_SIZE_FACTOR * scalar,
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

    // Discrete tick integration matching canvas-confetti `updateFetti`:
    //   x += cos(angle)*velocity + drift
    //   y += sin(angle)*velocity + gravity*3
    //   velocity *= decay
    // Closed form after n ticks (velocity vector decays geometrically).
    let n = elapsed_ms as f32 / CONFETTI_MS_PER_TICK;
    let decay = decay.clamp(0.0, 1.0);
    let velocity_span = if (1.0 - decay).abs() < f32::EPSILON {
        n
    } else {
        (1.0 - decay.powf(n)) / (1.0 - decay)
    };
    let gravity_per_tick = gravity * CONFETTI_GRAVITY_SCALE;
    let position = Point::new(
        particle.origin.x + particle.velocity.x * velocity_span + drift * n,
        particle.origin.y + particle.velocity.y * velocity_span + gravity_per_tick * n,
    );
    // Progress fade: alpha = 1 - tick/totalTicks
    let life = 1.0 - elapsed_ms as f32 / particle.life_ms.max(1) as f32;
    let t = elapsed_ms as f32 / 1000.0;

    Some(SampledConfettiParticle {
        position,
        size: particle.size,
        opacity: life.clamp(0.0, 1.0),
        rotation: particle.rotation + particle.spin * t,
        shape: particle.shape,
    })
}

fn remap_value(value: f32, start1: f32, end1: f32, start2: f32, end2: f32) -> f32 {
    let remapped = ((value - start1) * (end2 - start2)) / (end1 - start1) + start2;
    remapped.max(0.0)
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
    fn ambient_particles_allow_zero_quantity_and_zero_size() {
        let bounds = Rectangle::with_size(iced::Size::new(320.0, 180.0));

        assert!(ambient_particles(0, bounds, 42, 0.5).is_empty());
        assert_eq!(ambient_particles(1, bounds, 42, 0.0).len(), 1);
    }

    #[test]
    fn ambient_particle_does_not_wrap_coordinates() {
        let mut particle = AmbientParticle {
            position: Point::new(318.0, 178.0),
            translate: Vector::ZERO,
            size: 1.0,
            alpha: 0.0,
            target_alpha: 0.5,
            dx: 5.0,
            dy: 3.0,
            magnetism: 1.0,
        };

        let sampled = sample_ambient_particle(
            &mut particle,
            1.0,
            None,
            50.0,
            50.0,
            Vector::ZERO,
            Rectangle::with_size(iced::Size::new(320.0, 180.0)),
        );

        assert!(sampled.position.x > 320.0 || sampled.position.y > 180.0);
    }

    #[test]
    fn confetti_particles_allow_zero_quantity_and_zero_scale() {
        let origin = Point::new(10.0, 20.0);

        assert!(
            confetti_particles(
                0,
                origin,
                7,
                90.0,
                10.0,
                45.0,
                1.0,
                &[ParticleShape::Square],
            )
            .is_empty()
        );

        let particle =
            confetti_particles(1, origin, 7, 90.0, 10.0, 0.0, 0.0, &[ParticleShape::Square])[0];

        assert_eq!(particle.size, 0.0);
        assert_eq!(particle.velocity.x, 0.0);
        assert_eq!(particle.velocity.y, 0.0);
    }

    #[test]
    fn confetti_size_matches_canvas_confetti_ten_times_scalar() {
        let origin = Point::new(0.0, 0.0);
        for scalar in [0.5_f32, 1.0, 1.2, 2.0] {
            let particle = confetti_particles(
                1,
                origin,
                11,
                90.0,
                0.0,
                45.0,
                scalar,
                &[ParticleShape::Square],
            )[0];
            assert!(
                (particle.size - CONFETTI_SIZE_FACTOR * scalar).abs() < f32::EPSILON,
                "scalar={scalar}: size={} expected={}",
                particle.size,
                CONFETTI_SIZE_FACTOR * scalar
            );
        }
    }

    #[test]
    fn confetti_velocity_matches_canvas_confetti_range() {
        let origin = Point::new(0.0, 0.0);
        let start_velocity = 45.0;
        let particles = confetti_particles(
            64,
            origin,
            99,
            90.0,
            0.0,
            start_velocity,
            1.0,
            &[ParticleShape::Square],
        );

        for particle in &particles {
            // angle 90° → velocity is (0, -speed); speed ∈ [0.5*sv, 1.5*sv]
            let speed = -particle.velocity.y;
            assert!(
                speed >= start_velocity * 0.5 - 1e-3 && speed <= start_velocity * 1.5 + 1e-3,
                "speed {speed} outside canvas-confetti range"
            );
            assert!(particle.velocity.x.abs() < 1e-3);
        }
    }

    #[test]
    fn confetti_gravity_uses_triple_scale_per_tick() {
        let particle = ConfettiParticle {
            origin: Point::new(10.0, 20.0),
            velocity: Vector::ZERO,
            size: 10.0,
            rotation: 0.0,
            spin: 0.0,
            life_ms: 5_000,
            shape: ParticleShape::Square,
        };

        let sample = sample_confetti_particle(particle, 160, 1.0, 0.9, 0.0).expect("still alive");
        // n = 160/16 = 10 ticks; Δy = gravity*3*n = 30
        assert!(
            (sample.position.y - 50.0).abs() < 1e-3,
            "expected y=50, got {}",
            sample.position.y
        );
        assert!((sample.opacity - (1.0 - 160.0 / 5_000.0)).abs() < 1e-5);
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
