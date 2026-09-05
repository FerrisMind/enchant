use crate::primitives::{EffectCanvasProps, EffectProgram, StableRng, effect_canvas};
use iced::mouse;
use iced::time::Instant;
use iced::widget::button;
use iced::widget::canvas;
use iced::{Color, Element, Length, Padding, Point, Rectangle, Renderer, Size, Vector};
use std::fmt;
use std::time::Duration;

const MAX_COLORS: usize = 8;
const DEFAULT_COLORS: [Color; 4] = [
    Color::from_rgb8(0xFF, 0xC7, 0x00),
    Color::from_rgb8(0xFF, 0x00, 0x00),
    Color::from_rgb8(0x2E, 0x31, 0x91),
    Color::from_rgb8(0x41, 0xBB, 0xC7),
];
const BUTTON_COLORS: [Color; 3] = [
    Color::from_rgb8(0xFF, 0x00, 0x00),
    Color::from_rgb8(0x00, 0xFF, 0x00),
    Color::from_rgb8(0x00, 0x00, 0xFF),
];
const BUTTON_PARTICLE_COUNT: usize = 50;
const BUTTON_BACKGROUND: Color = Color::from_rgb8(0x25, 0x63, 0xEB);
const BUTTON_BACKGROUND_HOVER: Color = Color::from_rgb8(0x1D, 0x4E, 0xD8);
const BUTTON_BORDER_RADIUS: f32 = 8.0;
const DEFAULT_DURATION_MS: u64 = 3_500;
const CRAZY_PARTICLES_FREQUENCY: f32 = 0.1;
const CRAZY_PARTICLE_CRAZINESS: f32 = 0.3;
const BEZIER_MEDIAN: f32 = 0.5;
const POSSIBLE_ROTATION_TRANSFORMS: u32 = 6;
const ROTATION_SPEED_MIN_MS: f32 = 200.0;
const ROTATION_SPEED_MAX_MS: f32 = 800.0;

/// The particle geometry used by [`Confetti`].
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ConfettiParticleShape {
    /// Use both circles and rectangles.
    #[default]
    Mix,
    /// Use circles only.
    Circles,
    /// Use rectangles only.
    Rectangles,
}

/// Builder configuration for a neoconfetti-style burst.
#[must_use = "builders do nothing unless turned into an iced Element"]
#[derive(Clone, Copy, PartialEq)]
pub struct Confetti {
    particle_count: usize,
    particle_shape: ConfettiParticleShape,
    particle_size: f32,
    duration_ms: u64,
    force: f32,
    stage_height: f32,
    stage_width: f32,
    destroy_after_done: bool,
    colors: [Color; MAX_COLORS],
    color_len: u8,
    width: Length,
    height: Length,
    seed: u64,
}

impl fmt::Debug for Confetti {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Confetti")
            .field("particle_count", &self.particle_count)
            .field("particle_shape", &self.particle_shape)
            .field("particle_size", &self.particle_size)
            .field("duration_ms", &self.duration_ms)
            .field("force", &self.force)
            .field("stage_height", &self.stage_height)
            .field("stage_width", &self.stage_width)
            .field("destroy_after_done", &self.destroy_after_done)
            .field("colors", &self.palette())
            .field("width", &self.width)
            .field("height", &self.height)
            .field("seed", &self.seed)
            .finish()
    }
}

impl Default for Confetti {
    fn default() -> Self {
        let mut colors = [Color::TRANSPARENT; MAX_COLORS];
        colors[..DEFAULT_COLORS.len()].copy_from_slice(&DEFAULT_COLORS);

        Self {
            particle_count: 40,
            particle_shape: ConfettiParticleShape::Mix,
            particle_size: 12.0,
            duration_ms: DEFAULT_DURATION_MS,
            force: 0.5,
            stage_height: 800.0,
            stage_width: 1600.0,
            destroy_after_done: true,
            colors,
            color_len: DEFAULT_COLORS.len() as u8,
            width: Length::Fill,
            height: Length::Fill,
            seed: 0,
        }
    }
}

impl Confetti {
    /// Creates a builder with the animations wrapper defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the number of particles, clamped to at least one.
    #[must_use = "builder setters return updated configuration"]
    pub fn particle_count(mut self, particle_count: usize) -> Self {
        self.particle_count = particle_count.max(1);
        self
    }

