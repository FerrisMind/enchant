//! The iced [`BotAvatar`] widget: clock, pointer play and rasterized frames.

use std::cell::RefCell;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Instant;

use iced::advanced::image::{self, FilterMethod, Image, Renderer as _};
use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{self, Tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, mouse, window};
use tiny_skia::{Path, Pixmap};

use super::color::{auto_ink, parse_color, richer, shade};
use super::data::{bot_avatar_parts, bot_avatar_preset, bot_avatar_shape, bot_avatar_state_label};
use super::engine::{BotAvatarSim, JUMP_DEFAULTS, JumpConfig, Pose, rest_pose};
use super::path::{parse_svg_path, parse_svg_subpaths};
use super::render::draw::{LIGHT_FABRIC, LIGHT_OTHER};
use super::render::wear::Wear;
use super::render::whirl::WhirlConfig;
use super::render::{FurStyle, OVERSCAN, RISE, RenderConfig, draw_frame};
use super::types::{
    BotAvatarFace, BotAvatarFur, BotAvatarGlasses, BotAvatarHat, BotAvatarShading,
    BotAvatarSquashEase, BotAvatarState, BotAvatarTheme, BotAvatarType,
};

const DEFAULT_ACCESSORY: &str = "#27272b";

fn clamp(v: f32, lo: f32, hi: f32) -> f32 {
    if v.is_finite() { v.clamp(lo, hi) } else { lo }
}

fn q20(v: f32, lo: f32, hi: f32) -> f32 {
    (clamp(v, lo, hi) * 20.0).round() / 20.0
}

/// An animated bot avatar, ported from the Libraries.dev `bot-avatars`
/// package: eighteen body shapes, two faces, three states, five shading
/// modes and a wardrobe of hats, glasses, headphones and a bow tie.
///
/// The widget owns its animation clock and asks for a redraw only while it
/// can move. A pointer near it draws the eyes and the head; a click makes it
/// hop and spin. Use [`paused`](Self::paused) to hold the current frame,
/// [`speed`](Self::speed) to slow or hurry it, and
/// [`reduce_motion`](Self::reduce_motion) to hold the state's rest pose.
///
/// Frames are rasterised with `tiny-skia` and drawn through iced's image
/// renderer, with the canvas overscanned `OVERSCAN` times so a hop or a
/// flip has room to leave the avatar's box.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::bot_avatar::{
///     BotAvatar, BotAvatarGlasses, BotAvatarHat, BotAvatarState, BotAvatarType,
/// };
///
/// let avatar = BotAvatar::new()
///     .bot_type(BotAvatarType::Cat)
///     .state(BotAvatarState::Working)
///     .hat(BotAvatarHat::Beanie)
///     .glasses(BotAvatarGlasses::Round)
///     .size(96.0)
///     .on_click(|| ())
///     .into_element();
/// # let _ = avatar;
/// ```
#[must_use = "widgets do nothing unless turned into an iced Element"]
pub struct BotAvatar<Message> {
    bot_type: BotAvatarType,
    custom_path: Option<Arc<Path>>,
    custom_key: Option<String>,
    parts: Vec<Arc<Path>>,
    face: Option<BotAvatarFace>,
    state: BotAvatarState,
    size: f32,
    color: Option<String>,
    ink: Option<String>,
    brightness: Option<f32>,
    saturation: Option<f32>,
    speed: f32,
    paused: bool,
    seed: Option<f32>,
    shading: BotAvatarShading,
    hat: BotAvatarHat,
    glasses: BotAvatarGlasses,
    headphones: bool,
    bow_tie: bool,
    accessory_color: String,
    shadow: Option<f32>,
    highlight: Option<f32>,
    depth: f32,
    roundness: f32,
    fur: BotAvatarFur,
    light: Option<f32>,
    rim: Option<f32>,
    spread: Option<f32>,
    back_light: Option<f32>,
    light_front: Option<f32>,
    shine: Option<f32>,
    sheen: Option<f32>,
    back_softness: Option<f32>,
    held_pose: Option<Pose>,
    interactive: bool,
    theme: BotAvatarTheme,
    turn: f32,
    whirl: WhirlConfig,
    jump: JumpConfig,
    reduced_motion: bool,
    pixel_density: Option<f32>,
    on_click: Option<Box<dyn Fn() -> Message>>,
}

impl<Message> fmt::Debug for BotAvatar<Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BotAvatar")
            .field("bot_type", &self.bot_type)
            .field("state", &self.state)
            .field("size", &self.size)
            .field("shading", &self.shading)
            .field("paused", &self.paused)
            .finish_non_exhaustive()
    }
}

