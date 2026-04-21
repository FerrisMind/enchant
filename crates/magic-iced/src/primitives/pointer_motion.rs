use iced::time::Instant;
use iced::{Point, Vector};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerSample {
    pub position: Point,
    pub velocity: Vector,
    pub speed: f32,
    pub angle_deg: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerMotion {
    current: Option<Point>,
    previous: Option<Point>,
    velocity: Vector,
    speed: f32,
    angle_deg: f32,
    last_updated_at: Option<Instant>,
}

impl Default for PointerMotion {
    fn default() -> Self {
        Self {
            current: None,
            previous: None,
            velocity: Vector::ZERO,
            speed: 0.0,
            angle_deg: 90.0,
            last_updated_at: None,
        }
    }
}

impl PointerMotion {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn update(&mut self, position: Point, now: Instant) -> PointerSample {
        let delta_secs = self
            .last_updated_at
            .map(|last| now.saturating_duration_since(last).as_secs_f32())
            .unwrap_or(0.0);

        self.previous = self.current;
        self.current = Some(position);
        self.last_updated_at = Some(now);

        self.velocity = if let Some(previous) = self.previous {
            pointer_velocity(previous, position, delta_secs)
        } else {
            Vector::ZERO
        };
        self.speed = pointer_speed(self.velocity);

        if self.speed > 0.01 {
            self.angle_deg = pointer_angle_deg(self.velocity);
        }

        self.sample()
            .expect("pointer sample should exist after update")
    }

    pub fn sample(&self) -> Option<PointerSample> {
        Some(PointerSample {
            position: self.current?,
            velocity: self.velocity,
            speed: self.speed,
            angle_deg: self.angle_deg,
        })
    }

    pub fn position(&self) -> Option<Point> {
        self.current
    }

    pub fn velocity(&self) -> Vector {
        self.velocity
    }

    pub fn angle_deg(&self) -> f32 {
        self.angle_deg
    }
}

pub fn pointer_velocity(from: Point, to: Point, delta_secs: f32) -> Vector {
    if delta_secs <= f32::EPSILON {
        return Vector::ZERO;
    }

    Vector::new((to.x - from.x) / delta_secs, (to.y - from.y) / delta_secs)
}

pub fn pointer_speed(velocity: Vector) -> f32 {
    (velocity.x.powi(2) + velocity.y.powi(2)).sqrt()
}

pub fn pointer_angle_deg(velocity: Vector) -> f32 {
    velocity.y.atan2(velocity.x).to_degrees() + 90.0
}

pub fn pointer_distance(main_axis: f32, center_axis: f32) -> f32 {
    (main_axis - center_axis).abs()
}

pub fn pointer_strength(distance: f32, radius: f32) -> f32 {
    if radius <= f32::EPSILON {
        return 0.0;
    }

    (1.0 - distance / radius).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::time::Duration;

    #[test]
    fn pointer_motion_computes_velocity_and_angle() {
        let mut motion = PointerMotion::new();
        let start = Instant::now();
        motion.update(Point::new(10.0, 10.0), start);
        let sample = motion.update(Point::new(20.0, 10.0), start + Duration::from_millis(100));

        assert!((sample.velocity.x - 100.0).abs() < 0.01);
        assert!(sample.velocity.y.abs() < 0.01);
        assert!((sample.angle_deg - 90.0).abs() < 0.01);
    }

    #[test]
    fn pointer_strength_clamps() {
        assert_eq!(pointer_strength(0.0, 100.0), 1.0);
        assert_eq!(pointer_strength(100.0, 100.0), 0.0);
        assert_eq!(pointer_strength(300.0, 100.0), 0.0);
    }
}
