//! The vector body: a slice stack, a soft overlay, the face and the whirl.
//!
//! A faithful port of `draw` in `draw.ts`. The body is a stack of copies of
//! its outline, spaced along a depth axis with a pillow profile (smaller at
//! the caps); each copy is projected with the head's yaw and pitch, so the
//! stack reads as a rounded extruded solid that turns, flips and shows its
//! side. The nearest copies are lit with a directional gradient, the far side
//! sits in shade, and the face lives on the front cap and follows it.

use std::sync::Arc;

use tiny_skia::{
    BlendMode, Color, GradientStop, LinearGradient, Mask, Path, PathBuilder, Pixmap, RadialGradient,
    Shader, SpreadMode, Transform,
};

use super::super::canvas::{
    Affine, fill, mask, mask_fill, paint,
};
use super::super::color::Rgba;
use super::super::engine::Pose;
use super::super::types::BotAvatarShading;
use super::face::{FACE_R, draw_face};
use super::palette::{SLICES, Palette, palette};
use super::wear;
use super::whirl::{self, WhirlConfig};
use super::RenderConfig;

/// The canvas is drawn larger than the avatar's layout box, so a hop or a
/// flip can leave the box without being clipped.
pub(crate) const OVERSCAN: f32 = 1.5;

/// The body's centre sits this fraction of the box below the canvas centre:
/// hops and flips need the room above, not below.
pub(crate) const RISE: f32 = 0.1;

/// The stock half-depth of the body in its 100-unit space.
const HALF_DEPTH: f32 = 15.0;

/// The cap scale at the ends of the pillow, at the stock rim width.
const CAP: f32 = 0.9;

/// The stock light for each material, where a prop leaves it out.
pub(crate) struct LightDefaults {
    pub shadow: f32,
    pub highlight: f32,
    pub light: f32,
    pub rim: f32,
    pub spread: f32,
}

pub(crate) const LIGHT_FABRIC: LightDefaults = LightDefaults {
    shadow: 1.15,
    highlight: 1.45,
    light: 295.0,
    rim: 0.6,
    spread: 1.6,
};

pub(crate) const LIGHT_OTHER: LightDefaults = LightDefaults {
    shadow: 0.35,
    highlight: 1.3,
    light: 300.0,
    rim: 0.5,
    spread: 1.55,
};

fn profile(z: f32, cap: f32) -> f32 {
    cap + (1.0 - cap) * (1.0 - z * z).max(0.0).sqrt()
}

fn floor_foreshorten(v: f32) -> f32 {
    if v.abs() < 0.22 {
        if v < 0.0 { -0.22 } else { 0.22 }
    } else {
        v
    }
}

fn color(c: Rgba) -> Color {
    Color::from_rgba8(c.r, c.g, c.b, c.a)
}

fn linear(
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    stops: &[(f32, Rgba)],
) -> Option<Shader<'static>> {
    LinearGradient::new(
        tiny_skia::Point::from_xy(x0, y0),
        tiny_skia::Point::from_xy(x1, y1),
        stops
            .iter()
            .map(|&(p, c)| GradientStop::new(p, color(c)))
            .collect(),
        SpreadMode::Pad,
        Transform::identity(),
    )
}

fn radial(
    cx: f32,
    cy: f32,
    r0: f32,
    r1: f32,
    stops: &[(f32, Rgba)],
) -> Option<Shader<'static>> {
    let mut adjusted = Vec::with_capacity(stops.len() + 1);
    let base = (r0 / r1).clamp(0.0, 1.0);
    if base > 0.0 {
        adjusted.push(GradientStop::new(0.0, color(stops[0].1)));
        adjusted.push(GradientStop::new(base, color(stops[0].1)));
    }
    for &(p, c) in stops {
        adjusted.push(GradientStop::new((base + p * (1.0 - base)).clamp(0.0, 1.0), color(c)));
    }
    RadialGradient::new(
        tiny_skia::Point::from_xy(cx, cy),
        tiny_skia::Point::from_xy(cx, cy),
        r1,
        adjusted,
        SpreadMode::Pad,
        Transform::identity(),
    )
}

