//! Magic UI / canvas-confetti style confetti cannon for iced.

use crate::primitives::{
    CONFETTI_OVAL_SCALAR, CONFETTI_SIZE_FACTOR, CONFETTI_STAR_INNER_FACTOR,
    CONFETTI_STAR_OUTER_FACTOR, ConfettiParticle, EffectCanvasProps, EffectProgram, ParticleShape,
    confetti_particles, effect_canvas, sample_confetti_particle,
};
use iced::mouse;
use iced::time::Instant;
use iced::widget::button;
use iced::widget::canvas;
use iced::{Color, Element, Length, Padding, Point, Rectangle, Renderer, Size, Vector};
use std::f32::consts::{PI, TAU};
use std::fmt;
use std::time::Duration;

const MAX_COLORS: usize = 8;
const MAX_SHAPES: usize = 8;
/// Approximate canvas-confetti tick duration at ~60 fps.
const MS_PER_TICK: u64 = 16;

/// Default palette from canvas-confetti.
const DEFAULT_COLORS: [Color; 7] = [
    Color::from_rgb8(0x26, 0xCC, 0xFF),
    Color::from_rgb8(0xA2, 0x5A, 0xFD),
    Color::from_rgb8(0xFF, 0x5E, 0x7E),
    Color::from_rgb8(0x88, 0xFF, 0x5A),
    Color::from_rgb8(0xFC, 0xFF, 0x42),
    Color::from_rgb8(0xFF, 0xA6, 0x2D),
    Color::from_rgb8(0xFF, 0x36, 0xFF),
];

const BUTTON_BACKGROUND: Color = Color::from_rgb8(0x25, 0x63, 0xEB);
const BUTTON_BACKGROUND_HOVER: Color = Color::from_rgb8(0x1D, 0x4E, 0xD8);
const BUTTON_BORDER_RADIUS: f32 = 8.0;

/// Particle geometry matching Magic UI / canvas-confetti shapes.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConfettiShape {
    /// Filled circle.
    Circle,
    /// Axis-aligned square (rotated in flight).
    Square,
    /// Five-point star polygon.
    Star,
    /// SVG path data — rendered via a small path parser (M/L/A/Z) for Magic UI shapes.
    Path {
        /// Static SVG path string (Magic UI custom shapes).
        data: &'static str,
    },
    /// Text / emoji glyph drawn with canvas `fill_text`.
    Text {
        /// Static text (e.g. `"🦄"`).
        text: &'static str,
    },
}

impl ConfettiShape {
    fn to_particle_shape(self) -> ParticleShape {
        match self {
            Self::Circle | Self::Text { .. } => ParticleShape::Circle,
            Self::Square | Self::Path { .. } => ParticleShape::Square,
            Self::Star => ParticleShape::Star,
        }
    }
}

/// Magic UI / canvas-confetti fire options.
#[must_use = "builders do nothing unless passed to ConfettiState::fire"]
#[derive(Clone, Copy, PartialEq)]
pub struct ConfettiOptions {
    particle_count: usize,
    angle: f32,
    spread: f32,
    start_velocity: f32,
    decay: f32,
    gravity: f32,
    drift: f32,
    ticks: u64,
    origin_x: Option<f32>,
    origin_y: Option<f32>,
    colors: [Color; MAX_COLORS],
    color_len: u8,
    scalar: f32,
    shapes: [ConfettiShape; MAX_SHAPES],
    shape_len: u8,
    seed: u64,
}

impl fmt::Debug for ConfettiOptions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConfettiOptions")
            .field("particle_count", &self.particle_count)
            .field("angle", &self.angle)
            .field("spread", &self.spread)
            .field("start_velocity", &self.start_velocity)
            .field("decay", &self.decay)
            .field("gravity", &self.gravity)
            .field("drift", &self.drift)
            .field("ticks", &self.ticks)
            .field("origin_x", &self.origin_x)
            .field("origin_y", &self.origin_y)
            .field("colors", &self.colors())
            .field("scalar", &self.scalar)
            .field("shapes", &self.shapes())
            .field("seed", &self.seed)
            .finish()
    }
}

impl Default for ConfettiOptions {
    fn default() -> Self {
        let mut colors = [Color::TRANSPARENT; MAX_COLORS];
        colors[..DEFAULT_COLORS.len()].copy_from_slice(&DEFAULT_COLORS);

        let mut shapes = [ConfettiShape::Circle; MAX_SHAPES];
        shapes[0] = ConfettiShape::Square;
        shapes[1] = ConfettiShape::Circle;

        Self {
            particle_count: 50,
            angle: 90.0,
            spread: 45.0,
            start_velocity: 45.0,
            decay: 0.9,
            gravity: 1.0,
            drift: 0.0,
            ticks: 200,
            origin_x: None,
            origin_y: None,
            colors,
            color_len: DEFAULT_COLORS.len() as u8,
            scalar: 1.0,
            shapes,
            shape_len: 2,
            seed: 0,
        }
    }
}

