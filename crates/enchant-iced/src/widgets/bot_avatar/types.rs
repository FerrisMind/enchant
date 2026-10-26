//! The public vocabularies of the bot avatar.
//!
//! These are the names the web package exposes in `types.ts`: the eighteen
//! body shapes, the two faces, the three states, the five shading modes, the
//! four hats, the four pairs of glasses, and the four landing curves. The
//! enums are `#[non_exhaustive]` so the vocabulary can grow without breaking
//! callers; the [`ALL`](BotAvatarType::ALL) tables are there to iterate the
//! current set.

/// The eighteen body shapes.
///
/// Each one carries its own colour, face placement and default brightness and
/// saturation; see [`bot_avatar_preset`](super::bot_avatar_preset).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BotAvatarType {
    /// A four-lobed clover.
    #[default]
    Clover,
    /// A rounded six-petal flower.
    Flower,
    /// A rounded triangle.
    Triangle,
    /// A rounded square.
    Square,
    /// An asymmetric organic blob.
    Blob,
    /// A ghost with a wavy hem.
    Ghost,
    /// A plain circle.
    Circle,
    /// A drop, wide at the bottom.
    Drop,
    /// A rounded five-point star.
    Star,
    /// A droid with antennae and side ears.
    Droid,
    /// A boxy mech with antennae and side vents.
    Mech,
    /// An almond-headed alien.
    Alien,
    /// A rounded hexagon.
    Hexagon,
    /// A cat's head with two ears.
    Cat,
    /// A cloud of overlapping circles.
    Cloud,
    /// A wide pill.
    Pill,
    /// A smooth pebble.
    Pebble,
    /// A low, wide puddle.
    Puddle,
}

impl BotAvatarType {
    /// The lowercase identifier the web package uses (`"clover"`, …).
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Clover => "clover",
            Self::Flower => "flower",
            Self::Triangle => "triangle",
            Self::Square => "square",
            Self::Blob => "blob",
            Self::Ghost => "ghost",
            Self::Circle => "circle",
            Self::Drop => "drop",
            Self::Star => "star",
            Self::Droid => "droid",
            Self::Mech => "mech",
            Self::Alien => "alien",
            Self::Hexagon => "hexagon",
            Self::Cat => "cat",
            Self::Cloud => "cloud",
            Self::Pill => "pill",
            Self::Pebble => "pebble",
            Self::Puddle => "puddle",
        }
    }

    /// Parse the web package's lowercase identifier.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.key() == key)
    }
}

/// What the face is made of.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BotAvatarFace {
    /// Two eyes only.
    #[default]
    Eyes,
    /// Two eyes and a mouth that changes with the state.
    Mouth,
}

/// What the bot is doing. Each state is a pose plus its own motion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BotAvatarState {
    /// Idle: glancing about, breathing, blinking, an occasional flip.
    #[default]
    Idle,
    /// Working: hopping in place, every third hop spinning, an occasional
    /// laugh.
    Working,
    /// Sleeping: slow breathing, closed eyes, a nodding head.
    Sleeping,
}

impl BotAvatarState {
    /// Every state, in the web package's order.
    pub const ALL: [Self; 3] = [Self::Idle, Self::Working, Self::Sleeping];

    /// The lowercase identifier the web package uses.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Idle => "default",
            Self::Working => "working",
            Self::Sleeping => "sleeping",
        }
    }

    /// Parse the web package's lowercase identifier.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "default" | "idle" => Some(Self::Idle),
            "working" => Some(Self::Working),
            "sleeping" => Some(Self::Sleeping),
            _ => None,
        }
    }
}

/// How the landing squash of a jump plays out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BotAvatarSquashEase {
    /// All at once, then eases off.
    Sharp,
    /// A quick press that recovers without a wobble.
    #[default]
    Pulse,
    /// Eases in and out.
    Soft,
    /// Overshoots into a stretch and settles.
    Bouncy,
}

impl BotAvatarSquashEase {
    /// Every landing curve, in the web package's order.
    pub const ALL: [Self; 4] = [Self::Sharp, Self::Pulse, Self::Soft, Self::Bouncy];

    /// The lowercase identifier the web package uses.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Sharp => "sharp",
            Self::Pulse => "pulse",
            Self::Soft => "soft",
            Self::Bouncy => "bouncy",
        }
    }
}

/// How the body is lit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BotAvatarShading {
    /// A lit rim with a clean edge round the front, vector-style.
    Crisp,
    /// No edge, a soft shadow and highlight across the whole form.
    Smooth,
    /// The same inflated form as a real glossy material, shaded per pixel.
    Plastic,
    /// The depth alone, no lighting.
    Flat,
    /// A plush faux fur over the inflated form, baked per outline.
    #[default]
    Fabric,
}

impl BotAvatarShading {
    /// Every shading mode, in the web package's order.
    pub const ALL: [Self; 5] = [
        Self::Crisp,
        Self::Smooth,
        Self::Plastic,
        Self::Flat,
        Self::Fabric,
    ];

    /// The lowercase identifier the web package uses.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Crisp => "crisp",
            Self::Smooth => "smooth",
            Self::Plastic => "plastic",
            Self::Flat => "flat",
            Self::Fabric => "fabric",
        }
    }

    /// Whether the mode is a baked material (and needs a little build time
    /// before its first frame).
    #[must_use]
    pub const fn is_material(self) -> bool {
        matches!(self, Self::Plastic | Self::Fabric)
    }
}

