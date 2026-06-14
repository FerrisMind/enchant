use crate::primitives::{
    AnimatedValue, EffectCanvasProps, EffectProgram, MotionSpec, effect_canvas,
};
use iced::time::Instant;
use iced::widget::canvas::{LineCap, LineJoin};
use iced::widget::{canvas, container, stack, text};
use iced::{Color, Element, Length, Point, Radians, Rectangle, Renderer, Size};
use iced_shadcn::Theme;
use std::f32::consts::PI;
use twill::prelude::{Easing, TransitionDuration};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimatedCircularProgressBarProps {
    pub min: f32,
    pub max: f32,
    pub value: f32,
    pub size: f32,
    pub stroke_width: f32,
    pub motion: MotionSpec,
    pub track_color: Option<Color>,
    pub fill_color: Option<Color>,
    pub show_label: bool,
    pub font_size: u16,
}

impl Default for AnimatedCircularProgressBarProps {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 100.0,
            value: 0.0,
            size: 160.0,
            stroke_width: 10.0,
            motion: MotionSpec::new()
                .duration(TransitionDuration::CustomMs(1000))
                .easing(Easing::Out),
            track_color: None,
            fill_color: None,
            show_label: true,
            font_size: 24,
        }
    }
}

impl AnimatedCircularProgressBarProps {
    pub fn new(value: f32) -> Self {
        Self {
            value,
            ..Self::default()
        }
    }

    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.min = min;
        self.max = max.max(min + f32::EPSILON);
        self
    }

    pub fn motion(mut self, motion: MotionSpec) -> Self {
        self.motion = motion;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ProgressFingerprint {
    min: f32,
    max: f32,
    value: f32,
}

#[derive(Clone, Debug)]
pub struct AnimatedCircularProgressBarState {
    progress: AnimatedValue,
    started: bool,
    fingerprint: ProgressFingerprint,
}

impl AnimatedCircularProgressBarState {
    pub fn new(props: &AnimatedCircularProgressBarProps) -> Self {
        Self {
            progress: AnimatedValue::with_spec(
                normalized_progress(props.min, props.max, props.min),
                props.motion,
            ),
            started: false,
            fingerprint: ProgressFingerprint {
                min: props.min,
                max: props.max,
                value: props.value,
            },
        }
    }

    pub fn tick(&mut self, now: Instant, props: &AnimatedCircularProgressBarProps) {
        let fingerprint = ProgressFingerprint {
            min: props.min,
            max: props.max,
            value: props.value,
        };
        let target = normalized_progress(props.min, props.max, props.value);

        if !self.started {
            self.started = true;
            self.progress = AnimatedValue::with_spec(
                normalized_progress(props.min, props.max, props.min),
                props.motion,
            );
            self.progress.transition(target, now);
            self.fingerprint = fingerprint;
            return;
        }

        self.progress.set_spec(props.motion);

        if self.fingerprint != fingerprint {
            let current = self.progress.value_at(now);
            self.progress = AnimatedValue::with_spec(current, props.motion);
            self.progress.transition(target, now);
            self.fingerprint = fingerprint;
        }
    }

    pub fn progress_at(&self, now: Instant) -> f32 {
        self.progress.value_at(now).clamp(0.0, 1.0)
    }
}

pub fn animated_circular_progress_bar<'a, Message: 'a>(
    state: &AnimatedCircularProgressBarState,
    now: Instant,
    props: &AnimatedCircularProgressBarProps,
    theme: &Theme,
    foreground_override: Option<Color>,
) -> Element<'a, Message> {
    let progress = state.progress_at(now);
    let foreground = foreground_override.unwrap_or(theme.palette.foreground);
    let track_color = props.track_color.unwrap_or_else(|| {
        theme
            .registry
            .color("magic.progress.ring.track")
            .unwrap_or(Color {
                a: 0.1,
                ..foreground
            })
    });
    let fill_color = props
        .fill_color
        .or_else(|| theme.registry.color("magic.progress.ring.fill"))
        .unwrap_or(theme.palette.primary);

    let canvas = effect_canvas::<_, Message>(
        CircularProgressProgram {
            progress,
            stroke_width: props.stroke_width,
            track_color,
            fill_color,
        },
        EffectCanvasProps::new()
            .width(Length::Fixed(props.size))
            .height(Length::Fixed(props.size)),
    );

    let mut layers: Vec<Element<'a, Message>> = vec![canvas.into()];

    if props.show_label {
        let display_percent = (progress * 100.0).round() as i32;
        let label = container(text(format!("{display_percent}")).size(u32::from(props.font_size)))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style({
                let color = foreground;
                move |_| iced::widget::container::Style {
                    text_color: Some(color),
                    ..iced::widget::container::Style::default()
                }
            });
        layers.push(
            container(label)
                .width(Length::Fixed(props.size))
                .height(Length::Fixed(props.size))
                .into(),
        );
    }

    container(stack(layers))
        .width(Length::Fixed(props.size))
        .height(Length::Fixed(props.size))
        .into()
}

