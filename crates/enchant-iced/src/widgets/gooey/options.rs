//! Public options and tuning for [`Liquid`](super::Liquid) and
//! [`LiquidItem`](super::LiquidItem).
//!
//! The reference library exposes CSS values (numbers, `box-shadow` strings,
//! spring presets) through React props. This port keeps the same knobs, but
//! every setter validates and clamps its input so a malformed value can never
//! reach the rasterizer. Raw physics escape hatches ([`EvolveOptions`],
//! [`MoveOptions`], [`DissolveOptions`]) are plain data, matching the
//! reference's `advanced` props.

use std::time::Duration;

use iced::advanced::image::Handle;
use iced::{Color, Point, Size, Vector};

use super::geometry::CornerRadii;
use super::shadow;
use super::spring::{Easing, SpringConfig, Transition, TransitionPreset};

/// The default goo blur radius (upstream `blur={6}`), in logical pixels.
pub const DEFAULT_BLUR: f32 = 6.0;
/// The default alpha-contrast slope (upstream `contrast={18}`).
pub const DEFAULT_CONTRAST: f32 = 18.0;
/// The default silhouette fill (upstream `fill="#fff"`).
pub const DEFAULT_FILL: Color = Color::WHITE;
/// The default transparent margin added around the filter region.
pub const DEFAULT_FILTER_PADDING: f32 = 24.0;
/// The default undulation frequency of [`Liquid::waviness`](super::Liquid::waviness).
pub const DEFAULT_WAVINESS_FREQ: f32 = 0.018;

/// Tuning for the `morph` effect: the liquid springs behind size changes,
/// travels as a droplet and settles like jelly.
///
/// This mirrors upstream `MorphTuning`. Every knob is normalized; the raw
/// physics live in [`EvolveOptions`] via
/// [`Morph::advanced_evolve`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Morph {
    pub(crate) shape: bool,
    pub(crate) speed: f32,
    pub(crate) bounce: f32,
    pub(crate) content_blur: Option<f32>,
    pub(crate) evolve: Option<EvolveOptions>,
    pub(crate) blob_inset: f32,
    pub(crate) bridge_grow: f32,
}

impl Morph {
    /// The tuned look, with the shape physics **off** (plain merge needs no
    /// engine). Enable it with [`Morph::shape`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            shape: false,
            speed: 1.0,
            bounce: 0.5,
            content_blur: None,
            evolve: None,
            blob_inset: 0.0,
            bridge_grow: 0.0,
        }
    }

    /// Enables the shape physics (`true`) or keeps the liquid a plain merge.
    #[must_use]
    pub const fn shape(mut self, shape: bool) -> Self {
        self.shape = shape;
        self
    }

    /// Speed multiplier, `>= 0.25`. `1` is the tuned default.
    #[must_use]
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = finite(speed, 1.0).clamp(0.25, 8.0);
        self
    }

    /// `0..=1` bounciness. `0` is calm and critically damped, `1` very
    /// springy. Default `0.5`.
    #[must_use]
    pub fn bounce(mut self, bounce: f32) -> Self {
        self.bounce = finite(bounce, 0.5).clamp(0.0, 1.0);
        self
    }

    /// Maximum content cross-blur in pixels while the liquid moves. Applies
    /// to image content (the crate cannot read back arbitrary widget layers).
    /// Default [`EvolveOptions::default().content_blur`].
    #[must_use]
    pub fn content_blur(mut self, blur: f32) -> Self {
        self.content_blur = Some(non_negative(blur));
        self
    }

    /// Raw engine physics, merged over the values mapped from [`speed`] and
    /// [`bounce`](Self::bounce).
    ///
    /// [`speed`]: Self::speed
    #[must_use]
    pub const fn advanced_evolve(mut self, options: EvolveOptions) -> Self {
        self.evolve = Some(options);
        self
    }

    /// Shrinks the liquid by this many pixels per side so opaque content
    /// fully covers its own blob (round photos). Default `0`.
    #[must_use]
    pub fn blob_inset(mut self, inset: f32) -> Self {
        self.blob_inset = non_negative(inset);
        self
    }

    /// Pixels the blob swells back out near a neighbour, revealing the liquid
    /// coat that necks into the other surface. Default `0`.
    #[must_use]
    pub fn bridge_grow(mut self, grow: f32) -> Self {
        self.bridge_grow = non_negative(grow);
        self
    }

    /// Whether the shape physics is enabled.
    #[must_use]
    pub const fn is_shape(&self) -> bool {
        self.shape
    }

    /// The resolved physics, with the public knobs mapped exactly like
    /// upstream `mapMorphSprings`.
    #[must_use]
    pub fn resolve(&self) -> EvolveOptions {
        let speed = self.speed.max(0.25);
        let base = EVOLVE_DEFAULTS;
        let k = zeta(self.bounce) / zeta(0.5);
        EvolveOptions {
            mass_stiffness: base.mass_stiffness * speed * speed,
            mass_damping: base.mass_damping * speed * k,
            size_stiffness: base.size_stiffness * speed * speed,
            size_damping: base.size_damping * speed * k,
            radius_stiffness: base.radius_stiffness * speed * speed,
            radius_damping: base.radius_damping * speed,
            content_blur: self.content_blur.unwrap_or(base.content_blur),
            corner_duration: base.corner_duration / speed,
            ..base
        }
        .merged(self.evolve)
    }
}

