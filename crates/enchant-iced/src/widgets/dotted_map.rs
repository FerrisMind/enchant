use crate::primitives::{
    EffectCanvasProps, EffectProgram, GeoPoint, effect_canvas, project_equirectangular,
    stagger_offset, world_point_cloud,
};
use iced::border;
use iced::time::Instant;
use iced::widget::{canvas, image};
use iced::{Color, Element, Length, Point, Rectangle, Renderer};

#[derive(Clone, Debug)]
pub enum DottedMapOverlay {
    Label {
        text: String,
        badge: Option<image::Handle>,
    },
}

#[derive(Clone, Debug)]
pub struct DottedMapMarker {
    pub lat: f32,
    pub lng: f32,
    pub size: f32,
    pub pulse: bool,
    pub overlay: Option<DottedMapOverlay>,
}

impl DottedMapMarker {
    pub fn new(lat: f32, lng: f32) -> Self {
        Self {
            lat,
            lng,
            size: 0.8,
            pulse: false,
            overlay: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DottedMapState {
    points: Vec<GeoPoint>,
    started_at: Option<Instant>,
    map_samples: usize,
}

impl DottedMapState {
    pub fn new(props: &DottedMapProps) -> Self {
        Self {
            points: world_point_cloud(props.map_samples),
            started_at: None,
            map_samples: props.map_samples,
        }
    }

    pub fn tick(&mut self, now: Instant, props: &DottedMapProps) {
        if self.map_samples != props.map_samples {
            self.points = world_point_cloud(props.map_samples);
            self.map_samples = props.map_samples;
        }
        let _ = self.started_at.get_or_insert(now);
    }

    fn elapsed_ms(&self, now: Instant) -> u64 {
        self.started_at
            .map(|started| now.saturating_duration_since(started).as_millis() as u64)
            .unwrap_or(0)
    }
}

#[derive(Clone, Debug)]
pub struct DottedMapProps {
    pub width: Length,
    pub height: Length,
    pub map_width: f32,
    pub map_height: f32,
    pub map_samples: usize,
    pub markers: Vec<DottedMapMarker>,
    pub dot_color: Option<Color>,
    pub marker_color: Color,
    pub dot_radius: f32,
    pub stagger: bool,
    pub pulse: bool,
}

impl Default for DottedMapProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            height: Length::Fixed(320.0),
            map_width: 150.0,
            map_height: 75.0,
            map_samples: 5000,
            markers: Vec::new(),
            dot_color: None,
            marker_color: Color::from_rgb8(0xFF, 0x69, 0x00),
            dot_radius: 0.2,
            stagger: true,
            pulse: false,
        }
    }
}

impl DottedMapProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn dotted_map<'a, Message: 'a>(
    state: &'a DottedMapState,
    now: Instant,
    props: &'a DottedMapProps,
) -> Element<'a, Message> {
    effect_canvas::<_, Message>(
        DottedMapProgram { state, now, props },
        EffectCanvasProps::new()
            .width(props.width)
            .height(props.height),
    )
    .into()
}

struct DottedMapProgram<'a> {
    state: &'a DottedMapState,
    now: Instant,
    props: &'a DottedMapProps,
}

impl<Message> EffectProgram<Message> for DottedMapProgram<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let dot_color = self
            .props
            .dot_color
            .unwrap_or(Color::from_rgb8(0x6B, 0x72, 0x80));
        let elapsed = self.state.elapsed_ms(self.now) as f32 / 1000.0;
        let row_height = (bounds.height / 28.0).max(2.0);
        let x_step = (bounds.width / 80.0).max(self.props.dot_radius * 4.0);

        for point in &self.state.points {
            let projected =
                project_equirectangular(point.lat, point.lng, bounds.width, bounds.height);
            let row_index = (projected.y / row_height).floor().max(0.0) as usize;
            let x = projected.x + stagger_offset(row_index, x_step, self.props.stagger);
            frame.fill(
                &canvas::Path::circle(Point::new(x, projected.y), self.props.dot_radius.max(0.1)),
                dot_color,
            );
        }

        for marker in &self.props.markers {
            let projected =
                project_equirectangular(marker.lat, marker.lng, bounds.width, bounds.height);
            let row_index = (projected.y / row_height).floor().max(0.0) as usize;
            let x = projected.x + stagger_offset(row_index, x_step, self.props.stagger);
            let y = projected.y;
            let radius = marker.size.max(self.props.dot_radius);

            frame.fill(
                &canvas::Path::circle(Point::new(x, y), radius),
                self.props.marker_color,
            );

            let should_pulse = if self.props.pulse { true } else { marker.pulse };
            if should_pulse {
                let phase = (elapsed * 0.8).fract();
                for offset in [0.0_f32, 0.5] {
                    let progress = ((phase + offset) % 1.0).clamp(0.0, 1.0);
                    frame.stroke(
                        &canvas::Path::circle(Point::new(x, y), radius + progress * radius * 2.8),
                        canvas::Stroke::default().with_width(0.6).with_color(Color {
                            a: 1.0 - progress,
                            ..self.props.marker_color
                        }),
                    );
                }
            }

            if let Some(DottedMapOverlay::Label { text, badge }) = &marker.overlay {
                let pill_height = radius * 3.2;
                let pill_width = text.chars().count() as f32 * radius * 1.35 + radius * 3.8;
                let pill_x = x + radius * 1.8;
                let pill_y = y - pill_height * 0.5;
                frame.fill(
                    &canvas::Path::rounded_rectangle(
                        Point::new(pill_x, pill_y),
                        iced::Size::new(pill_width, pill_height),
                        border::Radius::from(pill_height * 0.5),
                    ),
                    Color::from_rgba8(0, 0, 0, 0.55),
                );

                if let Some(handle) = badge {
                    frame.draw_image(
                        Rectangle::new(
                            Point::new(pill_x + radius * 0.4, pill_y + radius * 0.35),
                            iced::Size::new(radius * 1.6, radius * 1.6),
                        ),
                        handle,
                    );
                }

                frame.fill_text(canvas::Text {
                    content: text.clone(),
                    position: Point::new(pill_x + radius * 2.4, y),
                    color: Color::WHITE,
                    size: iced::Pixels(radius * 1.5),
                    align_y: iced::alignment::Vertical::Center,
                    ..canvas::Text::default()
                });
            }
        }

        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dotted_map_defaults_match_docs() {
        let props = DottedMapProps::new();

        assert_eq!(props.map_width, 150.0);
        assert_eq!(props.map_height, 75.0);
        assert_eq!(props.map_samples, 5000);
        assert_eq!(props.dot_radius, 0.2);
        assert!(props.stagger);
    }

    #[test]
    fn dotted_map_state_builds_world_points() {
        let props = DottedMapProps::new();
        let state = DottedMapState::new(&props);

        assert!(state.points.len() > 1000);
    }
}
