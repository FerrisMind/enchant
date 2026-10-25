//! Thinking orbs: a dotted, depth-shaded mark that shows an assistant at work.
//!
//! A port of the Libraries.dev `thinking-orbs` component. Nine hand-tuned
//! states, each a different animation of the same grayscale dot vocabulary,
//! in two designed sizes (plus an interpolated middle one):
//!
//! | state | what it shows |
//! |---|---|
//! | [`Working`](OrbState::Working) | particles on tilted orbits |
//! | [`Searching`](OrbState::Searching) | a scan meridian sweeping a dotted globe |
//! | [`Solving`](OrbState::Solving) | bands scrambling in quarter turns, then clicking back |
//! | [`Listening`](OrbState::Listening) | a waveform rolling through latitude rings |
//! | [`Connecting`](OrbState::Connecting) | a constellation wiring itself, packets running the edges |
//! | [`Weaving`](OrbState::Weaving) | three strands plaiting around the sphere |
//! | [`Composing`](OrbState::Composing) | an undulating multi-band sash |
//! | [`Breathing`](OrbState::Breathing) | a face-on ring slowly morphing |
//! | [`Shaping`](OrbState::Shaping) | a dotted outline morphing circle, triangle, square |
//!
//! Beyond the stock look it supports speed, dark/light/auto theming, a
//! tint, dot density and dot size multipliers, per-knob overrides
//! ([`OrbOptions`]), a custom frame function, a paused mode, a reduced-motion
//! static frame, and the optional [`gravity`] pointer effect.
//!
//! # Examples
//!
//! ```rust,no_run
//! use enchant_iced::{OrbSize, OrbState, ThinkingOrb};
//! use iced::Element;
//!
//! fn view<'a, Message: 'a>() -> Element<'a, Message> {
//!     ThinkingOrb::new()
//!         .state(OrbState::Searching)
//!         .size(OrbSize::Large)
//!         .into_element()
//! }
//! ```
//!
//! Headless frames are available for export and tests:
//!
//! ```
//! use enchant_iced::{OrbState, ThinkingOrb};
//!
//! let frame = ThinkingOrb::new().state(OrbState::Working).frame_at(0.6);
//! assert!(!frame.dots().is_empty());
//! ```

mod frame;
pub mod gravity;
mod math;
mod modes;
mod options;
mod paint;
mod presets;
mod raster;
mod soft;
mod sprite;
mod widget;

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use iced::{Color, Element};

pub use frame::{Dot, Line, OrbFrame};
pub use gravity::{Gravity, GravityStatus, GravityTuning};
pub use options::{OrbKnob, OrbOptions};
pub use presets::{OrbSize, OrbState};
pub use sprite::{CursorSprite, CursorSpriteError};

use presets::{resolve_preset, scale_counts, scale_radii};

/// The phase the static frame is drawn at (reduced motion): a moment where
/// every state reads clearly.
pub const STATIC_FRAME_TIME: f64 = 0.6;

/// Which ink the orb is drawn in.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum OrbTheme {
    /// Follows the iced theme's palette (dark palettes get light ink).
    #[default]
    Auto,
    /// Dark ink for light backgrounds.
    Light,
    /// Light ink for dark backgrounds.
    Dark,
}

/// Whether the orb animates.
///
/// iced exposes no operating-system "reduce motion" setting, so this is the
/// host's switch: wire it to your own preference.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MotionPreference {
    /// Animate normally.
    #[default]
    Full,
    /// Draw one static, deterministic frame (at [`STATIC_FRAME_TIME`]) that
    /// still follows the live theme. Gravity is off.
    Reduced,
}

type CustomFrame = Arc<dyn Fn(f64, f64, &OrbOptions) -> OrbFrame + Send + Sync>;

