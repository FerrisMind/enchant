//! The glossy cap and its side.
//!
//! The form's texels are shaded by the matcap, the front is laid over a stack
//! of outline levels cut through the height field, and the exposed edges of
//! those levels carry the front's light round onto the side. This is the real
//! per-pixel material from `drawPlasticCap` in `plastic.ts`, with the caches
//! per frame rather than per canvas: an avatar is small enough that a fresh
//! 64²–128² texture costs less than the bookkeeping a cache frame would need.

use std::sync::Arc;

use tiny_skia::{
    BlendMode, FilterQuality, IntSize, LinearGradient, Paint, Pixmap, PixmapPaint, SpreadMode,
    Transform,
};

use super::super::super::canvas::{Affine, fill, paint_alpha};
use super::super::super::color::{Rgba, shade};
use super::super::RenderConfig;
use super::super::palette::Palette;
use super::form::{self, Form, Rig, cap_frame, relief_for};
use super::matcap::{Material, build_matcap, sample_matcap, shade_texels};

/// Texture size for an avatar `device_px` wide (logical px × density).
pub(crate) fn tier_for(device_px: f32) -> usize {
    if device_px <= 100.0 {
        64
    } else if device_px <= 224.0 {
        96
    } else {
        128
    }
}

fn levels_for(device_px: f32) -> usize {
    if device_px <= 100.0 {
        14
    } else if device_px <= 224.0 {
        20
    } else {
        26
    }
}

fn linear(c: Rgba) -> [f32; 3] {
    let l = c.linear();
    [l[0] as f32, l[1] as f32, l[2] as f32]
}

fn color(c: [f32; 3]) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba(
        (c[0] / 255.0).clamp(0.0, 1.0),
        (c[1] / 255.0).clamp(0.0, 1.0),
        (c[2] / 255.0).clamp(0.0, 1.0),
        1.0,
    )
    .unwrap_or(tiny_skia::Color::BLACK)
}

type ShadedCap = (Arc<Form>, Vec<f32>, Vec<u8>, f32);

/// Shade the cap for the current light and turn.
#[allow(clippy::too_many_arguments)]
pub(crate) fn shaded_cap(
    cfg: &RenderConfig,
    half_depth: f32,
    facing: f32,
    cy: f32,
    sy: f32,
    cp: f32,
    sp: f32,
    lx: f32,
    ly: f32,
    n: usize,
) -> Option<ShadedCap> {
    let form = form::form_for(&cfg.type_key, &cfg.path, n, half_depth)?;
    let rig = Rig {
        cy,
        sy,
        cp,
        sp,
        facing,
        roll: 0.0,
        lx,
        ly,
        bx: None,
        by: None,
    };
    let frame = cap_frame(&rig);
    let material = Material {
        shadow: cfg.shadow,
        highlight: cfg.highlight,
        spread: cfg.spread,
        rim: cfg.rim,
        vivid: cfg.vivid,
        front: cfg.light_front.to_radians(),
        shine: cfg.shine,
        back_soft: cfg.back_softness,
        sheen: cfg.sheen,
    };
    let mc = build_matcap(linear(cfg.color), &frame, &material);
    let ao_k = (1.2 * cfg.shadow).min(1.3);
    let mut ao_mul = [0.0f32; 256];
    for (a, v) in ao_mul.iter_mut().enumerate() {
        *v = (1.0 - ao_k * (1.0 - a as f32 / 255.0)).max(0.0);
    }
    let mut tex = vec![0u8; n * n * 4];
    shade_texels(&form, &mc, &ao_mul, None, &mut tex);
    Some((form, mc, tex, ao_k))
}

/// Turn a straight RGBA texture into a premultiplied `Pixmap`.
pub(crate) fn to_pixmap(mut tex: Vec<u8>, n: usize) -> Option<Pixmap> {
    for px in tex.chunks_exact_mut(4) {
        let a = u16::from(px[3]);
        px[0] = ((u16::from(px[0]) * a) / 255) as u8;
        px[1] = ((u16::from(px[1]) * a) / 255) as u8;
        px[2] = ((u16::from(px[2]) * a) / 255) as u8;
    }
    Pixmap::from_vec(tex, IntSize::from_wh(n as u32, n as u32)?)
}