impl Default for Morph {
    fn default() -> Self {
        Self::new()
    }
}

/// Damping ratio from a `0..=1` bounciness knob (upstream `zeta`).
#[must_use]
pub(crate) fn zeta(bounce: f32) -> f32 {
    (1.0 - 1.1 * bounce.clamp(0.0, 1.0)).max(0.12)
}

/// Tuning for the `move` effect: the surface trails a moving element like
/// liquid rubber with a droplet tail.
///
/// Mirrors upstream `MoveTuning`; raw physics via [`Move::advanced`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Move {
    pub(crate) springiness: f32,
    pub(crate) wobble: f32,
    pub(crate) stretch: f32,
    pub(crate) trail: f32,
    pub(crate) advanced: Option<MoveOptions>,
}

impl Move {
    /// The tuned defaults (`springiness .5`, `wobble .5`, `stretch .36`,
    /// `trail .575`).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            springiness: 0.5,
            wobble: 0.5,
            stretch: 0.36,
            trail: 0.575,
            advanced: None,
        }
    }

    /// How tightly the liquid chases the element, `0..=1`. `0` is heavy syrup,
    /// `1` near-instant. Default `0.5`.
    #[must_use]
    pub fn springiness(mut self, springiness: f32) -> Self {
        self.springiness = finite(springiness, 0.5).clamp(0.0, 1.0);
        self
    }

    /// How much the surface overshoots and wobbles on arrival, `0..=1`.
    #[must_use]
    pub fn wobble(mut self, wobble: f32) -> Self {
        self.wobble = finite(wobble, 0.5).clamp(0.0, 1.0);
        self
    }

    /// Velocity stretch of the drop, `0..=1`. `0` is rigid.
    #[must_use]
    pub fn stretch(mut self, stretch: f32) -> Self {
        self.stretch = finite(stretch, 0.36).clamp(0.0, 1.0);
        self
    }

    /// Trailing droplet size, `0..=1`. `0` disables the tail.
    #[must_use]
    pub fn trail(mut self, trail: f32) -> Self {
        self.trail = finite(trail, 0.575).clamp(0.0, 1.0);
        self
    }

    /// Raw spring values, merged over the mapped knobs.
    #[must_use]
    pub const fn advanced(mut self, options: MoveOptions) -> Self {
        self.advanced = Some(options);
        self
    }

    /// The resolved physics, mapped exactly like upstream `mapMove`.
    #[must_use]
    pub fn resolve(&self) -> MoveOptions {
        let p = self.springiness;
        let stiffness = MOVE_DEFAULTS.stiffness * 10.0_f32.powf(p - 0.5);
        let damping = MOVE_DEFAULTS.damping
            * (stiffness / MOVE_DEFAULTS.stiffness).sqrt()
            * (zeta(self.wobble) / zeta(0.5));
        MoveOptions {
            stiffness,
            damping,
            stretch: 0.5 * self.stretch,
            tail: 0.8 * self.trail,
            ..MOVE_DEFAULTS
        }
        .merged(self.advanced)
    }
}