    /// Sets the particle shape.
    #[must_use = "builder setters return updated configuration"]
    pub fn particle_shape(mut self, particle_shape: ConfettiParticleShape) -> Self {
        self.particle_shape = particle_shape;
        self
    }

    /// Sets the particle size in logical pixels.
    #[must_use = "builder setters return updated configuration"]
    pub fn particle_size(mut self, particle_size: f32) -> Self {
        self.particle_size = finite_or(particle_size, 1.0).max(1.0);
        self
    }

    /// Sets the burst duration in milliseconds.
    #[must_use = "builder setters return updated configuration"]
    pub fn duration_ms(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms.max(1);
        self
    }

    /// Sets the force in the inclusive range `0..=1`.
    #[must_use = "builder setters return updated configuration"]
    pub fn force(mut self, force: f32) -> Self {
        self.force = finite_or(force, 0.0).clamp(0.0, 1.0);
        self
    }

    /// Sets the stage height in logical pixels.
    #[must_use = "builder setters return updated configuration"]
    pub fn stage_height(mut self, stage_height: f32) -> Self {
        self.stage_height = finite_or(stage_height, 1.0).max(1.0);
        self
    }

    /// Sets the stage width in logical pixels.
    #[must_use = "builder setters return updated configuration"]
    pub fn stage_width(mut self, stage_width: f32) -> Self {
        self.stage_width = finite_or(stage_width, 1.0).max(1.0);
        self
    }

    /// Controls whether a finished burst is removed during [`ConfettiState::tick`].
    #[must_use = "builder setters return updated configuration"]
    pub fn destroy_after_done(mut self, destroy_after_done: bool) -> Self {
        self.destroy_after_done = destroy_after_done;
        self
    }

    /// Sets particle colors, keeping at most eight colors.
    ///
    /// An empty slice leaves the current palette unchanged.
    #[must_use = "builder setters return updated configuration"]
    pub fn colors(mut self, colors: &[Color]) -> Self {
        if colors.is_empty() {
            return self;
        }

        let color_len = colors.len().min(MAX_COLORS);
        self.colors[..color_len].copy_from_slice(&colors[..color_len]);
        self.color_len = color_len as u8;
        self
    }

    /// Sets the canvas width.
    #[must_use = "builder setters return updated configuration"]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the canvas height.
    #[must_use = "builder setters return updated configuration"]
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the deterministic particle seed.
    #[must_use = "builder setters return updated configuration"]
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    fn palette(&self) -> &[Color] {
        &self.colors[..usize::from(self.color_len)]
    }

    /// Builds the confetti canvas for the supplied state and clock.
    pub fn into_element<'a, Message: 'a>(
        self,
        state: &'a ConfettiState,
        now: Instant,
    ) -> Element<'a, Message> {
        effect_canvas::<_, Message>(
            ConfettiProgram {
                state,
                now,
                config: self,
            },
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

fn button_default_confetti() -> Confetti {
    Confetti::new()
        .particle_count(BUTTON_PARTICLE_COUNT)
        .force(0.5)
        .stage_height(800.0)
        .stage_width(1600.0)
        .colors(&BUTTON_COLORS)
}

/// Button that carries animations [`Confetti`] defaults and emits a press message.
///
/// Confetti is not fired automatically — on press the host should call
/// [`ConfettiState::clear`] then [`ConfettiState::fire`] with [`Self::confetti`].
#[must_use = "builders do nothing unless turned into an iced Element"]
pub struct ConfettiButton<'a, Message> {
    content: Element<'a, Message>,
    confetti: Confetti,
    on_press: Option<Message>,
    padding: Padding,
    width: Length,
    height: Length,
}

impl<'a, Message> fmt::Debug for ConfettiButton<'a, Message> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConfettiButton")
            .field("confetti", &self.confetti)
            .field("on_press", &self.on_press.is_some())
            .field("padding", &self.padding)
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

impl<'a, Message> ConfettiButton<'a, Message> {
    /// Creates a button with animations confetti-button defaults.
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            confetti: button_default_confetti(),
            on_press: None,
            padding: Padding::from([12.0, 24.0]),
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    /// Returns the confetti configuration used when firing from this button.
    pub fn confetti(&self) -> Confetti {
        self.confetti
    }

    /// Sets the press handler. The host should clear and fire confetti on this message.
    #[must_use = "builder setters return updated configuration"]
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Alias for [`Self::on_press`] — same message, confetti firing stays in the host.
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
        self.confetti = self.confetti.particle_count(particle_count);
        self
    }

