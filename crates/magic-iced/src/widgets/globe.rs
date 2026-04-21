use crate::primitives::{
    EffectCanvasProps, EffectProgram, SpherePoint, effect_canvas, lat_lng_to_sphere,
    project_sphere_point, rotate_sphere_point, world_point_cloud,
};
use iced::mouse;
use iced::time::Instant;
use iced::widget::{canvas, mouse_area};
use iced::{Color, Element, Length, Point, Rectangle, Renderer};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlobeMarker {
    pub lat: f32,
    pub lng: f32,
    pub size: f32,
}

impl GlobeMarker {
    pub const fn new(lat: f32, lng: f32, size: f32) -> Self {
        Self { lat, lng, size }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GlobeAction {
    Move(Point),
    Press,
    Release,
    Exit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GlobeState {
    rotation_x: f32,
    rotation_y: f32,
    pointer: Option<Point>,
    dragging: bool,
    last_tick_at: Option<Instant>,
    world_points: Vec<SpherePoint>,
    map_samples: usize,
    radius: f32,
}

impl GlobeState {
    pub fn new(props: &GlobeProps) -> Self {
        Self {
            rotation_x: 0.0,
            rotation_y: 0.3,
            pointer: None,
            dragging: false,
            last_tick_at: None,
            world_points: build_world_points(props.map_samples, props.radius),
            map_samples: props.map_samples,
            radius: props.radius,
        }
    }

    pub fn apply_action(&mut self, action: &GlobeAction, props: &GlobeProps) {
        self.ensure_cache(props);

        match action {
            GlobeAction::Move(position) => {
                if let Some(last) = self.pointer
                    && self.dragging
                {
                    let delta_x = position.x - last.x;
                    let delta_y = position.y - last.y;
                    self.rotation_y += delta_x * props.drag_sensitivity;
                    self.rotation_x += delta_y * props.drag_sensitivity;
                }
                self.pointer = Some(*position);
            }
            GlobeAction::Press => self.dragging = true,
            GlobeAction::Release | GlobeAction::Exit => {
                self.dragging = false;
                if matches!(action, GlobeAction::Exit) {
                    self.pointer = None;
                }
            }
        }
    }

    pub fn tick(&mut self, now: Instant, props: &GlobeProps) {
        self.ensure_cache(props);
        let dt = self
            .last_tick_at
            .map(|last| now.saturating_duration_since(last).as_secs_f32())
            .unwrap_or(0.0);
        self.last_tick_at = Some(now);

        if !self.dragging {
            self.rotation_y += props.auto_rotate_speed * dt;
        }
    }

    fn ensure_cache(&mut self, props: &GlobeProps) {
        if self.map_samples != props.map_samples
            || (self.radius - props.radius).abs() > f32::EPSILON
        {
            self.world_points = build_world_points(props.map_samples, props.radius);
            self.map_samples = props.map_samples;
            self.radius = props.radius;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GlobeProps {
    pub width: Length,
    pub height: Length,
    pub radius: f32,
    pub camera_distance: f32,
    pub auto_rotate_speed: f32,
    pub drag_sensitivity: f32,
    pub map_samples: usize,
    pub base_color: Option<Color>,
    pub marker_color: Option<Color>,
    pub glow_color: Option<Color>,
    pub graticule_color: Option<Color>,
    pub markers: Vec<GlobeMarker>,
}

impl Default for GlobeProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            height: Length::Fixed(360.0),
            radius: 118.0,
            camera_distance: 220.0,
            auto_rotate_speed: 0.55,
            drag_sensitivity: 0.01,
            map_samples: 640,
            base_color: None,
            marker_color: None,
            glow_color: None,
            graticule_color: None,
            markers: vec![
                GlobeMarker::new(14.5995, 120.9842, 3.0),
                GlobeMarker::new(19.0760, 72.8777, 5.0),
                GlobeMarker::new(30.0444, 31.2357, 3.8),
                GlobeMarker::new(39.9042, 116.4074, 4.2),
                GlobeMarker::new(40.7128, -74.0060, 5.0),
            ],
        }
    }
}

impl GlobeProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn globe<'a, Message: Clone + 'a>(
    state: &'a GlobeState,
    props: &'a GlobeProps,
    on_action: impl Fn(GlobeAction) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let canvas: Element<'a, Message> = effect_canvas::<_, Message>(
        GlobeProgram { state, props },
        EffectCanvasProps::new()
            .width(props.width)
            .height(props.height),
    )
    .into();

    mouse_area(canvas)
        .on_move({
            let on_action = on_action.clone();
            move |position| on_action(GlobeAction::Move(position))
        })
        .on_press(on_action(GlobeAction::Press))
        .on_release(on_action(GlobeAction::Release))
        .on_exit(on_action(GlobeAction::Exit))
        .interaction(mouse::Interaction::Grab)
        .into()
}

struct GlobeProgram<'a> {
    state: &'a GlobeState,
    props: &'a GlobeProps,
}

impl<Message> EffectProgram<Message> for GlobeProgram<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let center = Point::new(bounds.width * 0.5, bounds.height * 0.5);
        let base = self
            .props
            .base_color
            .unwrap_or(Color::from_rgb8(0xF8, 0xFA, 0xFC));
        let marker = self
            .props
            .marker_color
            .unwrap_or(Color::from_rgb8(0xFB, 0x64, 0x15));
        let glow = self
            .props
            .glow_color
            .unwrap_or(Color::from_rgba8(170, 220, 255, 0.24));
        let graticule = self
            .props
            .graticule_color
            .unwrap_or(Color::from_rgba8(255, 255, 255, 0.16));