impl<Message> Default for BotAvatar<Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Message> BotAvatar<Message> {
    /// A clover in the plush fabric material, 64 logical pixels square.
    pub fn new() -> Self {
        Self {
            bot_type: BotAvatarType::Clover,
            custom_path: None,
            custom_key: None,
            parts: Vec::new(),
            face: None,
            state: BotAvatarState::Idle,
            size: 64.0,
            color: None,
            ink: None,
            brightness: None,
            saturation: None,
            speed: 1.0,
            paused: false,
            seed: None,
            shading: BotAvatarShading::Fabric,
            hat: BotAvatarHat::None,
            glasses: BotAvatarGlasses::None,
            headphones: false,
            bow_tie: false,
            accessory_color: DEFAULT_ACCESSORY.to_owned(),
            shadow: None,
            highlight: None,
            depth: 0.65,
            roundness: 1.0,
            fur: BotAvatarFur::default(),
            light: None,
            rim: None,
            spread: None,
            back_light: None,
            light_front: None,
            shine: None,
            sheen: None,
            back_softness: None,
            held_pose: None,
            interactive: true,
            theme: BotAvatarTheme::Auto,
            turn: 1.0,
            whirl: WhirlConfig::default(),
            jump: JUMP_DEFAULTS,
            reduced_motion: false,
            pixel_density: None,
            on_click: None,
        }
    }

    /// Sets the body shape.
    pub fn bot_type(mut self, bot_type: BotAvatarType) -> Self {
        self.bot_type = bot_type;
        self.custom_path = None;
        self.custom_key = None;
        self.parts = Vec::new();
        self
    }

    /// Sets a custom body outline: SVG path data in the 100×100 body box,
    /// centred on `(50, 50)`, replacing the type's outline and thin parts.
    ///
    /// Invalid path data is ignored and the type's own outline is kept. The
    /// body should stay solid round the face.
    pub fn path(mut self, data: impl AsRef<str>) -> Self {
        let data = data.as_ref().trim();
        if data.is_empty() {
            return self;
        }
        if let Some(path) = parse_svg_path(data) {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            data.hash(&mut hasher);
            self.custom_key = Some(format!("path:{:x}", hasher.finish()));
            self.custom_path = Some(Arc::new(path));
            self.parts = Vec::new();
        }
        self
    }

    /// Clears a custom outline and returns to the type's shape.
    pub fn reset_path(mut self) -> Self {
        self.custom_path = None;
        self.custom_key = None;
        self.parts = Vec::new();
        self
    }

    /// Sets the face; the type's own is used by default.
    pub fn face(mut self, face: BotAvatarFace) -> Self {
        self.face = Some(face);
        self
    }

    /// Sets what the bot is doing.
    pub fn state(mut self, state: BotAvatarState) -> Self {
        self.state = state;
        self
    }

    /// Sets the rendered size, in logical pixels (the default is 64).
    pub fn size(mut self, size: f32) -> Self {
        self.size = clamp(size, 1.0, 4096.0);
        self
    }

    /// Sets the body colour (CSS `#rgb`, `#rrggbb`, `rgb()` or `hsl()`).
    /// Invalid values keep the type's palette colour.
    pub fn color(mut self, color: impl AsRef<str>) -> Self {
        let value = color.as_ref();
        if parse_color(value).is_some() {
            self.color = Some(value.to_owned());
        }
        self
    }

    /// Sets the face ink. Invalid values keep the automatic ink.
    pub fn ink(mut self, ink: impl AsRef<str>) -> Self {
        let value = ink.as_ref();
        if parse_color(value).is_some() {
            self.ink = Some(value.to_owned());
        }
        self
    }

    /// Sets the body colour's lightness, 0.5–1.5 being the useful range.
    pub fn brightness(mut self, brightness: f32) -> Self {
        self.brightness = Some(clamp(brightness, 0.0, 2.0));
        self
    }

    /// Sets the body colour's saturation, 0.5–2.5 being the useful range.
    pub fn saturation(mut self, saturation: f32) -> Self {
        self.saturation = Some(clamp(saturation, 0.0, 2.5));
        self
    }