impl ConfettiOptions {
    /// Creates options with canvas-confetti defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets particle count (at least 1).
    #[must_use = "builder setters return updated configuration"]
    pub fn with_particle_count(mut self, particle_count: usize) -> Self {
        self.particle_count = particle_count.max(1);
        self
    }

    /// Launch angle in degrees (90 = straight up).
    #[must_use = "builder setters return updated configuration"]
    pub fn with_angle(mut self, angle: f32) -> Self {
        self.angle = finite_or(angle, 90.0);
        self
    }

    /// Spread cone in degrees.
    #[must_use = "builder setters return updated configuration"]
    pub fn with_spread(mut self, spread: f32) -> Self {
        self.spread = finite_or(spread, 0.0).max(0.0);
        self
    }

    /// Initial speed.
    #[must_use = "builder setters return updated configuration"]
    pub fn with_start_velocity(mut self, start_velocity: f32) -> Self {
        self.start_velocity = finite_or(start_velocity, 0.0).max(0.0);
        self
    }

    /// Velocity decay in `0..=1`.
    #[must_use = "builder setters return updated configuration"]
    pub fn with_decay(mut self, decay: f32) -> Self {
        self.decay = finite_or(decay, 0.9).clamp(0.0, 1.0);
        self
    }

    /// Gravity multiplier (1 = full).
    #[must_use = "builder setters return updated configuration"]
    pub fn with_gravity(mut self, gravity: f32) -> Self {
        self.gravity = finite_or(gravity, 1.0);
        self
    }

    /// Horizontal drift.
    #[must_use = "builder setters return updated configuration"]
    pub fn with_drift(mut self, drift: f32) -> Self {
        self.drift = finite_or(drift, 0.0);
        self
    }

    /// Lifetime in canvas-confetti ticks (mapped to ms at ~60 fps).
    #[must_use = "builder setters return updated configuration"]
    pub fn with_ticks(mut self, ticks: u64) -> Self {
        self.ticks = ticks.max(1);
        self
    }

    /// Normalized origin `x` in `[0, 1]` for [`ConfettiState::fire`].
    #[must_use = "builder setters return updated configuration"]
    pub fn with_origin_x(mut self, origin_x: f32) -> Self {
        self.origin_x = Some(finite_or(origin_x, 0.5).clamp(0.0, 1.0));
        self
    }

    /// Normalized origin `y` in `[0, 1]` for [`ConfettiState::fire`].
    #[must_use = "builder setters return updated configuration"]
    pub fn with_origin_y(mut self, origin_y: f32) -> Self {
        self.origin_y = Some(finite_or(origin_y, 0.5).clamp(0.0, 1.0));
        self
    }

    /// Clears normalized origin so [`ConfettiState::fire`] uses canvas center.
    #[must_use = "builder setters return updated configuration"]
    pub fn clear_origin(mut self) -> Self {
        self.origin_x = None;
        self.origin_y = None;
        self
    }

    /// Particle colors (keeps at most eight). Empty slice leaves palette unchanged.
    #[must_use = "builder setters return updated configuration"]
    pub fn with_colors(mut self, colors: &[Color]) -> Self {
        if colors.is_empty() {
            return self;
        }

        let color_len = colors.len().min(MAX_COLORS);
        self.colors[..color_len].copy_from_slice(&colors[..color_len]);
        self.color_len = color_len as u8;
        self
    }

    /// Size scalar (≥ 0).
    #[must_use = "builder setters return updated configuration"]
    pub fn with_scalar(mut self, scalar: f32) -> Self {
        self.scalar = finite_or(scalar, 0.0).max(0.0);
        self
    }

    /// Shape mix (keeps at most eight). Empty slice leaves shapes unchanged.
    #[must_use = "builder setters return updated configuration"]
    pub fn with_shapes(mut self, shapes: &[ConfettiShape]) -> Self {
        if shapes.is_empty() {
            return self;
        }

        let shape_len = shapes.len().min(MAX_SHAPES);
        self.shapes[..shape_len].copy_from_slice(&shapes[..shape_len]);
        self.shape_len = shape_len as u8;
        self
    }

    /// Deterministic spawn seed.
    #[must_use = "builder setters return updated configuration"]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Number of particles.
    pub fn particle_count(&self) -> usize {
        self.particle_count
    }

