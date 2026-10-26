//! Things the bots can wear: a hat, glasses, headphones, a bow tie.
//!
//! Ported from `wear.ts` and `mesh.ts`. Each is a small 3D object built from
//! lit surfaces and placed on the body's own geometry — the top of the head,
//! the sides, the eyes, the front below the face — then projected through the
//! same rotation as the slice stack, so it turns, tips and flips with the
//! head instead of floating over it. Materials follow the body's light: felt
//! and knit, paper and foil, gold with cut stones and pearls, leather and
//! soft plastic with brushed metal, satin, acetate and glass. Each casts a
//! soft shadow where it sits on the body.
//!
//! Everything is drawn in body space (the caller passes the body's transform
//! for the slices); the glasses in face space.

#![allow(clippy::too_many_arguments)] // signatures follow the reference accessory routines

use std::collections::HashMap;
use std::f32::consts::{PI, TAU};
use std::rc::Rc;
use std::sync::{Arc, Mutex, OnceLock};

use tiny_skia::{FillRule, Mask, Path, PathBuilder, PathSegment, Pixmap};

use super::super::canvas::{Affine, mask, mask_fill, paint, paint_alpha};
use super::super::color::{Rgba, shade};
use super::super::types::{BotAvatarGlasses, BotAvatarHat};
use super::face::EyeSpot;
use super::mesh::{
    self, Layer, P2, PaintOpts, Proj, Rgb, Shade, V3, View, cross, cut_gem, felt, gold, hull,
    make_mesh, make_tube, make_view, metal, norm, paper, plastic, poly_path, project_mesh, satin,
    scale, section, segs, silver, sphere_sprite, sub, velvet,
};

/// Everything a bot can wear.
#[derive(Clone, Debug)]
pub(crate) struct Wear {
    pub hat: BotAvatarHat,
    pub glasses: BotAvatarGlasses,
    pub headphones: bool,
    pub bow_tie: bool,
    /// The hat's, the headphones' and the bow tie's colour.
    pub color: Rgba,
    /// The body is plush: the bow tie is the same felt rather than satin.
    pub fabric: bool,
}

impl Default for Wear {
    fn default() -> Self {
        Self {
            hat: BotAvatarHat::None,
            glasses: BotAvatarGlasses::None,
            headphones: false,
            bow_tie: false,
            color: Rgba::from_rgb(120, 122, 132),
            fabric: false,
        }
    }
}

impl Wear {
    /// Whether anything is worn at all.
    #[must_use]
    pub fn any(&self) -> bool {
        self.hat != BotAvatarHat::None
            || self.glasses != BotAvatarGlasses::None
            || self.headphones
            || self.bow_tie
    }
}

/// The rig as the slices use it: floored cos/sin of yaw and pitch, the
/// unfloored facing, the body's half-depth, and the light on screen.
#[derive(Clone, Copy, Debug)]
pub(crate) struct WearRig {
    pub cy: f32,
    pub sy: f32,
    pub cp: f32,
    pub sp: f32,
    pub facing: f32,
    pub lx: f32,
    pub ly: f32,
}

/* ── the outline's landmarks, once per shape ────────────────────────── */

struct Marks {
    top: f32,
    top_l: f32,
    top_r: f32,
    mid: f32,
    left: f32,
    right: f32,
    bottom: f32,
    widths: Vec<f32>,
    step: f32,
    hull: Vec<P2>,
}

const PAD: f32 = 3.0;
const SPAN: f32 = 106.0;

fn seat(m: &Marks, frac: f32) -> f32 {
    let w = m.right - m.left;
    for (i, &width) in m.widths.iter().enumerate() {
        if width >= frac * w {
            return i as f32 * m.step - PAD;
        }
    }
    m.top
}

fn path_hash(path: &Path) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for seg in path.segments() {
        match seg {
            PathSegment::MoveTo(p) => {
                1u8.hash(&mut h);
                p.x.to_bits().hash(&mut h);
                p.y.to_bits().hash(&mut h);
            }
            PathSegment::LineTo(p) => {
                2u8.hash(&mut h);
                p.x.to_bits().hash(&mut h);
                p.y.to_bits().hash(&mut h);
            }
            PathSegment::QuadTo(a, b) => {
                3u8.hash(&mut h);
                for p in [a, b] {
                    p.x.to_bits().hash(&mut h);
                    p.y.to_bits().hash(&mut h);
                }
            }
            PathSegment::CubicTo(a, b, c) => {
                4u8.hash(&mut h);
                for p in [a, b, c] {
                    p.x.to_bits().hash(&mut h);
                    p.y.to_bits().hash(&mut h);
                }
            }
            PathSegment::Close => 5u8.hash(&mut h),
        }
    }
    h.finish()
}

fn marks_for(path: &Path) -> Option<Arc<Marks>> {
    static CACHE: OnceLock<Mutex<HashMap<u64, Arc<Marks>>>> = OnceLock::new();
    let key = path_hash(path);
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(c) = cache.lock()
        && let Some(m) = c.get(&key)
    {
        return Some(Arc::clone(m));
    }
    let marks = Arc::new(build_marks(path)?);
    if let Ok(mut c) = cache.lock() {
        if c.len() > 64 {
            c.clear();
        }
        c.insert(key, Arc::clone(&marks));
    }
    Some(marks)
}

fn build_marks(path: &Path) -> Option<Marks> {
    const N: usize = 128;
    let u = SPAN / N as f32;
    let k = N as f32 / SPAN;
    let mut cov = Mask::new(N as u32, N as u32)?;
    cov.fill_path(
        path,
        FillRule::Winding,
        true,
        Affine::scale(k, k)
            .then(Affine::translate(PAD * k, PAD * k))
            .transform(),
    );
    let data = cov.data();
    let inside = |x: usize, y: usize| data[y * N + x] >= 128;
    let to_x = |i: usize| (i as f32 + 0.5) * u - PAD;
    let c0 = ((42.0 + PAD) / u).floor() as usize;
    let c1 = ((58.0 + PAD) / u).ceil() as usize;
    let mut top = N;
    let mut bottom = 0usize;
    for x in c0..=c1.min(N - 1) {
        for y in 0..N {
            if inside(x, y) {
                top = top.min(y);
                break;
            }
        }
        for y in (0..N).rev() {
            if inside(x, y) {
                bottom = bottom.max(y);
                break;
            }
        }
    }
    if top >= N {
        return None;
    }
    let row = |y: usize| -> (i32, i32) {
        let mut l = -1i32;
        let mut r = -1i32;
        for x in 0..N {
            if inside(x, y) {
                if l < 0 {
                    l = x as i32;
                }
                r = x as i32;
            }
        }
        (l, r)
    };
    let yt = (top + (8.0 / u).round() as usize).min(N - 1);
    let (tl, tr) = row(yt);
    let mut best = -1;
    let (mut bl, mut br, mut by) = (0i32, 0i32, 0usize);
    let y0 = ((33.0 + PAD) / u).round() as usize;
    let y1 = ((67.0 + PAD) / u).round() as usize;
    for y in y0..=y1.min(N - 1) {
        let (l, r) = row(y);
        if l >= 0 && r - l > best {
            best = r - l;
            bl = l;
            br = r;
            by = y;
        }
    }
    let mut widths = vec![0.0f32; N];
    let mut edge: Vec<P2> = Vec::new();
    for (y, width) in widths.iter_mut().enumerate() {
        let (l, r) = row(y);
        *width = if l < 0 { 0.0 } else { (r - l + 1) as f32 * u };
        if l >= 0 {
            edge.push([to_x(l as usize) - 0.5 * u, to_x(y)]);
            edge.push([to_x(r as usize) + 0.5 * u, to_x(y)]);
        }
    }
    Some(Marks {
        top: to_x(top) - 0.5 * u,
        top_l: to_x(tl.max(0) as usize),
        top_r: to_x(tr.max(0) as usize),
        mid: to_x(by),
        left: to_x(bl.max(0) as usize) - 0.5 * u,
        right: to_x(br.max(0) as usize) + 0.5 * u,
        bottom: to_x(bottom) + 0.5 * u,
        widths,
        step: u,
        hull: hull(&edge),
    })
}

/* ── projection ─────────────────────────────────────────────────────── */

fn tilt(v: V3, roll: f32, lean: f32) -> V3 {
    let (sr, cr) = roll.sin_cos();
    let x = v[0] * cr - v[1] * sr;
    let y = v[0] * sr + v[1] * cr;
    let (sl, cl) = lean.sin_cos();
    [x, y * cl - v[2] * sl, y * sl + v[2] * cl]
}

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn along(a: V3, d: V3, k: f32) -> V3 {
    [a[0] + d[0] * k, a[1] + d[1] * k, a[2] + d[2] * k]
}