    /// Sets a multiplier on every animation's speed.
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = clamp(speed, 0.0, 8.0);
        self
    }

    /// Freezes every animation on its current frame.
    pub fn paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        self
    }

    /// Offsets the blink and glance timing, 0–1.
    pub fn seed(mut self, seed: f32) -> Self {
        self.seed = Some(clamp(seed, 0.0, 1.0));
        self
    }

    /// Sets how the body is lit.
    pub fn shading(mut self, shading: BotAvatarShading) -> Self {
        self.shading = shading;
        self
    }

    /// Puts a hat on the head.
    pub fn hat(mut self, hat: BotAvatarHat) -> Self {
        self.hat = hat;
        self
    }

    /// Puts glasses over the eyes.
    pub fn glasses(mut self, glasses: BotAvatarGlasses) -> Self {
        self.glasses = glasses;
        self
    }

    /// Wears headphones.
    pub fn headphones(mut self, headphones: bool) -> Self {
        self.headphones = headphones;
        self
    }

    /// Wears a bow tie.
    pub fn bow_tie(mut self, bow_tie: bool) -> Self {
        self.bow_tie = bow_tie;
        self
    }

    /// Sets the hat's, headphones' and bow tie's colour.
    pub fn accessory_color(mut self, color: impl AsRef<str>) -> Self {
        let value = color.as_ref();
        if parse_color(value).is_some() {
            self.accessory_color = value.to_owned();
        }
        self
    }

    /// Sets the shadow side's strength, 0–2.
    pub fn shadow(mut self, shadow: f32) -> Self {
        self.shadow = Some(clamp(shadow, 0.0, 2.0));
        self
    }

    /// Sets the lit side's strength, 0–2.
    pub fn highlight(mut self, highlight: f32) -> Self {
        self.highlight = Some(clamp(highlight, 0.0, 2.0));
        self
    }

    /// Sets the body's thickness, 0.2–2.
    pub fn depth(mut self, depth: f32) -> Self {
        self.depth = clamp(depth, 0.2, 2.0);
        self
    }

    /// `plastic` and `fabric`: the profile through the depth, 0–1.
    pub fn roundness(mut self, roundness: f32) -> Self {
        self.roundness = clamp(roundness, 0.0, 1.0);
        self
    }

    /// `fabric`: the pile's length, 0.3–2.5.
    pub fn fur_length(mut self, length: f32) -> Self {
        self.fur.length = q20(length, 0.3, 2.5);
        self
    }

    /// `fabric`: how many fibres, 0.3–2.
    pub fn fur_density(mut self, density: f32) -> Self {
        self.fur.density = q20(density, 0.3, 2.0);
        self
    }

    /// `fabric`: the haze of fine hairs past the silhouette, 0–1.
    pub fn fur_fuzz(mut self, fuzz: f32) -> Self {
        self.fur.fuzz = q20(fuzz, 0.0, 1.0);
        self
    }

    /// `fabric`: how much the fibres gather into tufts, 0–1.
    pub fn fur_clumps(mut self, clumps: f32) -> Self {
        self.fur.clumps = q20(clumps, 0.0, 1.0);
        self
    }

    /// `fabric`: how wavy the fibres are, 0–1.
    pub fn fur_curl(mut self, curl: f32) -> Self {
        self.fur.curl = q20(curl, 0.0, 1.0);
        self
    }

    /// `fabric`: how much the pile hangs, 0–1.
    pub fn fur_gravity(mut self, gravity: f32) -> Self {
        self.fur.gravity = q20(gravity, 0.0, 1.0);
        self
    }

    /// Sets the whole pile style at once.
    pub fn fur(mut self, fur: BotAvatarFur) -> Self {
        self.fur = BotAvatarFur {
            length: q20(fur.length, 0.3, 2.5),
            density: q20(fur.density, 0.3, 2.0),
            fuzz: q20(fur.fuzz, 0.0, 1.0),
            clumps: q20(fur.clumps, 0.0, 1.0),
            curl: q20(fur.curl, 0.0, 1.0),
            gravity: q20(fur.gravity, 0.0, 1.0),
        };
        self
    }

    /// Where the light comes from, in degrees clockwise from the top.
    pub fn light(mut self, light: f32) -> Self {
        self.light = Some(clamp(light, -360.0, 360.0));
        self
    }

    /// Sets the rim: lit rim width, Fresnel rim or back light, 0–2.
    pub fn rim(mut self, rim: f32) -> Self {
        self.rim = Some(clamp(rim, 0.0, 2.0));
        self
    }

    /// Sets the light's spread, 0.4–2.5.
    pub fn spread(mut self, spread: f32) -> Self {
        self.spread = Some(clamp(spread, 0.4, 2.5));
        self
    }

    /// `fabric`: where the back light comes from, degrees clockwise from the top.
    pub fn back_light(mut self, back_light: f32) -> Self {
        self.back_light = Some(clamp(back_light, -360.0, 360.0));
        self
    }

    /// `fabric`: how far round to the front the key light sits, in degrees.
    pub fn light_front(mut self, light_front: f32) -> Self {
        self.light_front = Some(clamp(light_front, 0.0, 85.0));
        self
    }

    /// `fabric`: strength of the highlights along single fibres, 0–2.
    pub fn shine(mut self, shine: f32) -> Self {
        self.shine = Some(clamp(shine, 0.0, 2.0));
        self
    }

    /// `fabric`: the satin sheen over the body's edge, 0–2.
    pub fn sheen(mut self, sheen: f32) -> Self {
        self.sheen = Some(clamp(sheen, 0.0, 2.0));
        self
    }

    /// `fabric`: how soft the back light is, 0–1.
    pub fn back_softness(mut self, back_softness: f32) -> Self {
        self.back_softness = Some(clamp(back_softness, 0.0, 1.0));
        self
    }

    /// Holds the body at this orientation (radians) in the state's rest pose.
    ///
    /// The values that are `None` keep the rest pose's. A held pose overrides
    /// the animation's turns, hops and glances, and pointer play, while set.
    pub fn pose(mut self, yaw: Option<f32>, pitch: Option<f32>, roll: Option<f32>) -> Self {
        let mut pose = self.held_pose.unwrap_or_else(|| rest_pose(self.state));
        if let Some(yaw) = yaw {
            pose.yaw = yaw;
        }
        if let Some(pitch) = pitch {
            pose.pitch = pitch;
        }
        if let Some(roll) = roll {
            pose.roll = roll;
        }
        self.held_pose = Some(pose);
        self
    }

    /// Clears a held pose.
    pub fn reset_pose(mut self) -> Self {
        self.held_pose = None;
        self
    }

    /// Turns pointer play on or off.
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    /// Sets the surface the avatar sits on.
    pub fn theme(mut self, theme: BotAvatarTheme) -> Self {
        self.theme = theme;
        self
    }

    /// How far the head turns from side to side while idle, 0–2.
    pub fn turn(mut self, turn: f32) -> Self {
        self.turn = clamp(turn, 0.0, 2.0);
        self
    }

    /// The whirl round a spin: its strength, 0–2 (off at 0).
    pub fn whirl(mut self, strength: f32) -> Self {
        self.whirl.strength = clamp(strength, 0.0, 2.0);
        self
    }

    /// Size of the whirl's ring, 0.6–1.6.
    pub fn whirl_size(mut self, size: f32) -> Self {
        self.whirl.size = clamp(size, 0.6, 1.6);
        self
    }

    /// Thickness of the whirl's trail, 0.4–2.
    pub fn whirl_width(mut self, width: f32) -> Self {
        self.whirl.width = clamp(width, 0.4, 2.0);
        self
    }

    /// Length of the trail round the ring, 0.4–1.6.
    pub fn whirl_length(mut self, length: f32) -> Self {
        self.whirl.length = clamp(length, 0.4, 1.6);
        self
    }

    /// How flat the whirl's ring is seen, 0.5–1.8.
    pub fn whirl_tilt(mut self, tilt: f32) -> Self {
        self.whirl.tilt = clamp(tilt, 0.5, 1.8);
        self
    }

    /// Sets the jump's height, in body units.
    pub fn jump_height(mut self, height: f32) -> Self {
        self.jump.height = clamp(height, 0.0, 100.0);
        self
    }

    /// Sets the seconds the jump spends in the air.
    pub fn jump_time(mut self, time: f32) -> Self {
        self.jump.time = clamp(time, 0.2, 10.0);
        self
    }

    /// Sets how much the body stretches in the air, 0–2.
    pub fn jump_stretch(mut self, stretch: f32) -> Self {
        self.jump.stretch = clamp(stretch, 0.0, 2.0);
        self
    }

    /// Sets how much the body squashes on the ground, 0–2.
    pub fn jump_squash(mut self, squash: f32) -> Self {
        self.jump.squash = clamp(squash, 0.0, 2.0);
        self
    }

    /// Sets the seconds the landing squash takes, contact to recovered.
    pub fn jump_squash_time(mut self, seconds: f32) -> Self {
        self.jump.squash_time = clamp(seconds, 0.02, 10.0);
        self
    }

    /// Sets the landing squash's shape.
    pub fn jump_squash_ease(mut self, ease: BotAvatarSquashEase) -> Self {
        self.jump.squash_ease = ease;
        self
    }

    /// Sets the seconds the body holds its deepest squash on the ground.
    pub fn jump_ground_time(mut self, seconds: f32) -> Self {
        self.jump.ground_time = clamp(seconds, 0.0, 10.0);
        self
    }

    /// Sets how the weight settles through that hold.
    pub fn jump_ground_ease(mut self, ease: BotAvatarSquashEase) -> Self {
        self.jump.ground_ease = ease;
        self
    }

    /// Sets the seconds the body takes to rise back to its own shape.
    pub fn jump_rise_time(mut self, seconds: f32) -> Self {
        self.jump.rise_time = clamp(seconds, 0.02, 10.0);
        self
    }

    /// Sets how it rises.
    pub fn jump_rise_ease(mut self, ease: BotAvatarSquashEase) -> Self {
        self.jump.rise_ease = ease;
        self
    }

    /// Sets the seconds a click's landing squash takes.
    pub fn jump_click_squash_time(mut self, seconds: f32) -> Self {
        self.jump.click_squash_time = clamp(seconds, 0.02, 10.0);
        self
    }

    /// Sets when the landing squash begins: seconds before (negative) or
    /// after touch-down; 0 is the moment of contact.
    pub fn jump_land(mut self, seconds: f32) -> Self {
        self.jump.land = clamp(seconds, -2.0, 2.0);
        self
    }

    /// Sets whole turns in the air.
    pub fn jump_spin(mut self, spin: f32) -> Self {
        self.jump.spin = clamp(spin, 0.0, 8.0).round();
        self
    }

    /// Sets the lean into the jump, in degrees.
    pub fn jump_lean(mut self, lean: f32) -> Self {
        self.jump.lean = clamp(lean, -45.0, 45.0);
        self
    }

    /// Sets the seconds between idle jumps, roughly (±40 %); 0 for none.
    pub fn jump_every(mut self, every: f32) -> Self {
        self.jump.every = clamp(every, 0.0, 3600.0);
        self
    }

    /// Sets the whole jump configuration.
    pub fn jump(mut self, jump: JumpConfig) -> Self {
        self.jump = jump;
        self
    }

    /// Holds the avatar still: the state's rest pose, no loop.
    pub fn reduce_motion(mut self, reduce_motion: bool) -> Self {
        self.reduced_motion = reduce_motion;
        self
    }

    /// Sets the raster's pixel density.
    ///
    /// By default the window's scale factor is used, so the avatar stays
    /// sharp on a HiDPI display. Setting this overrides it.
    pub fn pixel_density(mut self, density: f32) -> Self {
        self.pixel_density = Some(clamp(density, 0.5, 4.0));
        self
    }

    /// Follows the window's scale factor again after [`pixel_density`](Self::pixel_density).
    pub fn follow_scale_factor(mut self) -> Self {
        self.pixel_density = None;
        self
    }

    /// Publishes `message` when the avatar is clicked.
    pub fn on_click(mut self, message: impl Fn() -> Message + 'static) -> Self {
        self.on_click = Some(Box::new(message));
        self
    }

    /// The body shape.
    pub fn kind(&self) -> BotAvatarType {
        self.bot_type
    }

    /// The face in use, resolved from the type when not set.
    pub fn face_kind(&self) -> BotAvatarFace {
        self.face
            .unwrap_or_else(|| bot_avatar_preset(self.bot_type).face)
    }

    /// The state.
    pub fn bot_state(&self) -> BotAvatarState {
        self.state
    }

    /// The rendered size in logical pixels.
    pub fn dimension(&self) -> f32 {
        self.size
    }

    /// Whether animation is frozen.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// The animation speed multiplier.
    pub fn animation_speed(&self) -> f32 {
        self.speed
    }

    /// Whether the widget is moving.
    pub fn is_animated(&self) -> bool {
        !self.paused && self.speed > 0.0 && !self.reduced_motion && self.held_pose.is_none()
    }

    /// The accessible label the web package uses for this state.
    pub fn state_label(&self) -> &'static str {
        bot_avatar_state_label(self.state)
    }

    /// Renders one frame headlessly, as straight RGBA8.
    ///
    /// The bitmap is `size * OVERSCAN * density` square. This is the same
    /// pipeline the widget draws, which makes it useful for snapshots and
    /// for comparing against the web reference.
    pub fn render_rgba(&self, size: f32, density: f32, pose: &Pose) -> (u32, u32, Vec<u8>) {
        let cfg = self.render_config(pose);
        let frame = draw_frame(size, density, pose, &cfg);
        to_straight_rgba(&frame)
    }

    /// Turns the widget into an iced [`Element`].
    pub fn into_element(self) -> Element<'static, Message>
    where
        Message: 'static,
    {
        Element::new(self)
    }

    fn seed_value(&self) -> f32 {
        if let Some(seed) = self.seed {
            return seed;
        }
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.custom_key
            .as_deref()
            .unwrap_or(self.bot_type.key())
            .hash(&mut hasher);
        (hasher.finish() % 1000) as f32 / 1000.0
    }

    fn resolved_outline(&self) -> (Arc<Path>, String) {
        if let Some(path) = &self.custom_path {
            return (
                Arc::clone(path),
                self.custom_key.clone().unwrap_or_else(|| "path".to_owned()),
            );
        }
        let key = self.bot_type.key().to_owned();
        if let Some(path) = parse_svg_path(bot_avatar_shape(self.bot_type)) {
            (Arc::new(path), key)
        } else {
            let fallback = parse_svg_path(bot_avatar_shape(BotAvatarType::Clover))
                .expect("clover outline is valid");
            (Arc::new(fallback), "clover".to_owned())
        }
    }

    fn resolved_parts(&self) -> Vec<Arc<Path>> {
        if self.custom_path.is_some() {
            return Vec::new();
        }
        if let Some(data) = bot_avatar_parts(self.bot_type) {
            return parse_svg_subpaths(data).into_iter().map(Arc::new).collect();
        }
        Vec::new()
    }

    fn body_colors(&self) -> (super::color::Rgba, super::color::Rgba) {
        let preset = bot_avatar_preset(self.bot_type);
        let picked =
            parse_color(self.color.as_deref().unwrap_or(preset.color)).unwrap_or_else(|| {
                parse_color(preset.color)
                    .unwrap_or_else(|| super::color::Rgba::from_rgb(65, 196, 255))
            });
        let brightness = self.brightness.unwrap_or(preset.brightness.unwrap_or(1.0));
        let saturation = self.saturation.unwrap_or(preset.saturation.unwrap_or(1.5));
        let vivid = clamp(saturation, 1.5, 2.5) - 1.5;
        let body =
            if (brightness - 1.0).abs() < f32::EPSILON && (saturation - 1.0).abs() < f32::EPSILON {
                picked
            } else {
                let shaded = shade(
                    picked,
                    f64::from((clamp(brightness, 0.0, 2.0) - 1.0) * 0.35),
                    f64::from((clamp(saturation, 0.0, 1.5) - 1.0) * 0.5),
                );
                richer(shaded, f64::from(vivid))
            };
        let ink = self
            .ink
            .as_deref()
            .and_then(parse_color)
            .unwrap_or_else(|| auto_ink(body));
        (body, ink)
    }

    /// Resolves [`BotAvatarTheme::Auto`] against a UI theme.
    ///
    /// The renderer itself does not read this: like the web package, the
    /// surface choice only describes where the avatar sits. Ink still falls
    /// back to `auto_ink`, which looks at the body colour alone.
    pub fn resolved_theme(&self, theme: &Theme) -> BotAvatarTheme {
        match self.theme {
            BotAvatarTheme::Auto => {
                if theme.palette().background.r > 0.5 {
                    BotAvatarTheme::Light
                } else {
                    BotAvatarTheme::Dark
                }
            }
            other => other,
        }
    }

    fn render_config(&self, _pose: &Pose) -> RenderConfig {
        let preset = bot_avatar_preset(self.bot_type);
        let (body, ink) = self.body_colors();
        let (path, type_key) = self.resolved_outline();
        let parts = self.resolved_parts();
        let lit = if self.shading == BotAvatarShading::Fabric {
            &LIGHT_FABRIC
        } else {
            &LIGHT_OTHER
        };
        let accessory = parse_color(&self.accessory_color)
            .unwrap_or_else(|| super::color::Rgba::from_rgb(39, 39, 43));
        RenderConfig {
            path,
            parts,
            type_key,
            face: self.face.unwrap_or(preset.face),
            face_x: preset.face_x,
            face_y: preset.face_y,
            face_scale: preset.face_scale,
            color: body,
            ink,
            shading: self.shading,
            shadow: self.shadow.unwrap_or(lit.shadow),
            highlight: self.highlight.unwrap_or(lit.highlight),
            depth: self.depth,
            roundness: self.roundness,
            light: self.light.unwrap_or(lit.light),
            rim: self.rim.unwrap_or(lit.rim),
            spread: self.spread.unwrap_or(lit.spread),
            vivid: clamp(
                self.saturation.unwrap_or(preset.saturation.unwrap_or(1.5)),
                1.5,
                2.5,
            ) - 1.5,
            fur: FurStyle {
                length: self.fur.length,
                density: self.fur.density,
                fuzz: self.fur.fuzz,
                clumps: self.fur.clumps,
                curl: self.fur.curl,
                gravity: self.fur.gravity,
            },
            back_light: self.back_light,
            light_front: self.light_front.unwrap_or(32.0),
            shine: self.shine.unwrap_or(0.0),
            sheen: self.sheen.unwrap_or(0.0),
            back_softness: self.back_softness.unwrap_or(1.0),
            whirl: self.whirl,
            wear: Wear {
                hat: self.hat,
                glasses: self.glasses,
                headphones: self.headphones,
                bow_tie: self.bow_tie,
                color: accessory,
                fabric: self.shading == BotAvatarShading::Fabric,
            },
        }
    }

    fn current_pose(&self, inner: &Inner) -> Pose {
        if self.reduced_motion {
            return rest_pose(self.state);
        }
        if let Some(held) = self.held_pose {
            let mut pose = rest_pose(self.state);
            pose.yaw = held.yaw;
            pose.pitch = held.pitch;
            pose.roll = held.roll;
            return pose;
        }
        *inner.sim.pose()
    }

    fn frame_key(&self, pose: &Pose, density: f32) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        let f = |v: f32, h: &mut std::collections::hash_map::DefaultHasher| v.to_bits().hash(h);
        self.bot_type.key().hash(&mut h);
        self.custom_key.hash(&mut h);
        self.state.key().hash(&mut h);
        f(self.size, &mut h);
        f(density, &mut h);
        f(self.speed, &mut h);
        self.paused.hash(&mut h);
        self.shading.key().hash(&mut h);
        self.hat.key().hash(&mut h);
        self.glasses.key().hash(&mut h);
        self.headphones.hash(&mut h);
        self.bow_tie.hash(&mut h);
        self.color.hash(&mut h);
        self.ink.hash(&mut h);
        self.accessory_color.hash(&mut h);
        f(self.brightness.unwrap_or(f32::NAN), &mut h);
        f(self.saturation.unwrap_or(f32::NAN), &mut h);
        f(self.depth, &mut h);
        f(self.roundness, &mut h);
        f(self.fur.length, &mut h);
        f(self.fur.density, &mut h);
        f(self.fur.fuzz, &mut h);
        f(self.fur.clumps, &mut h);
        f(self.fur.curl, &mut h);
        f(self.fur.gravity, &mut h);
        f(self.light.unwrap_or(f32::NAN), &mut h);
        f(self.rim.unwrap_or(f32::NAN), &mut h);
        f(self.spread.unwrap_or(f32::NAN), &mut h);
        f(self.back_light.unwrap_or(f32::NAN), &mut h);
        f(self.light_front.unwrap_or(f32::NAN), &mut h);
        f(self.shine.unwrap_or(f32::NAN), &mut h);
        f(self.sheen.unwrap_or(f32::NAN), &mut h);
        f(self.back_softness.unwrap_or(f32::NAN), &mut h);
        self.reduced_motion.hash(&mut h);
        f(self.turn, &mut h);
        f(self.whirl.strength, &mut h);
        f(self.whirl.size, &mut h);
        f(self.whirl.width, &mut h);
        f(self.whirl.length, &mut h);
        f(self.whirl.tilt, &mut h);
        f(pose.yaw, &mut h);
        f(pose.pitch, &mut h);
        f(pose.roll, &mut h);
        f(pose.x, &mut h);
        f(pose.y, &mut h);
        f(pose.sx, &mut h);
        f(pose.sy, &mut h);
        f(pose.eye_open, &mut h);
        f(pose.blink_l, &mut h);
        f(pose.blink_r, &mut h);
        f(pose.look_x, &mut h);
        f(pose.look_y, &mut h);
        f(pose.breath, &mut h);
        f(pose.laugh, &mut h);
        f(pose.whirl, &mut h);
        f(pose.whirl_angle, &mut h);
        h.finish()
    }

    fn sync_sim(&self, inner: &mut Inner) {
        let seed = self.seed_value();
        if inner.seed != seed {
            inner.seed = seed;
            inner.sim = BotAvatarSim::new(seed, self.state);
        }
        if inner.state != self.state {
            inner.sim.set_state(self.state);
            inner.state = self.state;
        }
        inner.sim.set_turn(self.turn);
        inner.sim.set_jump(self.jump);
    }
}