impl Default for Move {
    fn default() -> Self {
        Self::new()
    }
}

/// Tuning for the `bend` effect: the surface stays glued to the content and
/// the body deforms with velocity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bend {
    pub(crate) vertical: f32,
    pub(crate) horizontal: f32,
    pub(crate) advanced: Option<MoveOptions>,
}

impl Bend {
    /// Upstream defaults: vertical `0.6`, horizontal `0.35`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            vertical: 0.6,
            horizontal: 0.35,
            advanced: None,
        }
    }

    /// Vertical bow strength, `0..=1`.
    #[must_use]
    pub fn vertical(mut self, strength: f32) -> Self {
        self.vertical = finite(strength, 0.6).clamp(0.0, 1.0);
        self
    }

    /// Horizontal cap deformation, `0..=1`.
    #[must_use]
    pub fn horizontal(mut self, strength: f32) -> Self {
        self.horizontal = finite(strength, 0.35).clamp(0.0, 1.0);
        self
    }

    /// Raw physics, merged last.
    #[must_use]
    pub const fn advanced(mut self, options: MoveOptions) -> Self {
        self.advanced = Some(options);
        self
    }

    /// The resolved physics: springiness `1`, no stretch, no tail, plus the
    /// bend knobs (upstream `mapBend`).
    #[must_use]
    pub fn resolve(&self) -> MoveOptions {
        MoveOptions {
            stiffness: 1200.0,
            damping: 70.0,
            stretch: 0.0,
            tail: 0.0,
            bend: self.vertical,
            bend_x: self.horizontal,
            ..MOVE_DEFAULTS
        }
        .merged(self.advanced)
    }
}

impl Default for Bend {
    fn default() -> Self {
        Self::new()
    }
}

/// Raw physics of the `morph` shape engine (upstream `EvolveOptions`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EvolveOptions {
    /// Spring that moves the mass centre.
    pub mass_stiffness: f32,
    /// Damping of the mass centre.
    pub mass_damping: f32,
    /// Spring that follows size changes.
    pub size_stiffness: f32,
    /// Damping of the size spring.
    pub size_damping: f32,
    /// Spring that adapts the corner radius.
    pub radius_stiffness: f32,
    /// Damping of the radius spring.
    pub radius_damping: f32,
    /// Maximum content cross-blur in pixels.
    pub content_blur: f32,
    /// `0..=1` how round the droplet becomes at the start of a morph.
    pub roundness: f32,
    /// Milliseconds of the corner timeline.
    pub corner_duration: f32,
    /// Milliseconds of delay before the corner timeline starts.
    pub corner_delay: f32,
    /// Easing of the corner timeline.
    pub corner_ease: Easing,
    /// Milliseconds the travel lead takes to ramp in.
    pub anticipation: f32,
    /// Pixels the mass centre leads ahead of the element.
    pub travel: f32,
}

impl Default for EvolveOptions {
    fn default() -> Self {
        EVOLVE_DEFAULTS
    }
}