fn bounds(pts: &[P2]) -> (f32, f32, f32, f32, f32, f32, f32, f32) {
    let (mut x0, mut y0, mut x1, mut y1) = (
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    );
    for &[x, y] in pts {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    (
        x0,
        y0,
        x1,
        y1,
        (x0 + x1) * 0.5,
        (y0 + y1) * 0.5,
        x1 - x0,
        y1 - y0,
    )
}

fn smooth01(t: f32) -> f32 {
    t.clamp(0.0, 1.0).powi(2) * (3.0 - 2.0 * t.clamp(0.0, 1.0))
}

fn spow(c: f32, e: f32) -> f32 {
    c.signum() * c.abs().powf(e)
}

/* ── the view for the meshes ───────────────────────────────────────── */

const EL: f32 = 48.0 * PI / 180.0;

fn light_of(lx: f32, ly: f32) -> (V3, V3) {
    let le = EL.cos();
    let lz = EL.sin();
    let l = norm([lx * le, ly * le, lz]);
    (l, norm([l[0], l[1], l[2] + 1.0]))
}

fn view_of(r: WearRig, px: f32) -> View {
    let (l, h) = light_of(r.lx, r.ly);
    make_view(
        [
            r.cy,
            0.0,
            r.sy,
            r.sy * r.sp,
            r.cp,
            -r.cy * r.sp,
            -r.sy * r.cp,
            r.sp,
            r.cy * r.cp,
        ],
        [50.0, 50.0, 0.0],
        l,
        h,
        px,
    )
}

fn px_of(space: Affine) -> f32 {
    let m = space.0;
    (m[0] * m[3] - m[1] * m[2]).abs().sqrt().max(1.0)
}

fn rgb_of(c: Rgba) -> Rgb {
    [f32::from(c.r), f32::from(c.g), f32::from(c.b)]
}

fn rgb_shade(c: Rgba, dl: f64, ds: f64) -> Rgb {
    rgb_of(shade(c, dl, ds))
}

fn owner_key(m: &Marks) -> String {
    format!(
        "{:.3},{:.3},{:.3},{:.3},{:.3}",
        m.top, m.left, m.right, m.bottom, m.mid
    )
}

fn surface<P, I>(
    view: View,
    owner: &str,
    key: &str,
    surf: P,
    nu: usize,
    nv: usize,
    inside: I,
    smooth: bool,
) -> Proj
where
    P: Fn(f32, f32) -> V3,
    I: Fn(f32, f32) -> V3,
{
    let cache_key = format!("{owner}|{key}|{nu}x{nv}|{}", smooth);
    let mesh = mesh::keep_mesh(&cache_key, || make_mesh(&surf, nu, nv, &inside, smooth));
    project_mesh(view, &mesh)
}

fn tube<P, F>(
    view: View,
    owner: &str,
    key: &str,
    path: P,
    up: V3,
    prof: F,
    nu: usize,
    nv: usize,
    closed: bool,
) -> Proj
where
    P: Fn(f32) -> V3,
    F: Fn(f32, f32) -> [f32; 2],
{
    let cache_key = format!("{owner}|{key}|{nu}x{nv}|t{closed}");
    let mesh = mesh::keep_mesh(&cache_key, || make_tube(&path, up, &prof, nu, nv, closed));
    project_mesh(view, &mesh)
}

fn sphere_mesh(view: View, owner: &str, key: &str, c: V3, rad: f32, nu: usize, nv: usize) -> Proj {
    surface(
        view,
        owner,
        key,
        |u, v| {
            let th = u * TAU;
            let ph = v * PI;
            [
                c[0] + rad * ph.sin() * th.cos(),
                c[1] - rad * ph.cos(),
                c[2] + rad * ph.sin() * th.sin(),
            ]
        },
        nu,
        nv,
        |_, _| c,
        true,
    )
}

fn ring_mesh<F>(
    view: View,
    owner: &str,
    key: &str,
    place: F,
    rr: f32,
    y: f32,
    t: f32,
    z_scale: f32,
    nu: usize,
    nv: usize,
) -> Proj
where
    F: Fn(V3) -> V3 + Copy,
{
    surface(
        view,
        owner,
        key,
        |u, v| {
            let th = u * TAU;
            let ph = v * TAU;
            let r = rr + t * ph.cos();
            place([r * th.cos(), y - t * ph.sin(), r * z_scale * th.sin()])
        },
        nu,
        nv,
        move |u, _| place([rr * (u * TAU).cos(), y, rr * z_scale * (u * TAU).sin()]),
        true,
    )
}

fn offset_path<P, O>(path: P, up: V3, off: O) -> impl Fn(f32) -> V3
where
    P: Fn(f32) -> V3,
    O: Fn(f32) -> f32,
{
    move |u: f32| {
        let e = 1e-3;
        let t = norm(sub(path((u + e).min(1.0)), path((u - e).max(0.0))));
        let b = cross(t, up);
        along(path(u), b, off(u))
    }
}

fn yarn_ball(
    pixmap: &mut Pixmap,
    view: View,
    owner: &str,
    space: Affine,
    c: V3,
    rad: f32,
    base: Rgba,
    seed: i32,
) {
    let rgb = rgb_of(base);
    let n = segs(view, TAU * rad, 12, 20, 11.0);
    let ball = sphere_mesh(
        view,
        owner,
        &format!("ball{seed}"),
        c,
        rad * 0.86,
        n,
        (n >> 1).max(6),
    );
    let vis = mesh::paint_layers(
        pixmap,
        &[Layer::new(vec![ball], felt(view, rgb, 0.92))],
        view.px,
        space,
        PaintOpts::default(),
    );
    let centre = view.to_screen(c);
    let (_, _, _, _, _, _, w, h) = bounds(&vis);
    let r = w.max(h) * 0.5;
    let count = (50.0 + r * view.px * 5.0).min(340.0).round() as i32;
    // The strands are drawn in the context's space; body design units.
    let mut light = PathBuilder::new();
    let mut dark = PathBuilder::new();
    for k in 0..count {
        let a = mesh::rnd(k, seed) * TAU;
        let r0 = r * (0.25 + 0.7 * mesh::rnd(k, seed + 1).sqrt());
        let len = r * (0.25 + 0.35 * mesh::rnd(k, seed + 2));
        let x0 = centre[0] + a.cos() * r0;
        let y0 = centre[1] + a.sin() * r0;
        let bend = (mesh::rnd(k, seed + 3) - 0.5) * 0.6;
        let ex = (a + bend).cos();
        let ey = (a + bend).sin();
        let p = if mesh::rnd(k, seed + 4) > 0.45 {
            &mut light
        } else {
            &mut dark
        };
        p.move_to(x0, y0);
        p.quad_to(
            x0 + a.cos() * len * 0.5,
            y0 + a.sin() * len * 0.5,
            x0 + ex * len,
            y0 + ey * len,
        );
    }
    let l = view.l;
    let from = [centre[0] + l[0] * r, centre[1] + l[1] * r];
    let to = [centre[0] - l[0] * r, centre[1] - l[1] * r];
    if let Some(p) = light.finish() {
        let mut lp = mesh::linear_paint(
            from,
            to,
            &[
                (0.0, shade(base, 0.2, -0.03).with_alpha(0.85)),
                (1.0, shade(base, -0.12, 0.02).with_alpha(0.85)),
            ],
        );
        lp.anti_alias = true;
        mesh::stroke_path(pixmap, &p, &lp, (r * 0.06).max(0.3), space, None);
    }
    if let Some(p) = dark.finish() {
        let d = shade(base, -0.18, 0.03);
        mesh::stroke_path(
            pixmap,
            &p,
            &paint_alpha(d, 0.6),
            (r * 0.06).max(0.3),
            space,
            None,
        );
    }
}

/* ── the hats ───────────────────────────────────────────────────────── */

fn draw_beret(
    pixmap: &mut Pixmap,
    view: View,
    owner: &str,
    m: &Marks,
    base: Rgba,
    space: Affine,
    body_clip: Option<&Mask>,
) {
    let w = m.right - m.left;
    let r = 0.46 * w;
    let hh = 0.21 * w;
    let rb = 0.33 * w;
    let lift = 0.05 * w;
    let c: V3 = [
        50.0 - 0.07 * w,
        (m.top + 0.22 * w).max(seat(m, 0.5) + 0.1 * w),
        0.0,
    ];
    let (roll, lean) = (-0.24, -0.2);
    let place = |p: V3| add(c, tilt(p, roll, lean));
    let nu = segs(view, TAU * r, 32, 56, 11.0);
    let nv = segs(view, 1.2 * r, 10, 32, 6.0);
    let surf = |u: f32, v: f32| -> V3 {
        let th = u * TAU;
        let e = 0.74;
        let (mut rr, hh2);
        if v <= e {
            let phi = (v / e) * PI * 0.5;
            rr = r * phi.sin();
            hh2 = -(lift + hh * phi.cos().powf(0.8));
        } else {
            let f = smooth01((v - e) / (1.0 - e));
            rr = r + (rb - r) * f;
            hh2 = -lift * (1.0 - f);
        }
        let band = ((v / e).min(1.0) * PI * 0.9).sin();
        rr *= 1.0 + 0.035 * (th * 7.0 + 0.4).sin() * band;
        place([rr * th.cos(), hh2, rr * 0.93 * th.sin()])
    };
    let dome = surface(
        view,
        owner,
        "beret:dome",
        surf,
        nu,
        nv,
        |_, _| place([0.0, -lift - hh * 0.4, 0.0]),
        true,
    );
    let under = surface(
        view,
        owner,
        "beret:under",
        |u, v| {
            let th = u * TAU;
            let rr = rb * (1.0 - v);
            place([rr * th.cos(), -0.004 * w * v, rr * 0.93 * th.sin()])
        },
        24,
        2,
        |_, _| place([0.0, -lift - hh * 0.4, 0.0]),
        true,
    );
    let rgb = rgb_of(base);
    let band_mid = view.to_screen(place([0.0, 0.02 * w, 0.0]));
    mesh::contact(
        pixmap,
        body_clip,
        space,
        band_mid[0] + 0.02 * w,
        band_mid[1] + 0.045 * w,
        rb * 1.25,
        0.08 * w,
        0.4,
    );
    let binding = ring_mesh(
        view,
        owner,
        "beret:binding",
        place,
        rb,
        -0.005 * w,
        0.022 * w,
        0.93,
        segs(view, TAU * rb, 24, 48, 11.0),
        5,
    );
    let leather = rgb_shade(base, -0.06, 0.0);
    let vis = mesh::paint_layers(
        pixmap,
        &[
            Layer::new(vec![under], felt(view, rgb, 0.7)),
            Layer::new(vec![binding], plastic(view, leather, 0.45)),
            Layer::new(vec![dome], felt(view, rgb, 1.0)),
        ],
        view.px,
        space,
        PaintOpts::default(),
    );
    let outline = hull(&vis);
    mesh::grain_over(pixmap, &outline, 0.5, 0.14, space, None);
    mesh::fuzz(pixmap, &outline, rgb, 11, 0.4, view.px, space, None);
    let x0 = 0.02 * w;
    let y0 = -lift - hh + 0.006 * w;
    let stalk = tube(
        view,
        owner,
        "beret:stalk",
        |u| place([x0 + 0.014 * w * u * u, y0 - 0.045 * w * u, 0.0]),
        [0.0, 0.0, 1.0],
        |u, ph| {
            [
                0.016 * w * (1.0 - 0.25 * u) * ph.cos(),
                0.016 * w * (1.0 - 0.25 * u) * ph.sin(),
            ]
        },
        8,
        3,
        false,
    );
    mesh::paint_layers(
        pixmap,
        &[Layer::new(vec![stalk], felt(view, rgb, 0.95))],
        view.px,
        space,
        PaintOpts::default(),
    );
    mesh::sphere_sprite_rgb(
        pixmap,
        view,
        space,
        place([x0 + 0.014 * w, y0 - 0.047 * w, 0.0]),
        0.019 * w,
        scale(rgb, 1.15),
    );
}

fn draw_beanie(
    pixmap: &mut Pixmap,
    view: View,
    owner: &str,
    m: &Marks,
    base: Rgba,
    space: Affine,
    body_clip: Option<&Mask>,
) {
    let w = m.right - m.left;
    let head_w = (0.5 * w).max(m.top_r - m.top_l);
    let rd = 0.53 * head_w + 0.08 * w;
    let hd = 0.32 * w;
    let cuff_h = 0.1 * w;
    let rc = rd * 1.05;
    let c: V3 = [50.0, seat(m, 0.72) + 0.06 * w, 0.0];
    let (roll, lean) = (0.0, -0.14);
    let place = |p: V3| add(c, tilt(p, roll, lean));
    let dome_surf = |u: f32, v: f32| -> V3 {
        let th = u * TAU;
        let phi = v * PI * 0.5;
        let rr = rd * phi.sin() * (1.0 - 0.03 * (1.0 - v));
        let hh = -(cuff_h + hd * phi.cos());
        place([rr * th.cos(), hh, rr * 0.92 * th.sin()])
    };
    let in_dome = place([0.0, -cuff_h - hd * 0.4, 0.0]);
    let dome = surface(
        view,
        owner,
        "beanie:dome",
        dome_surf,
        segs(view, TAU * rd, 28, 48, 11.0),
        segs(view, 1.3 * hd, 9, 24, 6.0),
        |_, _| in_dome,
        true,
    );
    let cuff_surf = |u: f32, v: f32| -> V3 {
        let th = u * TAU;
        let e = 2.0 * v - 1.0;
        let rr = rc * (1.0 - 0.07 * e.powi(4)) * 1.01;
        place([rr * th.cos(), -cuff_h * (1.0 - v), rr * 0.92 * th.sin()])
    };
    let cuff = surface(
        view,
        owner,
        "beanie:cuff",
        cuff_surf,
        segs(view, TAU * rc, 28, 56, 11.0),
        4,
        |_, _| place([0.0, -cuff_h * 0.5, 0.0]),
        true,
    );
    let rgb = rgb_of(base);
    let cuff_rgb = rgb_shade(base, 0.03, 0.0);
    let (bbx0, bby0, bbx1, bby1, ..) = bounds_of(&[&cuff]);
    let _ = (bbx0, bby0, bbx1, bby1);
    let (_, _, _, y1, cx, _, bw, _) = bounds_of(&[&cuff]);
    mesh::contact(
        pixmap,
        body_clip,
        space,
        cx,
        y1 - 0.01 * w,
        bw * 0.5,
        0.06 * w,
        0.34,
    );
    let vis_dome = mesh::paint_layers(
        pixmap,
        &[Layer::new(vec![dome], felt(view, rgb, 0.96))],
        view.px,
        space,
        PaintOpts::default(),
    );
    let nu = 40usize;
    let nv = 11usize;
    let cell = (rd * TAU) / nu as f32 * view.px;
    if cell >= 2.2 {
        let grid = surface(
            view,
            owner,
            "beanie:stitch",
            dome_surf,
            nu,
            nv,
            |_, _| in_dome,
            false,
        );
        let mut dark = PathBuilder::new();
        let mut light = PathBuilder::new();
        let gw = nu + 1;
        for c in 0..nu * nv {
            if grid.f[c * 3 + 2] < 0.12 {
                continue;
            }
            let k = (c / nu) * gw + (c % nu);
            let a: P2 = [grid.x[k], grid.y[k]];
            let b: P2 = [grid.x[k + 1], grid.y[k + 1]];
            let cq: P2 = [grid.x[k + gw + 1], grid.y[k + gw + 1]];
            let d: P2 = [grid.x[k + gw], grid.y[k + gw]];
            let at = |s: f32, t: f32| -> P2 {
                let top = [a[0] + (b[0] - a[0]) * s, a[1] + (b[1] - a[1]) * s];
                let bot = [d[0] + (cq[0] - d[0]) * s, d[1] + (cq[1] - d[1]) * s];
                [
                    top[0] + (bot[0] - top[0]) * t,
                    top[1] + (bot[1] - top[1]) * t,
                ]
            };
            for (p, dx) in [(&mut dark, 0.0f32), (&mut light, -0.06f32)] {
                let qa = at(0.14 + dx, 0.12);
                let qb = at(0.5 + dx, 0.88);
                let qc = at(0.86 + dx, 0.12);
                p.move_to(qa[0], qa[1]);
                p.line_to(qb[0], qb[1]);
                p.line_to(qc[0], qc[1]);
            }
        }
        let cw = (rd * TAU) / nu as f32;
        let mut clip = mask(pixmap);
        if let Some(p) = poly_path(&hull(&vis_dome)) {
            mask_fill(&mut clip, &p, space);
        }
        if let Some(p) = dark.finish() {
            mesh::stroke_path(
                pixmap,
                &p,
                &paint_alpha(shade(base, -0.22, 0.03), 0.5),
                (cw * 0.2).max(0.35),
                space,
                Some(&clip),
            );
        }
        if let Some(p) = light.finish() {
            mesh::stroke_path(
                pixmap,
                &p,
                &paint_alpha(shade(base, 0.2, -0.03), 0.35),
                (cw * 0.12).max(0.3),
                space,
                Some(&clip),
            );
        }
    }
    let vis_cuff = mesh::paint_layers(
        pixmap,
        &[Layer::new(vec![cuff], felt(view, cuff_rgb, 0.98))],
        view.px,
        space,
        PaintOpts::default(),
    );
    let rib_gap = (TAU * rc) / 30.0;
    if rib_gap * view.px >= 3.0 {
        let mut groove = PathBuilder::new();
        let mut ridge = PathBuilder::new();
        for k in 0..60 {
            let u = k as f32 / 60.0;
            let th = u * TAU;
            if view.dir(tilt([th.cos(), 0.0, 0.92 * th.sin()], roll, lean))[2] < 0.08 {
                continue;
            }
            let a = view.to_screen(cuff_surf(u, 0.1));
            let b = view.to_screen(cuff_surf(u, 0.9));
            let p = if k % 2 != 0 { &mut groove } else { &mut ridge };
            p.move_to(a[0], a[1]);
            p.line_to(b[0], b[1]);
        }
        if let Some(p) = groove.finish() {
            mesh::stroke_path(
                pixmap,
                &p,
                &paint_alpha(shade(base, -0.2, 0.03), 0.45),
                rib_gap * 0.34,
                space,
                None,
            );
        }
        if let Some(p) = ridge.finish() {
            mesh::stroke_path(
                pixmap,
                &p,
                &paint_alpha(shade(base, 0.16, -0.02), 0.4),
                rib_gap * 0.22,
                space,
                None,
            );
        }
    }
    let mut outline_pts = vis_dome;
    outline_pts.extend(vis_cuff);
    let outline = hull(&outline_pts);
    mesh::fuzz(pixmap, &outline, rgb, 23, 0.35, view.px, space, None);
    yarn_ball(
        pixmap,
        view,
        owner,
        space,
        place([0.0, -cuff_h - hd - 0.055 * w, 0.0]),
        0.085 * w,
        shade(base, 0.06, 0.0),
        31,
    );
}

fn draw_party(
    pixmap: &mut Pixmap,
    view: View,
    owner: &str,
    m: &Marks,
    base: Rgba,
    space: Affine,
    body_clip: Option<&Mask>,
) {
    let w = m.right - m.left;
    let c: V3 = [50.0 + 0.06 * w, seat(m, 0.3) + 0.04 * w, 0.0];
    let r = 0.18 * w;
    let h = (0.3 * w).max((0.46 * w).min(c[1] + 31.0 - 0.1 * w));
    let (roll, lean) = (0.18, -0.12);
    let place = |p: V3| add(c, tilt(p, roll, lean));
    let cone_at = |u: f32, v: f32, grow: f32| -> V3 {
        let th = u * TAU;
        let rr = r * (1.0 - v) * (1.0 + 0.04 * (v * PI).sin()) * grow;
        place([rr * th.cos(), -h * v, rr * th.sin()])
    };
    let in_cone = |_u: f32, v: f32| place([0.0, -h * v.min(0.97), 0.0]);
    let n_around = segs(view, TAU * r, 20, 40, 11.0);
    let n_up = segs(view, h, 10, 24, 6.0);
    let cone = surface(
        view,
        owner,
        "party:cone",
        |u, v| cone_at(u, v, 1.0),
        n_around,
        n_up,
        in_cone,
        true,
    );
    let a = rgb_of(base);
    let b: Rgb = [252.0, 246.0, 234.0];
    let (_, _, _, by1, bcx, _, _, _) = bounds_of(&[&cone]);
    mesh::contact(pixmap, body_clip, space, bcx, by1, r * 1.15, 0.04 * w, 0.3);
    let trim = ring_mesh(
        view,
        owner,
        "party:trim",
        place,
        r * 1.02,
        0.004 * w,
        0.016 * w,
        1.0,
        segs(view, TAU * r, 20, 40, 11.0),
        5,
    );
    let mut stripes: Vec<Proj> = Vec::new();
    let k = 6;
    let twist = 3.2;
    for i in 0..k {
        stripes.push(surface(
            view,
            owner,
            &format!("party:stripe{i}"),
            |s, t| {
                cone_at(
                    (i as f32 + 0.5 * t - twist * s * 0.97) / k as f32,
                    s * 0.97,
                    1.004,
                )
            },
            n_up + 3,
            1,
            in_cone,
            true,
        ));
    }
    let vis = mesh::paint_layers(
        pixmap,
        &[
            Layer::new(vec![cone], paper(view, b)),
            Layer::new(vec![trim], gold(view)),
        ],
        view.px,
        space,
        PaintOpts::default(),
    );
    mesh::paint_layers(
        pixmap,
        &[Layer::new(stripes, paper(view, a))],
        view.px,
        space,
        PaintOpts::default(),
    );
    mesh::grain_over(pixmap, &hull(&vis), 0.18, 0.1, space, None);
    yarn_ball(
        pixmap,
        view,
        owner,
        space,
        place([0.0, -h - 0.035 * w, 0.0]),
        0.06 * w,
        Rgba::from_rgb(244, 239, 230),
        37,
    );
}

fn draw_crown(
    pixmap: &mut Pixmap,
    view: View,
    owner: &str,
    m: &Marks,
    space: Affine,
    body_clip: Option<&Mask>,
) {
    let w = m.right - m.left;
    let head_w = (0.45 * w).max(m.top_r - m.top_l);
    let r = 0.44 * head_w + 0.07 * w;
    let band_h = 0.11 * w;
    let point_h = 0.15 * w;
    let ez = 0.9;
    let c: V3 = [50.0, seat(m, 0.42) + 0.05 * w, 0.0];
    let (roll, lean) = (0.05, -0.22);
    let place = |p: V3| add(c, tilt(p, roll, lean));
    let dir_of = |v: V3| norm(tilt(v, roll, lean));
    let on = |th: f32, rr: f32, y: f32| place([rr * th.cos(), y, rr * ez * th.sin()]);
    let axis = |y: f32| place([0.0, y, 0.0]);
    let nu = segs(view, TAU * r, 32, 64, 11.0);
    let band = surface(
        view,
        owner,
        "crown:band",
        |u, v| {
            on(
                u * TAU,
                r * (1.0 - 0.018 * (v * PI).sin()),
                -band_h * (1.0 - v),
            )
        },
        nu,
        2,
        move |_, v| axis(-band_h * (1.0 - v)),
        true,
    );
    let rim_t = 0.017 * w;
    let rim_top = ring_mesh(
        view,
        owner,
        "crown:rimTop",
        place,
        r + 0.004 * w,
        -band_h,
        rim_t,
        ez,
        nu,
        5,
    );
    let rim_bot = ring_mesh(
        view,
        owner,
        "crown:rimBot",
        place,
        r + 0.004 * w,
        0.0,
        rim_t * 1.15,
        ez,
        nu,
        5,
    );
    let cap_base = -band_h * 0.55;
    let cap_h = (0.12 * w).max(c[1] - m.top + 0.04 * w + cap_base);
    let p0 = PI * 0.5;
    let cap = surface(
        view,
        owner,
        "crown:cap",
        |u, v| {
            let th = u * TAU;
            let phi = v * PI * 0.5;
            let puff =
                0.08 * (2.0 * phi).sin() * (0.55 + 0.45 * (5.0 * (th - p0 - PI / 5.0)).cos());
            on(
                th,
                r * (0.9 * phi.sin() + puff),
                cap_base - cap_h * phi.cos(),
            )
        },
        segs(view, TAU * r, 24, 48, 11.0),
        segs(view, 1.4 * cap_h, 8, 20, 6.0),
        move |_, _| axis(cap_base - cap_h * 0.4),
        true,
    );
    let mut points: Vec<Proj> = Vec::new();
    let mut buds: Vec<Proj> = Vec::new();
    let mut pearls: Vec<V3> = Vec::new();
    let mut bud_tops: Vec<V3> = Vec::new();
    #[allow(clippy::too_many_arguments)]
    fn plate(
        view: View,
        owner: &str,
        key: &str,
        on: &dyn Fn(f32, f32, f32) -> V3,
        r: f32,
        w: f32,
        band_h: f32,
        a0: f32,
        half_w: f32,
        h: f32,
        ridge: f32,
        flare: f32,
        out: &mut Vec<Proj>,
    ) {
        out.push(surface(
            view,
            owner,
            key,
            |u, v| {
                let s = u * 2.0 - 1.0;
                let th = a0 + s * half_w * (1.0 - v).powf(1.3);
                let rr = r + 0.004 * w + ridge * (1.0 - s.abs()) * (1.0 - 0.5 * v) + flare * v * v;
                on(th, rr, -band_h - h * v)
            },
            2,
            segs(view, h, 3, 5, 11.0),
            move |_, v| on(a0, r * 0.5, -band_h - h * v),
            false,
        ));
    }
    for i in 0..5 {
        let a0 = p0 + (i as f32 / 5.0) * TAU;
        plate(
            view,
            owner,
            &format!("crown:point{i}"),
            &on,
            r,
            w,
            band_h,
            a0,
            (TAU / 5.0) * 0.34,
            point_h,
            0.05 * w,
            0.03 * w,
            &mut points,
        );
        pearls.push(on(
            a0,
            r + 0.004 * w + 0.03 * w + 0.02 * w,
            -band_h - point_h - 0.018 * w,
        ));
        let b0 = a0 + PI / 5.0;
        plate(
            view,
            owner,
            &format!("crown:bud{i}"),
            &on,
            r,
            w,
            band_h,
            b0,
            (TAU / 5.0) * 0.12,
            point_h * 0.36,
            0.02 * w,
            0.01 * w,
            &mut buds,
        );
        bud_tops.push(on(
            b0,
            r + 0.004 * w + 0.012 * w,
            -band_h - point_h * 0.36 - 0.012 * w,
        ));
    }
    let (_, _, _, by1, bcx, _, bw, _) = bounds_of(&[&band]);
    mesh::contact(
        pixmap,
        body_clip,
        space,
        bcx,
        by1,
        bw * 0.5,
        0.045 * w,
        0.34,
    );
    let mid = view.depth(axis(cap_base));
    let g = gold(view);
    let facet = metal(
        view,
        [255.0, 196.0, 80.0],
        [92.0, 54.0, 10.0],
        [255.0, 222.0, 150.0],
    );
    let gold_both: Shade =
        Rc::new(move |n: V3| facet(if n[2] < 0.0 { [-n[0], -n[1], -n[2]] } else { n }));
    let pearl_r = 0.024 * w;
    let bud_r = 0.019 * w;
    for &p in &pearls {
        if view.depth(p) < mid {
            sphere_sprite(pixmap, view, space, p, pearl_r, mesh::SPRITE_PEARL);
        }
    }
    for &p in &bud_tops {
        if view.depth(p) < mid {
            sphere_sprite(pixmap, view, space, p, bud_r, mesh::SPRITE_GOLD);
        }
    }
    mesh::paint_layers(
        pixmap,
        &[
            Layer::new(vec![cap], velvet(view, [150.0, 22.0, 44.0])),
            Layer {
                bias: 0.01 * w,
                ..Layer::new(vec![band], Rc::clone(&g))
            },
            Layer {
                bias: 0.012 * w,
                ..Layer::new(vec![rim_top], Rc::clone(&g))
            },
            Layer {
                bias: 0.012 * w,
                ..Layer::new(vec![rim_bot], Rc::clone(&g))
            },
            Layer {
                cull: false,
                flat: true,
                ..Layer::new(points, Rc::clone(&gold_both))
            },
            Layer {
                cull: false,
                flat: true,
                ..Layer::new(buds, Rc::clone(&gold_both))
            },
        ],
        view.px,
        space,
        PaintOpts::default(),
    );
    let stones: [Rgb; 5] = [
        [208.0, 20.0, 60.0],
        [30.0, 150.0, 92.0],
        [36.0, 90.0, 220.0],
        [36.0, 90.0, 220.0],
        [30.0, 150.0, 92.0],
    ];
    for (i, &stone) in stones.iter().enumerate() {
        for (th, stud) in [
            (p0 + (i as f32 / 5.0) * TAU, false),
            (p0 + (i as f32 / 5.0) * TAU + PI / 5.0, true),
        ] {
            let n = dir_of([th.cos(), 0.0, ez * th.sin()]);
            if view.dir(n)[2] < 0.12 {
                continue;
            }
            let c = on(th, r + 0.01 * w, -band_h * 0.5);
            if stud {
                sphere_sprite(pixmap, view, space, c, 0.013 * w, mesh::SPRITE_GOLD);
                continue;
            }
            let size = if i == 0 { 0.034 } else { 0.027 } * w;
            sphere_sprite(pixmap, view, space, c, size * 1.3, mesh::SPRITE_GOLD);
            let t = dir_of([-th.sin(), 0.0, ez * th.cos()]);
            cut_gem(
                pixmap,
                view,
                space,
                along(c, n, 0.006 * w),
                n,
                t,
                size,
                stone,
            );
        }
    }
    for &p in &pearls {
        if view.depth(p) >= mid {
            sphere_sprite(pixmap, view, space, p, pearl_r, mesh::SPRITE_PEARL);
        }
    }
    for &p in &bud_tops {
        if view.depth(p) >= mid {
            sphere_sprite(pixmap, view, space, p, bud_r, mesh::SPRITE_GOLD);
        }
    }
    let orb_r = 0.034 * w;
    let orb = axis(cap_base - cap_h - orb_r * 0.75);
    let up = dir_of([0.0, -1.0, 0.0]);
    let bar = |key: &str, a: V3, b: V3| -> Proj {
        tube(
            view,
            owner,
            key,
            |u| along(a, sub(b, a), u),
            dir_of([0.0, 0.0, 1.0]),
            section(0.009 * w, 0.009 * w, 0.5),
            2,
            6,
            false,
        )
    };
    let c0 = along(orb, up, orb_r * 0.8);
    let c1 = along(orb, up, orb_r * 0.8 + 0.075 * w);
    let cm = along(orb, up, orb_r * 0.8 + 0.05 * w);
    let side = dir_of([1.0, 0.0, 0.0]);
    sphere_sprite(pixmap, view, space, orb, orb_r, mesh::SPRITE_GOLD);
    mesh::paint_layers(
        pixmap,
        &[
            Layer::new(vec![bar("crown:barV", c0, c1)], Rc::clone(&g)),
            Layer::new(
                vec![bar(
                    "crown:barH",
                    along(cm, side, -0.026 * w),
                    along(cm, side, 0.026 * w),
                )],
                Rc::clone(&g),
            ),
        ],
        view.px,
        space,
        PaintOpts::default(),
    );
}

fn bounds_of(ps: &[&Proj]) -> (f32, f32, f32, f32, f32, f32, f32, f32) {
    let (x0, y0, x1, y1) = mesh::bounds_of(ps);
    (
        x0,
        y0,
        x1,
        y1,
        (x0 + x1) * 0.5,
        (y0 + y1) * 0.5,
        x1 - x0,
        y1 - y0,
    )
}

/* ── headphones ─────────────────────────────────────────────────────── */

struct Phones {
    layers: Vec<Layer>,
    pivots: Vec<V3>,
    pivot_r: f32,
    stitch: Vec<V3>,
    seats: Vec<(V3, f32)>,
}

fn phones_of(view: View, owner: &str, m: &Marks, base: Rgba) -> Phones {
    let w = m.right - m.left;
    let rc = 0.15 * w;
    let oval = 1.1;
    let pad = 0.05 * w;
    let tc = 0.068 * w;
    let yc = m.mid - 0.07 * w;
    let tilt_a: f32 = 0.5;
    let shell_rgb = rgb_of(base);
    let pad_rgb = rgb_shade(base, -0.12, 0.0);
    let plate_rgb = rgb_shade(base, 0.08, 0.0);
    let nu = segs(view, TAU * rc, 20, 40, 11.0);
    let mut layers: Vec<Layer> = Vec::new();
    let mut pivots: Vec<V3> = Vec::new();
    let mut tops: Vec<V3> = Vec::new();
    let mut seats: Vec<(V3, f32)> = Vec::new();
    let g = silver(view);
    for s in [-1.0f32, 1.0] {
        let a: V3 = [s * tilt_a.cos(), 0.0, tilt_a.sin()];
        let e1: V3 = [0.0, 1.0, 0.0];
        let e2: V3 = [-tilt_a.sin(), 0.0, s * tilt_a.cos()];
        let o: V3 = [
            (if s < 0.0 { m.left } else { m.right }) - s * 0.02 * w,
            yc,
            0.01 * w,
        ];
        let at = |t: f32, rho: f32, th: f32| -> V3 {
            let c = th.cos() * oval * rho;
            let sn = th.sin() * rho;
            [
                o[0] + a[0] * t + e1[0] * c + e2[0] * sn,
                o[1] + a[1] * t + e1[1] * c + e2[1] * sn,
                o[2] + a[2] * t + e1[2] * c + e2[2] * sn,
            ]
        };
        let axis = |t: f32| along(o, a, t);
        seats.push((o, rc * oval));
        let rho0 = rc * 0.9 - pad * 0.5;
        let creases = if nu >= 48 { 1.0 } else { 0.0 };
        let cushion = surface(
            view,
            owner,
            &format!("phones:cushion{s}"),
            |u, v| {
                let th = u * TAU;
                let al = v * PI;
                let crease = 1.0 + 0.006 * creases * (th * 24.0).sin() * al.sin();
                at(
                    pad * 0.5 * (1.0 - al.cos()),
                    (rho0 + pad * 0.62 * al.sin()) * crease,
                    th,
                )
            },
            nu,
            5,
            move |_, v| axis(pad * 0.5 * (1.0 - (v * PI).cos())),
            true,
        );
        let shell = surface(
            view,
            owner,
            &format!("phones:shell{s}"),
            |u, v| {
                let al = v * PI * 0.5;
                at(
                    pad + tc * al.sin().powf(0.55),
                    rc * (0.7 + 0.3 * al.cos().powf(0.55)),
                    u * TAU,
                )
            },
            nu,
            5,
            move |_, _| axis(pad + tc * 0.5),
            true,
        );
        let ring_r = 0.66 * rc;
        let ring_t = 0.017 * w;
        let ring = surface(
            view,
            owner,
            &format!("phones:ring{s}"),
            |u, v| {
                at(
                    pad + tc + ring_t * 0.45 * (v * TAU).sin(),
                    ring_r + ring_t * (v * TAU).cos(),
                    u * TAU,
                )
            },
            nu,
            5,
            move |u, _| at(pad + tc, ring_r, u * TAU),
            true,
        );
        let plate_r = ring_r - ring_t * 0.5;
        let plate = surface(
            view,
            owner,
            &format!("phones:plate{s}"),
            |u, v| at(pad + tc + 0.013 * w * (1.0 - v * v), plate_r * v, u * TAU),
            nu,
            3,
            move |_, _| axis(pad + tc - 0.02 * w),
            true,
        );
        let ty = pad + tc * 0.45;
        let ry = rc * 1.04 + 0.022 * w;
        let yoke_at = |u: f32| -> V3 {
            let al = u * PI;
            at(ty, ry, al + PI * 0.5)
        };
        let yoke = tube(
            view,
            owner,
            &format!("phones:yoke{s}"),
            yoke_at,
            a,
            section(0.013 * w, 0.008 * w, 0.6),
            segs(view, PI * ry, 12, 24, 11.0),
            6,
            false,
        );
        pivots.push(yoke_at(0.0));
        pivots.push(yoke_at(1.0));
        tops.push(yoke_at(0.5));
        layers.push(Layer::new(vec![cushion], plastic(view, pad_rgb, 0.35)));
        layers.push(Layer::new(vec![shell], plastic(view, shell_rgb, 0.8)));
        layers.push(Layer {
            bias: 0.02 * w,
            ..Layer::new(vec![ring], Rc::clone(&g))
        });
        layers.push(Layer {
            bias: 0.01 * w,
            ..Layer::new(vec![plate], plastic(view, plate_rgb, 1.3))
        });
        layers.push(Layer::new(vec![yoke], Rc::clone(&g)));
    }
    let (pl, pr) = (tops[0], tops[1]);
    let zb = (pl[2] + pr[2]) * 0.5;
    let cx = (pl[0] + pr[0]) * 0.5;
    let ax = (pr[0] - pl[0]) * 0.5;
    let y_end = (pl[1] + pr[1]) * 0.5;
    let pad_t = |u: f32| 0.005 * w + 0.022 * w * (PI * u).sin().powf(0.35);
    let e = 2.0 / 2.2;
    let top_at = |x: f32| -> f32 {
        let mut y = f32::INFINITY;
        let h = &m.hull;
        for i in 0..h.len() {
            let p = h[i];
            let q = h[(i + 1) % h.len()];
            if (p[0] - q[0]).abs() < 1e-9 || x < p[0].min(q[0]) || x > p[0].max(q[0]) {
                continue;
            }
            y = y.min(p[1] + (q[1] - p[1]) * (x - p[0]) / (q[0] - p[0]));
        }
        y
    };
    let mut lift = y_end - m.top;
    for i in 1..64 {
        let t = PI * (1.0 - i as f32 / 64.0);
        let s = t.sin().abs().powf(e);
        let yt = top_at(cx + ax * spow(t.cos(), e));
        if s > 0.05 && yt.is_finite() {
            lift = lift.max((y_end - yt + 2.0 * pad_t(i as f32 / 64.0) + 0.006 * w) / s);
        }
    }
    let arch = |u: f32| -> V3 {
        let t = PI * (1.0 - u);
        [
            cx + ax * spow(t.cos(), e),
            y_end - lift * t.sin().abs().powf(e),
            zb,
        ]
    };
    let part = |u0: f32, u1: f32| move |u: f32| arch(u0 + (u1 - u0) * u);
    let z: V3 = [0.0, 0.0, 1.0];
    let span = ax * 2.0 + lift * 2.0;
    let nb = segs(view, span, 24, 48, 11.0);
    let us = 0.09;
    let shell_off = 0.016 * w;
    let shell_path = offset_path(part(us, 1.0 - us), z, move |_| shell_off);
    let taper = |u: f32| smooth01(u.min(1.0 - u) / 0.07);
    let band_shell = tube(
        view,
        owner,
        "phones:bandShell",
        &shell_path,
        z,
        move |u, ph| {
            let k = taper(u);
            [
                (0.024 + 0.016 * k) * w * spow(ph.cos(), 0.4),
                (0.008 + 0.008 * k) * w * spow(ph.sin(), 0.4),
            ]
        },
        nb,
        6,
        false,
    );
    let pad0 = us + 0.02;
    let pad1 = 1.0 - us - 0.02;
    let cushion_path = offset_path(part(pad0, pad1), z, move |u| {
        -pad_t(pad0 + (pad1 - pad0) * u)
    });
    let band_pad = tube(
        view,
        owner,
        "phones:bandPad",
        &cushion_path,
        z,
        move |u, ph| {
            [
                0.034 * w * spow(ph.cos(), 0.5),
                pad_t(pad0 + (pad1 - pad0) * u) * spow(ph.sin(), 0.7),
            ]
        },
        nb,
        6,
        false,
    );
    let sliders: Vec<Proj> = [part(0.0, us + 0.03), part(1.0 - us - 0.03, 1.0)]
        .into_iter()
        .enumerate()
        .map(|(i, p)| {
            tube(
                view,
                owner,
                &format!("phones:slider{i}"),
                p,
                z,
                section(0.022 * w, 0.006 * w, 0.4),
                4,
                6,
                false,
            )
        })
        .collect();
    layers.push(Layer::new(vec![band_shell], plastic(view, shell_rgb, 0.9)));
    layers.push(Layer::new(vec![band_pad], plastic(view, pad_rgb, 0.3)));
    layers.push(Layer::new(sliders, Rc::clone(&g)));
    let mut stitch = Vec::new();
    for i in 0..=48 {
        stitch.push(along(
            cushion_path(0.04 + (0.92 * i as f32) / 48.0),
            z,
            0.034 * w + 0.001 * w,
        ));
    }
    Phones {
        layers,
        pivots,
        pivot_r: 0.017 * w,
        stitch,
        seats,
    }
}

fn draw_phones(
    pixmap: &mut Pixmap,
    view: View,
    owner: &str,
    m: &Marks,
    base: Rgba,
    space: Affine,
    pass: bool,
    body_clip: Option<&Mask>,
) {
    let ph = phones_of(view, owner, m, base);
    let w = m.right - m.left;
    let near = move |d: f32| if pass { d >= 0.0 } else { d < 0.0 };
    if pass {
        for &(sc, ry) in &ph.seats {
            if view.depth(sc) < 0.0 {
                continue;
            }
            let sc = view.to_screen(sc);
            mesh::contact(
                pixmap,
                body_clip,
                space,
                sc[0],
                sc[1] + 0.01 * w,
                0.06 * w,
                ry * 1.05,
                0.26,
            );
        }
    }
    let layers: Vec<Layer> = ph
        .layers
        .into_iter()
        .map(|mut l| {
            l.keep = Some(Rc::new(near));
            l
        })
        .collect();
    mesh::paint_layers(pixmap, &layers, view.px, space, PaintOpts::default());
    for &p in &ph.pivots {
        if near(view.depth(p)) {
            sphere_sprite(pixmap, view, space, p, ph.pivot_r, mesh::SPRITE_SILVER);
        }
    }
    if pass && w * view.px > 120.0 && view.dir([0.0, 0.0, 1.0])[2] > 0.3 {
        let pts: Vec<P2> = ph.stitch.iter().map(|&p| view.to_screen(p)).collect();
        let mut b = PathBuilder::new();
        for (i, p) in pts.iter().enumerate() {
            if i == 0 {
                b.move_to(p[0], p[1]);
            } else {
                b.line_to(p[0], p[1]);
            }
        }
        if let Some(path) = b.finish() {
            let stroke = tiny_skia::Stroke {
                width: 0.0045 * w,
                line_cap: tiny_skia::LineCap::Round,
                dash: tiny_skia::StrokeDash::new(vec![0.022 * w, 0.014 * w], 0.0),
                ..tiny_skia::Stroke::default()
            };
            pixmap.stroke_path(
                &path,
                &paint_alpha(shade(base, 0.12, 0.0), 0.55),
                &stroke,
                space.transform(),
                None,
            );
        }
    }
}

/* ── the bow tie ────────────────────────────────────────────────────── */

#[allow(clippy::too_many_arguments)]
fn draw_bow_tie(
    pixmap: &mut Pixmap,
    view: View,
    r: WearRig,
    owner: &str,
    m: &Marks,
    base: Rgba,
    body_z: &dyn Fn(f32, f32) -> f32,
    body_clip: Option<&Mask>,
    face: (f32, f32),
    space: Affine,
    fabric_bow: bool,
) {
    if r.facing < 0.05 {
        return;
    }
    let w = m.right - m.left;
    let clear = face.0 + 22.0 * face.1;
    let cy_b = clear.max(m.bottom - 0.15 * w);
    let room = (m.bottom - 3.0 - cy_b).max(0.0);
    let k = w.min(100.0) * (room / 15.0).clamp(0.5, 1.0);
    let z_front = body_z(50.0, cy_b);
    let q = |x: f32, y: f32, z: f32| -> V3 { [50.0 + x * k, cy_b + y * k, z_front + z * k] };
    let rgb = rgb_of(base);
    let corners: Vec<P2> = [
        q(-0.36, -0.18, 0.0),
        q(0.36, 0.18, 0.0),
        q(-0.36, 0.18, 0.0),
        q(0.36, -0.18, 0.0),
    ]
    .iter()
    .map(|&p| view.to_screen(p))
    .collect();
    let (_, _, _, _, _, _, bw, bh) = bounds(&corners);
    let sh = view.to_screen(q(0.01, 0.08, 0.0));
    mesh::contact(
        pixmap,
        body_clip,
        space,
        sh[0],
        sh[1],
        bw * 0.52,
        bh * 0.5,
        0.3,
    );
    let nu = segs(view, 0.32 * k, 8, 16, 11.0);
    let nv = segs(view, 0.3 * k, 8, 16, 6.0);
    let soft = fabric_bow;
    let wing_shade = if soft {
        felt(view, rgb, 1.0)
    } else {
        satin(view, rgb)
    };
    let mut layers: Vec<Layer> = Vec::new();
    for dir in [-1.0f32, 1.0] {
        let wing = move |u: f32, v: f32| -> V3 {
            let x = 0.058 + 0.3 * u - 0.035 * u.powi(4) * (PI * v).sin();
            let h = 0.05 + 0.12 * u.powf(0.8);
            let y = -h * (PI * v).cos() + 0.012 * u * u;
            let bulge = 0.018 + 0.058 * (PI * (0.12 + 0.88 * u)).sin();
            let fold = 0.016 * (3.0 * PI * v).sin() * (1.0 - u).powf(1.3);
            q(dir * x, y, (bulge + fold) * (PI * v).sin() + 0.004)
        };
        layers.push(Layer::new(
            vec![surface(
                view,
                owner,
                &format!("bow:wing{dir}"),
                wing,
                nu,
                nv,
                move |u, _| q(dir * (0.058 + 0.3 * u), 0.0, -0.03),
                true,
            )],
            Rc::clone(&wing_shade),
        ));
        layers.push(Layer::new(
            vec![surface(
                view,
                owner,
                &format!("bow:end{dir}"),
                |s, t| {
                    let p = wing(1.0, t);
                    [p[0], p[1], p[2] - (p[2] - z_front) * s]
                },
                1,
                nv,
                move |_, _| q(dir * 0.2, 0.0, -0.03),
                true,
            )],
            Rc::clone(&wing_shade),
        ));
    }
    let knot = surface(
        view,
        owner,
        "bow:knot",
        |u, v| {
            let x = 0.068 * (2.0 * u - 1.0) * (1.0 - 0.12 * (PI * v).sin());
            let y = 0.075 * (2.0 * v - 1.0);
            let zx = (1.0 - (2.0 * u - 1.0).abs().powi(3)).max(0.0).sqrt();
            let zy = (1.0 - (2.0 * v - 1.0).abs().powi(5)).max(0.0).powf(0.4);
            q(x, y, 0.012 + 0.075 * zx * zy)
        },
        6,
        6,
        |_, _| q(0.0, 0.0, -0.02),
        true,
    );
    layers.push(Layer::new(
        vec![knot],
        if soft {
            felt(view, rgb, 0.9)
        } else {
            satin(view, scale(rgb, 0.9))
        },
    ));
    if !soft {
        mesh::paint_layers(pixmap, &layers, view.px, space, PaintOpts::default());
        return;
    }
    for i in (0..layers.len()).step_by(2) {
        let part: Vec<&Layer> = layers[i..(i + 2).min(layers.len())].iter().collect();
        let vis = mesh::paint_layers(
            pixmap,
            &layers[i..(i + 2).min(layers.len())],
            view.px,
            space,
            PaintOpts::default(),
        );
        let _ = part;
        let outline = hull(&vis);
        mesh::grain_over(pixmap, &outline, 0.5, 0.1, space, None);
        mesh::fuzz(
            pixmap,
            &outline,
            rgb,
            41 + i as i32,
            0.3,
            view.px,
            space,
            None,
        );
    }
}

/* ── glasses: in face space, on the eyes ────────────────────────────── */

struct Frame {
    rx: f32,
    ry: f32,
    n: f32,
    tr: f32,
    td: f32,
}

fn frame_of(kind: BotAvatarGlasses) -> Frame {
    match kind {
        BotAvatarGlasses::Round => Frame {
            rx: 10.8,
            ry: 10.8,
            n: 2.0,
            tr: 1.15,
            td: 1.2,
        },
        BotAvatarGlasses::Square => Frame {
            rx: 12.0,
            ry: 9.6,
            n: 4.2,
            tr: 1.45,
            td: 1.4,
        },
        BotAvatarGlasses::Shades => Frame {
            rx: 12.6,
            ry: 10.0,
            n: 3.4,
            tr: 1.6,
            td: 1.5,
        },
        BotAvatarGlasses::None => Frame {
            rx: 10.0,
            ry: 10.0,
            n: 2.0,
            tr: 1.0,
            td: 1.0,
        },
    }
}

/// Glasses over two eye spots, turned rigidly with the head.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_glasses(
    pixmap: &mut Pixmap,
    kind: BotAvatarGlasses,
    eyes: [EyeSpot; 2],
    lx: f32,
    ly: f32,
    pass: bool,
    yaw: f32,
    pitch: f32,
    space: Affine,
    face_clip: Option<&Mask>,
) {
    if kind == BotAvatarGlasses::None {
        return;
    }
    let f = frame_of(kind);
    // `sin_cos` gives (sin, cos); the matrix below wants cos first.
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    let (l, h) = light_of(lx, ly);
    let view = make_view(
        [cy, 0.0, sy, sp * sy, cp, -sp * cy, -cp * sy, sp, cp * cy],
        [0.0, 0.0, 0.0],
        l,
        h,
        px_of(space),
    );
    let z_plane = eyes[0].fz.max(eyes[1].fz) + 3.2;
    let z_at = |x: f32| z_plane - 0.0045 * x * x;
    let e2 = 2.0 / f.n;
    let rim_at = |e: EyeSpot, grow: f32| {
        move |u: f32| -> V3 {
            let t = u * TAU;
            let x = e.ox + (f.rx + grow) * spow(t.cos(), e2);
            let y = e.oy + (f.ry + grow) * spow(t.sin(), e2);
            [x, y, z_at(x)]
        }
    };
    let ring = |e: EyeSpot, grow: f32, n: usize| -> Vec<P2> {
        let fun = rim_at(e, grow);
        (0..n)
            .map(|i| view.to_screen(fun(i as f32 / n as f32)))
            .collect()
    };
    let vis = [
        ((eyes[0].z - 0.18) * 3.2).clamp(0.0, 1.0),
        ((eyes[1].z - 0.18) * 3.2).clamp(0.0, 1.0),
    ];
    if !pass {
        for i in 0..2 {
            if vis[i] <= 0.0 {
                continue;
            }
            let pts = ring(eyes[i], 0.0, 48);
            let mut moved = Vec::with_capacity(pts.len());
            for p in pts {
                moved.push([p[0] - lx * 1.6, p[1] - ly * 1.6 + 0.8]);
            }
            if let Some(path) = poly_path(&moved) {
                let stroke = tiny_skia::Stroke {
                    width: f.tr * 2.0 + 0.6,
                    line_join: tiny_skia::LineJoin::Round,
                    ..tiny_skia::Stroke::default()
                };
                pixmap.stroke_path(
                    &path,
                    &paint_alpha(Rgba::from_rgb(0, 0, 0), 0.22 * vis[i]),
                    &stroke,
                    space.transform(),
                    face_clip,
                );
            }
        }
        return;
    }
    let frame_rgb: Rgb = [22.0, 22.0, 26.0];
    let rim_shade = plastic(view, frame_rgb, 1.1);
    for i in 0..2 {
        if vis[i] <= 0.0 {
            continue;
        }
        let lens_pts = ring(eyes[i], -f.tr * 0.5, 48);
        let Some(lens) = poly_path(&lens_pts) else {
            continue;
        };
        let b = {
            let pts = ring(eyes[i], 0.0, 16);
            bounds(&pts)
        };
        let (x0, y0, x1, y1, cx, _, w, h2) = b;
        let _ = (x0, x1);
        let stops = if kind == BotAvatarGlasses::Shades {
            vec![
                (0.0, Rgba::from_rgb(58, 60, 78).with_alpha(0.97)),
                (0.55, Rgba::from_rgb(18, 18, 26).with_alpha(0.98)),
                (1.0, Rgba::from_rgb(30, 26, 44).with_alpha(0.98)),
            ]
        } else {
            vec![
                (0.0, Rgba::from_rgb(236, 244, 255).with_alpha(0.26)),
                (0.45, Rgba::from_rgb(214, 230, 255).with_alpha(0.08)),
                (1.0, Rgba::from_rgb(214, 230, 255).with_alpha(0.14)),
            ]
        };
        let stops: Vec<(f32, Rgba)> = stops
            .into_iter()
            .map(|(t, c)| (t, c.with_alpha(f32::from(c.a) / 255.0 * vis[i])))
            .collect();
        let mut lp = mesh::linear_paint([cx, y0], [cx, y1], &stops);
        lp.anti_alias = true;
        let local_mask = {
            let mut m = Mask::new(pixmap.width(), pixmap.height()).expect("non-zero mask");
            m.fill_path(&lens, FillRule::Winding, true, space.transform());
            m
        };
        let _ = h2;
        pixmap.fill_path(&lens, &lp, FillRule::Winding, space.transform(), None);
        let mut streak = |x0f: f32, width: f32, a: f32, mask: &Mask| {
            let mut p = PathBuilder::new();
            p.move_to(x0 + w * x0f, y1 + h2 * 0.1);
            p.line_to(x0 + w * (x0f + 0.55), y0 - h2 * 0.1);
            p.line_to(x0 + w * (x0f + 0.55 + width), y0 - h2 * 0.1);
            p.line_to(x0 + w * (x0f + width), y1 + h2 * 0.1);
            p.close();
            if let Some(path) = p.finish() {
                let mut paint = paint(Rgba::from_rgb(255, 255, 255).with_alpha(vis[i] * a));
                paint.anti_alias = true;
                pixmap.fill_path(
                    &path,
                    &paint,
                    FillRule::Winding,
                    space.transform(),
                    Some(mask),
                );
            }
        };
        let k_streak = if kind == BotAvatarGlasses::Shades {
            1.4
        } else {
            1.0
        };
        streak(-0.28, 0.26, 0.16 * k_streak, &local_mask);
        streak(0.06, 0.07, 0.3 * k_streak, &local_mask);
    }
    let nr = segs(view, TAU * f.rx * 1.1, 28, 56, 8.0);
    let rim = |e: EyeSpot| -> Proj {
        tube(
            view,
            "frames",
            &format!("rim:{:?}:{:.2},{:.2},{:.2}", kind, e.ox, e.oy, e.fz),
            rim_at(e, 0.0),
            [0.0, 0.0, 1.0],
            section(f.td, f.tr, 0.55),
            nr,
            6,
            true,
        )
    };
    let (left, right) = (eyes[0], eyes[1]);
    let round = kind == BotAvatarGlasses::Round;
    let bridge_at = |u: f32| -> V3 {
        let a: V3 = [
            left.ox + f.rx * if round { 0.9 } else { 0.98 },
            left.oy - f.ry * if round { 0.32 } else { 0.42 },
            0.0,
        ];
        let b: V3 = [
            right.ox - f.rx * if round { 0.9 } else { 0.98 },
            right.oy - f.ry * if round { 0.32 } else { 0.42 },
            0.0,
        ];
        let c: V3 = [
            0.0,
            (left.oy + right.oy) * 0.5 - f.ry * if round { 0.8 } else { 0.6 },
            0.0,
        ];
        let m1 = 1.0 - u;
        let x = m1 * m1 * a[0] + 2.0 * m1 * u * c[0] + u * u * b[0];
        let y = m1 * m1 * a[1] + 2.0 * m1 * u * c[1] + u * u * b[1];
        [x, y, z_at(x) + 0.3]
    };
    let bridge = tube(
        view,
        "frames",
        &format!("bridge:{:?}", kind),
        bridge_at,
        [0.0, 0.0, 1.0],
        section(f.td * 0.8, f.tr * 0.75, 0.7),
        8,
        6,
        false,
    );
    let mut parts = vec![
        (rim(left), vis[0], view.depth([left.ox, left.oy, z_plane])),
        (
            rim(right),
            vis[1],
            view.depth([right.ox, right.oy, z_plane]),
        ),
        (
            bridge,
            vis[0].min(vis[1]),
            view.depth([0.0, left.oy, z_plane]),
        ),
    ];
    parts.sort_by(|a, b| a.2.total_cmp(&b.2));
    for (part, a, _) in parts {
        if a <= 0.0 {
            continue;
        }
        mesh::paint_layers(
            pixmap,
            &[Layer::new(vec![part], Rc::clone(&rim_shade))],
            view.px,
            space,
            PaintOpts {
                mask: None,
                alpha: a,
            },
        );
    }
    for (e, s, a) in [(left, -1.0f32, vis[0]), (right, 1.0, vis[1])] {
        if a <= 0.0 {
            continue;
        }
        let x = e.ox + s * f.rx * 0.93;
        sphere_sprite(
            pixmap,
            view,
            space,
            [x, e.oy - f.ry * 0.5, z_at(x) + f.td],
            0.75,
            mesh::SPRITE_SILVER,
        );
    }
}