    #[must_use = "builder setters return updated configuration"]
    pub fn particle_shape(mut self, particle_shape: ConfettiParticleShape) -> Self {
        self.confetti = self.confetti.particle_shape(particle_shape);
        self
    }

    #[must_use = "builder setters return updated configuration"]
    pub fn particle_size(mut self, particle_size: f32) -> Self {
        self.confetti = self.confetti.particle_size(particle_size);
        self
    }

    #[must_use = "builder setters return updated configuration"]
    pub fn duration_ms(mut self, duration_ms: u64) -> Self {
        self.confetti = self.confetti.duration_ms(duration_ms);
        self
    }

    #[must_use = "builder setters return updated configuration"]
    pub fn force(mut self, force: f32) -> Self {
        self.confetti = self.confetti.force(force);
        self
    }

    #[must_use = "builder setters return updated configuration"]
    pub fn stage_height(mut self, stage_height: f32) -> Self {
        self.confetti = self.confetti.stage_height(stage_height);
        self
    }

    #[must_use = "builder setters return updated configuration"]
    pub fn stage_width(mut self, stage_width: f32) -> Self {
        self.confetti = self.confetti.stage_width(stage_width);
        self
    }

    #[must_use = "builder setters return updated configuration"]
    pub fn destroy_after_done(mut self, destroy_after_done: bool) -> Self {
        self.confetti = self.confetti.destroy_after_done(destroy_after_done);
        self
    }