struct CircularProgressProgram {
    progress: f32,
    stroke_width: f32,
    track_color: Color,
    fill_color: Color,
}

/// Reference SVG viewBox (`0 0 100 100`), `r=45`, `strokeWidth=10`, `size-40` (160px).
const VIEWBOX_SIZE: f32 = 100.0;
const REF_RADIUS: f32 = 45.0;
const REF_STROKE: f32 = 10.0;
/// `--gap-percent: 5` → 5% of a full turn between arc tips.
const GAP_PERCENT: f32 = 5.0;
/// Secondary arc max length is 90% of the ring (`90 - currentPercent` in reference).
const SECONDARY_CAP_PERCENT: f32 = 90.0;

fn reference_scale(bounds: Size) -> f32 {
    bounds.width.min(bounds.height) / VIEWBOX_SIZE
}

fn ring_geometry(scale: f32, stroke_width: f32) -> (f32, f32) {
    let radius = REF_RADIUS * scale;
    let stroke = if stroke_width > 0.0 {
        stroke_width * scale
    } else {
        REF_STROKE * scale
    };
    (radius, stroke)
}

fn rounded_percent(progress: f32) -> i32 {
    (progress.clamp(0.0, 1.0) * 100.0).round() as i32
}

fn arc_lengths(progress: f32) -> (f32, f32) {
    let percent = rounded_percent(progress) as f32;
    let primary = percent / 100.0;
    let secondary = ((SECONDARY_CAP_PERCENT - percent).max(0.0)) / 100.0;
    (primary, secondary)
}

/// Reference keeps secondary mounted while `currentPercent <= 90`.
fn secondary_is_mounted(percent: i32) -> bool {
    (0..=90).contains(&percent)
}

fn gap_radians() -> f32 {
    (GAP_PERCENT / 100.0) * 2.0 * PI
}

fn primary_tip_angle() -> f32 {
    -PI / 2.0
}

fn secondary_tip_angle() -> f32 {
    2.0 * PI - PI / 2.0 - gap_radians()
}

fn arc_angles(primary_fraction: f32, secondary_fraction: f32) -> (f32, f32, f32, f32) {
    // Iced arcs sweep clockwise from start_angle to end_angle (end must be > start).
    let primary_start = primary_tip_angle();
    let primary_end = primary_start + primary_fraction * 2.0 * PI;

    // Secondary anchor matches reference `rotate(252deg) scaleY(-1)`.
    let secondary_anchor = secondary_tip_angle();
    let secondary_start = secondary_anchor - secondary_fraction * 2.0 * PI;
    let secondary_end = secondary_anchor;

    (primary_start, primary_end, secondary_start, secondary_end)
}

fn point_on_ring(center: Point, radius: f32, angle: f32) -> Point {
    Point::new(
        center.x + radius * angle.cos(),
        center.y + radius * angle.sin(),
    )
}

impl<Message> EffectProgram<Message> for CircularProgressProgram {
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
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let scale = reference_scale(bounds.size());
        let (radius, stroke_width) = ring_geometry(scale, self.stroke_width);

        let percent = rounded_percent(self.progress);
        let (primary_fraction, secondary_fraction) = arc_lengths(self.progress);

        if primary_fraction > f32::EPSILON {
            let (primary_start, primary_end, _, _) = arc_angles(primary_fraction, 0.0);
            stroke_arc(
                &mut frame,
                center,
                radius,
                primary_start,
                primary_end,
                stroke_width,
                self.fill_color,
            );
        } else if percent < 100 {
            fill_round_cap(
                &mut frame,
                center,
                radius,
                primary_tip_angle(),
                stroke_width,
                self.fill_color,
            );
        }

        if secondary_is_mounted(percent) {
            if secondary_fraction > f32::EPSILON {
                let (_, _, secondary_start, secondary_end) = arc_angles(0.0, secondary_fraction);
                stroke_arc(
                    &mut frame,
                    center,
                    radius,
                    secondary_start,
                    secondary_end,
                    stroke_width,
                    self.track_color,
                );
            } else {
                fill_round_cap(
                    &mut frame,
                    center,
                    radius,
                    secondary_tip_angle(),
                    stroke_width,
                    self.track_color,
                );
            }
        }

        vec![frame.into_geometry()]
    }
}

fn stroke_arc(
    frame: &mut canvas::Frame,
    center: Point,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    width: f32,
    color: Color,
) {
    let mut builder = canvas::path::Builder::new();
    builder.arc(canvas::path::Arc {
        center,
        radius,
        start_angle: Radians(start_angle),
        end_angle: Radians(end_angle),
    });
    frame.stroke(&builder.build(), progress_stroke(width, color));
}