    /// Launch angle in degrees.
    pub fn angle(&self) -> f32 {
        self.angle
    }

    /// Spread in degrees.
    pub fn spread(&self) -> f32 {
        self.spread
    }

    /// Initial velocity.
    pub fn start_velocity(&self) -> f32 {
        self.start_velocity
    }

    /// Decay factor.
    pub fn decay(&self) -> f32 {
        self.decay
    }

    /// Gravity multiplier.
    pub fn gravity(&self) -> f32 {
        self.gravity
    }

    /// Drift.
    pub fn drift(&self) -> f32 {
        self.drift
    }

    /// Lifetime ticks.
    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Optional normalized origin x.
    pub fn origin_x(&self) -> Option<f32> {
        self.origin_x
    }

    /// Optional normalized origin y.
    pub fn origin_y(&self) -> Option<f32> {
        self.origin_y
    }

    /// Active color palette.
    pub fn colors(&self) -> &[Color] {
        &self.colors[..usize::from(self.color_len)]
    }

    /// Size scalar.
    pub fn scalar(&self) -> f32 {
        self.scalar
    }

    /// Active shapes.
    pub fn shapes(&self) -> &[ConfettiShape] {
        &self.shapes[..usize::from(self.shape_len)]
    }

    /// Spawn seed.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    fn life_ms(&self) -> u64 {
        self.ticks.saturating_mul(MS_PER_TICK).max(1)
    }

    fn particle_shapes_buf(&self) -> ([ParticleShape; MAX_SHAPES], usize) {
        let mut buf = [ParticleShape::Circle; MAX_SHAPES];
        let len = self.shapes().len().max(1);
        for (index, shape) in self.shapes().iter().enumerate() {
            buf[index] = shape.to_particle_shape();
        }
        if self.shapes().is_empty() {
            buf[0] = ParticleShape::Square;
            buf[1] = ParticleShape::Circle;
            (buf, 2)
        } else {
            (buf, len)
        }
    }
}

/// Full-stage confetti canvas overlay (Magic UI `Confetti` component).
#[must_use = "builders do nothing unless turned into an iced Element"]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Confetti {
    width: Length,
    height: Length,
}

impl Default for Confetti {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            height: Length::Fill,
        }
    }
}

impl Confetti {
    /// Creates a fill-sized confetti canvas.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets canvas width.
    #[must_use = "builder setters return updated configuration"]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets canvas height.
    #[must_use = "builder setters return updated configuration"]
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Builds the confetti canvas for the supplied state and clock.
    pub fn into_element<'a, Message: 'a>(
        self,
        state: &'a ConfettiState,
        now: Instant,
    ) -> Element<'a, Message> {
        effect_canvas::<_, Message>(
            ConfettiProgram { state, now },
            EffectCanvasProps::new()
                .width(self.width)
                .height(self.height),
        )
        .into()
    }

    /// Alias for [`Self::into_element`].
    pub fn view<'a, Message: 'a>(
        self,
        state: &'a ConfettiState,
        now: Instant,
    ) -> Element<'a, Message> {
        self.into_element(state, now)
    }
}

/// Button that carries [`ConfettiOptions`] and emits a press message.
///
/// Confetti is not fired automatically — on press the host should call
/// [`ConfettiState::fire_at`] with [`Self::options`] and the button center
/// (or top-center) in canvas-local coordinates.
#[must_use = "builders do nothing unless turned into an iced Element"]
pub struct ConfettiButton<'a, Message> {
    content: Element<'a, Message>,
    options: ConfettiOptions,
    on_press: Option<Message>,
    padding: Padding,
    width: Length,
    height: Length,
}

impl<'a, Message> fmt::Debug for ConfettiButton<'a, Message> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConfettiButton")
            .field("options", &self.options)
            .field("on_press", &self.on_press.is_some())
            .field("padding", &self.padding)
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

impl<'a, Message> ConfettiButton<'a, Message> {
    /// Creates a button with canvas-confetti defaults.
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            options: ConfettiOptions::new(),
            on_press: None,
            padding: Padding::from([12.0, 24.0]),
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    /// Returns the options used when firing from this button.
    pub fn options(&self) -> ConfettiOptions {
        self.options
    }

    /// Replaces fire options.
    #[must_use = "builder setters return updated configuration"]
    pub fn with_options(mut self, options: ConfettiOptions) -> Self {
        self.options = options;
        self
    }

    /// Sets the press handler. The host should fire confetti on this message.
    #[must_use = "builder setters return updated configuration"]
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Alias for [`Self::on_press`].
    #[must_use = "builder setters return updated configuration"]
    pub fn on_fire<F>(mut self, fire: F) -> Self
    where
        F: FnOnce() -> Message,
    {
        self.on_press = Some(fire());
        self
    }

