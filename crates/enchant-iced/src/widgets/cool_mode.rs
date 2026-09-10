//! Builder-first cool-mode particle burst (Magic UI / svelte-animations `CoolMode`).
//!
//! Press-hold on the **wrapped content** emits particles; the particle layer fills the
//! CoolMode outer bounds and does not steal clicks. Physics matches the Magic rAF step
//! (`speedUp -= 1` capped by `size`, 30 ms emit delay, hard limit).

use crate::primitives::StableRng;
use iced::advanced::Renderer as _;
use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{self, Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::border::Radius;
use iced::mouse;
use iced::widget::canvas::{self, Fill, Text};
use iced::{
    Color, Element, Event, Length, Point, Pixels, Rectangle, Renderer, Size, Vector,
};
use std::borrow::Cow;
use std::cell::RefCell;
use std::f32::consts::PI;
use std::fmt;
use std::time::Duration;

const DEFAULT_PARTICLE_COUNT: usize = 45;
const EMIT_DELAY: Duration = Duration::from_millis(30);
const FRAME_SECS: f32 = 1.0 / 60.0;
const MAX_STEPS_PER_TICK: u32 = 8;
const SIZE_POOL: [f32; 5] = [15.0, 20.0, 25.0, 35.0, 45.0];
const EMOJI_FONT_SCALE: f32 = 3.0;
const RNG_SEED: u64 = 0xC001_C0DE;

/// Visual kind for emitted particles (Magic `options.particle`).
#[non_exhaustive]
#[derive(Clone, Debug, Default)]
pub enum CoolParticleKind {
    /// Random HSL circle fill `hsl(h, 70%, 50%)` (Magic default).
    #[default]
    Circle,
    /// Emoji or text character (drawn ~3× larger, matching svelte).
    Text(Cow<'static, str>),
    /// Image particle with circular clip.
    Image(iced::widget::image::Handle),
}

/// Builder wrapping content with a Magic-style cool-mode particle overlay.
#[must_use = "builders do nothing unless turned into an iced Element"]
pub struct CoolMode<'a, Message> {
    content: Element<'a, Message>,
    kind: CoolParticleKind,
    size: Option<f32>,
    particle_count: usize,
    speed_horz: Option<f32>,
    speed_up: Option<f32>,
    width: Length,
    height: Length,
}

impl<Message> fmt::Debug for CoolMode<'_, Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CoolMode")
            .field("kind", &self.kind)
            .field("size", &self.size)
            .field("particle_count", &self.particle_count)
            .field("speed_horz", &self.speed_horz)
            .field("speed_up", &self.speed_up)
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

impl<'a, Message> CoolMode<'a, Message> {
    /// Creates a cool-mode wrapper with Magic defaults (circle particles, limit 45).
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            kind: CoolParticleKind::Circle,
            size: None,
            particle_count: DEFAULT_PARTICLE_COUNT,
            speed_horz: None,
            speed_up: None,
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    /// Alias for [`Self::new`].
    pub fn from_content(content: impl Into<Element<'a, Message>>) -> Self {
        Self::new(content)
    }

    /// Particle visual kind (default [`CoolParticleKind::Circle`]).
    pub fn particle(mut self, kind: CoolParticleKind) -> Self {
        self.kind = kind;
        self
    }

    /// Fixed particle size in px; omit for the Magic size pool.
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size.max(0.0));
        self
    }

    /// Clears a fixed size so particles sample the Magic pool again.
    pub fn clear_size(mut self) -> Self {
        self.size = None;
        self
    }

    /// Maximum live particles (Magic `particleCount` / hard limit; default 45).
    pub fn particle_count(mut self, count: usize) -> Self {
        self.particle_count = count.max(1);
        self
    }

    /// Fixed horizontal speed; omit for `random * 10`.
    pub fn speed_horz(mut self, speed: f32) -> Self {
        self.speed_horz = Some(speed.max(0.0));
        self
    }

    /// Clears fixed horizontal speed.
    pub fn clear_speed_horz(mut self) -> Self {
        self.speed_horz = None;
        self
    }

    /// Fixed upward speed; omit for `random * 25`.
    pub fn speed_up(mut self, speed: f32) -> Self {
        self.speed_up = Some(speed.max(0.0));
        self
    }

    /// Clears fixed upward speed.
    pub fn clear_speed_up(mut self) -> Self {
        self.speed_up = None;
        self
    }

    /// Widget width (default [`Length::Fill`]).
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Widget height (default [`Length::Fill`]).
    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    /// Builds the layered element for the host animation clock.
    pub fn into_element(self, elapsed: Duration) -> Element<'a, Message>
    where
        Message: 'a + 'static,
    {
        Element::new(CoolModeWidget {
            content: self.content,
            elapsed,
            kind: self.kind,
            size: self.size,
            particle_count: self.particle_count,
            speed_horz: self.speed_horz,
            speed_up: self.speed_up,
            width: self.width,
            height: self.height,
        })
    }

    /// Alias for [`Self::into_element`].
    pub fn view(self, elapsed: Duration) -> Element<'a, Message>
    where
        Message: 'a + 'static,
    {
        self.into_element(elapsed)
    }
}