/// Zero-length SVG dashes with `strokeLinecap: round` render as a dot, not an arc segment.
fn fill_round_cap(
    frame: &mut canvas::Frame,
    center: Point,
    radius: f32,
    angle: f32,
    stroke_width: f32,
    color: Color,
) {
    let point = point_on_ring(center, radius, angle);
    frame.fill(&canvas::Path::circle(point, stroke_width / 2.0), color);
}

fn normalized_progress(min: f32, max: f32, value: f32) -> f32 {
    let span = (max - min).max(f32::EPSILON);
    ((value - min) / span).clamp(0.0, 1.0)
}

fn progress_stroke(width: f32, color: Color) -> canvas::Stroke<'static> {
    canvas::Stroke::default()
        .with_width(width)
        .with_line_cap(LineCap::Round)
        .with_line_join(LineJoin::Round)
        .with_color(color)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::time::Duration;

    #[test]
    fn normalized_progress_clamps_to_range() {
        assert_eq!(normalized_progress(0.0, 100.0, -20.0), 0.0);
        assert_eq!(normalized_progress(0.0, 100.0, 50.0), 0.5);
        assert_eq!(normalized_progress(0.0, 100.0, 200.0), 1.0);
    }

    #[test]
    fn circular_progress_animates_to_target() {
        let props = AnimatedCircularProgressBarProps::new(75.0).motion(
            MotionSpec::new()
                .duration(TransitionDuration::CustomMs(100))
                .easing(Easing::Linear),
        );
        let mut state = AnimatedCircularProgressBarState::new(&props);
        let start = Instant::now();

        state.tick(start, &props);

        let halfway = state.progress_at(start + Duration::from_millis(50));
        assert!(halfway > 0.35 && halfway < 0.40);
    }

    #[test]
    fn ring_geometry_matches_reference_viewbox_scale() {
        let scale = reference_scale(Size::new(160.0, 160.0));
        assert!((scale - 1.6).abs() < f32::EPSILON);

        let (radius, stroke) = ring_geometry(scale, REF_STROKE);
        assert!((radius - 72.0).abs() < f32::EPSILON);
        assert!((stroke - 16.0).abs() < f32::EPSILON);
    }

    #[test]
    fn arc_lengths_match_reference_percent_rules() {
        let (primary, secondary) = arc_lengths(0.0);
        assert_eq!(primary, 0.0);
        assert!((secondary - 0.9).abs() < f32::EPSILON);

        let (primary, secondary) = arc_lengths(0.5);
        assert!((primary - 0.5).abs() < f32::EPSILON);
        assert!((secondary - 0.4).abs() < f32::EPSILON);

        let (primary, secondary) = arc_lengths(1.0);
        assert!((primary - 1.0).abs() < f32::EPSILON);
        assert_eq!(secondary, 0.0);
    }

    #[test]
    fn arc_gap_is_five_percent_at_mid_progress() {
        let (_, primary_end, secondary_start, _) = arc_angles(0.5, 0.4);
        let gap_deg = (secondary_start - primary_end) * 180.0 / PI;
        assert!((gap_deg - 18.0).abs() < 0.5);
    }

    #[test]
    fn secondary_arc_uses_positive_clockwise_sweep() {
        let (_, _, secondary_start, secondary_end) = arc_angles(0.0, 0.9);
        assert!(secondary_end > secondary_start);
        assert!((secondary_end - secondary_start - 0.9 * 2.0 * PI).abs() < f32::EPSILON);
    }

    #[test]
    fn primary_arc_uses_positive_clockwise_sweep_at_ninety_percent() {
        let (primary_start, primary_end, _, _) = arc_angles(0.9, 0.0);
        assert!(primary_end > primary_start);
        assert!((primary_end - primary_start - 0.9 * 2.0 * PI).abs() < f32::EPSILON);
    }

    #[test]
    fn secondary_stays_mounted_through_ninety_percent() {
        assert!(secondary_is_mounted(0));
        assert!(secondary_is_mounted(90));
        assert!(!secondary_is_mounted(91));
        assert!(!secondary_is_mounted(100));
    }

    #[test]
    fn zero_percent_keeps_gap_between_primary_dot_and_secondary_arc() {
        let (_, primary_end, secondary_start, _) = arc_angles(0.0, 0.9);
        let gap_deg = (secondary_start - primary_end) * 180.0 / PI;
        assert!((gap_deg - 18.0).abs() < 0.5);
    }

    #[test]
    fn ninety_percent_keeps_gap_between_primary_arc_and_secondary_dot() {
        let (_, primary_end, secondary_start, _) = arc_angles(0.9, 0.0);
        let gap_deg = (secondary_start - primary_end) * 180.0 / PI;
        assert!((gap_deg - 18.0).abs() < 0.5);
    }
}
