use iced::{Point, Rectangle};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpherePoint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl SpherePoint {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectedSpherePoint {
    pub position: Point,
    pub scale: f32,
    pub opacity: f32,
    pub depth: f32,
}

pub fn fibonacci_sphere(count: usize, radius: f32) -> Vec<SpherePoint> {
    let count = count.max(1);
    let radius = radius.max(1.0);
    let offset = 2.0 / count as f32;
    let increment = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
    let mut points = Vec::with_capacity(count);

    for index in 0..count {
        let y = index as f32 * offset - 1.0 + offset * 0.5;
        let ring = (1.0 - y * y).max(0.0).sqrt();
        let phi = index as f32 * increment;

        points.push(SpherePoint::new(
            phi.cos() * ring * radius,
            y * radius,
            phi.sin() * ring * radius,
        ));
    }

    points
}

pub fn lat_lng_to_sphere(lat_deg: f32, lng_deg: f32, radius: f32) -> SpherePoint {
    let lat = lat_deg.to_radians();
    let lng = lng_deg.to_radians();
    let radius = radius.max(1.0);
    let cos_lat = lat.cos();

    SpherePoint::new(
        radius * cos_lat * lng.sin(),
        radius * lat.sin(),
        radius * cos_lat * lng.cos(),
    )
}

pub fn rotate_sphere_point(point: SpherePoint, rotation_x: f32, rotation_y: f32) -> SpherePoint {
    let cos_x = rotation_x.cos();
    let sin_x = rotation_x.sin();
    let cos_y = rotation_y.cos();
    let sin_y = rotation_y.sin();

    let y = point.y * cos_x - point.z * sin_x;
    let z_after_x = point.y * sin_x + point.z * cos_x;
    let x = point.x * cos_y - z_after_x * sin_y;
    let z = point.x * sin_y + z_after_x * cos_y;

    SpherePoint::new(x, y, z)
}

pub fn project_sphere_point(
    point: SpherePoint,
    bounds: Rectangle,
    camera_distance: f32,
) -> ProjectedSpherePoint {
    let camera_distance = camera_distance.max(1.0);
    let depth = (point.z + camera_distance) / (camera_distance * 2.0);
    let scale = ((point.z + camera_distance) / (camera_distance * 1.6)).clamp(0.2, 2.4);
    let opacity = depth.clamp(0.18, 1.0);

    ProjectedSpherePoint {
        position: Point::new(
            bounds.x + bounds.width * 0.5 + point.x,
            bounds.y + bounds.height * 0.5 + point.y,
        ),
        scale,
        opacity,
        depth,
    }
}

pub fn focus_rotation(point: SpherePoint) -> (f32, f32) {
    let x = point.x;
    let y = point.y;
    let z = point.z;
    let rotation_x = -y.atan2((x * x + z * z).sqrt());
    let rotation_y = x.atan2(z);

    (rotation_x, rotation_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fibonacci_sphere_produces_requested_amount() {
        let points = fibonacci_sphere(16, 40.0);

        assert_eq!(points.len(), 16);
    }

    #[test]
    fn lat_lng_projection_places_equator_on_horizontal_ring() {
        let point = lat_lng_to_sphere(0.0, 90.0, 100.0);

        assert!(point.x > 99.0);
        assert!(point.y.abs() < 0.001);
    }

    #[test]
    fn focus_rotation_keeps_front_facing_point_stable() {
        let point = SpherePoint::new(0.0, 0.0, 100.0);
        let rotation = focus_rotation(point);

        assert!(rotation.0.abs() < 0.001);
        assert!(rotation.1.abs() < 0.001);
    }

    #[test]
    fn project_sphere_point_includes_bounds_origin() {
        let projected = project_sphere_point(
            SpherePoint::new(10.0, -5.0, 0.0),
            Rectangle::new(iced::Point::new(25.0, 40.0), iced::Size::new(200.0, 100.0)),
            100.0,
        );

        assert_eq!(projected.position, Point::new(135.0, 85.0));
    }
}