impl EvolveOptions {
    /// Field-wise merge where a value equal to [`EVOLVE_DEFAULTS`] means "keep
    /// the mapped value" — the closest Rust analogue of upstream's partial
    /// `{...mapped, ...advanced.evolve}` spread.
    fn merged(mut self, over: Option<EvolveOptions>) -> Self {
        let Some(o) = over else {
            return self;
        };
        let d = EVOLVE_DEFAULTS;
        if o.mass_stiffness != d.mass_stiffness {
            self.mass_stiffness = o.mass_stiffness;
        }
        if o.mass_damping != d.mass_damping {
            self.mass_damping = o.mass_damping;
        }
        if o.size_stiffness != d.size_stiffness {
            self.size_stiffness = o.size_stiffness;
        }
        if o.size_damping != d.size_damping {
            self.size_damping = o.size_damping;
        }
        if o.radius_stiffness != d.radius_stiffness {
            self.radius_stiffness = o.radius_stiffness;
        }
        if o.radius_damping != d.radius_damping {
            self.radius_damping = o.radius_damping;
        }
        if o.content_blur != d.content_blur {
            self.content_blur = o.content_blur;
        }
        if o.roundness != d.roundness {
            self.roundness = o.roundness;
        }
        if o.corner_duration != d.corner_duration {
            self.corner_duration = o.corner_duration;
        }
        if o.corner_delay != d.corner_delay {
            self.corner_delay = o.corner_delay;
        }
        if o.corner_ease != d.corner_ease {
            self.corner_ease = o.corner_ease;
        }
        if o.anticipation != d.anticipation {
            self.anticipation = o.anticipation;
        }
        if o.travel != d.travel {
            self.travel = o.travel;
        }
        self
    }
}

/// The tuned upstream `EVOLVE_DEFAULTS`.
pub const EVOLVE_DEFAULTS: EvolveOptions = EvolveOptions {
    mass_stiffness: 320.0,
    mass_damping: 17.0,
    size_stiffness: 170.0,
    size_damping: 11.5,
    radius_stiffness: 900.0,
    radius_damping: 60.0,
    content_blur: 7.0,
    roundness: 1.0,
    corner_duration: 460.0,
    corner_delay: 0.0,
    corner_ease: Easing::CubicBezier {
        x1: 0.3,
        y1: 1.05,
        x2: 0.4,
        y2: 1.0,
    },
    anticipation: 90.0,
    travel: 32.0,
};

/// Raw physics of the `move` effect (upstream `MoveOptions`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveOptions {
    /// Spring stiffness pulling the surface after the element.
    pub stiffness: f32,
    /// Spring damping.
    pub damping: f32,
    /// Maximum axial stretch at speed (`0` = rigid).
    pub stretch: f32,
    /// Trailing droplet size as a fraction of the body.
    pub tail: f32,
    /// How far the tail extends behind the body, `0..=1`.
    pub force: f32,
    /// Vertical body bow, `0..=1`.
    pub bend: f32,
    /// Horizontal cap deformation, `0..=1`.
    pub bend_x: f32,
}

impl Default for MoveOptions {
    fn default() -> Self {
        MOVE_DEFAULTS
    }
}

impl MoveOptions {
    /// Field-wise merge where a value equal to [`MOVE_DEFAULTS`] means "keep
    /// the mapped value".
    fn merged(mut self, over: Option<MoveOptions>) -> Self {
        let Some(o) = over else {
            return self;
        };
        let d = MOVE_DEFAULTS;
        if o.stiffness != d.stiffness {
            self.stiffness = o.stiffness;
        }
        if o.damping != d.damping {
            self.damping = o.damping;
        }
        if o.stretch != d.stretch {
            self.stretch = o.stretch;
        }
        if o.tail != d.tail {
            self.tail = o.tail;
        }
        if o.force != d.force {
            self.force = o.force;
        }
        if o.bend != d.bend {
            self.bend = o.bend;
        }
        if o.bend_x != d.bend_x {
            self.bend_x = o.bend_x;
        }
        self
    }
}

/// The tuned upstream `MOVE_DEFAULTS`.
pub const MOVE_DEFAULTS: MoveOptions = MoveOptions {
    stiffness: 380.0,
    damping: 18.0,
    stretch: 0.18,
    tail: 0.46,
    force: 0.5,
    bend: 0.0,
    bend_x: 0.0,
};