struct Inner {
    sim: BotAvatarSim,
    seed: f32,
    state: BotAvatarState,
    last: Option<Instant>,
    image: Option<image::Handle>,
    key: u64,
    scale: f32,
}

impl Inner {
    fn new(seed: f32, state: BotAvatarState) -> Self {
        Self {
            sim: BotAvatarSim::new(seed, state),
            seed,
            state,
            last: None,
            image: None,
            key: u64::MAX,
            scale: 1.0,
        }
    }
}

struct AvatarState {
    inner: RefCell<Inner>,
}

fn to_straight_rgba(pixmap: &Pixmap) -> (u32, u32, Vec<u8>) {
    let data = pixmap.data();
    let mut out = vec![0u8; data.len()];
    for (src, dst) in data.chunks_exact(4).zip(out.chunks_exact_mut(4)) {
        let a = src[3];
        if a == 0 {
            continue;
        }
        if a == 255 {
            dst.copy_from_slice(src);
        } else {
            let a16 = u16::from(a);
            dst[0] = ((u16::from(src[0]) * 255 + a16 / 2) / a16).min(255) as u8;
            dst[1] = ((u16::from(src[1]) * 255 + a16 / 2) / a16).min(255) as u8;
            dst[2] = ((u16::from(src[2]) * 255 + a16 / 2) / a16).min(255) as u8;
            dst[3] = a;
        }
    }
    (pixmap.width(), pixmap.height(), out)
}