/* ── the pass entry points ──────────────────────────────────────────── */

/// Behind the body: what of the headphones is behind its middle plane.
pub(crate) fn draw_wear_behind(
    pixmap: &mut Pixmap,
    path: &Path,
    space: Affine,
    r: WearRig,
    wear: &Wear,
) {
    if !wear.headphones {
        return;
    }
    let Some(m) = marks_for(path) else {
        return;
    };
    let owner = owner_key(&m);
    draw_phones(
        pixmap,
        view_of(r, px_of(space)),
        &owner,
        &m,
        wear.color,
        space,
        false,
        None,
    );
}

/// Over the body: the hat, the headphones' near part, the bow tie.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_wear_front(
    pixmap: &mut Pixmap,
    path: &Path,
    space: Affine,
    r: WearRig,
    wear: &Wear,
    body_clip: Option<&Mask>,
    body_z: &dyn Fn(f32, f32) -> f32,
    face: (f32, f32),
) {
    let Some(m) = marks_for(path) else {
        return;
    };
    let owner = owner_key(&m);
    let view = view_of(r, px_of(space));
    if wear.bow_tie {
        draw_bow_tie(
            pixmap,
            view,
            r,
            &owner,
            &m,
            wear.color,
            body_z,
            body_clip,
            face,
            space,
            wear.fabric,
        );
    }
    if wear.headphones {
        draw_phones(pixmap, view, &owner, &m, wear.color, space, true, body_clip);
    }
    match wear.hat {
        BotAvatarHat::None => {}
        BotAvatarHat::Beret => draw_beret(pixmap, view, &owner, &m, wear.color, space, body_clip),
        BotAvatarHat::Beanie => draw_beanie(pixmap, view, &owner, &m, wear.color, space, body_clip),
        BotAvatarHat::Party => draw_party(pixmap, view, &owner, &m, wear.color, space, body_clip),
        BotAvatarHat::Crown => draw_crown(pixmap, view, &owner, &m, space, body_clip),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::bot_avatar::data::{bot_avatar_preset, bot_avatar_shape};
    use crate::widgets::bot_avatar::path::parse_svg_path;
    use crate::widgets::bot_avatar::types::BotAvatarType;

    fn shape() -> Path {
        parse_svg_path(bot_avatar_shape(BotAvatarType::Clover)).unwrap()
    }

    #[test]
    fn marks_are_stable_and_bounded() {
        let path = shape();
        let a = marks_for(&path).unwrap();
        let b = marks_for(&path).unwrap();
        assert!(Arc::ptr_eq(&a, &b));
        assert!(a.left < a.right);
        assert!(a.top < a.bottom);
        assert!(a.widths.iter().any(|&w| w > 0.0));
    }

    #[test]
    fn a_hat_renders_over_the_body() {
        let path = shape();
        let mut pix = Pixmap::new(128, 128).unwrap();
        let m = marks_for(&path).unwrap();
        let rig = WearRig {
            cy: 1.0,
            sy: 0.0,
            cp: 1.0,
            sp: 0.0,
            facing: 1.0,
            lx: -0.5,
            ly: -0.86,
        };
        // The view projects around the body's centre, so a standalone
        // accessory needs the same 50-unit offset the body transform carries.
        let space = Affine::translate(50.0, 50.0);
        let view = view_of(rig, 1.0);
        let base = bot_avatar_preset(BotAvatarType::Clover).color;
        draw_beret(
            &mut pix,
            view,
            &owner_key(&m),
            &m,
            Rgba::parse(base).unwrap(),
            space,
            None,
        );
        let painted = pix.pixels().iter().filter(|p| p.alpha() > 0).count();
        assert!(painted > 100, "{painted}");
    }
}