    #[must_use = "builder setters return updated configuration"]
    pub fn particle_count(mut self, particle_count: usize) -> Self {
        self.options = self.options.with_particle_count(particle_count);
        self
    }

    #[must_use = "builder setters return updated configuration"]
    pub fn angle(mut self, angle: f32) -> Self {
        self.options = self.options.with_angle(angle);
        self
    }

    #[must_use = "builder setters return updated configuration"]
    pub fn spread(mut self, spread: f32) -> Self {
        self.options = self.options.with_spread(spread);
        self
    }

    #[must_use = "builder setters return updated configuration"]
    pub fn start_velocity(mut self, start_velocity: f32) -> Self {
        self.options = self.options.with_start_velocity(start_velocity);
        self
    }

    #[must_use = "builder setters return updated configuration"]
    pub fn colors(mut self, colors: &[Color]) -> Self {
        self.options = self.options.with_colors(colors);
        self
    }

    #[must_use = "builder setters return updated configuration"]
    pub fn shapes(mut self, shapes: &[ConfettiShape]) -> Self {
        self.options = self.options.with_shapes(shapes);
        self
    }

    #[must_use = "builder setters return updated configuration"]
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    #[must_use = "builder setters return updated configuration"]
    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    #[must_use = "builder setters return updated configuration"]
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Builds the styled button element.
    pub fn into_element(self) -> Element<'a, Message>
    where
        Message: Clone + 'a,
    {
        let padding = self.padding;
        let width = self.width;
        let height = self.height;

        let mut control = button(self.content)
            .padding(padding)
            .style(|_, status| confetti_button_style(status));

        if let Some(message) = self.on_press {
            control = control.on_press(message);
        }

        iced::widget::container(control)
            .width(width)
            .height(height)
            .into()
    }

    /// Alias for [`Self::into_element`].
    pub fn view(self) -> Element<'a, Message>
    where
        Message: Clone + 'a,
    {
        self.into_element()
    }
}

fn confetti_button_style(status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => BUTTON_BACKGROUND_HOVER,
        button::Status::Disabled => BUTTON_BACKGROUND.scale_alpha(0.5),
        button::Status::Active => BUTTON_BACKGROUND,
    };

    button::Style {
        background: Some(iced::Background::Color(background)),
        text_color: Color::WHITE,
        border: iced::Border {
            radius: BUTTON_BORDER_RADIUS.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: iced::Shadow::default(),
        snap: false,
    }
}

/// Runtime state for active confetti bursts.
#[derive(Clone, Debug, Default)]
pub struct ConfettiState {
    bursts: Vec<ConfettiBurst>,
    nonce: u64,
}

impl ConfettiState {
    /// Creates an empty confetti state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Fires a burst using normalized `origin` from options (default center) mapped to `canvas_size`.
    pub fn fire(&mut self, now: Instant, opts: &ConfettiOptions, canvas_size: Size) {
        let width = finite_or(canvas_size.width, 0.0).max(0.0);
        let height = finite_or(canvas_size.height, 0.0).max(0.0);
        let origin = Point::new(
            opts.origin_x().unwrap_or(0.5) * width,
            opts.origin_y().unwrap_or(0.5) * height,
        );
        self.fire_at(now, opts, origin);
    }

    /// Fires a burst at an absolute canvas-local origin (e.g. measured button center).
    ///
    /// Bursts accumulate — call [`Self::clear`] first when replacing a previous shot.
    pub fn fire_at(&mut self, now: Instant, opts: &ConfettiOptions, origin: Point) {
        let origin = Point::new(finite_or(origin.x, 0.0), finite_or(origin.y, 0.0));
        self.bursts
            .push(ConfettiBurst::new(now, opts, self.nonce, origin));
        self.nonce = self.nonce.wrapping_add(1);
    }

    /// Removes all active bursts.
    pub fn clear(&mut self) {
        self.bursts.clear();
    }

    /// Drops bursts whose particles have all expired.
    pub fn tick(&mut self, now: Instant) {
        self.bursts.retain(|burst| {
            let elapsed_ms = now.saturating_duration_since(burst.started_at).as_millis() as u64;
            burst
                .particles
                .iter()
                .any(|particle| elapsed_ms < particle.physics.life_ms)
        });
    }

    /// Number of active bursts.
    pub fn burst_count(&self) -> usize {
        self.bursts.len()
    }

    /// Total live particles across bursts (for tests).
    pub fn particle_count(&self) -> usize {
        self.bursts.iter().map(|burst| burst.particles.len()).sum()
    }

