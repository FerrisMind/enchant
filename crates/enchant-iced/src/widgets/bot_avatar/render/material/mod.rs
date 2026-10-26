//! Baked materials: the glossy plastic and the plush fabric.
//!
//! The form is baked once per outline, depth and texture size; its matcap is
//! evaluated for the current light and turn every frame, then the cap's
//! texels are shaded from it. Fabric uses the same form under a pile, which is
//! built in [`fabric`].

pub(crate) mod fabric;
pub(crate) mod form;
pub(crate) mod matcap;
pub(crate) mod plastic;

pub(crate) use form::surface_at;

use tiny_skia::Pixmap;

use super::super::canvas::Affine;
use super::super::engine::Pose;
use super::RenderConfig;
use super::palette::Palette;

/// Draw the material body. Returns whether the material was ready.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_cap(
    pixmap: &mut Pixmap,
    cfg: &RenderConfig,
    _pose: &Pose,
    half_depth: f32,
    plush: bool,
    body: Affine,
    pal: &Palette,
    facing: f32,
    cy: f32,
    sy: f32,
    cp: f32,
    sp: f32,
    lx: f32,
    ly: f32,
    dev: f32,
) -> bool {
    if cfg.shading == super::super::types::BotAvatarShading::Fabric && plush {
        // Fabric lays a real pile over the same form; see fabric.rs.
        fabric::draw(
            pixmap, cfg, body, pal, facing, cy, sy, cp, sp, lx, ly, half_depth, dev,
        )
    } else {
        plastic::draw(
            pixmap, cfg, plush, body, pal, facing, cy, sy, cp, sp, lx, ly, half_depth, dev,
        )
    }
}