/// Builder for a thinking orb.
///
/// The orb ticks on its own clock (shared by every orb in the process, so
/// they stay in phase) and asks iced for redraws while it animates and is on
/// screen. To drive it from your own clock instead, pass [`Self::elapsed`].
///
/// # Examples
///
/// ```rust,no_run
/// use enchant_iced::{Gravity, OrbSize, OrbState, OrbTheme, ThinkingOrb};
/// use iced::{Color, Element};
///
/// fn view<'a, Message: 'a>() -> Element<'a, Message> {
///     ThinkingOrb::new()
///         .state(OrbState::Connecting)
///         .size(OrbSize::Inline)
///         .theme(OrbTheme::Dark)
///         .speed(1.5)
///         .color(Color::from_rgb8(0x7c, 0x3a, 0xed))
///         .dots(1.3)
///         .gravity(Gravity::new().reach(200.0))
///         .into_element()
/// }
/// ```
#[must_use = "builders do nothing unless turned into an iced Element"]
#[derive(Clone)]
pub struct ThinkingOrb {
    state: OrbState,
    size: OrbSize,
    theme: OrbTheme,
    speed: f32,
    paused: bool,
    color: Option<Color>,
    dots: f32,
    dot_size: f32,
    options: OrbOptions,
    custom_frame: Option<CustomFrame>,
    gravity: Option<Gravity>,
    motion: MotionPreference,
    label: Option<String>,
    elapsed: Option<Duration>,
    pixel_density: f32,
}

impl Default for ThinkingOrb {
    fn default() -> Self {
        Self {
            state: OrbState::Working,
            size: OrbSize::Large,
            theme: OrbTheme::Auto,
            speed: 1.0,
            paused: false,
            color: None,
            dots: 1.0,
            dot_size: 1.0,
            options: OrbOptions::new(),
            custom_frame: None,
            gravity: None,
            motion: MotionPreference::Full,
            label: None,
            elapsed: None,
            pixel_density: 1.0,
        }
    }
}

impl fmt::Debug for ThinkingOrb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ThinkingOrb")
            .field("state", &self.state)
            .field("size", &self.size)
            .field("theme", &self.theme)
            .field("speed", &self.speed)
            .field("paused", &self.paused)
            .field("color", &self.color)
            .field("dots", &self.dots)
            .field("dot_size", &self.dot_size)
            .field("options", &self.options)
            .field("custom_frame", &self.custom_frame.is_some())
            .field("gravity", &self.gravity)
            .field("motion", &self.motion)
            .field("label", &self.label)
            .field("elapsed", &self.elapsed)
            .field("pixel_density", &self.pixel_density)
            .finish()
    }
}

impl ThinkingOrb {
    /// A large, auto-themed [`OrbState::Working`] orb at normal speed.
    pub fn new() -> Self {
        Self::default()
    }

    /// Which animation to show.
    pub fn state(mut self, state: OrbState) -> Self {
        self.state = state;
        self
    }

    /// Which designed size to render (the widget is a square of that many
    /// logical pixels).
    pub fn size(mut self, size: OrbSize) -> Self {
        self.size = size;
        self
    }