impl<Message> Widget<Message, Theme, Renderer> for BotAvatar<Message> {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<AvatarState>()
    }

    fn state(&self) -> widget::tree::State {
        let seed = self.seed_value();
        widget::tree::State::new(AvatarState {
            inner: RefCell::new(Inner::new(seed, self.state)),
        })
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(self.size), Length::Fixed(self.size))
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.resolve(
            Length::Fixed(self.size),
            Length::Fixed(self.size),
            Size::new(self.size, self.size),
        ))
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if !bounds.intersects(viewport) {
            return;
        }
        let state = tree.state.downcast_ref::<AvatarState>();
        let mut inner = state.inner.borrow_mut();
        self.sync_sim(&mut inner);

        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
            && cursor.is_over(bounds)
            && self.interactive
        {
            if self.is_animated() {
                inner.sim.poke();
                shell.request_redraw();
            }
            if let Some(message) = &self.on_click {
                shell.publish(message());
            }
        }

        if let Event::Window(window::Event::Rescaled(scale)) = event {
            let scale = clamp(*scale, 0.5, 4.0);
            if (inner.scale - scale).abs() > f32::EPSILON {
                inner.scale = scale;
                shell.request_redraw();
            }
        }

        let Event::Window(window::Event::RedrawRequested(now)) = event else {
            return;
        };
        if self.is_animated() {
            let dt = inner.last.map_or(0.0, |last| {
                now.saturating_duration_since(last).as_secs_f64()
            });
            inner.last = Some(*now);
            if self.interactive {
                let pointer = cursor.position();
                if let Some(p) = pointer {
                    let bx = bounds.width.max(1.0);
                    let by = bounds.height.max(1.0);
                    let dx = (p.x - (bounds.x + bx * 0.5)) / bx;
                    let dy = (p.y - (bounds.y + by * 0.5)) / by;
                    let d = (dx * dx + dy * dy).sqrt();
                    let strength = if d < 1.0 {
                        1.0
                    } else if d > 3.0 {
                        0.0
                    } else {
                        1.0 - (d - 1.0) / 2.0
                    };
                    let n = d.max(1.0);
                    inner.sim.set_pointer(dx / n, dy / n, strength);
                } else {
                    inner.sim.set_pointer(0.0, 0.0, 0.0);
                }
            } else {
                inner.sim.set_pointer(0.0, 0.0, 0.0);
            }
            inner.sim.update((dt * f64::from(self.speed)) as f32);
            shell.request_redraw();
        } else {
            inner.last = None;
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &iced::advanced::renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if bounds.width < 1.0 || bounds.height < 1.0 || !bounds.intersects(viewport) {
            return;
        }
        let state = tree.state.downcast_ref::<AvatarState>();
        let mut inner = state.inner.borrow_mut();
        self.sync_sim(&mut inner);
        let pose = self.current_pose(&inner);
        let density = self.pixel_density.unwrap_or(inner.scale);
        let key = self.frame_key(&pose, density);
        if inner.key != key || inner.image.is_none() {
            let cfg = self.render_config(&pose);
            let frame = draw_frame(self.size, density, &pose, &cfg);
            let (w, h, rgba) = to_straight_rgba(&frame);
            inner.image = Some(image::Handle::from_rgba(w, h, rgba));
            inner.key = key;
        }
        let Some(handle) = inner.image.clone() else {
            return;
        };
        let side = (OVERSCAN - 1.0) * 0.5;
        let destination = Rectangle {
            x: bounds.x - side * bounds.width,
            y: bounds.y - (side + RISE) * bounds.height,
            width: bounds.width * OVERSCAN,
            height: bounds.height * OVERSCAN,
        };
        renderer.draw_image(
            Image::new(handle)
                .filter_method(if (density - 1.0).abs() < f32::EPSILON {
                    FilterMethod::Nearest
                } else {
                    FilterMethod::Linear
                })
                .snap(true),
            destination,
            destination,
        );
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if self.interactive && cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
    }
}

