use iced::{Point, Vector};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrbitPoint {
    pub angle_deg: f32,
    pub position: Point,
}

pub fn circle_point(center: Point, radius: f32, angle_deg: f32) -> Point {
    let radians = angle_deg.to_radians();

    Point::new(
        center.x + radians.cos() * radius,
        center.y + radians.sin() * radius,
    )
}

pub fn orbit_points(
    center: Point,
    radius: f32,
    count: usize,
    angle_offset_deg: f32,
) -> Vec<OrbitPoint> {
    if count == 0 {
        return Vec::new();
    }

    let step = 360.0 / count as f32;
    let mut points = Vec::with_capacity(count);

    for index in 0..count {
        let angle_deg = angle_offset_deg + step * index as f32;
        points.push(OrbitPoint {
            angle_deg,
            position: circle_point(center, radius, angle_deg),
        });
    }

    points
}

pub fn orbit_translation(radius: f32, angle_deg: f32) -> Vector {
    let point = circle_point(Point::ORIGIN, radius, angle_deg);
    Vector::new(point.x, point.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circle_point_matches_cardinal_angles() {
        let center = Point::new(10.0, 20.0);

        assert_eq!(circle_point(center, 5.0, 0.0), Point::new(15.0, 20.0));
        assert_eq!(circle_point(center, 5.0, 90.0), Point::new(10.0, 25.0));
    }

    #[test]
    fn orbit_points_spread_evenly() {
        let points = orbit_points(Point::ORIGIN, 10.0, 4, 0.0);

        assert_eq!(points.len(), 4);
        assert_eq!(points[0].angle_deg, 0.0);
        assert_eq!(points[1].angle_deg, 90.0);
        assert_eq!(points[2].angle_deg, 180.0);
        assert_eq!(points[3].angle_deg, 270.0);
    }
}