    /// Light, dark, or follow the iced theme.
    pub fn theme(mut self, theme: OrbTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Playback speed multiplier on top of the state's own tempo. Negative or
    /// non-finite values are treated as 0 (frozen).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = if speed.is_finite() {
            speed.max(0.0)
        } else {
            0.0
        };
        self
    }

    /// Freezes the animation on its current frame. A paused orb still draws.
    pub fn paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        self
    }

    /// Tints the ink: the depth ramp is kept, as a ramp on this color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Device pixels per logical pixel the orb is painted at (default 1,
    /// clamped to 1..=4). Pass the window's scale factor on high-density
    /// screens so the dots stay crisp; the reference caps it at 2.
    pub fn pixel_density(mut self, density: f32) -> Self {
        self.pixel_density = if density.is_finite() {
            density.clamp(1.0, 4.0)
        } else {
            1.0
        };
        self
    }

    /// Back to the stock grayscale ink.
    pub fn clear_color(mut self) -> Self {
        self.color = None;
        self
    }

    /// Dot density multiplier (min 0.1), applied to every count of the
    /// state's preset with the same paired scaler the sizes use, so density
    /// changes keep the animation balanced.
    pub fn dots(mut self, dots: f32) -> Self {
        self.dots = if dots.is_finite() { dots.max(0.1) } else { 1.0 };
        self
    }

    /// Dot size multiplier (min 0.1), applied to every radius of the preset.
    pub fn dot_size(mut self, dot_size: f32) -> Self {
        self.dot_size = if dot_size.is_finite() {
            dot_size.max(0.1)
        } else {
            1.0
        };
        self
    }

    /// Raw knob overrides, applied last, over everything the preset and the
    /// multipliers set. Replaces any earlier overrides.
    pub fn options(mut self, options: OrbOptions) -> Self {
        self.options = options;
        self
    }

    /// Overrides a single knob. See [`Self::options`].
    pub fn option(mut self, knob: OrbKnob, value: f64) -> Self {
        self.options.set(knob, value);
        self
    }

    /// Replaces the state's frame function with your own: it receives the
    /// pixel size, the engine time in seconds and the resolved options, and
    /// returns the frame to draw (see [`OrbFrame::finalize`]).
    pub fn custom_frame(
        mut self,
        frame: impl Fn(f64, f64, &OrbOptions) -> OrbFrame + Send + Sync + 'static,
    ) -> Self {
        self.custom_frame = Some(Arc::new(frame));
        self
    }

    /// Enables the pointer-gravity effect. It stays dormant until a pointer
    /// raster is supplied with [`gravity::set_sprite`].
    pub fn gravity(mut self, gravity: Gravity) -> Self {
        self.gravity = Some(gravity);
        self
    }

    /// Turns gravity off again.
    pub fn without_gravity(mut self) -> Self {
        self.gravity = None;
        self
    }

    /// Whether to animate or hold a static frame.
    pub fn motion(mut self, motion: MotionPreference) -> Self {
        self.motion = motion;
        self
    }

    /// The accessible description. Defaults to the state's status text
    /// ("Searching…").
    ///
    /// iced 0.14 has no accessibility tree for custom widgets, so this is
    /// not announced by itself; read it with [`Self::accessible_label`] to
    /// feed a tooltip, a live region of your own, or a platform bridge.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Drives the orb from the host's clock instead of the shared one. The
    /// value is the base clock before the preset and `speed` multipliers.
    /// With a host clock the orb never requests redraws of its own.
    pub fn elapsed(mut self, elapsed: Duration) -> Self {
        self.elapsed = Some(elapsed);
        self
    }

    /// The accessible description: the custom label, else the state's text.
    pub fn accessible_label(&self) -> &str {
        self.label.as_deref().unwrap_or_else(|| self.state.label())
    }

    /// The orb as a square iced element.
    pub fn into_element<'a, Message: 'a>(self) -> Element<'a, Message> {
        Element::new(self)
    }

    /// Alias for [`Self::into_element`].
    pub fn view<'a, Message: 'a>(self) -> Element<'a, Message> {
        self.into_element()
    }

    /// Every knob the engine will see: preset, then density and dot-size
    /// multipliers, then raw overrides.
    pub fn resolved_options(&self) -> OrbOptions {
        self.resolve().opts
    }

    /// The frame this orb would draw at engine time `t` (seconds, after the
    /// preset and speed multipliers). Useful for export and tests.
    pub fn frame_at(&self, t: f64) -> OrbFrame {
        let resolved = self.resolve();
        let size = f64::from(self.size.px());
        match &self.custom_frame {
            Some(frame) => frame(size, t, &resolved.opts),
            None => (resolved.mode.frame_fn())(size, t, &resolved.opts),
        }
    }

    /// The engine time (seconds) for a base clock reading: the clock times
    /// the state's tempo times `speed`; reduced motion pins the static phase.
    pub fn engine_time(&self, clock_seconds: f64) -> f64 {
        if self.motion == MotionPreference::Reduced {
            return STATIC_FRAME_TIME;
        }
        clock_seconds * self.resolve().speed * f64::from(self.speed)
    }

    fn resolve(&self) -> presets::Resolved {
        let mut resolved = resolve_preset(self.state, self.size);
        if (self.dots - 1.0).abs() > f32::EPSILON {
            resolved.opts = scale_counts(&resolved.opts, f64::from(self.dots.max(0.1)));
        }
        if (self.dot_size - 1.0).abs() > f32::EPSILON {
            resolved.opts = scale_radii(&resolved.opts, f64::from(self.dot_size.max(0.1)));
        }
        resolved.opts = resolved.opts.merged(&self.options);
        resolved
    }
}

