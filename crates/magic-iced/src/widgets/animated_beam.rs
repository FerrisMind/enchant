use crate::primitives::{
    AnchorBounds, BeamPath, BeamPathSpec, EffectCanvasProps, EffectProgram, beam_path,
    effect_canvas, point_at,
};
use iced::widget::canvas::{self, LineCap, Stroke};
use iced::{Color, Element, Length, Rectangle, Renderer};
use std::time::Duration;

/// Matches `animated-beam.tsx` transition ease: cubic-bezier(0.16, 1, 0.3, 1).
const BEAM_EASE_X1: f32 = 0.16;
const BEAM_EASE_Y1: f32 = 1.0;
const BEAM_EASE_X2: f32 = 0.3;
const BEAM_EASE_Y2: f32 = 1.0;
/// Second gradient stop offset from the reference SVG.
const BEAM_GRADIENT_STOP_MID: f32 = 0.325;
const BEAM_PATH_SEGMENTS: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimatedBeamProps {
    pub curvature: f32,
    pub reverse: bool,
    pub duration_secs: f32,
    pub delay_secs: f32,
    pub repeat_delay_secs: f32,
    pub path_color: Color,
    pub path_width: f32,
    pub path_opacity: f32,
    pub gradient_start_color: Color,
    pub gradient_stop_color: Color,
    pub start_x_offset: f32,
    pub start_y_offset: f32,
    pub end_x_offset: f32,
    pub end_y_offset: f32,
}

impl Default for AnimatedBeamProps {
    fn default() -> Self {
        Self {
            curvature: 0.0,
            reverse: false,
            duration_secs: 5.0,
            delay_secs: 0.0,
            repeat_delay_secs: 0.0,
            path_color: Color::from_rgb8(0x80, 0x80, 0x80),
            path_width: 2.0,
            path_opacity: 0.2,
            gradient_start_color: Color::from_rgb8(0xFF, 0xAA, 0x40),
            gradient_stop_color: Color::from_rgb8(0x9C, 0x40, 0xFF),
            start_x_offset: 0.0,
            start_y_offset: 0.0,
            end_x_offset: 0.0,
            end_y_offset: 0.0,
        }
    }
}

impl AnimatedBeamProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }
}

pub fn animated_beam<'a, Message: 'a>(
    from: AnchorBounds,
    to: AnchorBounds,
    elapsed: Duration,
    props: AnimatedBeamProps,
) -> Element<'a, Message> {
    effect_canvas::<_, Message>(
        AnimatedBeamProgram {
            from,
            to,
            elapsed,
            props,
        },
        EffectCanvasProps::new()
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .into()
}

struct AnimatedBeamProgram {
    from: AnchorBounds,
    to: AnchorBounds,
    elapsed: Duration,
    props: AnimatedBeamProps,
}

impl<Message> EffectProgram<Message> for AnimatedBeamProgram {
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
        let from = scene_local_anchor(self.from);
        let to = scene_local_anchor(self.to);
        let path = beam_path(
            from,
            to,
            BeamPathSpec::new()
                .curvature(self.props.curvature)
                .start_offset(self.props.start_x_offset, self.props.start_y_offset)
                .end_offset(self.props.end_x_offset, self.props.end_y_offset),
        );
        let path_shape = crate::primitives::canvas_path(path);

        frame.stroke(
            &path_shape,
            Stroke::default()
                .with_width(self.props.path_width)
                .with_line_cap(LineCap::Round)
                .with_color(Color {
                    a: self.props.path_opacity,
                    ..self.props.path_color
                }),
        );

        if let Some(eased) = beam_eased_progress(self.elapsed, &self.props) {
            draw_gradient_beam_stroke(&mut frame, path, bounds.width, eased, &self.props);
        }

        vec![frame.into_geometry()]
    }
}