    #[must_use = "builder setters return updated configuration"]
    pub fn colors(mut self, colors: &[Color]) -> Self {
        self.confetti = self.confetti.colors(colors);
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

/// Runtime state for one active confetti burst.
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

    /// Clears existing particles and starts one new burst.
    pub fn fire(&mut self, now: Instant, config: &Confetti) {
        self.bursts.clear();
        self.bursts
            .push(ConfettiBurst::new(now, *config, self.nonce));
        self.nonce = self.nonce.wrapping_add(1);
    }

    /// Removes all active bursts.
    pub fn clear(&mut self) {
        self.bursts.clear();
    }

    /// Removes finished bursts when destruction is enabled.
    pub fn tick(&mut self, now: Instant, config: &Confetti) {
        if !config.destroy_after_done {
            return;
        }

        self.bursts.retain(|burst| {
            now.saturating_duration_since(burst.started_at)
                < Duration::from_millis(config.duration_ms)
        });
    }

    /// Returns the number of active bursts.
    pub fn burst_count(&self) -> usize {
        self.bursts.len()
    }
}

#[derive(Clone, Debug)]
struct ConfettiBurst {
    started_at: Instant,
    particles: Vec<Particle>,
}

impl ConfettiBurst {
    fn new(started_at: Instant, config: Confetti, nonce: u64) -> Self {
        let mut rng = StableRng::new(config.seed.wrapping_add(nonce));
        let particles = (0..config.particle_count)
            .map(|index| spawn_particle(index, config, &mut rng))
            .collect();

        Self {
            started_at,
            particles,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Particle {
    x_landing_point: f32,
    duration_ms: u64,
    x1: f32,
    x2: f32,
    x3: f32,
    x4: f32,
    y1: f32,
    y2: f32,
    y3: f32,
    y4: f32,
    width: f32,
    height: f32,
    rotation_transform: u8,
    rotation_duration_ms: f32,
    color: Color,
    is_circle: bool,
}

#[derive(Clone, Copy, Debug)]
struct SampledParticle {
    position: Point,
    width: f32,
    height: f32,
    rotation: f32,
    is_circle: bool,
}

fn spawn_particle(index: usize, config: Confetti, rng: &mut StableRng) -> Particle {
    let degree = index as f32 * 360.0 / config.particle_count as f32;
    let rotated_degree = if degree + 90.0 > 360.0 {
        degree + 90.0 - 360.0
    } else {
        degree + 90.0
    };
    let distance = (rotated_degree - 180.0).abs();
    let x_landing_point = map_range(
        distance,
        0.0,
        180.0,
        -config.stage_width / 2.0,
        config.stage_width / 2.0,
    );
    let duration_ms = config
        .duration_ms
        .saturating_sub((rng.next_f32() * 1_000.0).round() as u64)
        .max(1);
    let x1 = if rng.next_f32() < CRAZY_PARTICLES_FREQUENCY {
        round(rng.next_f32() * CRAZY_PARTICLE_CRAZINESS, 2)
    } else {
        0.0
    };
    let x4 = round((2.0 * distance / 180.0 - 1.0).abs(), 4);
    let y1 = round(rng.next_f32() * BEZIER_MEDIAN, 4);
    let y2 = round(
        rng.next_f32() * config.force * if rng.next_f32() > 0.5 { 1.0 } else { -1.0 },
        4,
    );
    let y4 = round(
        (config.force - 2.0 * (degree - 180.0).abs() / 180.0 * config.force).max(0.0),
        4,
    );
    let rotation_transform =
        (rng.next_f32() * (POSSIBLE_ROTATION_TRANSFORMS - 1) as f32).round() as u8;
    let is_circle = match config.particle_shape {
        ConfettiParticleShape::Circles => true,
        ConfettiParticleShape::Rectangles => false,
        ConfettiParticleShape::Mix => rotation_transform != 1 && rng.next_f32() > 0.5,
    };

    Particle {
        x_landing_point,
        duration_ms,
        x1,
        x2: -x1,
        x3: x1,
        x4,
        y1,
        y2,
        y3: BEZIER_MEDIAN,
        y4,
        width: if is_circle {
            config.particle_size
        } else {
            (rng.next_f32() * 4.0).round() + config.particle_size / 2.0
        },
        height: if is_circle {
            config.particle_size
        } else {
            (rng.next_f32() * 2.0).round() + config.particle_size
        },
        rotation_transform,
        rotation_duration_ms: rng.range_f32(ROTATION_SPEED_MIN_MS, ROTATION_SPEED_MAX_MS),
        color: config.palette()[index % config.palette().len().max(1)],
        is_circle,
    }
}

fn sample_neoconfetti(
    particle: &Particle,
    animation_fraction: f32,
    stage_height: f32,
) -> SampledParticle {
    let fraction = animation_fraction.clamp(0.0, 1.0);
    let x = sample_cubic_bezier(fraction, particle.x1, particle.x2, particle.x3, particle.x4)
        * particle.x_landing_point;
    let y = sample_cubic_bezier(fraction, particle.y1, particle.y2, particle.y3, particle.y4)
        * stage_height;
    let rotation = fraction * std::f32::consts::TAU * 800.0
        / particle.rotation_duration_ms.max(1.0)
        * if particle.rotation_transform & 1 == 1 {
            -1.0
        } else {
            1.0
        };

    SampledParticle {
        position: Point::new(x, y),
        width: particle.width,
        height: particle.height,
        rotation,
        is_circle: particle.is_circle,
    }
}

fn sample_cubic_bezier(time: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let time = time.clamp(0.0, 1.0);
    let mut parameter = time;

    for _ in 0..8 {
        let current = cubic_component(parameter, x1, x2) - time;
        let derivative = cubic_derivative(parameter, x1, x2);
        if derivative.abs() < 1e-6 {
            break;
        }
        parameter = (parameter - current / derivative).clamp(0.0, 1.0);
    }

    cubic_component(parameter, y1, y2)
}

fn cubic_component(parameter: f32, first_control: f32, second_control: f32) -> f32 {
    let inverse = 1.0 - parameter;
    3.0 * inverse * inverse * parameter * first_control
        + 3.0 * inverse * parameter * parameter * second_control
        + parameter * parameter * parameter
}

fn cubic_derivative(parameter: f32, first_control: f32, second_control: f32) -> f32 {
    let inverse = 1.0 - parameter;
    3.0 * inverse * inverse * first_control
        + 6.0 * inverse * parameter * (second_control - first_control)
        + 3.0 * parameter * parameter * (1.0 - second_control)
}

fn map_range(value: f32, from_start: f32, from_end: f32, to_start: f32, to_end: f32) -> f32 {
    (value - from_start) * (to_end - to_start) / (from_end - from_start) + to_start
}

fn round(value: f32, precision: u32) -> f32 {
    let factor = 10_f32.powi(precision as i32);
    (value * factor).round() / factor
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

struct ConfettiProgram<'a> {
    state: &'a ConfettiState,
    now: Instant,
    config: Confetti,
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
            let origin = Point::new(bounds.width / 2.0, 0.0);

            for particle in &burst.particles {
                let fraction = elapsed_ms as f32 / particle.duration_ms as f32;
                let sample = sample_neoconfetti(particle, fraction, self.config.stage_height);
                let color = particle.color;
                let position =
                    Point::new(origin.x + sample.position.x, origin.y + sample.position.y);

                frame.with_save(|frame| {
                    frame.translate(Vector::new(position.x, position.y));
                    frame.rotate(sample.rotation);
                    if sample.is_circle {
                        frame.fill(
                            &canvas::Path::circle(Point::ORIGIN, sample.width / 2.0),
                            color,
                        );
                    } else {
                        frame.fill_rectangle(
                            Point::new(-sample.width / 2.0, -sample.height / 2.0),
                            Size::new(sample.width, sample.height),
                            color,
                        );
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
    fn confetti_defaults_match_animations_wrapper() {
        let confetti = Confetti::new();

        assert_eq!(confetti.particle_count, 40);
        assert!((confetti.force - 0.5).abs() < f32::EPSILON);
        assert_eq!(confetti.particle_size, 12.0);
        assert_eq!(confetti.duration_ms, 3500);
        assert_eq!(confetti.stage_height, 800.0);
        assert_eq!(confetti.stage_width, 1600.0);
        assert!(confetti.destroy_after_done);
        assert_eq!(confetti.particle_shape, ConfettiParticleShape::Mix);
        assert_eq!(confetti.palette().len(), 4);
        assert_eq!(confetti.palette()[0], Color::from_rgb8(0xFF, 0xC7, 0x00));
    }

    #[test]
    fn setters_validate_ranges() {
        let confetti = Confetti::new()
            .particle_count(0)
            .force(2.0)
            .particle_size(-1.0)
            .duration_ms(0)
            .stage_height(0.0);

        assert!(confetti.particle_count >= 1);
        assert!((0.0..=1.0).contains(&confetti.force));
        assert!(confetti.particle_size > 0.0);
        assert!(confetti.duration_ms >= 1);
        assert!(confetti.stage_height >= 1.0);
    }

    #[test]
    fn fire_adds_burst_and_destroy_clears_after_duration() {
        let mut state = ConfettiState::new();
        let confetti = Confetti::new()
            .duration_ms(100)
            .destroy_after_done(true);
        let started_at = Instant::now();

        state.fire(started_at, &confetti);
        assert_eq!(state.burst_count(), 1);

        state.tick(started_at + Duration::from_millis(150), &confetti);
        assert_eq!(state.burst_count(), 0);
    }

    #[test]
    fn sample_at_t0_near_origin_and_later_falls() {
        let particle = Particle {
            x_landing_point: 0.0,
            duration_ms: 1000,
            x1: 0.0,
            x2: 0.0,
            x3: 0.0,
            x4: 0.0,
            y1: 0.25,
            y2: 0.25,
            y3: 0.5,
            y4: 0.5,
            width: 12.0,
            height: 12.0,
            rotation_transform: 0,
            rotation_duration_ms: 500.0,
            color: Color::WHITE,
            is_circle: true,
        };

        let at_start = sample_neoconfetti(&particle, 0.0, 800.0);
        let later = sample_neoconfetti(&particle, 0.9, 800.0);

        assert!(at_start.position.y.abs() < 40.0);
        assert!(later.position.y > at_start.position.y);
    }

    #[test]
    fn confetti_button_defaults_match_animations_button() {
        let button = ConfettiButton::<'static, ()>::new(iced::widget::text(""));

        assert_eq!(button.confetti().particle_count, BUTTON_PARTICLE_COUNT);
        assert!((button.confetti().force - 0.5).abs() < f32::EPSILON);
        assert_eq!(button.confetti().stage_height, 800.0);
        assert_eq!(button.confetti().stage_width, 1600.0);
        assert_eq!(button.confetti().palette().len(), 3);
        assert_eq!(button.confetti().palette()[0], BUTTON_COLORS[0]);
        assert_eq!(button.confetti().palette()[1], BUTTON_COLORS[1]);
        assert_eq!(button.confetti().palette()[2], BUTTON_COLORS[2]);
    }

    #[test]
    fn particle_color_baked_at_fire_time() {
        let mut state = ConfettiState::new();
        let fire_config = Confetti::new().colors(&[Color::WHITE]);
        let started_at = Instant::now();

        state.fire(started_at, &fire_config);
        let baked_color = state.bursts[0].particles[0].color;

        let shrunk_config = Confetti::new().colors(&[]);
        assert_eq!(baked_color, Color::WHITE);
        assert_eq!(shrunk_config.palette().len(), 4);
        assert_eq!(state.bursts[0].particles[0].color, baked_color);
    }
}