fn rect_path(x: f32, y: f32, w: f32, h: f32) -> Option<Path> {
    let mut b = PathBuilder::new();
    b.push_rect(tiny_skia::Rect::from_xywh(x, y, w, h)?);
    b.finish()
}

struct Painter<'a> {
    pixmap: &'a mut Pixmap,
    cfg: &'a RenderConfig,
    pose: &'a Pose,
    body: Affine,
    pal: Palette,
    lx: f32,
    ly: f32,
    half_depth: f32,
    cap: f32,
    rim_k: f32,
    shadow: f32,
    highlight: f32,
    spread: f32,
    facing: f32,
    cy: f32,
    sy: f32,
    cp: f32,
    sp: f32,
    mode: BotAvatarShading,
    dev_scale: f32,
}

impl Painter<'_> {
    fn draw_solid(&mut self, path: &Path, half_depth: f32, plush: bool) -> bool {
        // The lit gradient, in the body's own space: light from the upper left.
        let lit_gradient = if self.mode == BotAvatarShading::Crisp {
            linear(
                self.lx * 56.0,
                self.ly * 56.0,
                -self.lx * 56.0,
                -self.ly * 56.0,
                &[
                    (0.0, self.pal.light),
                    (0.45, self.pal.near),
                    (1.0, self.pal.dark),
                ],
            )
        } else {
            None
        };
        let cap_gradient = if self.mode == BotAvatarShading::Crisp {
            linear(
                self.lx * 46.0,
                self.ly * 46.0,
                -self.lx * 46.0,
                -self.ly * 46.0,
                &[(0.0, self.pal.cap_top), (1.0, self.pal.cap_bottom)],
            )
        } else {
            None
        };

        // Plastic and fabric draw the whole body themselves. Until that lands,
        // the smooth overlay is the stand-in the web package uses while a bake
        // is still running.
        let mut material_done = false;
        if self.mode.is_material() {
            material_done = super::material::draw_cap(
                self.pixmap,
                self.cfg,
                self.pose,
                half_depth,
                plush,
                self.body,
                &self.pal,
                self.facing,
                self.cy,
                self.sy,
                self.cp,
                self.sp,
                self.lx,
                self.ly,
                self.dev_scale,
            );
        }
        let mode = if self.mode.is_material() && !material_done {
            BotAvatarShading::Smooth
        } else {
            self.mode
        };
        let soft = mode == BotAvatarShading::Smooth;

        let order = if self.facing >= 0.0 { 1.0 } else { -1.0 };
        let mut union: Option<Mask> = if soft {
            Some(mask(self.pixmap))
        } else {
            None
        };
        if !material_done {
            for j in 0..SLICES {
                let k = if order > 0.0 {
                    j
                } else {
                    SLICES - 1 - j
                };
                let z = -1.0 + (2.0 * k as f32) / (SLICES - 1) as f32;
                let s = profile(z, self.cap);
                let near = j as f32 / (SLICES - 1) as f32;
                let m0 = self.cy * s;
                let m1 = self.sy * self.sp * s;
                let m3 = self.cp * s;
                let e = z * self.sy * half_depth - 50.0 * m0;
                let f = -z * self.cy * self.sp * half_depth - 50.0 * m1 - 50.0 * m3;
                let slice = Affine([m0, m1, 0.0, m3, e, f]);
                let Some(transformed) = path.clone().transform(slice.transform()) else {
                    continue;
                };
                let last = j == SLICES - 1;
                if soft {
                    let p = paint(self.pal.smooth_mix[j]);
                    fill(self.pixmap, &transformed, &p, self.body, None);
                } else if last {
                    if mode == BotAvatarShading::Crisp {
                        if let Some(shader) = cap_gradient.clone() {
                            let mut p = paint(self.pal.base);
                            p.shader = shader;
                            fill(self.pixmap, &transformed, &p, self.body, None);
                        } else {
                            fill(self.pixmap, &transformed, &paint(self.pal.base), self.body, None);
                        }
                    } else {
                        fill(self.pixmap, &transformed, &paint(self.pal.base), self.body, None);
                    }
                } else if near > 0.6 && mode == BotAvatarShading::Crisp {
                    if let Some(shader) = lit_gradient.clone() {
                        let mut p = paint(self.pal.near);
                        p.shader = shader;
                        fill(self.pixmap, &transformed, &p, self.body, None);
                    } else {
                        fill(self.pixmap, &transformed, &paint(self.pal.near), self.body, None);
                    }
                } else {
                    fill(
                        self.pixmap,
                        &transformed,
                        &paint(self.pal.crisp_mix[j]),
                        self.body,
                        None,
                    );
                }
                if let Some(m) = union.as_mut() {
                    mask_fill(m, &transformed, self.body);
                }
            }
        }

        // Smooth: a soft shadow from the lower right and a light from the
        // upper left, laid over the whole form so nothing has an edge.
        if let Some(m) = union.as_ref() {
            if mode == BotAvatarShading::Smooth {
                let sa = (0.34 * self.shadow).min(1.0);
                if let Some(path) = rect_path(-120.0, -120.0, 240.0, 240.0) {
                    if let Some(shader) = radial(
                        -self.lx * 45.0,
                        -self.ly * 45.0,
                        4.0 * self.spread,
                        84.0 * self.spread,
                        &[
                            (0.0, Rgba::new(0, 0, 0, 255).with_alpha(sa)),
                            (0.5, Rgba::new(0, 0, 0, 255).with_alpha(sa * 0.35)),
                            (1.0, Rgba::new(0, 0, 0, 0)),
                        ],
                    ) {
                        let mut p = paint(Rgba::new(0, 0, 0, 255));
                        p.shader = shader;
                        p.blend_mode = BlendMode::Multiply;
                        fill(self.pixmap, &path, &p, self.body, Some(m));
                    }
                    let ha = (0.22 * self.highlight).min(1.0);
                    if let Some(shader) = radial(
                        self.lx * 37.0,
                        self.ly * 37.0,
                        0.0,
                        62.0 * self.spread,
                        &[
                            (0.0, Rgba::new(255, 255, 255, 255).with_alpha(ha)),
                            (0.6, Rgba::new(255, 255, 255, 255).with_alpha(ha * 0.23)),
                            (1.0, Rgba::new(255, 255, 255, 0)),
                        ],
                    ) {
                        let mut p = paint(Rgba::new(255, 255, 255, 255));
                        p.shader = shader;
                        fill(self.pixmap, &path, &p, self.body, Some(m));
                    }
                }
            }
        }
        let _ = plush;
        material_done
    }
}

