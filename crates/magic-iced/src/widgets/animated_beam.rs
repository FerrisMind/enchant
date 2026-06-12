use crate::primitives::{
    AnchorBounds, BeamPathSpec, EffectCanvasProps, EffectProgram, beam_path, effect_canvas,
    point_at,
};
use iced::widget::canvas::{self, Stroke};
use iced::{Color, Element, Length, Rectangle, Renderer};
use std::time::Duration;

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
            path_color: Color::from_rgba(0.6, 0.6, 0.6, 0.2),
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
                .with_color(Color {
                    a: self.props.path_opacity,
                    ..self.props.path_color
                }),
        );

        if let Some(progress) = beam_motion_phase(self.elapsed, &self.props) {
            let (head, head_grows) = beam_head_progress(progress, self.props.reverse, from, to);

            for index in (0..12).rev() {
                let factor = index as f32 / 11.0;
                let path_progress = if head_grows {
                    (head - factor * 0.12).clamp(0.0, 1.0)
                } else {
                    (head + factor * 0.12).clamp(0.0, 1.0)
                };
                let sample = point_at(path, path_progress);
                let color = mix_color(
                    self.props.gradient_start_color,
                    self.props.gradient_stop_color,
                    1.0 - factor,
                );

                frame.fill(
                    &canvas::Path::circle(sample, 2.0 + (1.0 - factor) * 3.0),
                    Color {
                        a: 0.12 + (1.0 - factor) * 0.75,
                        ..color
                    },
                );
            }
        }

        vec![frame.into_geometry()]
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

fn scene_local_anchor(anchor: AnchorBounds) -> AnchorBounds {
    anchor
}

/// Maps timeline progress to path head position, matching `animated-beam.tsx` gradient sweep.
///
/// `reverse` flips the gradient direction. On left→right paths that inverts travel
/// (bidirectional lower arc). On right→left paths `reverse` aligns sweep with from→to
/// (main demo right-side beams into center).
fn beam_head_progress(
    progress: f32,
    reverse: bool,
    from: AnchorBounds,
    to: AnchorBounds,
) -> (f32, bool) {
    let path_left_to_right = to.center.x > from.center.x;
    let invert = reverse && path_left_to_right;
    let head = if invert { 1.0 - progress } else { progress };
    let head_grows_over_time = !invert;
    (head, head_grows_over_time)
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
    fn reverse_inverts_only_on_left_to_right_paths() {
        let left = AnchorBounds::from_bounds(Rectangle::new(
            iced::Point::new(0.0, 0.0),
            iced::Size::new(48.0, 48.0),
        ));
        let right = AnchorBounds::from_bounds(Rectangle::new(
            iced::Point::new(200.0, 0.0),
            iced::Size::new(48.0, 48.0),
        ));
        let center = AnchorBounds::from_bounds(Rectangle::new(
            iced::Point::new(100.0, 0.0),
            iced::Size::new(64.0, 64.0),
        ));

        let (main_left, grows) = beam_head_progress(0.25, false, left, center);
        assert!((main_left - 0.25).abs() < f32::EPSILON);
        assert!(grows);

        let (main_right, grows) = beam_head_progress(0.25, true, right, center);
        assert!((main_right - 0.25).abs() < f32::EPSILON);
        assert!(grows);

        let (bidirectional, grows) = beam_head_progress(0.25, true, left, right);
        assert!((bidirectional - 0.75).abs() < f32::EPSILON);
        assert!(!grows);
    }

    #[test]
    fn scene_local_anchor_preserves_scene_coordinates() {
        let anchor = AnchorBounds::from_bounds(Rectangle::new(
            iced::Point::new(120.0, 80.0),
            iced::Size::new(40.0, 20.0),
        ));
        let translated = scene_local_anchor(anchor);

        assert_eq!(translated.bounds.x, 120.0);
        assert_eq!(translated.bounds.y, 80.0);
        assert_eq!(translated.center.x, 140.0);
        assert_eq!(translated.center.y, 90.0);
    }
}