/// Magic binds pointer events to the children wrapper only — not the particle overlay.
fn press_targets_content(cursor: mouse::Cursor, content_bounds: Rectangle) -> bool {
    cursor.is_over(content_bounds)
}

struct CoolModeWidget<'a, Message> {
    content: Element<'a, Message>,
    elapsed: Duration,
    kind: CoolParticleKind,
    size: Option<f32>,
    particle_count: usize,
    speed_horz: Option<f32>,
    speed_up: Option<f32>,
    width: Length,
    height: Length,
}

impl<Message> CoolModeWidget<'_, Message> {
    fn emit_config(&self) -> CoolModeEmitConfig {
        CoolModeEmitConfig {
            size: self.size,
            particle_count: self.particle_count,
            speed_horz: self.speed_horz,
            speed_up: self.speed_up,
        }
    }
}

impl<Message> Widget<Message, iced::Theme, Renderer> for CoolModeWidget<'_, Message>
where
    Message: 'static,
{
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<CoolModeState>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(CoolModeState::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        // Outer CoolMode fills the parent (particle travel area).
        let outer = limits.resolve(self.width, self.height, Size::ZERO);
        // Content keeps its intrinsic size — do not force Fill (Magic children wrapper).
        let content_limits = layout::Limits::new(Size::ZERO, outer);
        let mut content_node =
            self.content
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, &content_limits);
        let content_size = content_node.size();
        // Center Shrink content inside the Fill stage (example may also center CoolMode).
        let offset = Point::new(
            ((outer.width - content_size.width) * 0.5).max(0.0),
            ((outer.height - content_size.height) * 0.5).max(0.0),
        );
        content_node = content_node.move_to(offset);
        layout::Node::with_children(outer, vec![content_node])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            layout.children().next().expect("cool_mode content layout"),
            renderer,
            operation,
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let outer_bounds = layout.bounds();
        let content_layout = layout.children().next().expect("cool_mode content layout");
        let content_bounds = content_layout.bounds();
        let over_content = press_targets_content(cursor, content_bounds);

        let state = tree.state.downcast_mut::<CoolModeState>();
        let mut sim = state.sim.borrow_mut();
        let mut dirty = false;

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(iced::touch::Event::FingerPressed { .. })
                if over_content =>
            {
                if let Some(position) = cursor.position_in(outer_bounds) {
                    sim.set_pointer(Some(position));
                }
                if !sim.auto_add {
                    sim.set_press(true);
                    dirty = true;
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(iced::touch::Event::FingerLifted { .. }) => {
                if sim.auto_add {
                    sim.set_press(false);
                    dirty = true;
                }
            }
            Event::Mouse(mouse::Event::CursorLeft)
            | Event::Touch(iced::touch::Event::FingerLost { .. }) => {
                if sim.auto_add {
                    sim.set_press(false);
                    dirty = true;
                }
                sim.set_pointer(None);
            }
            Event::Mouse(mouse::Event::CursorMoved { .. })
            | Event::Touch(iced::touch::Event::FingerMoved { .. }) => {
                if over_content {
                    if let Some(position) = cursor.position_in(outer_bounds) {
                        sim.set_pointer(Some(position));
                        dirty = true;
                    }
                } else if sim.auto_add {
                    // Magic: pointerleave on the children wrapper stops emission.
                    sim.set_press(false);
                    dirty = true;
                }
            }
            _ => {}
        }

        let before = sim.particles.len();
        let was_auto = sim.auto_add;
        sim.advance(self.elapsed, outer_bounds.height, &self.emit_config());
        if sim.particles.len() != before || was_auto || !sim.particles.is_empty() {
            dirty = true;
        }
        drop(sim);

        if dirty {
            shell.request_redraw();
        }

        // Content receives events; particle layer is not a hit target (no overlay child).
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            content_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let outer_bounds = layout.bounds();
        let content_layout = layout.children().next().expect("cool_mode content layout");

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            content_layout,
            cursor,
            viewport,
        );

        let state = tree.state.downcast_ref::<CoolModeState>();
        let mut sim = state.sim.borrow_mut();
        sim.advance(self.elapsed, outer_bounds.height, &self.emit_config());

        if outer_bounds.width < 1.0 || outer_bounds.height < 1.0 {
            return;
        }

        let mut frame = canvas::Frame::new(renderer, outer_bounds.size());
        for particle in sim.particles.iter() {
            paint_particle(&mut frame, particle, &self.kind);
        }
        let geometry = frame.into_geometry();
        renderer.with_translation(
            Vector::new(outer_bounds.x, outer_bounds.y),
            |renderer| {
                renderer.draw_geometry(geometry);
            },
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().expect("cool_mode content layout"),
            cursor,
            viewport,
            renderer,
        )
    }
}

