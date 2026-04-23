use iced::Point;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeoPoint {
    pub lat: f32,
    pub lng: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct GeoRegion {
    lat_min: f32,
    lat_max: f32,
    lng_min: f32,
    lng_max: f32,
}

const WORLD_REGIONS: [GeoRegion; 10] = [
    GeoRegion {
        lat_min: 7.0,
        lat_max: 72.0,
        lng_min: -168.0,
        lng_max: -52.0,
    },
    GeoRegion {
        lat_min: -56.0,
        lat_max: 13.0,
        lng_min: -82.0,
        lng_max: -35.0,
    },
    GeoRegion {
        lat_min: 35.0,
        lat_max: 72.0,
        lng_min: -12.0,
        lng_max: 42.0,
    },
    GeoRegion {
        lat_min: -35.0,
        lat_max: 37.0,
        lng_min: -20.0,
        lng_max: 52.0,
    },
    GeoRegion {
        lat_min: 5.0,
        lat_max: 77.0,
        lng_min: 42.0,
        lng_max: 179.0,
    },
    GeoRegion {
        lat_min: -48.0,
        lat_max: -8.0,
        lng_min: 112.0,
        lng_max: 179.0,
    },
    GeoRegion {
        lat_min: 60.0,
        lat_max: 83.0,
        lng_min: -74.0,
        lng_max: -10.0,
    },
    GeoRegion {
        lat_min: 12.0,
        lat_max: 28.0,
        lng_min: -101.0,
        lng_max: -76.0,
    },
    GeoRegion {
        lat_min: 20.0,
        lat_max: 48.0,
        lng_min: 122.0,
        lng_max: 146.0,
    },
    GeoRegion {
        lat_min: -10.0,
        lat_max: 10.0,
        lng_min: 95.0,
        lng_max: 141.0,
    },
];

pub fn world_point_cloud(map_samples: usize) -> Vec<GeoPoint> {
    if map_samples == 0 {
        return Vec::new();
    }

    let per_region = map_samples.div_ceil(WORLD_REGIONS.len()).max(1);
    let lat_steps = (per_region as f32).sqrt().ceil() as usize;
    let lng_steps = lat_steps.max(2) * 2;
    let mut points = Vec::with_capacity(WORLD_REGIONS.len() * lat_steps * lng_steps);

    for region in WORLD_REGIONS {
        for lat_index in 0..lat_steps {
            for lng_index in 0..lng_steps {
                let lat = lerp(
                    region.lat_min,
                    region.lat_max,
                    lat_index as f32 / lat_steps.saturating_sub(1).max(1) as f32,
                );
                let lng = lerp(
                    region.lng_min,
                    region.lng_max,
                    lng_index as f32 / lng_steps.saturating_sub(1).max(1) as f32,
                );

                if should_keep_region_point(region, lat, lng, lat_index, lng_index) {
                    points.push(GeoPoint { lat, lng });
                }
            }
        }
    }

    points
}

pub fn project_equirectangular(lat: f32, lng: f32, width: f32, height: f32) -> Point {
    if width <= 0.0 || height <= 0.0 {
        return Point::ORIGIN;
    }

    Point::new(
        ((lng + 180.0) / 360.0).clamp(0.0, 1.0) * width,
        ((90.0 - lat) / 180.0).clamp(0.0, 1.0) * height,
    )
}

pub fn stagger_offset(row_index: usize, x_step: f32, stagger: bool) -> f32 {
    if stagger && row_index % 2 == 1 {
        x_step * 0.5
    } else {
        0.0
    }
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}

fn should_keep_region_point(
    region: GeoRegion,
    lat: f32,
    lng: f32,
    lat_index: usize,
    lng_index: usize,
) -> bool {
    let edge = lat_index == 0
        || lng_index == 0
        || lat_index.is_multiple_of(3)
        || lng_index.is_multiple_of(3);
    if edge {
        return true;
    }

    match region {
        GeoRegion {
            lng_min: -168.0, ..
        } => lat > 16.0 || lng < -90.0,
        GeoRegion { lng_min: -82.0, .. } => lat < 5.0 || lng < -50.0,
        GeoRegion {
            lng_min: -12.0,
            lat_min: 35.0,
            ..
        } => lat > 42.0 || lng < 26.0,
        GeoRegion {
            lng_min: -20.0,
            lat_min: -35.0,
            ..
        } => lat < 25.0 || lng > 10.0,
        GeoRegion { lng_min: 42.0, .. } => lat > 16.0 || lng > 70.0,
        GeoRegion { lat_min: -48.0, .. } => lat < -18.0 || lng > 130.0,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_point_cloud_is_stable_and_non_empty() {
        let points = world_point_cloud(256);

        assert!(points.len() > 128);
        assert_eq!(points, world_point_cloud(256));
    }

    #[test]
    fn world_point_cloud_returns_empty_for_zero_samples() {
        assert!(world_point_cloud(0).is_empty());
    }

    #[test]
    fn world_point_cloud_respects_low_sample_counts() {
        assert!(world_point_cloud(8).len() < world_point_cloud(64).len());
    }

    #[test]
    fn project_equirectangular_maps_equator_center() {
        let point = project_equirectangular(0.0, 0.0, 200.0, 100.0);

        assert!((point.x - 100.0).abs() < 0.001);
        assert!((point.y - 50.0).abs() < 0.001);
    }

    #[test]
    fn project_equirectangular_returns_origin_for_zero_bounds() {
        assert_eq!(project_equirectangular(0.0, 0.0, 0.0, 100.0), Point::ORIGIN);
        assert_eq!(project_equirectangular(0.0, 0.0, 200.0, 0.0), Point::ORIGIN);
    }
}