/// Raw options of the `dissolve` modifier (upstream `DissolveOptions`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DissolveOptions {
    /// `0..=1` ceiling that scales warp, blur, gravity and the edge hole.
    pub strength: f32,
    /// Turbulence displacement in pixels.
    pub warp: f32,
    /// Gaussian blur sigma in pixels.
    pub blur: f32,
    /// `0..=1` two-liquid mixing (erosion) amount.
    pub mix: f32,
    /// Stretch toward the neighbour, in pixels.
    pub gravity: f32,
    /// `0..=1` how much the imagery is drawn out toward the neighbour.
    pub taper: f32,
    /// Turbulence frequency multiplier.
    pub warp_freq: f32,
    /// Noise churn speed while dragging.
    pub flow_speed: f32,
    /// Noise octaves (1..=3).
    pub detail: u32,
    /// Melt zone radius in pixels. `None` = `goo_blur * 2.2 + 4`.
    pub zone: Option<f32>,
    /// Proximity range in pixels. `None` = `max(10, goo_blur * 2.5)`.
    pub range: Option<f32>,
    /// Milliseconds of the structural release.
    pub release_ms: f32,
    /// Milliseconds of the opacity fade. `None` falls back to `release_ms`.
    pub fade_ms: Option<f32>,
    /// Blur sigma of the seam colour-averaging wash. `None` = `blur * 1.6`.
    pub seam_blur: Option<f32>,
    /// Overlap fraction at which the melt has fully sunk away.
    pub sink: f32,
    /// Magnetic pull of the melted body toward the contact, in pixels.
    pub pull: f32,
    /// Whether the melt is active (upstream `active`).
    pub active: bool,
}

impl Default for DissolveOptions {
    fn default() -> Self {
        Self {
            strength: 1.0,
            warp: 26.0,
            blur: 8.0,
            mix: 0.7,
            gravity: 60.0,
            taper: 1.0,
            warp_freq: 1.7,
            flow_speed: 22.0,
            detail: 2,
            zone: None,
            range: None,
            release_ms: 110.0,
            fade_ms: None,
            seam_blur: None,
            sink: 0.45,
            pull: 0.0,
            active: true,
        }
    }
}

impl DissolveOptions {
    /// The upstream tuned dissolve (`mapDissolve(true)`).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            strength: 1.0,
            warp: 26.0,
            blur: 8.0,
            mix: 0.7,
            gravity: 60.0,
            taper: 1.0,
            warp_freq: 1.7,
            flow_speed: 22.0,
            detail: 2,
            zone: None,
            range: None,
            release_ms: 110.0,
            fade_ms: None,
            seam_blur: None,
            sink: 0.45,
            pull: 0.0,
            active: true,
        }
    }

    /// Scales the melt as a ceiling.
    #[must_use]
    pub fn strength(mut self, strength: f32) -> Self {
        self.strength = finite(strength, 1.0).clamp(0.0, 1.0);
        self
    }

    /// Turns the melt on or off without removing its configuration.
    #[must_use]
    pub const fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Overrides the melt zone radius.
    #[must_use]
    pub fn zone(mut self, zone: f32) -> Self {
        self.zone = Some(non_negative(zone));
        self
    }

    /// Overrides the proximity range.
    #[must_use]
    pub fn range(mut self, range: f32) -> Self {
        self.range = Some(non_negative(range));
        self
    }

    /// Overrides the release duration.
    #[must_use]
    pub fn release(mut self, release: Duration) -> Self {
        self.release_ms = clamp_ms(release);
        self
    }

    /// Overrides the opacity fade duration.
    #[must_use]
    pub fn fade(mut self, fade: Duration) -> Self {
        self.fade_ms = Some(clamp_ms(fade));
        self
    }
}

/// Options of the image-to-image melt (upstream `ImageMeltOptions`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageMeltOptions {
    /// Gaussian blur sigma of the melted imagery.
    pub blur: f32,
    /// Goo contrast used by the melt mask.
    pub contrast: f32,
    /// Fraction of the pair distance at which the melt reaches full strength.
    pub reach: f32,
    /// Fade duration in milliseconds.
    pub fade: f32,
    /// Turbulence displacement in pixels.
    pub warp: f32,
    /// `0..=1` mixing amount.
    pub mix: f32,
    /// Blur used by the mixing pass.
    pub mix_blur: f32,
    /// Gravity stretch toward the neighbour.
    pub gravity: f32,
    /// Undulation amplitude in pixels.
    pub waviness: f32,
}

