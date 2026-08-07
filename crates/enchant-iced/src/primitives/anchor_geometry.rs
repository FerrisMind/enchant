use iced::Task;
use iced::advanced::widget::Operation;
use iced::advanced::widget::operation::Outcome;
use iced::widget::Id;
use iced::widget::canvas;
use iced::widget::container;
use iced::{Element, Point, Rectangle, Vector};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnchorBounds {
    pub bounds: Rectangle,
    pub center: Point,
}

impl AnchorBounds {
    pub fn from_bounds(bounds: Rectangle) -> Self {
        Self {
            center: Point::new(
                bounds.x + bounds.width / 2.0,
                bounds.y + bounds.height / 2.0,
            ),
            bounds,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnchorMeasurements(Box<[Option<AnchorBounds>]>);

impl AnchorMeasurements {
    pub fn new(values: Vec<Option<AnchorBounds>>) -> Self {
        Self(values.into_boxed_slice())
    }

    pub fn get(&self, index: usize) -> Option<AnchorBounds> {
        self.0.get(index).copied().flatten()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = Option<AnchorBounds>> + '_ {
        self.0.iter().copied()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnchorRemeasure {
    pending: bool,
}

impl Default for AnchorRemeasure {
    fn default() -> Self {
        Self { pending: true }
    }
}

impl AnchorRemeasure {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn schedule(&mut self) {
        self.pending = true;
    }

    pub fn take(&mut self) -> bool {
        std::mem::take(&mut self.pending)
    }
}

pub fn anchor<'a, Message: 'a>(
    id: impl Into<Id>,
    content: impl Into<Element<'a, Message>>,
) -> container::Container<'a, Message> {
    container(content).id(id)
}

pub fn measure_anchors(ids: impl IntoIterator<Item = Id>) -> Task<AnchorMeasurements> {
    iced_runtime::task::widget(MeasureAnchors::new(ids.into_iter().collect()))
}

struct MeasureAnchors {
    ids: Vec<Id>,
    values: Vec<Option<AnchorBounds>>,
}

impl MeasureAnchors {
    fn new(ids: Vec<Id>) -> Self {
        let values = vec![None; ids.len()];
        Self { ids, values }
    }
}

impl Operation<AnchorMeasurements> for MeasureAnchors {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<AnchorMeasurements>)) {
        operate(self);
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        let Some(target) = id else {
            return;
        };

        if let Some((index, _)) = self
            .ids
            .iter()
            .enumerate()
            .find(|(_, candidate)| *candidate == target)
        {
            self.values[index] = Some(AnchorBounds::from_bounds(bounds));
        }
    }

    fn finish(&self) -> Outcome<AnchorMeasurements> {
        Outcome::Some(AnchorMeasurements::new(self.values.clone()))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeamPathSpec {
    pub curvature: f32,
    pub start_offset: Vector,
    pub end_offset: Vector,
}

impl Default for BeamPathSpec {
    fn default() -> Self {
        Self {
            curvature: 0.0,
            start_offset: Vector::default(),
            end_offset: Vector::default(),
        }
    }
}

impl BeamPathSpec {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn curvature(mut self, curvature: f32) -> Self {
        self.curvature = curvature;
        self
    }

    pub fn start_offset(mut self, x: f32, y: f32) -> Self {
        self.start_offset = Vector::new(x, y);
        self
    }

    pub fn end_offset(mut self, x: f32, y: f32) -> Self {
        self.end_offset = Vector::new(x, y);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeamPath {
    pub start: Point,
    pub control: Point,
    pub end: Point,
}

pub fn beam_path(from: AnchorBounds, to: AnchorBounds, spec: BeamPathSpec) -> BeamPath {
    let start = Point::new(
        from.center.x + spec.start_offset.x,
        from.center.y + spec.start_offset.y,
    );
    let end = Point::new(
        to.center.x + spec.end_offset.x,
        to.center.y + spec.end_offset.y,
    );
    let control = Point::new((start.x + end.x) / 2.0, start.y - spec.curvature);

    BeamPath {
        start,
        control,
        end,
    }
}

pub fn point_at(path: BeamPath, t: f32) -> Point {
    let one_minus_t = 1.0 - t;
    let x = one_minus_t.powi(2) * path.start.x
        + 2.0 * one_minus_t * t * path.control.x
        + t.powi(2) * path.end.x;
    let y = one_minus_t.powi(2) * path.start.y
        + 2.0 * one_minus_t * t * path.control.y
        + t.powi(2) * path.end.y;

    Point::new(x, y)
}

pub fn canvas_path(path: BeamPath) -> canvas::Path {
    canvas::Path::new(|builder| {
        builder.move_to(path.start);
        builder.quadratic_curve_to(path.control, path.end);
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Size;

    #[test]
    fn measure_anchors_collects_bounds_in_requested_order() {
        let first = Id::new("first-anchor");
        let second = Id::new("second-anchor");
        let mut op = MeasureAnchors::new(vec![first.clone(), second.clone()]);

        op.container(
            Some(&second),
            Rectangle::new(Point::new(20.0, 30.0), Size::new(40.0, 50.0)),
        );
        op.container(
            Some(&first),
            Rectangle::new(Point::new(1.0, 2.0), Size::new(10.0, 12.0)),
        );

        let Outcome::Some(measurements) = op.finish() else {
            panic!("expected anchor measurements");
        };

        assert_eq!(
            measurements.get(0),
            Some(AnchorBounds::from_bounds(Rectangle::new(
                Point::new(1.0, 2.0),
                Size::new(10.0, 12.0)
            )))
        );
        assert_eq!(
            measurements.get(1),
            Some(AnchorBounds::from_bounds(Rectangle::new(
                Point::new(20.0, 30.0),
                Size::new(40.0, 50.0)
            )))
        );
    }

    #[test]
    fn beam_path_uses_anchor_centers_and_offsets() {
        let from =
            AnchorBounds::from_bounds(Rectangle::new(Point::new(0.0, 0.0), Size::new(20.0, 20.0)));
        let to = AnchorBounds::from_bounds(Rectangle::new(
            Point::new(100.0, 50.0),
            Size::new(30.0, 30.0),
        ));

        let path = beam_path(
            from,
            to,
            BeamPathSpec::new()
                .curvature(40.0)
                .start_offset(5.0, -5.0)
                .end_offset(-10.0, 10.0),
        );

        assert_eq!(path.start, Point::new(15.0, 5.0));
        assert_eq!(path.end, Point::new(105.0, 75.0));
        assert_eq!(path.control, Point::new(60.0, -35.0));
    }

    #[test]
    fn point_at_returns_quadratic_positions() {
        let path = BeamPath {
            start: Point::new(0.0, 0.0),
            control: Point::new(50.0, -50.0),
            end: Point::new(100.0, 0.0),
        };

        assert_eq!(point_at(path, 0.0), path.start);
        assert_eq!(point_at(path, 1.0), path.end);
        assert_eq!(point_at(path, 0.5), Point::new(50.0, -25.0));
    }

    #[test]
    fn point_at_allows_extrapolation_past_endpoints() {
        let path = BeamPath {
            start: Point::new(0.0, 0.0),
            control: Point::new(50.0, -50.0),
            end: Point::new(100.0, 0.0),
        };

        assert_eq!(point_at(path, -0.5), Point::new(-50.0, 75.0));
        assert_eq!(point_at(path, 1.5), Point::new(150.0, 75.0));
    }

    #[test]
    fn anchor_remeasure_is_trigger_driven_instead_of_polled() {
        let mut remeasure = AnchorRemeasure::new();

        assert!(remeasure.take());
        assert!(!remeasure.take());

        remeasure.schedule();

        assert!(remeasure.take());
        assert!(!remeasure.take());
    }
}