/// Draw one frame.
///
/// `box` is the avatar's layout size in logical pixels, `density` the device
/// pixel ratio. The returned pixmap is `box × OVERSCAN × density` square.
pub(crate) fn draw_frame(box_px: f32, density: f32, pose: &Pose, cfg: &RenderConfig) -> Pixmap {
    let density = density.clamp(0.25, 4.0);
    let box_px = box_px.max(1.0);
    let full = box_px * OVERSCAN * density;
    let side = full.round().max(1.0) as u32;
    let mut pixmap = Pixmap::new(side, side).expect("non-zero canvas");
    let s = box_px / 100.0 * density;
    let cy0 = pose.yaw.cos();
    let sy = pose.yaw.sin();
    let cp0 = pose.pitch.cos();
    let sp = pose.pitch.sin();
    let facing = cy0 * cp0;
    let cy = floor_foreshorten(cy0);
    let cp = floor_foreshorten(cp0);
    let cr = pose.roll.cos();
    let sr = pose.roll.sin();
    let kx = pose.sx * s;
    let ky = pose.sy * s;
    let lift = 50.0 * (1.0 - pose.sy) * s;
    let body = Affine([
        cr * kx,
        sr * kx,
        -sr * ky,
        cr * ky,
        full / 2.0 + pose.x * s - sr * lift,
        full / 2.0 + RISE * box_px * density + pose.y * s + cr * lift,
    ]);
    let lit = if cfg.shading == BotAvatarShading::Fabric {
        &LIGHT_FABRIC
    } else {
        &LIGHT_OTHER
    };
    let shadow = cfg.shadow.clamp(0.0, 2.0);
    let highlight = cfg.highlight.clamp(0.0, 2.0);
    let half_depth = HALF_DEPTH * cfg.depth.clamp(0.2, 2.0);
    let rim_k = cfg.rim.clamp(0.0, 2.0);
    let cap = 1.0 - (1.0 - CAP) * rim_k;
    let spread = cfg.spread.clamp(0.4, 2.5);
    let light = cfg.light.clamp(-360.0, 360.0);
    let la = light.to_radians();
    let lx = la.sin();
    let ly = -la.cos();
    let pal = palette(cfg.color, shadow, highlight);
    let mode = cfg.shading;
    let _ = lit;

    let mut painter = Painter {
        pixmap: &mut pixmap,
        cfg,
        pose,
        body,
        pal,
        lx,
        ly,
        half_depth,
        cap,
        rim_k,
        shadow,
        highlight,
        spread,
        facing,
        cy,
        sy,
        cp,
        sp,
        mode,
        dev_scale: box_px * density,
    };

    // The far half of the whirl sits behind everything.
    whirl::draw(
        painter.pixmap,
        pose,
        cfg.color,
        lx,
        ly,
        false,
        cfg.whirl,
        body,
    );

    // What the bot wears, placed on its own geometry and turned with it.
    let rig = wear::WearRig {
        cy,
        sy,
        cp,
        sp,
        facing,
        half_depth,
        lx,
        ly,
    };
    if cfg.wear.any() {
        wear::draw_wear_behind(painter.pixmap, &cfg.path, body, rig, &cfg.wear);
    }

    let key = cfg.type_key.as_str();
    for (i, part) in cfg.parts.iter().enumerate() {
        painter.draw_solid(part, half_depth * 0.4, false);
        let _ = (key, i);
    }
    let material_done = painter.draw_solid(&cfg.path, half_depth, true);

    // The face: each feature sits on a sphere behind the front cap, so a turn
    // slides it round the head — the eye moving toward the edge narrows, the
    // other comes to the front, and past the side they go.
    let zf = if facing >= 0.0 { 1.0 } else { -1.0 };
    let cushion = material_done;
    let sf = if cushion {
        1.0
    } else {
        profile(zf, cap)
    };
    let z_face = if cushion {
        0.55
            * super::material::surface_at(
                key,
                half_depth,
                cfg.face_x,
                cfg.face_y,
                cfg.roundness,
            )
            .unwrap_or(0.8 * half_depth)
    } else {
        half_depth
    };
    let fm0 = cy * sf;
    let fm1 = sy * sp * sf;
    let fm3 = cp * sf;
    let fe = zf * sy * z_face - 50.0 * fm0;
    let ffo = -zf * cy * sp * z_face - 50.0 * fm1 - 50.0 * fm3;
    let front = Affine([fm0, fm1, 0.0, fm3, fe, ffo]);

    let mut front_clip = mask(painter.pixmap);
    mask_fill(&mut front_clip, &cfg.path, front.then(body));
    if facing > -0.2 {
        let face_transform = Affine::scale(cfg.face_scale, cfg.face_scale)
            .then(Affine::translate(
                cfg.face_x - 50.0,
                cfg.face_y - 50.0,
            ))
            .then(body);
        let eyes = draw_face(
            painter.pixmap,
            pose,
            cfg.face,
            cfg.ink,
            face_transform,
            Some(&front_clip),
        );
        let _ = FACE_R;
        if cfg.wear.glasses != super::super::types::BotAvatarGlasses::None {
            wear::draw_glasses(
                painter.pixmap,
                cfg.wear.glasses,
                eyes,
                lx,
                ly,
                false,
                pose.yaw,
                pose.pitch,
                face_transform,
                Some(&front_clip),
            );
            wear::draw_glasses(
                painter.pixmap,
                cfg.wear.glasses,
                eyes,
                lx,
                ly,
                true,
                pose.yaw,
                pose.pitch,
                face_transform,
                None,
            );
        }
    }
    if cfg.wear.any() {
        let surface_z = |x: f32, y: f32| -> f32 {
            if cushion {
                0.55 * super::material::surface_at(key, half_depth, x, y, cfg.roundness)
                    .unwrap_or(0.8 * half_depth)
            } else {
                half_depth * sf
            }
        };
        wear::draw_wear_front(
            painter.pixmap,
            &cfg.path,
            body,
            rig,
            &cfg.wear,
            Some(&front_clip),
            &surface_z,
            (cfg.face_y, cfg.face_scale),
        );
    }

    // The near half of the whirl passes in front of the face.
    whirl::draw(
        painter.pixmap,
        pose,
        cfg.color,
        lx,
        ly,
        true,
        cfg.whirl,
        body,
    );

    pixmap
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::bot_avatar::data::{bot_avatar_preset, bot_avatar_shape};
    use crate::widgets::bot_avatar::engine::{BotAvatarSim, rest_pose};
    use crate::widgets::bot_avatar::path::parse_svg_path;
    use crate::widgets::bot_avatar::types::{BotAvatarState, BotAvatarType};

    fn config(shading: BotAvatarShading) -> RenderConfig {
        let preset = bot_avatar_preset(BotAvatarType::Clover);
        RenderConfig {
            path: Arc::new(parse_svg_path(bot_avatar_shape(BotAvatarType::Clover)).unwrap()),
            parts: Vec::new(),
            type_key: "clover".into(),
            face: preset.face,
            face_x: preset.face_x,
            face_y: preset.face_y,
            face_scale: preset.face_scale,
            color: Rgba::parse(preset.color).unwrap(),
            ink: Rgba::from_rgb(30, 26, 51),
            shading,
            shadow: LIGHT_OTHER.shadow,
            highlight: LIGHT_OTHER.highlight,
            depth: 0.65,
            roundness: 1.0,
            light: LIGHT_OTHER.light,
            rim: LIGHT_OTHER.rim,
            spread: LIGHT_OTHER.spread,
            vivid: 0.09,
            fur: Default::default(),
            back_light: None,
            light_front: 32.0,
            shine: 0.0,
            sheen: 0.0,
            back_softness: 1.0,
            theme: crate::widgets::bot_avatar::BotAvatarTheme::Dark,
            whirl: WhirlConfig::default(),
            still: true,
            wear: Default::default(),
        }
    }

    #[test]
    fn flat_and_crisp_frames_have_opaque_body_pixels() {
        for shading in [BotAvatarShading::Flat, BotAvatarShading::Crisp] {
            let cfg = config(shading);
            let pose = rest_pose(BotAvatarState::Idle);
            let frame = draw_frame(64.0, 1.0, &pose, &cfg);
            assert_eq!(frame.width(), 96);
            let opaque = frame.pixels().iter().filter(|p| p.alpha() > 200).count();
            assert!(opaque > 1500, "{shading:?}: {opaque}");
        }
    }

    #[test]
    fn smooth_frame_has_soft_edges() {
        let cfg = config(BotAvatarShading::Smooth);
        let pose = rest_pose(BotAvatarState::Idle);
        let frame = draw_frame(64.0, 1.0, &pose, &cfg);
        let partial = frame
            .pixels()
            .iter()
            .filter(|p| p.alpha() > 0 && p.alpha() < 250)
            .count();
        assert!(partial > 20, "{partial}");
    }

    #[test]
    fn rendering_is_deterministic() {
        let cfg = config(BotAvatarShading::Crisp);
        let mut sim = BotAvatarSim::new(0.4, BotAvatarState::Idle);
        for _ in 0..20 {
            sim.update(1.0 / 60.0);
        }
        let a = draw_frame(80.0, 2.0, &sim.pose(), &cfg);
        let b = draw_frame(80.0, 2.0, &sim.pose(), &cfg);
        assert!(a.data() == b.data());
    }
}