/// Something to wear on the head.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BotAvatarHat {
    /// Nothing.
    #[default]
    None,
    /// A soft felt beret, tipped to one side.
    Beret,
    /// A knitted beanie with a ribbed cuff and a pom.
    Beanie,
    /// A paper party cone.
    Party,
    /// A gold crown with cut stones.
    Crown,
}

impl BotAvatarHat {
    /// Every hat, in the web package's order.
    pub const ALL: [Self; 5] = [Self::None, Self::Beret, Self::Beanie, Self::Party, Self::Crown];

    /// The lowercase identifier the web package uses.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Beret => "beret",
            Self::Beanie => "beanie",
            Self::Party => "party",
            Self::Crown => "crown",
        }
    }
}

/// Something to wear over the eyes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BotAvatarGlasses {
    /// Nothing.
    #[default]
    None,
    /// Round wire frames.
    Round,
    /// Square acetate frames.
    Square,
    /// Dark sunglasses.
    Shades,
}

impl BotAvatarGlasses {
    /// Every pair of glasses, in the web package's order.
    pub const ALL: [Self; 4] = [Self::None, Self::Round, Self::Square, Self::Shades];

    /// The lowercase identifier the web package uses.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Round => "round",
            Self::Square => "square",
            Self::Shades => "shades",
        }
    }
}

/// The surface the avatar sits on.
///
/// The whirl is white on a dark surface and black on a light one, so it stays
/// visible either way. [`Auto`](Self::Auto) follows the application theme.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BotAvatarTheme {
    /// Follow the surrounding application theme.
    #[default]
    Auto,
    /// A dark surface.
    Dark,
    /// A light surface.
    Light,
}

impl BotAvatarTheme {
    /// The lowercase identifier the web package uses.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }
}

/// The pile's style for the fabric material.
///
/// [`BotAvatar::fur`](crate::widgets::bot_avatar::BotAvatar::fur) takes a whole
/// style; the per-knob builders on the widget (`fur_length`, `fur_density`, …)
/// change one value at a time. Every value is clamped to its documented range
/// by the widget, so out-of-range values are harmless.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BotAvatarFur {
    /// Fibre length as a multiple of the default pile.
    pub length: f32,
    /// How many fibres per unit of area.
    pub density: f32,
    /// The haze of fine hairs past the silhouette.
    pub fuzz: f32,
    /// How much the fibres gather into tufts.
    pub clumps: f32,
    /// How wavy the fibres are.
    pub curl: f32,
    /// How much the pile hangs under its own weight.
    pub gravity: f32,
}

impl BotAvatarFur {
    /// The web package's defaults.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            length: 1.0,
            density: 1.6,
            fuzz: 0.9,
            clumps: 0.4,
            curl: 0.7,
            gravity: 0.9,
        }
    }

    /// Sets the fibre length.
    #[must_use]
    pub const fn with_length(mut self, length: f32) -> Self {
        self.length = length;
        self
    }

    /// Sets the fibre density.
    #[must_use]
    pub const fn with_density(mut self, density: f32) -> Self {
        self.density = density;
        self
    }

    /// Sets the silhouette's fuzz.
    #[must_use]
    pub const fn with_fuzz(mut self, fuzz: f32) -> Self {
        self.fuzz = fuzz;
        self
    }

    /// Sets how strongly the fibres clump.
    #[must_use]
    pub const fn with_clumps(mut self, clumps: f32) -> Self {
        self.clumps = clumps;
        self
    }

    /// Sets how wavy the fibres are.
    #[must_use]
    pub const fn with_curl(mut self, curl: f32) -> Self {
        self.curl = curl;
        self
    }

    /// Sets how much the pile hangs.
    #[must_use]
    pub const fn with_gravity(mut self, gravity: f32) -> Self {
        self.gravity = gravity;
        self
    }
}

impl Default for BotAvatarFur {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_table_is_complete_and_unique() {
        assert_eq!(BotAvatarType::ALL.len(), 18);
        assert_eq!(BotAvatarState::ALL.len(), 3);
        assert_eq!(BotAvatarShading::ALL.len(), 5);
        assert_eq!(BotAvatarHat::ALL.len(), 5);
        assert_eq!(BotAvatarGlasses::ALL.len(), 4);
        for (i, t) in BotAvatarType::ALL.into_iter().enumerate() {
            assert_eq!(BotAvatarType::from_key(t.key()), Some(t));
            assert!(!BotAvatarType::ALL[..i].contains(&t));
        }
        assert_eq!(BotAvatarState::from_key("default"), Some(BotAvatarState::Idle));
        assert_eq!(BotAvatarState::from_key("idle"), Some(BotAvatarState::Idle));
        assert_eq!(BotAvatarState::from_key("sleeping"), Some(BotAvatarState::Sleeping));
        assert_eq!(BotAvatarState::from_key("nope"), None);
    }

    #[test]
    fn defaults_match_the_web_package() {
        assert_eq!(BotAvatarType::default(), BotAvatarType::Clover);
        assert_eq!(BotAvatarFace::default(), BotAvatarFace::Eyes);
        assert_eq!(BotAvatarState::default(), BotAvatarState::Idle);
        assert_eq!(BotAvatarShading::default(), BotAvatarShading::Fabric);
        assert_eq!(BotAvatarSquashEase::default(), BotAvatarSquashEase::Pulse);
        assert_eq!(BotAvatarHat::default(), BotAvatarHat::None);
        assert_eq!(BotAvatarGlasses::default(), BotAvatarGlasses::None);
        assert_eq!(BotAvatarTheme::default(), BotAvatarTheme::Auto);
    }
}
