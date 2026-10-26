//! The cartoon motion ring round a spinning body.
//!
//! Ported from `drawWhirl` in `draw.ts`: one tapered trail on a tilted ring,
//! made of the body's own material. It is a translucent plastic tube lit by
//! the same key light — a lighter flank toward it, a darker underside, a white
//! specular ridge — and a soft halo so it reads as a puff of plastic cloud.
//! The near half is larger, thicker and stronger and is drawn over the body
//! and face; the far half is smaller and fainter and goes behind.

#![allow(clippy::too_many_arguments)] // signature follows the reference whirl pass

use tiny_skia::{PathBuilder, Pixmap};

use super::super::canvas::{Affine, paint_alpha, stroke_butt};
use super::super::color::{Rgba, shade};
use super::super::engine::Pose;

/// The ring's stock radius in body units.
const RADIUS: f32 = 57.0;
/// How much of a circle the trail covers at stock length.
const SPAN: f32 = std::f32::consts::PI * 1.55;
/// The ring's vertical squash at stock tilt.
const RATIO: f32 = 0.4;
/// The ring's tilt, radians.
const TILT: f32 = -0.28;
/// How many strokes make the trail.
const SEGMENTS: usize = 34;

/// The whirl's user-facing knobs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WhirlConfig {
    pub strength: f32,
    pub size: f32,
    pub width: f32,
    pub length: f32,
    pub tilt: f32,
}

impl Default for WhirlConfig {
    fn default() -> Self {
        Self {
            strength: 0.0,
            size: 1.0,
            width: 1.0,
            length: 1.0,
            tilt: 1.0,
        }
    }
}

#[derive(Clone, Copy)]
struct Ink {
    base: Rgba,
    light: Rgba,
    dark: Rgba,
    halo: Rgba,
}

fn ink(color: Rgba) -> Ink {
    Ink {
        base: shade(color, 0.1, 0.02),
        light: shade(color, 0.3, 0.04),
        dark: shade(color, -0.22, 0.08),
        halo: shade(color, 0.2, 0.0),
    }
}

/// Draw one half of the whirl in body space.
pub(crate) fn draw(
    pixmap: &mut Pixmap,
    pose: &Pose,
    color: Rgba,
    lx: f32,
    ly: f32,
    near: bool,
    knobs: WhirlConfig,
    body: Affine,
) {
    let k = (pose.whirl * knobs.strength).min(1.0);
    if k <= 0.01 {
        return;
    }
    let span = SPAN * knobs.length;
    let ink = ink(color);
    let head = -pose.whirl_angle;
    let rx = RADIUS * knobs.size;
    let ry = rx * RATIO * knobs.tilt * if near { 1.14 } else { 0.86 };
    let light_a = ly.atan2(lx) - TILT;
    let to_device = Affine::rotate(TILT)
        .then(Affine::translate(0.0, 5.0))
        .then(body);

    // The near half casts a soft shadow on the body it crosses.
    if near {
        for i in 0..SEGMENTS {
            let f = i as f32 / SEGMENTS as f32;
            let a1 = head + f * span;
            let a0 = a1 + span / SEGMENTS as f32 + 0.012;
            if ((a0 + a1) / 2.0).sin() <= 0.0 {
                continue;
            }
            let fade = (1.0 - f).powf(1.3);
            if let Some(path) = arc_path(rx, ry, a1, a0, 3.5) {
                let p = paint_alpha(Rgba::new(0, 0, 0, 255), (0.2 * k * fade).clamp(0.0, 1.0));
                stroke_butt(
                    pixmap,
                    &path,
                    &p,
                    (2.0 + 8.0 * fade) * 1.5 * knobs.width,
                    to_device,
                    None,
                );
            }
        }
    }

    for i in 0..SEGMENTS {
        let f = i as f32 / SEGMENTS as f32;
        let a1 = head + f * span;
        let a0 = a1 + span / SEGMENTS as f32 + 0.012;
        let mid = (a0 + a1) / 2.0;
        if (mid.sin() > 0.0) != near {
            continue;
        }
        let depth = 0.6 + 0.4 * mid.sin();
        let fade = (1.0 - f).powf(1.3);
        let puff = 1.0 + 0.18 * (f * 9.0 + 1.2).sin();
        let width = (2.0 + 8.0 * fade) * depth * knobs.width * puff;
        let a = k * (0.3 + 0.7 * fade) * depth;
        let facing = 0.5 + 0.5 * (mid - light_a).cos();
        let pass = |pixmap: &mut Pixmap, color: Rgba, alpha: f32, width: f32, dy: f32| {
            let Some(p) = arc_path(rx, ry, a1, a0, dy) else {
                return;
            };
            if alpha > 0.002 {
                stroke_butt(
                    pixmap,
                    &p,
                    &paint_alpha(color, alpha.clamp(0.0, 1.0)),
                    width,
                    to_device,
                    None,
                );
            }
        };
        pass(pixmap, ink.halo, a * 0.2, width * 2.6, 0.0);
        pass(pixmap, ink.dark, a * 0.45, width * 0.8, width * 0.32);
        pass(pixmap, ink.base, a * 0.72, width, 0.0);
        pass(
            pixmap,
            ink.light,
            a * 0.78 * (0.4 + 0.6 * facing),
            width * 0.62,
            -width * 0.16,
        );
        let white = a * 0.9 * (0.15 + 0.85 * facing * facing);
        if white > 0.002 {
            pass(
                pixmap,
                Rgba::new(255, 255, 255, 255),
                white,
                width * 0.24,
                -width * 0.3,
            );
        }
    }
}

/// A polyline along an ellipse arc, with the whole ring offset by `dy` so the
/// underside and specular passes sit below or above the centre line.
fn arc_path(rx: f32, ry: f32, from: f32, to: f32, dy: f32) -> Option<tiny_skia::Path> {
    let steps = 8;
    let mut b = PathBuilder::new();
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let a = from + (to - from) * t;
        let x = rx * a.cos();
        let y = ry * a.sin() + dy;
        if i == 0 {
            b.move_to(x, y);
        } else {
            b.line_to(x, y);
        }
    }
    b.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::bot_avatar::engine::BotAvatarSim;
    use crate::widgets::bot_avatar::types::BotAvatarState;

    #[test]
    fn arc_path_spans_the_requested_angle() {
        let p = arc_path(10.0, 4.0, 0.0, std::f32::consts::PI, 0.0).unwrap();
        let b = p.bounds();
        assert!((b.left() + 10.0).abs() < 0.1);
        assert!((b.right() - 10.0).abs() < 0.1);
        assert!(b.top().abs() < 0.1);
        assert!((b.bottom() - 4.0).abs() < 0.1);
    }

    #[test]
    fn a_resting_pose_draws_nothing() {
        let mut sim = BotAvatarSim::new(0.3, BotAvatarState::Idle);
        sim.update(1.0 / 60.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();
        draw(
            &mut pixmap,
            sim.pose(),
            Rgba::from_rgb(65, 196, 255),
            0.0,
            0.0,
            true,
            WhirlConfig::default(),
            Affine::IDENTITY,
        );
        assert!(pixmap.pixels().iter().all(|p| p.alpha() == 0));
    }
}