impl<'a, Message: 'a> From<ThinkingOrb> for Element<'a, Message> {
    fn from(orb: ThinkingOrb) -> Self {
        orb.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_reference_component() {
        let orb = ThinkingOrb::new();
        assert_eq!(orb.state, OrbState::Working);
        assert_eq!(orb.size, OrbSize::Large);
        assert_eq!(orb.theme, OrbTheme::Auto);
        assert_eq!(orb.speed, 1.0);
        assert!(!orb.paused);
        assert_eq!(orb.dots, 1.0);
        assert_eq!(orb.dot_size, 1.0);
        assert!(orb.gravity.is_none());
        assert_eq!(orb.accessible_label(), "Working…");
    }

    #[test]
    fn setters_validate() {
        let orb = ThinkingOrb::new()
            .speed(-3.0)
            .dots(0.0)
            .dot_size(f32::NAN)
            .label("Thinking hard");
        assert_eq!(orb.speed, 0.0);
        assert_eq!(orb.dots, 0.1);
        assert_eq!(orb.dot_size, 1.0);
        assert_eq!(orb.accessible_label(), "Thinking hard");
        assert!(format!("{orb:?}").contains("ThinkingOrb"));
    }

    #[test]
    fn engine_time_multiplies_tempo_and_speed_and_pins_reduced_motion() {
        let orb = ThinkingOrb::new().state(OrbState::Working).speed(2.0);
        assert!((orb.engine_time(1.0) - 1.885 * 2.0).abs() < 1e-9);
        let reduced = orb.motion(MotionPreference::Reduced);
        assert_eq!(reduced.engine_time(123.0), STATIC_FRAME_TIME);
    }

    #[test]
    fn multipliers_and_overrides_stack_in_order() {
        let base = ThinkingOrb::new().state(OrbState::Working);
        let dense = base.clone().dots(2.0);
        assert!(
            dense.resolved_options().get(OrbKnob::OrbitN)
                > base.resolved_options().get(OrbKnob::OrbitN)
        );
        let big = base.clone().dot_size(2.0);
        assert!(
            big.resolved_options().get(OrbKnob::PartR)
                > base.resolved_options().get(OrbKnob::PartR)
        );
        let forced = base.clone().dots(2.0).option(OrbKnob::OrbitN, 3.0);
        assert_eq!(forced.resolved_options().get(OrbKnob::OrbitN), Some(3.0));
    }

    #[test]
    fn every_state_and_size_draws_something() {
        for state in OrbState::ALL {
            for size in OrbSize::ALL {
                let frame = ThinkingOrb::new().state(state).size(size).frame_at(0.9);
                assert!(!frame.dots().is_empty(), "{state} at {size} drew no dots");
            }
        }
    }

    #[test]
    fn a_custom_frame_replaces_the_state_frame() {
        let orb = ThinkingOrb::new().custom_frame(|size, _t, _o| {
            OrbFrame::new(
                vec![Dot::new(size / 2.0, size / 2.0, 0.0, 2.0, 0.0)],
                vec![],
            )
        });
        assert_eq!(orb.frame_at(0.0).dots().len(), 1);
    }
}
