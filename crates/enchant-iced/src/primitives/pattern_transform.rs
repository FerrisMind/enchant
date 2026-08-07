use iced::animation::Animation;
use iced::time::Instant;
use iced::{Point, Rectangle};
use std::f32::consts::PI;
use std::time::Duration;
use twill::backends::iced::to_easing;
use twill::prelude::Easing;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatternSkewAxis {
    X,
    Y,
}

#[derive(Clone, Copy, Debug)]
pub struct PatternTransform {
    pub offset_y: f32,
    pub origin: Point,
    pub skew_axis: PatternSkewAxis,
    pub skew_tan: f32,
}

impl PatternTransform {
    pub fn skew_x(degrees: f32, bounds_width: f32, pattern_height: f32, offset_y: f32) -> Self {
        Self::new(
            PatternSkewAxis::X,
            degrees,
            bounds_width,
            pattern_height,
            offset_y,
        )
    }

    pub fn new(
        skew_axis: PatternSkewAxis,
        degrees: f32,
        bounds_width: f32,
        pattern_height: f32,
        offset_y: f32,
    ) -> Self {
        Self {
            offset_y,
            origin: Point::new(bounds_width / 2.0, pattern_height * 0.5 + offset_y),
            skew_axis,
            skew_tan: (degrees * PI / 180.0).tan(),
        }
    }

    pub fn map_point(&self, point: Point) -> Point {
        let x = point.x;
        let y = point.y + self.offset_y;

        match self.skew_axis {
            PatternSkewAxis::X => Point::new(
                x,
                self.origin.y + (y - self.origin.y) + self.skew_tan * (x - self.origin.x),
            ),
            PatternSkewAxis::Y => Point::new(
                self.origin.x + (x - self.origin.x) + self.skew_tan * (y - self.origin.y),
                y,
            ),
        }
    }

    pub fn map_rect(&self, rect: Rectangle) -> [Point; 4] {
        [
            self.map_point(rect.position()),
            self.map_point(Point::new(rect.x + rect.width, rect.y)),
            self.map_point(Point::new(rect.x + rect.width, rect.y + rect.height)),
            self.map_point(Point::new(rect.x, rect.y + rect.height)),
        ]
    }
}

pub fn sample_easing(easing: Easing, progress: f32) -> f32 {
    let progress = progress.clamp(0.0, 1.0);
    let start = Instant::now();
    let mut animation = Animation::new(0.0)
        .duration(Duration::from_secs(1))
        .easing(to_easing(easing));
    animation.go_mut(1.0, start);
    animation.interpolate_with(|value| value, start + Duration::from_secs_f32(progress))
}

pub fn pulse_opacity(cycle_time: f32, duration: f32, easing: Easing) -> f32 {
    let active = duration * 2.0;
    if cycle_time > active {
        return 0.0;
    }

    if cycle_time <= duration {
        sample_easing(easing, cycle_time / duration)
    } else {
        sample_easing(easing, 1.0 - (cycle_time - duration) / duration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skew_x_keeps_vertical_lines_vertical() {
        let transform = PatternTransform::skew_x(12.0, 400.0, 800.0, -120.0);
        let top = transform.map_point(Point::new(100.0, 40.0));
        let bottom = transform.map_point(Point::new(100.0, 120.0));

        assert!((top.x - bottom.x).abs() < f32::EPSILON);
        assert!(top.y < bottom.y);
    }

    #[test]
    fn skew_x_tilts_horizontal_lines() {
        let transform = PatternTransform::skew_x(12.0, 400.0, 800.0, -120.0);
        let left = transform.map_point(Point::new(40.0, 80.0));
        let right = transform.map_point(Point::new(120.0, 80.0));

        assert!((right.y - left.y).abs() > 1.0);
    }

    #[test]
    fn pulse_opacity_fades_in_and_out() {
        assert_eq!(pulse_opacity(0.0, 3.0, Easing::Linear), 0.0);
        assert!(pulse_opacity(3.0, 3.0, Easing::Linear) > 0.99);
        assert_eq!(pulse_opacity(6.0, 3.0, Easing::Linear), 0.0);
    }
}