impl Default for ImageMeltOptions {
    fn default() -> Self {
        IMAGE_MELT_DEFAULTS
    }
}

/// The tuned upstream `IMAGE_MELT_DEFAULTS`.
pub const IMAGE_MELT_DEFAULTS: ImageMeltOptions = ImageMeltOptions {
    blur: 7.0,
    contrast: 40.0,
    reach: 0.8,
    fade: 17.0,
    warp: 0.0,
    mix: 1.0,
    mix_blur: 8.0,
    gravity: 1.9,
    waviness: 12.0,
};

/// The parsed `box-shadow` stack of a group.
///
/// Build one from a CSS string ([`Shadow::parse`] / `From<&str>`) or from
/// typed [`ShadowLayer`]s. An invalid string yields no layers rather than an
/// error: the reference silently ignores unknown tokens too.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Shadow {
    layers: Vec<ShadowLayer>,
}

impl Shadow {
    /// An empty stack.
    #[must_use]
    pub const fn none() -> Self {
        Self { layers: Vec::new() }
    }

    /// Parses the CSS `box-shadow` syntax.
    ///
    /// Unknown colors default to `rgba(0, 0, 0, 0.35)`, unknown tokens are
    /// ignored, `none` and empty strings yield no layers.
    #[must_use]
    pub fn parse(input: &str) -> Self {
        Self {
            layers: shadow::parse(input),
        }
    }

    /// Adds one typed layer.
    #[must_use]
    pub fn push(mut self, layer: ShadowLayer) -> Self {
        self.layers.push(layer);
        self
    }

    /// The layers, in CSS order (first = topmost).
    #[must_use]
    pub fn layers(&self) -> &[ShadowLayer] {
        &self.layers
    }

    /// Whether the stack is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }
}

/// One CSS `box-shadow` layer, re-exported from the parser.
pub use shadow::ShadowLayer;

impl From<&str> for Shadow {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for Shadow {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl From<ShadowLayer> for Shadow {
    fn from(layer: ShadowLayer) -> Self {
        Self {
            layers: vec![layer],
        }
    }
}

impl From<Vec<ShadowLayer>> for Shadow {
    fn from(layers: Vec<ShadowLayer>) -> Self {
        Self { layers }
    }
}

impl From<bool> for DissolveOptions {
    fn from(enabled: bool) -> Self {
        let mut options = Self::new();
        options.active = enabled;
        options
    }
}

impl From<f32> for DissolveOptions {
    fn from(strength: f32) -> Self {
        Self::new().strength(strength)
    }
}

/// How a [`Liquid`](super::Liquid) positions its items.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum LiquidLayout {
    /// Items flow left to right with `spacing` between them.
    Row {
        /// Gap between adjacent items in logical pixels.
        spacing: f32,
    },
    /// Items flow top to bottom with `spacing` between them.
    Column {
        /// Gap between adjacent items in logical pixels.
        spacing: f32,
    },
    /// Items are placed at their own [`LiquidItem::position`].
    Free,
}

impl Default for LiquidLayout {
    fn default() -> Self {
        Self::Row { spacing: 0.0 }
    }
}

/// A resolved item box in logical coordinates (top-left origin).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemBox {
    /// Top-left corner.
    pub origin: Point,
    /// Width and height.
    pub size: Size,
}