    /// Samples the first particle of the first burst at `elapsed` (for tests).
    pub fn sample_first(
        &self,
        elapsed: Duration,
    ) -> Option<(Point, f32, ParticleShape)> {
        let burst = self.bursts.first()?;
        let particle = burst.particles.first()?;
        let elapsed_ms = elapsed.as_millis() as u64;
        sample_confetti_particle(
            particle.physics,
            elapsed_ms,
            burst.gravity,
            burst.decay,
            burst.drift,
        )
        .map(|sample| (sample.position, sample.opacity, sample.shape))
    }
}

#[derive(Clone, Debug)]
struct ConfettiBurst {
    started_at: Instant,
    gravity: f32,
    decay: f32,
    drift: f32,
    particles: Vec<LiveParticle>,
}

#[derive(Clone, Copy, Debug)]
struct LiveParticle {
    physics: ConfettiParticle,
    color: Color,
    shape: ConfettiShape,
}

impl ConfettiBurst {
    fn new(started_at: Instant, opts: &ConfettiOptions, nonce: u64, origin: Point) -> Self {
        let (shape_buf, shape_len) = opts.particle_shapes_buf();
        let mut physics = confetti_particles(
            opts.particle_count(),
            origin,
            opts.seed().wrapping_add(nonce),
            opts.angle(),
            opts.spread(),
            opts.start_velocity(),
            opts.scalar(),
            &shape_buf[..shape_len],
        );

        let life_ms = opts.life_ms();
        let colors = opts.colors();
        let color_len = colors.len().max(1);
        let shapes = opts.shapes();
        let shape_fallback = [ConfettiShape::Square, ConfettiShape::Circle];
        let shape_src = if shapes.is_empty() {
            &shape_fallback[..]
        } else {
            shapes
        };

        let particles = physics
            .drain(..)
            .enumerate()
            .map(|(index, mut particle)| {
                particle.life_ms = life_ms;
                LiveParticle {
                    physics: particle,
                    color: colors[index % color_len],
                    shape: shape_src[index % shape_src.len()],
                }
            })
            .collect();

        Self {
            started_at,
            gravity: opts.gravity(),
            decay: opts.decay(),
            drift: opts.drift(),
            particles,
        }
    }
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

fn star_path(outer_radius: f32, inner_radius: f32) -> canvas::Path {
    canvas::Path::new(|builder| {
        for i in 0..10 {
            let radius = if i % 2 == 0 {
                outer_radius
            } else {
                inner_radius
            };
            let angle = -PI / 2.0 + f32::from(i as u16) * TAU / 10.0;
            let point = Point::new(radius * angle.cos(), radius * angle.sin());
            if i == 0 {
                builder.move_to(point);
            } else {
                builder.line_to(point);
            }
        }
        builder.close();
    })
}

/// Builds an iced path from Magic UI / canvas-confetti SVG path data, scaled to `size`.
///
/// Known Magic UI demo paths (triangle / square / coin / tree) map to distinct geometry.
/// Other data uses a small `M`/`L`/`Z` parser; unknown input falls back to a triangle.
fn path_from_svg_data(data: &str, size: f32) -> canvas::Path {
    let trimmed = data.trim();
    match trimmed {
        "M0 10 L5 0 L10 10z" | "M0 10 L5 0 L10 10 Z" => triangle_path(size),
        "M0 0 L10 0 L10 10 L0 10 Z" | "M0 0 L10 0 L10 10 L0 10z" => square_path(size),
        "M5 0 A5 5 0 1 0 5 10 A5 5 0 1 0 5 0 Z"
        | "M5 0 A5 5 0 1 0 5 10 A5 5 0 1 0 5 0 z" => {
            canvas::Path::circle(Point::ORIGIN, size / 2.0)
        }
        "M5 0 L10 10 L0 10 Z" | "M5 0 L10 10 L0 10z" => tree_path(size),
        _ => try_parse_polyline_path(trimmed, size).unwrap_or_else(|| triangle_path(size)),
    }
}

fn triangle_path(size: f32) -> canvas::Path {
    let half = size / 2.0;
    canvas::Path::new(|builder| {
        builder.move_to(Point::new(0.0, -half));
        builder.line_to(Point::new(half, half));
        builder.line_to(Point::new(-half, half));
        builder.close();
    })
}

fn square_path(size: f32) -> canvas::Path {
    let half = size / 2.0;
    canvas::Path::new(|builder| {
        builder.move_to(Point::new(-half, -half));
        builder.line_to(Point::new(half, -half));
        builder.line_to(Point::new(half, half));
        builder.line_to(Point::new(-half, half));
        builder.close();
    })
}

/// Tree silhouette: upright triangle with a short trunk so it reads apart from the plain triangle.
fn tree_path(size: f32) -> canvas::Path {
    let half = size / 2.0;
    let trunk_w = size * 0.18;
    let trunk_h = size * 0.22;
    canvas::Path::new(|builder| {
        builder.move_to(Point::new(0.0, -half));
        builder.line_to(Point::new(half, half - trunk_h));
        builder.line_to(Point::new(-half, half - trunk_h));
        builder.close();
        builder.move_to(Point::new(-trunk_w, half - trunk_h));
        builder.line_to(Point::new(trunk_w, half - trunk_h));
        builder.line_to(Point::new(trunk_w, half));
        builder.line_to(Point::new(-trunk_w, half));
        builder.close();
    })
}

fn try_parse_polyline_path(data: &str, size: f32) -> Option<canvas::Path> {
    let commands = tokenize_svg_path(data)?;
    let mut points: Vec<Point> = Vec::new();
    let mut closed = false;
    for command in commands {
        match command {
            SvgCommand::MoveTo(point) | SvgCommand::LineTo(point) => points.push(point),
            SvgCommand::Arc { to, .. } => points.push(to),
            SvgCommand::Close => closed = true,
        }
    }
    if points.len() < 3 {
        return None;
    }

    let mut min = points[0];
    let mut max = points[0];
    for point in &points {
        min.x = min.x.min(point.x);
        min.y = min.y.min(point.y);
        max.x = max.x.max(point.x);
        max.y = max.y.max(point.y);
    }
    let extent = (max.x - min.x).max(max.y - min.y).max(1.0);
    let scale = size / extent;
    let cx = (min.x + max.x) * 0.5;
    let cy = (min.y + max.y) * 0.5;
    let map = |point: Point| Point::new((point.x - cx) * scale, (point.y - cy) * scale);

    Some(canvas::Path::new(move |builder| {
        builder.move_to(map(points[0]));
        for point in points.iter().skip(1) {
            builder.line_to(map(*point));
        }
        if closed {
            builder.close();
        }
    }))
}

#[derive(Clone, Copy)]
enum SvgCommand {
    MoveTo(Point),
    LineTo(Point),
    Arc {
        to: Point,
    },
    Close,
}

fn tokenize_svg_path(data: &str) -> Option<Vec<SvgCommand>> {
    let mut chars = data.chars().peekable();
    let mut commands = Vec::new();
    let mut current = None::<char>;

    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() || ch == ',' {
            chars.next();
            continue;
        }
        if ch.is_ascii_alphabetic() {
            current = Some(chars.next()?);
            if matches!(current, Some('Z' | 'z')) {
                commands.push(SvgCommand::Close);
                current = None;
            }
            continue;
        }
        let cmd = current?;
        match cmd {
            'M' | 'm' => {
                let x = read_number(&mut chars)?;
                let y = read_number(&mut chars)?;
                commands.push(SvgCommand::MoveTo(Point::new(x, y)));
                current = Some(if cmd == 'M' { 'L' } else { 'l' });
            }
            'L' | 'l' => {
                let x = read_number(&mut chars)?;
                let y = read_number(&mut chars)?;
                commands.push(SvgCommand::LineTo(Point::new(x, y)));
            }
            'A' | 'a' => {
                let _rx = read_number(&mut chars)?;
                let _ry = read_number(&mut chars)?;
                let _rot = read_number(&mut chars)?;
                let _large = read_number(&mut chars)?;
                let _sweep = read_number(&mut chars)?;
                let x = read_number(&mut chars)?;
                let y = read_number(&mut chars)?;
                commands.push(SvgCommand::Arc {
                    to: Point::new(x, y),
                });
            }
            'Z' | 'z' => commands.push(SvgCommand::Close),
            _ => return None,
        }
    }

