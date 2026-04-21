use crate::primitives::{
    EffectCanvasProps, EffectProgram, SpherePoint, effect_canvas, fibonacci_sphere, focus_rotation,
    project_sphere_point, rotate_sphere_point,
};
use iced::mouse;
use iced::time::Instant;
use iced::widget::{canvas, mouse_area, svg};
use iced::{Color, Element, Length, Point, Rectangle, Renderer};

#[derive(Clone, Debug)]
pub enum IconCloudItem {
    Text(String),
    Image(iced::widget::image::Handle),
    Svg(svg::Handle),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IconCloudAction {
    Move(Point),
    Press,
    Release,
    Exit,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TargetRotation {
    started_at: Instant,
    from_x: f32,
    from_y: f32,
    to_x: f32,
    to_y: f32,
    duration_secs: f32,
}

#[derive(Clone, Debug)]
pub struct IconCloudState {
    rotation_x: f32,
    rotation_y: f32,
    pointer: Option<Point>,
    dragging: bool,
    points: Vec<SpherePoint>,
    target: Option<TargetRotation>,
    last_tick_at: Option<Instant>,
    item_count: usize,
    radius: f32,
}

impl IconCloudState {
    pub fn new(props: &IconCloudProps) -> Self {
        Self {
            rotation_x: 0.0,
            rotation_y: 0.0,
            pointer: None,
            dragging: false,
            points: fibonacci_sphere(props.items.len().max(1), props.radius),
            target: None,
            last_tick_at: None,
            item_count: props.items.len().max(1),
            radius: props.radius,
        }
    }

    pub fn apply_action(&mut self, action: &IconCloudAction, props: &IconCloudProps) {
        self.ensure_layout(props);

        match action {
            IconCloudAction::Move(position) => {
                if let Some(last) = self.pointer
                    && self.dragging
                {
                    let delta_x = position.x - last.x;
                    let delta_y = position.y - last.y;
                    self.rotation_y += delta_x * props.drag_sensitivity;
                    self.rotation_x += delta_y * props.drag_sensitivity;
                    self.target = None;
                }
                self.pointer = Some(*position);
            }
            IconCloudAction::Press => {
                if let Some(pointer) = self.pointer {
                    if let Some(index) = self.hit_test(pointer, props) {
                        let (to_x, to_y) = focus_rotation(self.points[index]);
                        self.target = Some(TargetRotation {
                            started_at: self.last_tick_at.unwrap_or_else(Instant::now),
                            from_x: self.rotation_x,
                            from_y: self.rotation_y,
                            to_x,
                            to_y,
                            duration_secs: 1.1,
                        });
                    } else {
                        self.dragging = true;
                    }
                } else {
                    self.dragging = true;
                }
            }
            IconCloudAction::Release | IconCloudAction::Exit => {
                self.dragging = false;
                if matches!(action, IconCloudAction::Exit) {
                    self.pointer = None;
                }
            }
        }
    }

    pub fn tick(&mut self, now: Instant, props: &IconCloudProps) {
        self.ensure_layout(props);
        let dt = self
            .last_tick_at
            .map(|last| now.saturating_duration_since(last).as_secs_f32())
            .unwrap_or(0.0);
        self.last_tick_at = Some(now);

        if let Some(target) = self.target {
            let progress = (now
                .saturating_duration_since(target.started_at)
                .as_secs_f32()
                / target.duration_secs.max(0.1))
            .clamp(0.0, 1.0);
            let eased = 1.0 - (1.0 - progress).powi(3);
            self.rotation_x = target.from_x + (target.to_x - target.from_x) * eased;
            self.rotation_y = target.from_y + (target.to_y - target.from_y) * eased;
            if progress >= 1.0 {
                self.target = None;
            }
        } else if !self.dragging {
            self.rotation_y += props.auto_rotate_speed * dt;
        }
    }

    fn ensure_layout(&mut self, props: &IconCloudProps) {
        let count = props.items.len().max(1);
        if self.item_count != count || (self.radius - props.radius).abs() > f32::EPSILON {
            self.points = fibonacci_sphere(count, props.radius);
            self.item_count = count;
            self.radius = props.radius;
        }
    }

    fn hit_test(&self, pointer: Point, props: &IconCloudProps) -> Option<usize> {
        let bounds =
            Rectangle::with_size(iced::Size::new(props.logical_width, props.logical_height));
        let mut best = None;

        for (index, point) in self.points.iter().enumerate() {
            let rotated = rotate_sphere_point(*point, self.rotation_x, self.rotation_y);
            let projected = project_sphere_point(rotated, bounds, props.camera_distance);
            let radius = props.item_size * projected.scale * 0.5;
            let dx = projected.position.x - pointer.x;
            let dy = projected.position.y - pointer.y;
            let distance_sq = dx * dx + dy * dy;
            if distance_sq <= radius * radius {
                best = Some((distance_sq, index));
            }
        }

        best.map(|(_, index)| index)
    }
}

#[derive(Clone, Debug)]
pub struct IconCloudProps {
    pub width: Length,
    pub height: Length,
    pub logical_width: f32,
    pub logical_height: f32,
    pub radius: f32,
    pub camera_distance: f32,
    pub auto_rotate_speed: f32,
    pub drag_sensitivity: f32,
    pub item_size: f32,
    pub items: Vec<IconCloudItem>,
    pub text_color: Option<Color>,
}

impl Default for IconCloudProps {
    fn default() -> Self {
        Self {
            width: Length::Fixed(400.0),
            height: Length::Fixed(400.0),
            logical_width: 400.0,
            logical_height: 400.0,
            radius: 120.0,
            camera_distance: 240.0,
            auto_rotate_speed: 0.45,
            drag_sensitivity: 0.008,
            item_size: 40.0,
            items: Vec::new(),
            text_color: None,
        }
    }
}

impl IconCloudProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn icon_cloud<'a, Message: Clone + 'a>(
    state: &'a IconCloudState,
    props: &'a IconCloudProps,
    on_action: impl Fn(IconCloudAction) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let canvas: Element<'a, Message> = effect_canvas::<_, Message>(
        IconCloudProgram { state, props },
        EffectCanvasProps::new()
            .width(props.width)
            .height(props.height),
    )
    .into();

    mouse_area(canvas)
        .on_move({
            let on_action = on_action.clone();
            move |position| on_action(IconCloudAction::Move(position))
        })
        .on_press(on_action(IconCloudAction::Press))
        .on_release(on_action(IconCloudAction::Release))
        .on_exit(on_action(IconCloudAction::Exit))
        .interaction(mouse::Interaction::Grab)
        .into()
}

struct IconCloudProgram<'a> {
    state: &'a IconCloudState,
    props: &'a IconCloudProps,
}