impl ItemBox {
    /// Builds a box.
    #[must_use]
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }

    /// The centre of the box.
    #[must_use]
    pub fn center(&self) -> Point {
        Point::new(
            self.origin.x + self.size.width / 2.0,
            self.origin.y + self.size.height / 2.0,
        )
    }

    /// The right edge.
    #[must_use]
    pub fn right(&self) -> f32 {
        self.origin.x + self.size.width
    }

    /// The bottom edge.
    #[must_use]
    pub fn bottom(&self) -> f32 {
        self.origin.y + self.size.height
    }

    pub(crate) fn translated(mut self, offset: Vector) -> Self {
        self.origin.x += offset.x;
        self.origin.y += offset.y;
        self
    }

    pub(crate) fn scaled(mut self, scale: f32) -> Self {
        let center = self.center();
        self.size.width = (self.size.width * scale).max(0.0);
        self.size.height = (self.size.height * scale).max(0.0);
        self.origin = Point::new(
            center.x - self.size.width / 2.0,
            center.y - self.size.height / 2.0,
        );
        self
    }

    pub(crate) fn inset(self, amount: f32) -> Self {
        Self::new(
            self.origin.x + amount,
            self.origin.y + amount,
            (self.size.width - amount * 2.0).max(0.0),
            (self.size.height - amount * 2.0).max(0.0),
        )
    }
}

/// Everything a [`LiquidItem`](super::LiquidItem) can be tuned with.
///
/// This is the non-generic half of an item; the content and drag callback stay
/// on the item itself.
#[derive(Debug, Clone)]
pub struct ItemOptions {
    pub(crate) radius: Option<CornerRadii>,
    pub(crate) scale: f32,
    pub(crate) transition: Transition,
    /// Whether [`ItemOptions::transition`] was set explicitly.
    ///
    /// Move effects spring after a target that already carries CSS-style
    /// easing only when the caller asked for one; the default would otherwise
    /// smooth every drag twice.
    pub(crate) transition_set: bool,
    pub(crate) delay: Option<Duration>,
    pub(crate) observe: bool,
    pub(crate) position: Option<Point>,
    pub(crate) offset: Vector,
    pub(crate) size: Option<Size>,
    pub(crate) morph: Option<Morph>,
    pub(crate) move_effect: Option<Move>,
    pub(crate) bend: Option<Bend>,
    pub(crate) dissolve: Option<DissolveOptions>,
    pub(crate) melt: Option<ImageMeltOptions>,
    pub(crate) draggable: bool,
    pub(crate) drag_bounds: Option<ItemBox>,
    pub(crate) image: Option<Handle>,
    pub(crate) index: usize,
    pub(crate) silhouette: bool,
    pub(crate) behind: bool,
}

impl Default for ItemOptions {
    fn default() -> Self {
        Self {
            radius: None,
            scale: 1.0,
            transition: Transition::Smooth,
            transition_set: false,
            delay: None,
            observe: false,
            position: None,
            offset: Vector::ZERO,
            size: None,
            morph: None,
            move_effect: None,
            bend: None,
            dissolve: None,
            melt: None,
            draggable: false,
            drag_bounds: None,
            image: None,
            index: 0,
            silhouette: true,
            behind: false,
        }
    }
}

impl ItemOptions {
    /// The effective transition.
    #[must_use]
    pub fn transition(&self) -> Transition {
        self.transition
    }

    /// Whether the item's liquid is driven by its own element movement.
    #[must_use]
    pub fn is_observed(&self) -> bool {
        self.observe
            || self.morph.is_some_and(|m| m.shape)
            || self.dissolve.is_some()
            || self.move_effect.is_some()
            || self.bend.is_some()
    }
}

/// Convenience: a transition from any supported spec.
#[must_use]
pub fn transition(spec: impl Into<Transition>) -> Transition {
    spec.into()
}

/// A preset shorthand used by [`LiquidItem::transition`](super::LiquidItem::transition).
#[must_use]
pub fn preset(preset: TransitionPreset) -> Transition {
    Transition::Preset(preset)
}

/// A custom spring shorthand.
#[must_use]
pub fn spring(config: SpringConfig) -> Transition {
    Transition::Spring(config)
}

fn finite(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

fn non_negative(value: f32) -> f32 {
    finite(value, 0.0).max(0.0)
}

fn clamp_ms(duration: Duration) -> f32 {
    (duration.as_secs_f32() * 1000.0).clamp(0.0, 60_000.0)
}