#[derive(Clone, Copy, Debug)]
struct CoolParticle {
    left: f32,
    top: f32,
    size: f32,
    direction: f32,
    speed_horz: f32,
    speed_up: f32,
    spin_speed: f32,
    spin_val: f32,
    fill: Color,
}

#[derive(Debug)]
struct CoolModeSim {
    particles: Vec<CoolParticle>,
    auto_add: bool,
    pointer: Option<Point>,
    last_emit_elapsed: Option<Duration>,
    last_sim_elapsed: Duration,
    started: bool,
    rng: StableRng,
}

impl Default for CoolModeSim {
    fn default() -> Self {
        Self {
            particles: Vec::with_capacity(DEFAULT_PARTICLE_COUNT),
            auto_add: false,
            pointer: None,
            last_emit_elapsed: None,
            last_sim_elapsed: Duration::ZERO,
            started: false,
            rng: StableRng::new(RNG_SEED),
        }
    }
}

impl CoolModeSim {
    fn ensure_capacity(&mut self, limit: usize) {
        if self.particles.capacity() < limit {
            self.particles.reserve(limit - self.particles.capacity());
        }
    }

    fn set_press(&mut self, pressed: bool) {
        self.auto_add = pressed;
    }

    fn set_pointer(&mut self, pointer: Option<Point>) {
        self.pointer = pointer;
    }

    fn try_emit(&mut self, elapsed: Duration, config: &CoolModeEmitConfig) {
        if !self.auto_add || self.particles.len() >= config.particle_count {
            return;
        }
        if let Some(last) = self.last_emit_elapsed
            && elapsed.saturating_sub(last) < EMIT_DELAY
        {
            return;
        }
        let Some(pointer) = self.pointer else {
            return;
        };
        self.emit_at(pointer, config);
        self.last_emit_elapsed = Some(elapsed);
    }