fn draw_gradient_beam_stroke(
    frame: &mut canvas::Frame,
    path: BeamPath,
    container_width: f32,
    eased: f32,
    props: &AnimatedBeamProps,
) {
    let (grad_x1, grad_x2) = gradient_endpoints(props.reverse, eased, container_width);
    let last_segment = BEAM_PATH_SEGMENTS.saturating_sub(1);

    for index in 0..BEAM_PATH_SEGMENTS {
        let t0 = index as f32 / BEAM_PATH_SEGMENTS as f32;
        let t1 = (index + 1) as f32 / BEAM_PATH_SEGMENTS as f32;
        let p0 = point_at(path, t0);
        let p1 = point_at(path, t1);
        let mid = point_at(path, (t0 + t1) * 0.5);
        let color = sample_beam_gradient(
            mid.x,
            grad_x1,
            grad_x2,
            props.gradient_start_color,
            props.gradient_stop_color,
        );

        if color.a <= 0.01 {
            continue;
        }

        let segment = canvas::Path::new(|builder| {
            builder.move_to(p0);
            builder.line_to(p1);
        });

        let line_cap = if index == 0 || index == last_segment {
            LineCap::Round
        } else {
            LineCap::Butt
        };

        frame.stroke(
            &segment,
            Stroke::default()
                .with_width(props.path_width)
                .with_line_cap(line_cap)
                .with_color(color),
        );
    }
}

fn beam_motion_phase(elapsed: Duration, props: &AnimatedBeamProps) -> Option<f32> {
    let active_duration = props.duration_secs.max(0.01);
    let initial_delay = props.delay_secs.max(0.0);
    let repeat_delay = props.repeat_delay_secs.max(0.0);
    let elapsed_secs = elapsed.as_secs_f32();

    if elapsed_secs < initial_delay {
        return None;
    }

    let cycle = active_duration + repeat_delay;
    let time_in_cycle = (elapsed_secs - initial_delay).rem_euclid(cycle);
    if time_in_cycle > active_duration {
        return None;
    }

    let progress = (time_in_cycle / active_duration).clamp(0.0, 1.0);

    Some(progress)
}

fn beam_eased_progress(elapsed: Duration, props: &AnimatedBeamProps) -> Option<f32> {
    beam_motion_phase(elapsed, props).map(|linear| {
        cubic_bezier_ease(
            linear,
            BEAM_EASE_X1,
            BEAM_EASE_Y1,
            BEAM_EASE_X2,
            BEAM_EASE_Y2,
        )
    })
}

/// `userSpaceOnUse` gradient endpoints from `animated-beam.tsx`.
fn gradient_endpoints(reverse: bool, eased: f32, width: f32) -> (f32, f32) {
    if reverse {
        (width * (0.90 - eased), width * (1.0 - eased))
    } else {
        (width * (0.10 + eased), width * eased)
    }
}

/// Samples the animated beam gradient at a container-space X coordinate.
fn sample_beam_gradient(x: f32, grad_x1: f32, grad_x2: f32, start: Color, stop: Color) -> Color {
    let span = grad_x2 - grad_x1;
    if span.abs() < f32::EPSILON {
        return Color::TRANSPARENT;
    }

    let t = (x - grad_x1) / span;

    if t < 0.0 {
        return Color::TRANSPARENT;
    }

    if t <= BEAM_GRADIENT_STOP_MID {
        let local = (t / BEAM_GRADIENT_STOP_MID).clamp(0.0, 1.0);
        return mix_color(start, stop, local);
    }

    if t < 1.0 {
        let fade = 1.0 - ((t - BEAM_GRADIENT_STOP_MID) / (1.0 - BEAM_GRADIENT_STOP_MID));
        return Color { a: fade, ..stop };
    }

    Color::TRANSPARENT
}

fn cubic_bezier_ease(t: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }

    let sample = |t: f32, p1: f32, p2: f32| {
        let c = 3.0 * p1;
        let b = 3.0 * (p2 - p1) - c;
        let a = 1.0 - c - b;
        ((a * t + b) * t + c) * t
    };
    let derivative = |t: f32, p1: f32, p2: f32| {
        let c = 3.0 * p1;
        let b = 3.0 * (p2 - p1) - c;
        let a = 1.0 - c - b;
        (3.0 * a * t * t) + (2.0 * b * t) + c
    };

    let mut u = t;
    for _ in 0..6 {
        let x = sample(u, x1, x2);
        let dx = derivative(u, x1, x2);
        if dx.abs() < 1e-6 {
            break;
        }
        u = (u - (x - t) / dx).clamp(0.0, 1.0);
    }

    sample(u, y1, y2)
}

