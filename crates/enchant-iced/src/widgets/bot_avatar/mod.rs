//! Animated bot avatars, ported from the Libraries.dev `bot-avatars` package.
//!
//! A [`BotAvatar`] is a small, living toy: eighteen body shapes, two faces,
//! three states (idle, working, sleeping), and five ways of lighting the
//! body — from clean vector shading to a baked plush fur. It glances about,
//! breathes, blinks, hops, flips and sleeps; a pointer near it draws its eye,
//! and a click makes it jump and spin. Hats, glasses, headphones and a bow tie
//! turn with the head.
//!
//! ```
//! use enchant_iced::widgets::bot_avatar::{BotAvatar, BotAvatarType};
//! use iced::Element;
//!
//! let avatar: Element<'_, ()> = BotAvatar::new()
//!     .bot_type(BotAvatarType::Cat)
//!     .size(96.0)
//!     .into_element();
//! # let _ = avatar;
//! ```
//!
//! # Rendering
//!
//! Frames are rasterised with `tiny-skia` into an RGBA image and drawn through
//! iced's image renderer. The body is a stack of copies of its outline through
//! a pillow profile, turned by the pose's yaw and pitch; the face lives on a
//! sphere behind the front cap; the plastic and fabric materials bake a relief
//! of the outline once and then shade it per frame.
//!
//! # Surface
//!
//! The component keeps a [`BotAvatarSim`] alive between frames. Animation is
//! driven by redraw requests, so it costs nothing while the widget is hidden.
//! Set [`BotAvatar::paused`] or [`BotAvatar::speed`] to freeze or slow it, and
//! [`BotAvatar::reduce_motion`] (or the application's own preference) to hold
//! it still while a platform asks for less motion.

mod canvas;
mod color;
mod data;
mod engine;
mod path;
mod render;
mod types;
mod widget;

pub use color::Rgba;
pub use data::{
    BOT_AVATAR_FACES, BOT_AVATAR_STATES, BOT_AVATAR_TYPES, BotAvatarPreset, bot_avatar_palette,
    bot_avatar_parts, bot_avatar_preset, bot_avatar_shape, bot_avatar_state_label,
};
pub use engine::{BotAvatarSim, JUMP_DEFAULTS, JumpConfig, Pose, rest_pose};
pub use types::{
    BotAvatarFace, BotAvatarFur, BotAvatarGlasses, BotAvatarHat, BotAvatarShading,
    BotAvatarSquashEase, BotAvatarState, BotAvatarTheme, BotAvatarType,
};
pub use widget::{BotAvatar, bot_avatar};