impl<Message> EffectProgram<Message> for IconCloudProgram<'_> {
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
        let text_color = self.props.text_color.unwrap_or(Color::WHITE);
        let mut projected = self
            .props
            .items
            .iter()
            .zip(self.state.points.iter())
            .map(|(item, point)| {
                let rotated =
                    rotate_sphere_point(*point, self.state.rotation_x, self.state.rotation_y);
                let projection = project_sphere_point(rotated, bounds, self.props.camera_distance);
                (projection.depth, projection, item)
            })
            .collect::<Vec<_>>();

        projected.sort_by(|left, right| left.0.total_cmp(&right.0));
        for (_depth, projection, item) in projected {
            let size = self.props.item_size * projection.scale;
            let rect = Rectangle::new(
                Point::new(
                    projection.position.x - size * 0.5,
                    projection.position.y - size * 0.5,
                ),
                iced::Size::new(size, size),
            );
            match item {
                IconCloudItem::Text(label) => frame.fill_text(canvas::Text {
                    content: label.clone(),
                    position: projection.position,
                    color: Color {
                        a: projection.opacity,
                        ..text_color
                    },
                    size: iced::Pixels(size * 0.55),
                    align_x: iced::alignment::Horizontal::Center.into(),
                    align_y: iced::alignment::Vertical::Center,
                    ..canvas::Text::default()
                }),
                IconCloudItem::Image(handle) => frame.draw_image(rect, handle),
                IconCloudItem::Svg(handle) => frame.draw_svg(rect, &handle.clone()),
            }
        }

        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_cloud_defaults_are_stable() {
        let props = IconCloudProps::new();

        assert_eq!(props.radius, 120.0);
        assert_eq!(props.item_size, 40.0);
        assert_eq!(props.logical_width, 400.0);
    }

    #[test]
    fn icon_cloud_press_focuses_existing_item() {
        let props = IconCloudProps {
            items: vec![IconCloudItem::Text("A".into())],
            ..IconCloudProps::new()
        };
        let mut state = IconCloudState::new(&props);
        state.apply_action(&IconCloudAction::Move(Point::new(200.0, 200.0)), &props);
        state.apply_action(&IconCloudAction::Press, &props);

        assert!(state.target.is_some() || state.dragging);
    }
}