fn scene_local_anchor(anchor: AnchorBounds) -> AnchorBounds {
    anchor
}

fn mix_color(from: Color, to: Color, progress: f32) -> Color {
    let progress = progress.clamp(0.0, 1.0);

    Color {
        r: from.r + (to.r - from.r) * progress,
        g: from.g + (to.g - from.g) * progress,
        b: from.b + (to.b - from.b) * progress,
        a: from.a + (to.a - from.a) * progress,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Point;

    #[test]
    fn animated_beam_defaults_are_stable() {
        let props = AnimatedBeamProps::new();

        assert_eq!(props.duration_secs, 5.0);
        assert_eq!(props.path_width, 2.0);
        assert_eq!(props.path_opacity, 0.2);
    }

    #[test]
    fn beam_motion_phase_respects_delay_and_repeat_delay() {
        let delayed = beam_motion_phase(Duration::from_secs_f32(0.2), &AnimatedBeamProps::new());
        let paused = beam_motion_phase(
            Duration::from_secs_f32(5.3),
            &AnimatedBeamProps {
                repeat_delay_secs: 0.5,
                ..AnimatedBeamProps::new()
            },
        );

        assert_eq!(delayed, Some(0.04));
        assert!(paused.is_none());
    }

    #[test]
    fn beam_easing_matches_reference_curve() {
        assert!(
            (cubic_bezier_ease(0.0, BEAM_EASE_X1, BEAM_EASE_Y1, BEAM_EASE_X2, BEAM_EASE_Y2)).abs()
                < f32::EPSILON
        );
        assert!(
            (cubic_bezier_ease(1.0, BEAM_EASE_X1, BEAM_EASE_Y1, BEAM_EASE_X2, BEAM_EASE_Y2) - 1.0)
                .abs()
                < f32::EPSILON
        );
        let mid = cubic_bezier_ease(0.5, BEAM_EASE_X1, BEAM_EASE_Y1, BEAM_EASE_X2, BEAM_EASE_Y2);
        assert!(mid > 0.5, "ease-out curve should pass 0.5 early");
    }

    #[test]
    fn gradient_endpoints_match_reference_sweep() {
        let forward_start = gradient_endpoints(false, 0.0, 100.0);
        let forward_end = gradient_endpoints(false, 1.0, 100.0);
        assert!((forward_start.0 - 10.0).abs() < f32::EPSILON);
        assert!(forward_start.1.abs() < f32::EPSILON);
        assert!((forward_end.0 - 110.0).abs() < f32::EPSILON);
        assert!((forward_end.1 - 100.0).abs() < f32::EPSILON);

        let reverse_start = gradient_endpoints(true, 0.0, 100.0);
        let reverse_end = gradient_endpoints(true, 1.0, 100.0);
        assert!((reverse_start.0 - 90.0).abs() < 1e-3);
        assert!((reverse_start.1 - 100.0).abs() < 1e-3);
        assert!((reverse_end.0 + 10.0).abs() < 1e-3);
        assert!(reverse_end.1.abs() < 1e-3);
    }

    #[test]
    fn gradient_samples_reference_stop_layout() {
        let start = Color::from_rgb8(0xFF, 0xAA, 0x40);
        let stop = Color::from_rgb8(0x9C, 0x40, 0xFF);

        assert_eq!(sample_beam_gradient(0.0, 0.0, 100.0, start, stop), start);
        assert!(sample_beam_gradient(-1.0, 0.0, 100.0, start, stop).a < f32::EPSILON);
        assert_eq!(sample_beam_gradient(32.5, 0.0, 100.0, start, stop), stop);
        assert!(sample_beam_gradient(100.0, 0.0, 100.0, start, stop).a < f32::EPSILON);
    }

    #[test]
    fn scene_local_anchor_preserves_scene_coordinates() {
        let anchor = AnchorBounds::from_bounds(Rectangle::new(
            Point::new(120.0, 80.0),
            iced::Size::new(40.0, 20.0),
        ));
        let translated = scene_local_anchor(anchor);

        assert_eq!(translated.bounds.x, 120.0);
        assert_eq!(translated.bounds.y, 80.0);
        assert_eq!(translated.center.x, 140.0);
        assert_eq!(translated.center.y, 90.0);
    }
}