    fn emit_at(&mut self, pointer: Point, config: &CoolModeEmitConfig) {
        if self.particles.len() >= config.particle_count {
            return;
        }

        let size = config
            .size
            .unwrap_or_else(|| SIZE_POOL[self.rng.range_usize(0, SIZE_POOL.len())]);
        let speed_horz = config
            .speed_horz
            .unwrap_or_else(|| self.rng.next_f32() * 10.0);
        let speed_up = config
            .speed_up
            .unwrap_or_else(|| self.rng.next_f32() * 25.0);
        let spin_val = self.rng.next_f32() * 360.0;
        let spin_speed = self.rng.next_f32() * 35.0 * if self.rng.bool(0.5) { -1.0 } else { 1.0 };
        let direction = if self.rng.bool(0.5) { -1.0 } else { 1.0 };
        let hue = self.rng.next_f32() * 360.0;
        let fill = hsl_to_color(hue, 0.70, 0.50);

        self.particles.push(CoolParticle {
            left: pointer.x - size * 0.5,
            top: pointer.y - size * 0.5,
            size,
            direction,
            speed_horz,
            speed_up,
            spin_speed,
            spin_val,
            fill,
        });
    }

    fn advance(&mut self, elapsed: Duration, bounds_height: f32, config: &CoolModeEmitConfig) {
        self.ensure_capacity(config.particle_count);

        if !self.started {
            self.started = true;
            self.last_sim_elapsed = elapsed;
            self.try_emit(elapsed, config);
            return;
        }

        let delta = elapsed.saturating_sub(self.last_sim_elapsed);
        let steps = ((delta.as_secs_f32() / FRAME_SECS).floor() as u32).min(MAX_STEPS_PER_TICK);

        if steps == 0 {
            // Leave `last_sim_elapsed` unchanged so sub-frame deltas accumulate.
            self.try_emit(elapsed, config);
            return;
        }

        let mut sim_t = self.last_sim_elapsed;
        for _ in 0..steps {
            sim_t = sim_t.saturating_add(Duration::from_secs_f32(FRAME_SECS));
            self.try_emit(sim_t, config);
            step_all(&mut self.particles);
            retain_in_bounds(&mut self.particles, bounds_height);
        }
        // Advance only by consumed whole frames; leftover delta carries forward.
        self.last_sim_elapsed = sim_t;
    }
}

#[derive(Clone, Copy, Debug)]
struct CoolModeEmitConfig {
    size: Option<f32>,
    particle_count: usize,
    speed_horz: Option<f32>,
    speed_up: Option<f32>,
}

fn step_particle(particle: &mut CoolParticle) {
    particle.left -= particle.speed_horz * particle.direction;
    particle.top -= particle.speed_up;
    particle.speed_up = particle.size.min(particle.speed_up - 1.0);
    particle.spin_val += particle.spin_speed;
}

fn step_all(particles: &mut [CoolParticle]) {
    for particle in particles.iter_mut() {
        step_particle(particle);
    }
}

fn retain_in_bounds(particles: &mut Vec<CoolParticle>, bounds_height: f32) {
    particles.retain(|particle| particle.top < bounds_height + particle.size);
}

#[derive(Default)]
struct CoolModeState {
    sim: RefCell<CoolModeSim>,
}