        for factor in [1.18_f32, 1.08, 1.0] {
            frame.fill(
                &canvas::Path::circle(center, self.props.radius * factor),
                Color {
                    a: glow.a / factor,
                    ..glow
                },
            );
        }

        frame.fill(&canvas::Path::circle(center, self.props.radius), base);

        for index in 1..5 {
            let radius = self.props.radius * index as f32 / 5.0;
            frame.stroke(
                &canvas::Path::circle(center, radius),
                canvas::Stroke::default()
                    .with_color(graticule)
                    .with_width(1.0),
            );
        }

        let mut dots = Vec::with_capacity(self.state.world_points.len());
        for point in &self.state.world_points {
            let rotated = rotate_sphere_point(*point, self.state.rotation_x, self.state.rotation_y);
            if rotated.z < -self.props.radius * 0.92 {
                continue;
            }

            dots.push(project_sphere_point(
                rotated,
                bounds,
                self.props.camera_distance,
            ));
        }

        dots.sort_by(|left, right| left.depth.total_cmp(&right.depth));
        for dot in dots {
            frame.fill(
                &canvas::Path::circle(dot.position, 1.6 * dot.scale),
                Color {
                    a: dot.opacity * 0.42,
                    ..graticule
                },
            );
        }

        let mut markers = self
            .props
            .markers
            .iter()
            .map(|marker_data| {
                let sphere =
                    lat_lng_to_sphere(marker_data.lat, marker_data.lng, self.props.radius * 0.98);
                let rotated =
                    rotate_sphere_point(sphere, self.state.rotation_x, self.state.rotation_y);
                let projected = project_sphere_point(rotated, bounds, self.props.camera_distance);
                (rotated.z, projected, marker_data.size)
            })
            .collect::<Vec<_>>();

        markers.sort_by(|left, right| left.0.total_cmp(&right.0));
        for (depth, projected, size) in markers {
            if depth < -self.props.radius * 0.75 {
                continue;
            }

            frame.fill(
                &canvas::Path::circle(projected.position, size * projected.scale),
                Color {
                    a: projected.opacity,
                    ..marker
                },
            );
        }

        vec![frame.into_geometry()]
    }
}

fn build_world_points(map_samples: usize, radius: f32) -> Vec<SpherePoint> {
    world_point_cloud(map_samples)
        .into_iter()
        .map(|point| lat_lng_to_sphere(point.lat, point.lng, radius * 0.98))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globe_defaults_match_docs() {
        let props = GlobeProps::new();

        assert_eq!(props.radius, 118.0);
        assert_eq!(props.map_samples, 640);
        assert!(!props.markers.is_empty());
    }

    #[test]
    fn globe_drag_updates_rotation() {
        let props = GlobeProps::new();
        let mut state = GlobeState::new(&props);

        state.apply_action(&GlobeAction::Move(Point::new(10.0, 10.0)), &props);
        state.apply_action(&GlobeAction::Press, &props);
        state.apply_action(&GlobeAction::Move(Point::new(30.0, 20.0)), &props);

        assert!(state.rotation_y > 0.3);
        assert!(state.rotation_x > 0.0);
    }
}