    Some(commands)
}

fn read_number(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Option<f32> {
    while matches!(chars.peek(), Some(c) if c.is_whitespace() || *c == ',') {
        chars.next();
    }
    let mut buf = String::new();
    if matches!(chars.peek(), Some('+' | '-')) {
        buf.push(chars.next()?);
    }
    let mut seen_digit = false;
    let mut seen_dot = false;
    while let Some(&ch) = chars.peek() {
        if ch.is_ascii_digit() {
            seen_digit = true;
            buf.push(ch);
            chars.next();
        } else if ch == '.' && !seen_dot {
            seen_dot = true;
            buf.push(ch);
            chars.next();
        } else {
            break;
        }
    }
    if !seen_digit {
        return None;
    }
    buf.parse().ok()
}

struct ConfettiProgram<'a> {
    state: &'a ConfettiState,
    now: Instant,
}

impl<Message> EffectProgram<Message> for ConfettiProgram<'_> {
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

        for burst in &self.state.bursts {
            let elapsed_ms = self
                .now
                .saturating_duration_since(burst.started_at)
                .as_millis() as u64;

            for particle in &burst.particles {
                let Some(sample) = sample_confetti_particle(
                    particle.physics,
                    elapsed_ms,
                    burst.gravity,
                    burst.decay,
                    burst.drift,
                ) else {
                    continue;
                };

                let color = particle.color.scale_alpha(sample.opacity);
                frame.with_save(|frame| {
                    frame.translate(Vector::new(sample.position.x, sample.position.y));
                    frame.rotate(sample.rotation);
                    match particle.shape {
                        ConfettiShape::Circle => {
                            // canvas-confetti: ellipse radii = |Δ| * ovalScalar (0.6)
                            // with |Δ| ≈ size = 10*scalar → radius = size * 0.6
                            frame.fill(
                                &canvas::Path::circle(
                                    Point::ORIGIN,
                                    sample.size * CONFETTI_OVAL_SCALAR,
                                ),
                                color,
                            );
                        }
                        ConfettiShape::Square => {
                            frame.fill_rectangle(
                                Point::new(-sample.size / 2.0, -sample.size / 2.0),
                                Size::new(sample.size, sample.size),
                                color,
                            );
                        }
                        ConfettiShape::Star => {
                            // size = 10*scalar → outer=8*scalar, inner=4*scalar
                            let scalar = sample.size / CONFETTI_SIZE_FACTOR;
                            frame.fill(
                                &star_path(
                                    CONFETTI_STAR_OUTER_FACTOR * scalar,
                                    CONFETTI_STAR_INNER_FACTOR * scalar,
                                ),
                                color,
                            );
                        }
                        ConfettiShape::Path { data } => {
                            frame.fill(&path_from_svg_data(data, sample.size), color);
                        }
                        ConfettiShape::Text { text } => {
                            // canvas-confetti text bitmap: fontSize = 10 * scalar
                            frame.fill_text(canvas::Text {
                                content: (*text).to_string(),
                                position: Point::ORIGIN,
                                color,
                                size: iced::Pixels(sample.size.max(1.0)),
                                align_x: iced::alignment::Horizontal::Center.into(),
                                align_y: iced::alignment::Vertical::Center,
                                ..canvas::Text::default()
                            });
                        }
                    }
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
    fn confetti_options_defaults_match_canvas_confetti() {
        let opts = ConfettiOptions::new();

        assert_eq!(opts.particle_count(), 50);
        assert!((opts.angle() - 90.0).abs() < f32::EPSILON);
        assert!((opts.spread() - 45.0).abs() < f32::EPSILON);
        assert!((opts.start_velocity() - 45.0).abs() < f32::EPSILON);
        assert!((opts.decay() - 0.9).abs() < f32::EPSILON);
        assert!((opts.gravity() - 1.0).abs() < f32::EPSILON);
        assert!((opts.drift() - 0.0).abs() < f32::EPSILON);
        assert_eq!(opts.ticks(), 200);
        assert!(opts.origin_x().is_none());
        assert!(opts.origin_y().is_none());
        assert!((opts.scalar() - 1.0).abs() < f32::EPSILON);
        assert_eq!(opts.colors().len(), 7);
        assert_eq!(opts.colors()[0], Color::from_rgb8(0x26, 0xCC, 0xFF));
        assert_eq!(opts.shapes().len(), 2);
        assert_eq!(opts.shapes()[0], ConfettiShape::Square);
        assert_eq!(opts.shapes()[1], ConfettiShape::Circle);
        assert_eq!(opts.seed(), 0);
    }

    #[test]
    fn setters_validate_ranges() {
        let opts = ConfettiOptions::new()
            .with_particle_count(0)
            .with_spread(-10.0)
            .with_start_velocity(-5.0)
            .with_decay(2.0)
            .with_ticks(0)
            .with_scalar(-1.0)
            .with_origin_x(2.0)
            .with_origin_y(-1.0);

        assert!(opts.particle_count() >= 1);
        assert!(opts.spread() >= 0.0);
        assert!(opts.start_velocity() >= 0.0);
        assert!((0.0..=1.0).contains(&opts.decay()));
        assert!(opts.ticks() >= 1);
        assert!(opts.scalar() >= 0.0);
        assert_eq!(opts.origin_x(), Some(1.0));
        assert_eq!(opts.origin_y(), Some(0.0));
    }

    #[test]
    fn fire_spawns_particles_and_accumulates_bursts() {
        let mut state = ConfettiState::new();
        let opts = ConfettiOptions::new()
            .with_particle_count(12)
            .with_seed(7);
        let now = Instant::now();
        let canvas = Size::new(400.0, 300.0);

        state.fire(now, &opts, canvas);
        assert_eq!(state.burst_count(), 1);
        assert_eq!(state.particle_count(), 12);

        state.fire(now, &opts.with_particle_count(5), canvas);
        assert_eq!(state.burst_count(), 2);
        assert_eq!(state.particle_count(), 17);
    }

    #[test]
    fn fire_maps_normalized_origin_to_canvas_pixels() {
        let mut state = ConfettiState::new();
        let opts = ConfettiOptions::new()
            .with_particle_count(4)
            .with_origin_x(0.0)
            .with_origin_y(0.5)
            .with_seed(1);
        let now = Instant::now();

        state.fire(now, &opts, Size::new(200.0, 100.0));
        let sample = state
            .sample_first(Duration::ZERO)
            .expect("particle at t0");
        assert!(
            (sample.0.x).abs() < 1.0,
            "expected x≈0, got {}",
            sample.0.x
        );
        assert!(
            (sample.0.y - 50.0).abs() < 1.0,
            "expected y≈50, got {}",
            sample.0.y
        );
    }

    #[test]
    fn fire_at_bakes_pixel_origin() {
        let mut state = ConfettiState::new();
        let opts = ConfettiOptions::new()
            .with_particle_count(8)
            .with_seed(3);
        let now = Instant::now();
        let origin = Point::new(120.0, 80.0);

        state.fire_at(now, &opts, origin);
        let sample = state
            .sample_first(Duration::ZERO)
            .expect("particle at t0");
        assert!((sample.0.x - 120.0).abs() < 1.0);
        assert!((sample.0.y - 80.0).abs() < 1.0);
    }

    #[test]
    fn sample_moves_under_gravity() {
        let mut state = ConfettiState::new();
        let opts = ConfettiOptions::new()
            .with_particle_count(1)
            .with_angle(90.0)
            .with_spread(0.0)
            .with_start_velocity(0.0)
            .with_gravity(1.0)
            .with_decay(0.9)
            .with_drift(0.0)
            .with_seed(9);
        let now = Instant::now();

        state.fire_at(now, &opts, Point::new(50.0, 10.0));
        let start = state
            .sample_first(Duration::ZERO)
            .expect("t0")
            .0;
        let later = state
            .sample_first(Duration::from_millis(500))
            .expect("t=500ms")
            .0;

        assert!(
            later.y > start.y,
            "gravity should pull down: start={} later={}",
            start.y,
            later.y
        );
    }

    #[test]
    fn particles_expire_after_ticks() {
        let mut state = ConfettiState::new();
        let opts = ConfettiOptions::new()
            .with_particle_count(6)
            .with_ticks(10)
            .with_seed(2);
        let started = Instant::now();

        state.fire_at(started, &opts, Point::ORIGIN);
        assert_eq!(state.burst_count(), 1);

        let life = Duration::from_millis(opts.life_ms());
        assert!(
            state.sample_first(life).is_none(),
            "sample past life should be gone"
        );

        state.tick(started + life + Duration::from_millis(1));
        assert_eq!(state.burst_count(), 0);
    }

    #[test]
    fn particle_color_baked_at_fire_time() {
        let mut state = ConfettiState::new();
        let fire_opts = ConfettiOptions::new().with_colors(&[Color::WHITE]);
        let started = Instant::now();

        state.fire_at(started, &fire_opts, Point::ORIGIN);
        let baked = state.bursts[0].particles[0].color;

        let unchanged = ConfettiOptions::new().with_colors(&[]);
        assert_eq!(baked, Color::WHITE);
        assert_eq!(unchanged.colors().len(), 7);
        assert_eq!(state.bursts[0].particles[0].color, baked);
    }

    #[test]
    fn confetti_button_defaults_use_canvas_confetti_options() {
        let button = ConfettiButton::<'static, ()>::new(iced::widget::text(""));
        let opts = button.options();

        assert_eq!(opts.particle_count(), 50);
        assert!((opts.angle() - 90.0).abs() < f32::EPSILON);
        assert_eq!(opts.colors().len(), 7);
    }

    #[test]
    fn clear_removes_bursts() {
        let mut state = ConfettiState::new();
        let opts = ConfettiOptions::new().with_particle_count(3);
        state.fire_at(Instant::now(), &opts, Point::ORIGIN);
        assert_eq!(state.burst_count(), 1);
        state.clear();
        assert_eq!(state.burst_count(), 0);
        assert_eq!(state.particle_count(), 0);
    }
}