fn paint_particle(frame: &mut canvas::Frame, particle: &CoolParticle, kind: &CoolParticleKind) {
    let center = Point::new(
        particle.left + particle.size * 0.5,
        particle.top + particle.size * 0.5,
    );
    let angle = particle.spin_val * PI / 180.0;

    match kind {
        CoolParticleKind::Circle => {
            frame.with_save(|frame| {
                frame.translate(iced::Vector::new(center.x, center.y));
                frame.rotate(angle);
                frame.fill(
                    &canvas::Path::circle(Point::ORIGIN, particle.size * 0.5),
                    Fill::from(particle.fill),
                );
            });
        }
        CoolParticleKind::Text(label) => {
            let font_size = (particle.size * EMOJI_FONT_SCALE).max(1.0);
            frame.with_save(|frame| {
                frame.translate(iced::Vector::new(center.x, center.y));
                frame.rotate(angle);
                frame.fill_text(Text {
                    content: label.to_string(),
                    position: Point::ORIGIN,
                    color: Color::WHITE,
                    size: Pixels(font_size),
                    align_x: iced::alignment::Horizontal::Center.into(),
                    align_y: iced::alignment::Vertical::Center,
                    ..Text::default()
                });
            });
        }
        CoolParticleKind::Image(handle) => {
            let extent = particle.size.max(0.0);
            let size = Size::new(extent, extent);
            let rect = Rectangle::new(Point::new(-extent * 0.5, -extent * 0.5), size);
            // Magic CSS `border-radius: 50%`: clip_bounds = particle box, Radius = size/2 → circle.
            let mut image = canvas::Image::new(handle.clone());
            image.border_radius = Radius::from(extent * 0.5);
            frame.with_save(|frame| {
                frame.translate(iced::Vector::new(center.x, center.y));
                frame.rotate(angle);
                frame.with_clip(rect, |frame| {
                    frame.draw_image(rect, image);
                });
            });
        }
    }
}