/// Draw the cap and its side; returns whether the material is ready.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw(
    pixmap: &mut Pixmap,
    cfg: &RenderConfig,
    _plush: bool,
    body: Affine,
    pal: &Palette,
    facing: f32,
    cy: f32,
    sy: f32,
    cp: f32,
    sp: f32,
    lx: f32,
    ly: f32,
    half_depth: f32,
    dev: f32,
) -> bool {
    let n = tier_for(dev);
    let Some((form, mc, tex, _)) = shaded_cap(cfg, half_depth, facing, cy, sy, cp, sp, lx, ly, n)
    else {
        return false;
    };
    let Some(cap) = to_pixmap(tex, n) else {
        return false;
    };
    draw_body(
        pixmap, cfg, body, pal, facing, cy, sy, cp, sp, lx, ly, &form, &mc, &cap, false, dev,
    )
}

/// Draw the side stack and lay the already-shaded front over it.
///
/// `front` is a premultiplied texture covering `[-PAD, 100 + PAD]`; `fabric`
/// switches the side's material from a clear coat's sampled light to the
/// matte shades a pile is seen in.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_body(
    pixmap: &mut Pixmap,
    cfg: &RenderConfig,
    body: Affine,
    pal: &Palette,
    facing: f32,
    cy: f32,
    sy: f32,
    cp: f32,
    sp: f32,
    lx: f32,
    ly: f32,
    form: &Form,
    mc: &[f32],
    front: &Pixmap,
    fabric: bool,
    dev: f32,
) -> bool {
    let n = form.n;
    let k_levels = levels_for(dev);
    let rel = relief_for(form, k_levels, cfg.roundness);
    let m1 = sy * sp;
    let rot = Affine([cy, m1, 0.0, cp, 0.0, 0.0]);
    // The slice's own affine, centred on the body box: rotate about the
    // centre, then step along the depth. This is the reference's `at(z)`.
    let slice_at = |z: f32| {
        rot.then(Affine::translate(
            z * sy - 50.0 * cy,
            -z * cy * sp - 50.0 * m1 - 50.0 * cp,
        ))
    };

    // The side's own light, in body design coordinates. A single gradient
    // runs round every level, so the front's light carries on to the side.
    let (lx0, ly0) = if (lx * lx + ly * ly).sqrt() < 0.05 {
        (0.0, -1.0)
    } else {
        let l = (lx * lx + ly * ly).sqrt();
        (lx / l, ly / l)
    };
    let (side_light, side_mid, side_dark) = if fabric {
        let lit = pal.base.mix(shade(pal.base, 0.08, 0.02), 0.65);
        let dark = shade(pal.base, -0.22 * f64::from(cfg.shadow.min(1.4)), 0.03);
        (
            [f32::from(lit.r), f32::from(lit.g), f32::from(lit.b)],
            [
                f32::from(pal.base.r),
                f32::from(pal.base.g),
                f32::from(pal.base.b),
            ],
            [f32::from(dark.r), f32::from(dark.g), f32::from(dark.b)],
        )
    } else {
        (
            sample_matcap(mc, lx0 * 0.85, ly0 * 0.85),
            sample_matcap(mc, 0.0, 0.0),
            sample_matcap(mc, -lx0 * 0.85, -ly0 * 0.85),
        )
    };
    let stops = [
        (
            0.0,
            color([
                side_light[0].clamp(0.0, 255.0),
                side_light[1].clamp(0.0, 255.0),
                side_light[2].clamp(0.0, 255.0),
            ]),
        ),
        (0.5, color(side_mid)),
        (
            1.0,
            color([
                side_dark[0].clamp(0.0, 255.0),
                side_dark[1].clamp(0.0, 255.0),
                side_dark[2].clamp(0.0, 255.0),
            ]),
        ),
    ];
    let side_shader = LinearGradient::new(
        tiny_skia::Point::from_xy(lx0 * 60.0, ly0 * 60.0),
        tiny_skia::Point::from_xy(-lx0 * 60.0, -ly0 * 60.0),
        stops
            .iter()
            .map(|&(p, c)| tiny_skia::GradientStop::new(p, c))
            .collect(),
        SpreadMode::Pad,
        Transform::identity(),
    );
    let mut side_paint = Paint {
        anti_alias: true,
        ..Paint::default()
    };
    if let Some(shader) = side_shader {
        side_paint.shader = shader;
    } else {
        side_paint.set_color_rgba8(side_mid[0] as u8, side_mid[1] as u8, side_mid[2] as u8, 255);
    }

    let near_sign = if facing >= 0.0 { 1.0 } else { -1.0 };
    let back_shade = if fabric {
        (0.42 * (cfg.shadow / 1.15).min(1.4)).min(0.5)
    } else {
        (0.55 * (cfg.shadow / 0.35).min(1.6)).min(0.7)
    };
    let c0 = (rel.cx[0], rel.cy[0]);
    let slice_path = |k: usize, z: f32| -> Option<tiny_skia::Path> {
        let sk = rel.scale[k];
        let local = Affine([
            sk,
            0.0,
            0.0,
            sk,
            rel.cx[k] - sk * c0.0,
            rel.cy[k] - sk * c0.1,
        ]);
        cfg.path
            .as_ref()
            .clone()
            .transform(local.then(slice_at(z)).transform())
    };

    // Far half, deepest first.
    for k in (1..k_levels).rev() {
        let z = -near_sign * rel.z[k];
        let Some(path) = slice_path(k, z) else {
            continue;
        };
        fill(pixmap, &path, &side_paint, body, None);
        let dark = back_shade * (1.0 - (-2.4 * k as f32 / k_levels as f32).exp());
        if dark > 0.01 {
            let p = paint_alpha(Rgba::new(0, 0, 0, 255), dark);
            fill(pixmap, &path, &p, body, None);
        }
    }
    // The equator.
    if let Some(path) = slice_path(0, 0.0) {
        fill(pixmap, &path, &side_paint, body, None);
    }
    // Near half, over the equator.
    for k in 1..k_levels {
        let z = near_sign * rel.z[k];
        if let Some(path) = slice_path(k, z) {
            fill(pixmap, &path, &side_paint, body, None);
        }
    }

    // The lit front, stretched along the turn to span from the equator on the
    // side turning away to the shoulder's silhouette on the side turning in.
    let dxs = if cy.abs() > 1e-5 { sy / cy } else { 0.0 };
    let dys = if (cy * cp).abs() > 1e-5 {
        -sp / (cy * cp)
    } else {
        0.0
    };
    let dl1 = (dxs * dxs + dys * dys).sqrt();
    let mut lead = 50.0f32;
    for k in 1..k_levels {
        lead = lead.max(50.0 * rel.scale[k] + rel.z[k] * dl1);
    }
    let (ex, ey) = if dl1 > 1e-6 {
        (near_sign * dxs / dl1, near_sign * dys / dl1)
    } else {
        (1.0, 0.0)
    };
    let stretch = (lead + 50.0) / 100.0;
    let shift = (lead - 50.0) / 2.0;
    let a = 1.0 + (stretch - 1.0) * ex * ex;
    let b = (stretch - 1.0) * ex * ey;
    let d = 1.0 + (stretch - 1.0) * ey * ey;
    let stretch_m = Affine([
        a,
        b,
        b,
        d,
        shift * ex - 50.0 * a - 50.0 * b,
        shift * ey - 50.0 * b - 50.0 * d,
    ]);
    let cap_m = stretch_m.then(rot).then(body);
    let tex_scale = form::SPAN / n as f32;
    let sprite_m = Affine::scale(tex_scale, tex_scale)
        .then(Affine::translate(-form::PAD, -form::PAD))
        .then(cap_m);
    let face = facing.abs();
    let front_a = if face >= 0.72 {
        1.0
    } else if face <= 0.34 {
        0.0
    } else {
        ((face - 0.34) / 0.38).powf(1.5)
    };
    if front_a > 0.01 {
        let paint = PixmapPaint {
            opacity: front_a,
            blend_mode: BlendMode::SourceOver,
            quality: FilterQuality::Bicubic,
        };
        pixmap.draw_pixmap(0, 0, front.as_ref(), &paint, sprite_m.transform(), None);
    }
    true
}
