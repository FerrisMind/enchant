use iced::Point;
use iced::Radians;
use iced::Size;
use iced::Vector;
use iced::widget::canvas;
use std::f32::consts::{FRAC_PI_2, PI};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RoundedRectTrack {
    width: f32,
    height: f32,
    radius: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RoundedRectSample {
    pub point: Point,
    pub tangent: Vector,
}

impl RoundedRectTrack {
    pub(crate) fn new(size: Size, radius: f32) -> Self {
        let width = size.width.max(1.0);
        let height = size.height.max(1.0);
        let radius = radius.max(0.0).min(width.min(height) / 2.0);

        Self {
            width,
            height,
            radius,
        }
    }

    pub(crate) fn perimeter(&self) -> f32 {
        let straight = 2.0 * (self.width + self.height - 4.0 * self.radius).max(0.0);
        let arcs = 2.0 * PI * self.radius;
        straight + arcs
    }

    pub(crate) fn sample(&self, progress: f32) -> RoundedRectSample {
        let progress = progress.rem_euclid(1.0);
        let distance = progress * self.perimeter();

        let top = (self.width - 2.0 * self.radius).max(0.0);
        let side = (self.height - 2.0 * self.radius).max(0.0);
        let arc = FRAC_PI_2 * self.radius;

        let segments = [top, arc, side, arc, top, arc, side, arc];

        let mut remaining = distance;
        for (index, segment) in segments.into_iter().enumerate() {
            if remaining <= segment || index == segments.len() - 1 {
                return self.sample_segment(index, remaining.min(segment.max(0.0)));
            }
            remaining -= segment;
        }

        self.sample_segment(0, 0.0)
    }

    pub(crate) fn path(&self, origin: Point) -> canvas::Path {
        let r = self.radius;
        let w = self.width;
        let h = self.height;

        canvas::Path::new(|builder| {
            builder.move_to(Point::new(origin.x + r, origin.y));
            builder.line_to(Point::new(origin.x + w - r, origin.y));
            builder.arc(canvas::path::Arc {
                center: Point::new(origin.x + w - r, origin.y + r),
                radius: r,
                start_angle: Radians(-FRAC_PI_2),
                end_angle: Radians(0.0),
            });
            builder.line_to(Point::new(origin.x + w, origin.y + h - r));
            builder.arc(canvas::path::Arc {
                center: Point::new(origin.x + w - r, origin.y + h - r),
                radius: r,
                start_angle: Radians(0.0),
                end_angle: Radians(FRAC_PI_2),
            });
            builder.line_to(Point::new(origin.x + r, origin.y + h));
            builder.arc(canvas::path::Arc {
                center: Point::new(origin.x + r, origin.y + h - r),
                radius: r,
                start_angle: Radians(FRAC_PI_2),
                end_angle: Radians(PI),
            });
            builder.line_to(Point::new(origin.x, origin.y + r));
            builder.arc(canvas::path::Arc {
                center: Point::new(origin.x + r, origin.y + r),
                radius: r,
                start_angle: Radians(PI),
                end_angle: Radians(PI + FRAC_PI_2),
            });
            builder.close();
        })
    }

    fn sample_segment(&self, index: usize, distance: f32) -> RoundedRectSample {
        let r = self.radius;
        let w = self.width;
        let h = self.height;

        match index {
            0 => RoundedRectSample {
                point: Point::new(r + distance, 0.0),
                tangent: Vector::new(1.0, 0.0),
            },
            1 => self.sample_arc(Point::new(w - r, r), -FRAC_PI_2, distance),
            2 => RoundedRectSample {
                point: Point::new(w, r + distance),
                tangent: Vector::new(0.0, 1.0),
            },
            3 => self.sample_arc(Point::new(w - r, h - r), 0.0, distance),
            4 => RoundedRectSample {
                point: Point::new(w - r - distance, h),
                tangent: Vector::new(-1.0, 0.0),
            },
            5 => self.sample_arc(Point::new(r, h - r), FRAC_PI_2, distance),
            6 => RoundedRectSample {
                point: Point::new(0.0, h - r - distance),
                tangent: Vector::new(0.0, -1.0),
            },
            _ => self.sample_arc(Point::new(r, r), PI, distance),
        }
    }

    fn sample_arc(&self, center: Point, start_angle: f32, distance: f32) -> RoundedRectSample {
        if self.radius <= f32::EPSILON {
            return RoundedRectSample {
                point: center,
                tangent: Vector::new(1.0, 0.0),
            };
        }

        let angle = start_angle + (distance / self.radius).clamp(0.0, FRAC_PI_2);
        RoundedRectSample {
            point: Point::new(
                center.x + self.radius * angle.cos(),
                center.y + self.radius * angle.sin(),
            ),
            tangent: Vector::new(-angle.sin(), angle.cos()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(left: f32, right: f32) {
        assert!(
            (left - right).abs() < 0.01,
            "expected {left} ~= {right}, delta={}",
            (left - right).abs()
        );
    }

    #[test]
    fn perimeter_matches_rounded_rect_formula() {
        let track = RoundedRectTrack::new(Size::new(120.0, 80.0), 12.0);
        let expected = 2.0 * (120.0 + 80.0 - 48.0) + 2.0 * PI * 12.0;
        approx_eq(track.perimeter(), expected);
    }

    #[test]
    fn sample_starts_on_top_edge_and_turns_clockwise() {
        let track = RoundedRectTrack::new(Size::new(100.0, 60.0), 10.0);
        let start = track.sample(0.0);
        let quarter = track.sample(0.25);
        let half = track.sample(0.5);

        approx_eq(start.point.x, 10.0);
        approx_eq(start.point.y, 0.0);
        assert!(quarter.point.x > start.point.x);
        assert!(quarter.point.y >= 0.0);
        assert!(half.point.y >= 50.0);
    }
}