fn hsl_to_color(h: f32, s: f32, l: f32) -> Color {
    let h = if h.is_finite() {
        h.rem_euclid(360.0)
    } else {
        0.0
    };
    let s = s.clamp(0.0, 1.0);
    let l = l.clamp(0.0, 1.0);
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = l - c / 2.0;

    let (r1, g1, b1) = match h as i32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    Color::from_rgb(r1 + m, g1 + m, b1 + m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::text;

    #[test]
    fn cool_mode_defaults_match_magic() {
        let mode: CoolMode<'_, ()> = CoolMode::new(text("Click Me!"));

        assert_eq!(mode.particle_count, 45);
        assert!(matches!(mode.kind, CoolParticleKind::Circle));
        assert!(mode.size.is_none());
        assert!(mode.speed_horz.is_none());
        assert!(mode.speed_up.is_none());
        assert_eq!(mode.width, Length::Fill);
        assert_eq!(mode.height, Length::Fill);
    }

    #[test]
    fn cool_mode_setters_clamp() {
        let mode: CoolMode<'_, ()> = CoolMode::new(text("x"))
            .particle_count(0)
            .size(-4.0)
            .speed_horz(-2.0)
            .speed_up(-9.0);

        assert_eq!(mode.particle_count, 1);
        assert_eq!(mode.size, Some(0.0));
        assert_eq!(mode.speed_horz, Some(0.0));
        assert_eq!(mode.speed_up, Some(0.0));

        let cleared = mode.clear_size().clear_speed_horz().clear_speed_up();
        assert!(cleared.size.is_none());
        assert!(cleared.speed_horz.is_none());
        assert!(cleared.speed_up.is_none());
    }

    #[test]
    fn press_targets_content_gates_empty_stage() {
        let content = Rectangle::new(Point::new(160.0, 190.0), Size::new(100.0, 40.0));
        let over_button = mouse::Cursor::Available(Point::new(200.0, 210.0));
        let over_empty_stage = mouse::Cursor::Available(Point::new(20.0, 20.0));

        assert!(press_targets_content(over_button, content));
        assert!(
            !press_targets_content(over_empty_stage, content),
            "empty CoolMode stage must not start emission"
        );
        assert!(!press_targets_content(mouse::Cursor::Unavailable, content));
    }

    #[test]
    fn physics_step_matches_magic() {
        let mut particle = CoolParticle {
            left: 100.0,
            top: 200.0,
            size: 20.0,
            direction: 1.0,
            speed_horz: 5.0,
            speed_up: 12.0,
            spin_speed: 3.0,
            spin_val: 10.0,
            fill: Color::WHITE,
        };

        step_particle(&mut particle);

        // left -= speedHorz * direction; top -= speedUp; speedUp = min(size, speedUp - 1)
        assert!((particle.left - 95.0).abs() < f32::EPSILON);
        assert!((particle.top - 188.0).abs() < f32::EPSILON);
        assert!((particle.speed_up - 11.0).abs() < f32::EPSILON);
        assert!((particle.spin_val - 13.0).abs() < f32::EPSILON);

        // min(size, speedUp - 1) when speedUp - 1 < size still yields speedUp - 1.
        let mut falling = CoolParticle {
            left: 0.0,
            top: 50.0,
            size: 20.0,
            direction: -1.0,
            speed_horz: 0.0,
            speed_up: 0.5,
            spin_speed: 0.0,
            spin_val: 0.0,
            fill: Color::WHITE,
        };
        let prev_up = falling.speed_up;
        step_particle(&mut falling);
        assert!((falling.top - (50.0 - prev_up)).abs() < f32::EPSILON);
        assert!((falling.speed_up - falling.size.min(prev_up - 1.0)).abs() < f32::EPSILON);
        assert!((falling.speed_up - (-0.5)).abs() < f32::EPSILON);
    }

    #[test]
    fn emit_respects_delay_and_hard_limit() {
        let mut sim = CoolModeSim::default();
        let config = CoolModeEmitConfig {
            size: Some(20.0),
            particle_count: 3,
            speed_horz: Some(1.0),
            speed_up: Some(5.0),
        };

        sim.set_pointer(Some(Point::new(40.0, 40.0)));
        sim.set_press(true);

        sim.advance(Duration::from_millis(0), 400.0, &config);
        assert_eq!(sim.particles.len(), 1);

        sim.advance(Duration::from_millis(20), 400.0, &config);
        assert_eq!(sim.particles.len(), 1, "must wait 30ms between emits");

        sim.advance(Duration::from_millis(35), 400.0, &config);
        assert_eq!(sim.particles.len(), 2);

        sim.advance(Duration::from_millis(70), 400.0, &config);
        assert_eq!(sim.particles.len(), 3);

        sim.advance(Duration::from_millis(120), 400.0, &config);
        assert_eq!(
            sim.particles.len(),
            3,
            "must not exceed particle_count hard limit"
        );
    }

    #[test]
    fn press_sets_auto_add_release_and_exit_clear_it() {
        let mut sim = CoolModeSim::default();
        assert!(!sim.auto_add);

        sim.set_press(true);
        assert!(sim.auto_add);

        sim.set_press(false);
        assert!(!sim.auto_add);

        sim.set_press(true);
        assert!(sim.auto_add);
        sim.set_press(false);
        assert!(!sim.auto_add);
    }

    #[test]
    fn sub_frame_deltas_accumulate_before_step() {
        let mut sim = CoolModeSim::default();
        let config = CoolModeEmitConfig {
            size: Some(20.0),
            particle_count: 4,
            speed_horz: Some(0.0),
            speed_up: Some(0.0),
        };

        sim.advance(Duration::ZERO, 400.0, &config);
        assert_eq!(sim.last_sim_elapsed, Duration::ZERO);

        // Two partial frames must not consume the clock.
        sim.advance(Duration::from_millis(8), 400.0, &config);
        assert_eq!(sim.last_sim_elapsed, Duration::ZERO);
        sim.advance(Duration::from_millis(16), 400.0, &config);
        assert_eq!(sim.last_sim_elapsed, Duration::ZERO);

        // Crossing one FRAME_SECS advances last_sim by exactly one frame, not wall elapsed.
        sim.advance(Duration::from_millis(17), 400.0, &config);
        assert_eq!(
            sim.last_sim_elapsed,
            Duration::from_secs_f32(FRAME_SECS),
            "partial leftovers must remain so the next tick can accumulate"
        );
        assert_ne!(
            sim.last_sim_elapsed,
            Duration::from_millis(17),
            "must not snap last_sim to wall-clock elapsed"
        );
    }
}