impl<Message> From<BotAvatar<Message>> for Element<'static, Message>
where
    Message: 'static,
{
    fn from(avatar: BotAvatar<Message>) -> Self {
        Element::new(avatar)
    }
}

/// The avatar inside an iced layout, sized by its own `size`.
pub fn bot_avatar<Message>() -> BotAvatar<Message> {
    BotAvatar::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_web_package() {
        let avatar = BotAvatar::<()>::new();
        assert_eq!(avatar.dimension(), 64.0);
        assert_eq!(avatar.kind(), BotAvatarType::Clover);
        assert_eq!(avatar.face_kind(), BotAvatarFace::Eyes);
        assert_eq!(avatar.state, BotAvatarState::Idle);
        assert_eq!(avatar.shading, BotAvatarShading::Fabric);
        assert!(avatar.is_animated());
    }

    #[test]
    fn a_custom_path_is_parsed_and_can_be_reset() {
        let avatar = BotAvatar::<()>::new().path("M10 10H90V90H10Z").reset_path();
        assert!(avatar.custom_path.is_none());
        let avatar = avatar.path("M10 10H90V90H10Z");
        assert!(avatar.custom_path.is_some());
        assert!(avatar.custom_key.as_deref().unwrap().starts_with("path:"));
    }

    #[test]
    fn a_held_pose_overrides_the_simulation() {
        let avatar = BotAvatar::<()>::new().pose(Some(0.6), None, None);
        let inner = Inner::new(0.3, BotAvatarState::Idle);
        let pose = avatar.current_pose(&inner);
        assert!((pose.yaw - 0.6).abs() < 1e-6);
    }

    #[test]
    fn headless_rendering_produces_rgba() {
        let avatar = BotAvatar::<()>::new()
            .size(32.0)
            .shading(BotAvatarShading::Crisp);
        let pose = rest_pose(BotAvatarState::Idle);
        let (w, h, rgba) = avatar.render_rgba(32.0, 1.0, &pose);
        assert_eq!((w, h), (48, 48));
        assert_eq!(rgba.len(), 48 * 48 * 4);
        assert!(rgba.chunks_exact(4).any(|p| p[3] > 0));
    }
}
