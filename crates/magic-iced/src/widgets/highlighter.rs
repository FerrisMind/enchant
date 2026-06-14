use crate::primitives::{
    AnimatedValue, EffectCanvasProps, EffectProgram, MotionSpec, effect_canvas,
};
use iced::time::Instant;
use iced::widget::{canvas, container};
use iced::{Color, Element, Length, Padding, Point, Rectangle, Renderer};
use iced_shadcn::{DecorativeSurfaceProps, Theme, decorative_surface};
use twill::prelude::{Easing, TransitionDuration};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HighlighterAction {
    #[default]
    Highlight,
    Underline,
    Box,
    Circle,
    StrikeThrough,
    CrossedOff,
    Bracket,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HighlighterProps {
    pub action: HighlighterAction,
    pub color: Color,
    pub stroke_width: f32,
    pub animation_duration_ms: u64,
    pub iterations: u8,
    pub padding: f32,
    pub multiline: bool,
    pub is_view: bool,
    /// Rough-notation annotation seed. When `None`, a stable default is chosen per action.
    pub seed: Option<u32>,
}

impl Default for HighlighterProps {
    fn default() -> Self {
        Self {
            action: HighlighterAction::Highlight,
            color: Color::from_rgb8(0xFF, 0xD1, 0xDC),
            stroke_width: 1.5,
            animation_duration_ms: 600,
            iterations: 2,
            padding: 2.0,
            multiline: true,
            is_view: false,
            seed: None,
        }
    }
}

impl HighlighterProps {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct HighlighterFingerprint {
    action: HighlighterAction,
    color: Color,
    stroke_width: f32,
    animation_duration_ms: u64,
    iterations: u8,
    padding: f32,
    multiline: bool,
    is_view: bool,
    seed: Option<u32>,
}

impl From<&HighlighterProps> for HighlighterFingerprint {
    fn from(props: &HighlighterProps) -> Self {
        Self {
            action: props.action,
            color: props.color,
            stroke_width: props.stroke_width,
            animation_duration_ms: props.animation_duration_ms,
            iterations: props.iterations,
            padding: props.padding,
            multiline: props.multiline,
            is_view: props.is_view,
            seed: props.seed,
        }
    }
}

#[derive(Clone, Debug)]
pub struct HighlighterState {
    progress: AnimatedValue,
    started: bool,
    fingerprint: HighlighterFingerprint,
    seed: u32,
}

impl HighlighterState {
    pub fn new(props: &HighlighterProps) -> Self {
        Self::with_seed(props, resolve_seed(props, None))
    }

    pub fn seed(&self) -> u32 {
        self.seed
    }

    /// Replay the draw animation with a new rough-notation seed (new hand-drawn strokes).
    pub fn reset(&mut self, now: Instant, props: &HighlighterProps) {
        *self = Self::with_seed(props, random_seed());
        self.tick(now, props);
    }

    fn with_seed(props: &HighlighterProps, seed: u32) -> Self {
        Self {
            progress: AnimatedValue::with_spec(
                0.0,
                MotionSpec::new()
                    .duration(TransitionDuration::CustomMs(
                        props.animation_duration_ms.min(u16::MAX as u64) as u16,
                    ))
                    .easing(Easing::Out),
            ),
            started: false,
            fingerprint: HighlighterFingerprint::from(props),
            seed,
        }
    }

    pub fn tick(&mut self, now: Instant, props: &HighlighterProps) {
        let fingerprint = HighlighterFingerprint::from(props);
        if self.fingerprint != fingerprint {
            *self = Self::with_seed(props, resolve_seed(props, Some(self.seed)));
        }

        self.progress.set_spec(
            MotionSpec::new()
                .duration(TransitionDuration::CustomMs(
                    props.animation_duration_ms.min(u16::MAX as u64) as u16,
                ))
                .easing(Easing::Out),
        );

        if !self.started {
            self.started = true;
            self.progress.transition(1.0, now);
        }
    }

    pub fn progress_at(&self, now: Instant) -> f32 {
        self.progress.value_at(now).clamp(0.0, 1.0)
    }
}

pub fn highlighter<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    state: &HighlighterState,
    now: Instant,
    props: &HighlighterProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let overflow = annotation_overflow(props);

    let build_overlay = || {
        container(effect_canvas::<_, Message>(
            HighlighterProgram {
                progress: state.progress_at(now),
                props: *props,
                seed: state.seed(),
            },
            EffectCanvasProps::new()
                .width(Length::Fill)
                .height(Length::Fill)
                .draw_overflow(overflow),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .clip(false)
        .into()
    };

    let underlays = if matches!(props.action, HighlighterAction::Highlight) {
        vec![build_overlay()]
    } else {
        Vec::new()
    };
    let overlays = if matches!(props.action, HighlighterAction::Highlight) {
        Vec::new()
    } else {
        vec![build_overlay()]
    };

    decorative_surface(
        container(content.into())
            .width(Length::Shrink)
            .height(Length::Shrink)
            .clip(false),
        underlays,
        overlays,
        DecorativeSurfaceProps::new().clip(false),
        theme,
    )
}

/// Vertical draw overflow only — horizontal strokes stay within the text bounds.
fn annotation_overflow(props: &HighlighterProps) -> Padding {
    let options = rough_options(props.action);
    let rough = options.max_randomness_offset * options.roughness + options.bowing + 1.0;

    match props.action {
        HighlighterAction::Highlight => Padding {
            top: rough,
            right: 0.0,
            bottom: rough,
            left: 0.0,
        },
        HighlighterAction::Underline => Padding {
            top: 0.0,
            right: 0.0,
            bottom: props.stroke_width + rough + props.padding,
            left: 0.0,
        },
        _ => Padding {
            top: rough,
            right: rough,
            bottom: rough,
            left: rough,
        },
    }
}

fn text_annotation_rect(draw_bounds: Rectangle, props: &HighlighterProps) -> Rectangle {
    let overflow = annotation_overflow(props);
    let layout_width = draw_bounds.width - overflow.left - overflow.right;
    let layout_height = draw_bounds.height - overflow.top - overflow.bottom;

    Rectangle {
        x: draw_bounds.x + overflow.left,
        y: draw_bounds.y + overflow.top,
        width: layout_width.max(1.0),
        height: layout_height.max(1.0),
    }
}

/// Highlight stroke center and width for iced's top-aligned text line box.
///
/// Browser `getBoundingClientRect` on inline spans tracks ink tighter than iced
/// `min_bounds()`, so centering on the full layout height places the marker high.
fn highlight_stroke_geometry(inner: Rectangle) -> (f32, f32) {
    let top_inset = inner.height * 0.20;
    let bottom_inset = inner.height * 0.08;
    let ink_height = (inner.height - top_inset - bottom_inset).max(1.0);
    let y = inner.y + top_inset + ink_height * 0.5;
    let stroke_width = ink_height * 0.95;
    (y, stroke_width)
}

struct HighlighterProgram {
    progress: f32,
    props: HighlighterProps,
    seed: u32,
}

impl<Message> EffectProgram<Message> for HighlighterProgram {
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
        let progress = self.progress.clamp(0.0, 1.0);
        let iterations = self.props.iterations.max(1);
        let options = rough_options(self.props.action);
        let inner = text_annotation_rect(bounds, &self.props);
        let mut rng = RoughRng::new(self.seed);

        for iteration in 0..iterations {
            let local = (progress - iteration as f32 * 0.08).clamp(0.0, 1.0);
            if local <= 0.0 {
                continue;
            }

            let rtl = iteration % 2 == 1;

            match self.props.action {
                HighlighterAction::Highlight => {
                    let (y, stroke_width) = highlight_stroke_geometry(inner);
                    let (from, to) = horizontal_segment(inner.x, inner.x + inner.width, y, rtl);
                    draw_rough_segment(
                        &mut frame,
                        from,
                        to,
                        self.props.color,
                        stroke_width,
                        local,
                        &options,
                        &mut rng,
                        canvas::LineCap::Butt,
                        true,
                    );
                }
                HighlighterAction::Underline => {
                    let y = inner.y + inner.height + self.props.padding;
                    let (from, to) = horizontal_segment(inner.x, inner.x + inner.width, y, rtl);
                    draw_rough_segment(
                        &mut frame,
                        from,
                        to,
                        self.props.color,
                        self.props.stroke_width,
                        local,
                        &options,
                        &mut rng,
                        canvas::LineCap::Butt,
                        true,
                    );
                }
                HighlighterAction::StrikeThrough => {
                    let y = inner.y + inner.height * 0.5;
                    let (from, to) = horizontal_segment(inner.x, inner.x + inner.width, y, rtl);
                    draw_rough_segment(
                        &mut frame,
                        from,
                        to,
                        self.props.color,
                        self.props.stroke_width,
                        local,
                        &options,
                        &mut rng,
                        canvas::LineCap::Butt,
                        true,
                    );
                }
                HighlighterAction::CrossedOff => {
                    draw_rough_segment(
                        &mut frame,
                        Point::new(inner.x, inner.y),
                        Point::new(inner.x + inner.width, inner.y + inner.height),
                        self.props.color,
                        self.props.stroke_width,
                        local,
                        &options,
                        &mut rng,
                        canvas::LineCap::Round,
                        false,
                    );
                    draw_rough_segment(
                        &mut frame,
                        Point::new(inner.x + inner.width, inner.y),
                        Point::new(inner.x, inner.y + inner.height),
                        self.props.color,
                        self.props.stroke_width,
                        local,
                        &options,
                        &mut rng,
                        canvas::LineCap::Round,
                        false,
                    );
                }
                HighlighterAction::Box => {
                    draw_rough_rectangle(
                        &mut frame,
                        inner,
                        self.props.color,
                        self.props.stroke_width,
                        local,
                        &options,
                        &mut rng,
                    );
                }
                HighlighterAction::Circle => {
                    draw_rough_ellipse(
                        &mut frame,
                        Point::new(inner.x + inner.width * 0.5, inner.y + inner.height * 0.5),
                        inner.width,
                        inner.height,
                        self.props.color,
                        self.props.stroke_width,
                        local,
                        &options,
                        &mut rng,
                    );
                }
                HighlighterAction::Bracket => {
                    let x = self.props.stroke_width * 2.0;
                    let inset = self.props.stroke_width;
                    draw_rough_segment(
                        &mut frame,
                        Point::new(inner.x + x, inner.y + inset),
                        Point::new(inner.x + x, inner.y + inner.height - inset),
                        self.props.color,
                        self.props.stroke_width,
                        local,
                        &options,
                        &mut rng,
                        canvas::LineCap::Round,
                        false,
                    );
                    draw_rough_segment(
                        &mut frame,
                        Point::new(inner.x + inner.width - x, inner.y + inset),
                        Point::new(inner.x + inner.width - x, inner.y + inner.height - inset),
                        self.props.color,
                        self.props.stroke_width,
                        local,
                        &options,
                        &mut rng,
                        canvas::LineCap::Round,
                        false,
                    );
                }
            }
        }

        vec![frame.into_geometry()]
    }
}

#[derive(Clone, Copy, Debug)]
struct RoughOptions {
    roughness: f32,
    bowing: f32,
    max_randomness_offset: f32,
    disable_multi_stroke: bool,
}

fn rough_options(action: HighlighterAction) -> RoughOptions {
    match action {
        HighlighterAction::Highlight => RoughOptions {
            roughness: 3.0,
            bowing: 1.0,
            max_randomness_offset: 2.0,
            disable_multi_stroke: true,
        },
        _ => RoughOptions {
            roughness: 1.5,
            bowing: 1.0,
            max_randomness_offset: 2.0,
            disable_multi_stroke: true,
        },
    }
}

/// rough-notation assigns one `randomSeed()` per annotation; iterations share the same RNG.
fn resolve_seed(props: &HighlighterProps, previous: Option<u32>) -> u32 {
    if let Some(seed) = props.seed {
        seed
    } else if let Some(previous) = previous {
        previous
    } else {
        random_seed()
    }
}

fn random_seed() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU32 = AtomicU32::new(1);

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u32)
        .unwrap_or(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mixed = nanos
        .wrapping_mul(16_807)
        .wrapping_add(count.wrapping_mul(48_271))
        .wrapping_add(1);
    mixed | 1
}

#[derive(Clone, Copy, Debug)]
struct RoughRng {
    seed: i32,
}

impl RoughRng {
    fn new(seed: u32) -> Self {
        Self {
            seed: seed as i32 | 1,
        }
    }

    fn next(&mut self) -> f32 {
        self.seed = (self.seed.wrapping_mul(16_807)) % 2_147_483_647;
        (self.seed - 1) as f32 / 2_147_483_646.0
    }
}

#[derive(Clone, Copy, Debug)]
struct CubicBezier {
    start: Point,
    control1: Point,
    control2: Point,
    end: Point,
}

fn horizontal_segment(x0: f32, x1: f32, y: f32, rtl: bool) -> (Point, Point) {
    if rtl {
        (Point::new(x1, y), Point::new(x0, y))
    } else {
        (Point::new(x0, y), Point::new(x1, y))
    }
}

fn offset(min: f32, max: f32, roughness: f32, roughness_gain: f32, rng: &mut RoughRng) -> f32 {
    roughness * roughness_gain * (rng.next() * (max - min) + min)
}

fn offset_opt(value: f32, roughness: f32, roughness_gain: f32, rng: &mut RoughRng) -> f32 {
    offset(-value, value, roughness, roughness_gain, rng)
}

fn roughness_gain(length: f32) -> f32 {
    if length < 200.0 {
        1.0
    } else if length > 500.0 {
        0.4
    } else {
        -0.001_666_8 * length + 1.233_334
    }
}

fn rough_cubic(
    from: Point,
    to: Point,
    options: &RoughOptions,
    overlay: bool,
    rng: &mut RoughRng,
) -> CubicBezier {
    let x1 = from.x;
    let y1 = from.y;
    let x2 = to.x;
    let y2 = to.y;
    let length_sq = (x1 - x2).powi(2) + (y1 - y2).powi(2);
    let length = length_sq.sqrt();
    let gain = roughness_gain(length);
    let mut offset_limit = options.max_randomness_offset;
    if offset_limit * offset_limit * 100.0 > length_sq {
        offset_limit = length / 10.0;
    }
    let half_offset = offset_limit / 2.0;
    let diverge_point = 0.2 + rng.next() * 0.2;
    let mut mid_disp_x = options.bowing * options.max_randomness_offset * (y2 - y1) / 200.0;
    let mut mid_disp_y = options.bowing * options.max_randomness_offset * (x1 - x2) / 200.0;
    mid_disp_x = offset_opt(mid_disp_x, options.roughness, gain, rng);
    mid_disp_y = offset_opt(mid_disp_y, options.roughness, gain, rng);

    if overlay {
        let sh = offset_opt(half_offset, options.roughness, gain, rng);
        let c1x = mid_disp_x
            + x1
            + (x2 - x1) * diverge_point
            + offset_opt(half_offset, options.roughness, gain, rng);
        let c1y = mid_disp_y
            + y1
            + (y2 - y1) * diverge_point
            + offset_opt(half_offset, options.roughness, gain, rng);
        let c2x = mid_disp_x
            + x1
            + 2.0 * (x2 - x1) * diverge_point
            + offset_opt(half_offset, options.roughness, gain, rng);
        let c2y = mid_disp_y
            + y1
            + 2.0 * (y2 - y1) * diverge_point
            + offset_opt(half_offset, options.roughness, gain, rng);
        let ex = x2 + offset_opt(half_offset, options.roughness, gain, rng);
        let ey = y2 + offset_opt(half_offset, options.roughness, gain, rng);
        CubicBezier {
            start: Point::new(
                x1 + sh,
                y1 + offset_opt(half_offset, options.roughness, gain, rng),
            ),
            control1: Point::new(c1x, c1y),
            control2: Point::new(c2x, c2y),
            end: Point::new(ex, ey),
        }
    } else {
        let sx = x1 + offset_opt(offset_limit, options.roughness, gain, rng);
        let sy = y1 + offset_opt(offset_limit, options.roughness, gain, rng);
        let c1x = mid_disp_x
            + x1
            + (x2 - x1) * diverge_point
            + offset_opt(offset_limit, options.roughness, gain, rng);
        let c1y = mid_disp_y
            + y1
            + (y2 - y1) * diverge_point
            + offset_opt(offset_limit, options.roughness, gain, rng);
        let c2x = mid_disp_x
            + x1
            + 2.0 * (x2 - x1) * diverge_point
            + offset_opt(offset_limit, options.roughness, gain, rng);
        let c2y = mid_disp_y
            + y1
            + 2.0 * (y2 - y1) * diverge_point
            + offset_opt(offset_limit, options.roughness, gain, rng);
        let ex = x2 + offset_opt(offset_limit, options.roughness, gain, rng);
        let ey = y2 + offset_opt(offset_limit, options.roughness, gain, rng);
        CubicBezier {
            start: Point::new(sx, sy),
            control1: Point::new(c1x, c1y),
            control2: Point::new(c2x, c2y),
            end: Point::new(ex, ey),
        }
    }
}

fn sample_cubic(curve: CubicBezier, steps: usize) -> Vec<Point> {
    (0..=steps)
        .map(|step| {
            let t = step as f32 / steps as f32;
            cubic_point(curve, t)
        })
        .collect()
}

fn cubic_point(curve: CubicBezier, t: f32) -> Point {
    let u = 1.0 - t;
    let tt = t * t;
    let uu = u * u;
    let uuu = uu * u;
    let ttt = tt * t;

    Point::new(
        uuu * curve.start.x
            + 3.0 * uu * t * curve.control1.x
            + 3.0 * u * tt * curve.control2.x
            + ttt * curve.end.x,
        uuu * curve.start.y
            + 3.0 * uu * t * curve.control1.y
            + 3.0 * u * tt * curve.control2.y
            + ttt * curve.end.y,
    )
}

fn trim_polyline(points: &[Point], progress: f32) -> Vec<Point> {
    if points.len() < 2 || progress >= 1.0 {
        return points.to_vec();
    }

    let mut lengths = Vec::with_capacity(points.len() - 1);
    let mut total = 0.0;
    for window in points.windows(2) {
        let segment = distance(window[0], window[1]);
        lengths.push(segment);
        total += segment;
    }

    let target = total * progress.clamp(0.0, 1.0);
    let mut remaining = target;
    let mut trimmed = vec![points[0]];
    for (index, segment) in lengths.iter().enumerate() {
        if remaining <= 0.0 {
            break;
        }
        let start = points[index];
        let end = points[index + 1];
        if remaining >= *segment {
            trimmed.push(end);
            remaining -= segment;
        } else {
            let t = remaining / segment.max(f32::EPSILON);
            trimmed.push(lerp_point(start, end, t));
            remaining = 0.0;
        }
    }
    trimmed
}

fn stroke_polyline(
    frame: &mut canvas::Frame,
    points: &[Point],
    color: Color,
    width: f32,
    line_cap: canvas::LineCap,
) {
    if points.len() < 2 {
        return;
    }

    let path = canvas::Path::new(|builder| {
        builder.move_to(points[0]);
        for point in points.iter().skip(1) {
            builder.line_to(*point);
        }
    });

    frame.stroke(
        &path,
        canvas::Stroke::default()
            .with_width(width.max(1.0))
            .with_color(color)
            .with_line_cap(line_cap)
            .with_line_join(canvas::LineJoin::Round),
    );
}

fn stroke_bezier(
    frame: &mut canvas::Frame,
    curve: CubicBezier,
    color: Color,
    width: f32,
    progress: f32,
    line_cap: canvas::LineCap,
) {
    if progress >= 1.0 {
        let path = canvas::Path::new(|builder| {
            builder.move_to(curve.start);
            builder.bezier_curve_to(curve.control1, curve.control2, curve.end);
        });
        frame.stroke(
            &path,
            canvas::Stroke::default()
                .with_width(width.max(1.0))
                .with_color(color)
                .with_line_cap(line_cap)
                .with_line_join(canvas::LineJoin::Round),
        );
        return;
    }

    let points = sample_cubic(curve, 32);
    let points = trim_polyline(&points, progress);
    stroke_polyline(frame, &points, color, width, line_cap);
}

fn clamp_horizontal_curve(curve: &mut CubicBezier, from: Point, to: Point) {
    let min_x = from.x.min(to.x);
    let max_x = from.x.max(to.x);
    curve.start.x = curve.start.x.clamp(min_x, max_x);
    curve.end.x = curve.end.x.clamp(min_x, max_x);
    curve.control1.x = curve.control1.x.clamp(min_x, max_x);
    curve.control2.x = curve.control2.x.clamp(min_x, max_x);
}

#[allow(clippy::too_many_arguments)]
fn draw_rough_segment(
    frame: &mut canvas::Frame,
    from: Point,
    to: Point,
    color: Color,
    width: f32,
    progress: f32,
    options: &RoughOptions,
    rng: &mut RoughRng,
    line_cap: canvas::LineCap,
    clamp_horizontal: bool,
) {
    let mut curves = vec![rough_cubic(from, to, options, false, rng)];
    if !options.disable_multi_stroke {
        curves.push(rough_cubic(from, to, options, true, rng));
    }

    for mut curve in curves {
        if clamp_horizontal {
            clamp_horizontal_curve(&mut curve, from, to);
        }
        stroke_bezier(frame, curve, color, width, progress, line_cap);
    }
}

fn draw_rough_rectangle(
    frame: &mut canvas::Frame,
    bounds: Rectangle,
    color: Color,
    width: f32,
    progress: f32,
    options: &RoughOptions,
    rng: &mut RoughRng,
) {
    let top_left = bounds.position();
    let top_right = Point::new(bounds.x + bounds.width, bounds.y);
    let bottom_right = Point::new(bounds.x + bounds.width, bounds.y + bounds.height);
    let bottom_left = Point::new(bounds.x, bounds.y + bounds.height);
    let corners = [top_left, top_right, bottom_right, bottom_left, top_left];

    for window in corners.windows(2) {
        draw_rough_segment(
            frame,
            window[0],
            window[1],
            color,
            width,
            progress,
            options,
            rng,
            canvas::LineCap::Round,
            false,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_rough_ellipse(
    frame: &mut canvas::Frame,
    center: Point,
    width: f32,
    height: f32,
    color: Color,
    stroke_width: f32,
    progress: f32,
    options: &RoughOptions,
    rng: &mut RoughRng,
) {
    let rx = width * 0.5;
    let ry = height * 0.5;
    let segments = 36;
    let mut points = Vec::with_capacity(segments + 1);
    for index in 0..=segments {
        let angle = index as f32 / segments as f32 * std::f32::consts::TAU;
        let wobble = offset_opt(0.5, options.roughness, 1.0, rng);
        points.push(Point::new(
            center.x + angle.cos() * rx + wobble,
            center.y + angle.sin() * ry + wobble,
        ));
    }

    let points = trim_polyline(&points, progress);
    stroke_polyline(frame, &points, color, stroke_width, canvas::LineCap::Round);
}

fn distance(a: Point, b: Point) -> f32 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    (dx * dx + dy * dy).sqrt()
}

fn lerp_point(a: Point, b: Point, t: f32) -> Point {
    Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::time::Duration;

    #[test]
    fn highlighter_defaults_match_docs() {
        let props = HighlighterProps::new();

        assert_eq!(props.action, HighlighterAction::Highlight);
        assert_eq!(props.stroke_width, 1.5);
        assert_eq!(props.animation_duration_ms, 600);
        assert_eq!(props.iterations, 2);
    }

    #[test]
    fn highlighter_progress_advances() {
        let props = HighlighterProps::new();
        let mut state = HighlighterState::new(&props);
        let start = Instant::now();

        state.tick(start, &props);
        assert!(state.progress_at(start + Duration::from_millis(700)) > 0.9);
    }

    #[test]
    fn highlighter_reset_rolls_new_seed() {
        let props = HighlighterProps::new();
        let mut state = HighlighterState::new(&props);
        let start = Instant::now();
        state.tick(start, &props);
        let seed_before = state.seed();
        let progress_before = state.progress_at(start + Duration::from_millis(700));

        state.reset(start + Duration::from_millis(800), &props);
        assert_ne!(state.seed(), seed_before);
        assert!(state.progress_at(start + Duration::from_millis(850)) < 0.2);
        assert!(progress_before > 0.9);
    }

    #[test]
    fn trim_polyline_respects_progress() {
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(20.0, 0.0),
        ];
        let trimmed = trim_polyline(&points, 0.5);
        assert_eq!(trimmed.len(), 2);
        assert!((trimmed[1].x - 10.0).abs() < 0.01);
    }

    #[test]
    fn rough_cubic_is_deterministic_for_seed() {
        let options = rough_options(HighlighterAction::Underline);
        let mut rng_a = RoughRng::new(7);
        let mut rng_b = RoughRng::new(7);
        let curve_a = rough_cubic(
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            &options,
            false,
            &mut rng_a,
        );
        let curve_b = rough_cubic(
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            &options,
            false,
            &mut rng_b,
        );
        assert_eq!(curve_a.start, curve_b.start);
        assert_eq!(curve_a.end, curve_b.end);
    }

    #[test]
    fn annotation_overflow_keeps_inline_strokes_inside_horizontally() {
        let overflow = annotation_overflow(&HighlighterProps::new());
        assert_eq!(overflow.left, 0.0);
        assert_eq!(overflow.right, 0.0);
        assert!(overflow.top > 0.0 && overflow.bottom > 0.0);
        assert_eq!(overflow.top, overflow.bottom);
    }

    #[test]
    fn highlight_stroke_sits_below_layout_center() {
        let inner = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 40.0,
        };
        let (y, stroke_width) = highlight_stroke_geometry(inner);
        assert!(y > inner.y + inner.height * 0.5);
        assert!(stroke_width < inner.height * 0.95);
    }

    #[test]
    fn text_annotation_rect_matches_layout_width() {
        let props = HighlighterProps::new();
        let overflow = annotation_overflow(&props);
        let draw_bounds = Rectangle::with_size(iced::Size::new(
            100.0 + overflow.left + overflow.right,
            40.0 + overflow.top + overflow.bottom,
        ));
        let inner = text_annotation_rect(draw_bounds, &props);
        assert_eq!(inner.x, overflow.left);
        assert_eq!(inner.y, overflow.top);
        assert_eq!(inner.width, 100.0);
        assert_eq!(inner.height, 40.0);
    }
}
