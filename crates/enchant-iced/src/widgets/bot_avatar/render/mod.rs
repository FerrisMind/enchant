//! The rasteriser.
//!
//! A frame is drawn in the avatar's own 100×100 design space, on a canvas
//! `OVERSCAN` times the layout box so a hop or a flip has room to leave it.
//! The body's centre sits [`RISE`] of the box below the canvas centre: hops
//! and flips need the room above, not below.

pub(crate) mod draw;
pub(crate) mod face;
pub(crate) mod material;
pub(crate) mod mesh;
pub(crate) mod palette;
pub(crate) mod wear;
pub(crate) mod whirl;

pub(crate) use draw::{OVERSCAN, RISE, draw_frame};

use super::color::Rgba;

/// The pile's style for the fabric material. Every value is clamped by the
/// widget before it reaches the renderer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FurStyle {
    pub length: f32,
    pub density: f32,
    pub fuzz: f32,
    pub clumps: f32,
    pub curl: f32,
    pub gravity: f32,
}

impl Default for FurStyle {
    fn default() -> Self {
        Self {
            length: 1.0,
            density: 1.6,
            fuzz: 0.9,
            clumps: 0.4,
            curl: 0.7,
            gravity: 0.9,
        }
    }
}

/// Everything the renderer needs to paint one frame.
#[derive(Clone, Debug)]
pub(crate) struct RenderConfig {
    pub path: std::sync::Arc<tiny_skia::Path>,
    pub parts: Vec<std::sync::Arc<tiny_skia::Path>>,
    pub type_key: String,
    pub face: super::types::BotAvatarFace,
    pub face_x: f32,
    pub face_y: f32,
    pub face_scale: f32,
    pub color: Rgba,
    pub ink: Rgba,
    pub shading: super::types::BotAvatarShading,
    pub shadow: f32,
    pub highlight: f32,
    pub depth: f32,
    pub roundness: f32,
    pub light: f32,
    pub rim: f32,
    pub spread: f32,
    pub vivid: f32,
    pub fur: FurStyle,
    pub back_light: Option<f32>,
    pub light_front: f32,
    pub shine: f32,
    pub sheen: f32,
    pub back_softness: f32,
    pub whirl: whirl::WhirlConfig,
    /// What the bot wears.
    pub wear: wear::Wear,
}
