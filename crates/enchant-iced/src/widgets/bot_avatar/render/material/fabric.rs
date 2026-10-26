//! Fabric: a plush pile over the same form.
//!
//! Ported from the fabric half of `plastic.ts`. The form is dressed in fibres
//! combed from a parting behind the top of the head, gathered into locks and
//! tufts, lit by its own key and back light, with a haze of hairs breaking the
//! silhouette. The pile bakes once per form, resolution, light direction and
//! style, and is kept in a process-wide cache; the draw lays the slices, the
//! front, the halo and the pile's film over them.

#![allow(clippy::too_many_arguments)] // signatures follow the reference pile passes

use std::collections::HashMap;
use std::f32::consts::PI;
use std::sync::{Arc, Mutex, OnceLock};

use tiny_skia::{
    BlendMode, FillRule, FilterQuality, LineCap, LineJoin, Mask, Path, PathBuilder, Pixmap,
    PixmapPaint, Stroke, Transform,
};

use super::super::super::canvas::Affine;
use super::super::super::color::Rgba;
use super::super::palette::Palette;
use super::super::{FurStyle, RenderConfig};
use super::form::{self, Form, PAD, Relief, Rig, SPAN, cap_frame, edge_lift, relief_for};
use super::matcap::{Material, build_fabric_matcap, shade_texels};
use super::plastic::{tier_for, to_pixmap};

/// The pile's neutral grey: below it the film darkens, above it lightens.
const FUR_MID: f32 = 128.0;
const LOCK_COVER: f32 = 3.2;
const LOCK_DENSE: f32 = 1.0 / 2.56;
// Kept at the reference's precision; this is not an f32 round number.
#[allow(clippy::excessive_precision)]
const LOCK_TAPER: f32 = 2.598_076_2;
const LOCK_PART: f32 = 0.6;
const PILE_AMB: f32 = 0.07;
const PILE_FILL: f32 = 0.14;
const PILE_WRAP: f32 = 0.3;
const PILE_SHADOW: f32 = 0.6;
const PILE_AO: f32 = 0.35;
const PILE_ROOT: f32 = 0.3;
const PILE_KK: f32 = 0.3;
const PILE_GAIN: f32 = 1.26;
const PILE_LIFT: f32 = 1.6;
const PILE_BIAS: f32 = 0.045;
const PILE_COS: f32 = 0.877_582_6;
const PILE_SIN: f32 = 0.479_425_54;
const PILE_CREASE: f32 = 0.22;
const PILE_H: f32 = 0.5;
const GAP_REF: f32 = 0.36;
const PILE_LIT: f32 = 1.1;
const TIP_TOP: f32 = 1.625;
const STRAND_TAPER: f32 = 0.5;
const STRAND_RISE: f32 = 0.9;
const STRAND_SHORT: f32 = 0.7;
const STRAND_CAST_H: f32 = 0.15;
const STRAND_CAST: f32 = 1.3;
const FILM_M: f32 = 20.0;
const FILM_SPAN: f32 = SPAN + 2.0 * FILM_M;
const HALO_A: f32 = 0.8;
const HALO_STREAK: f32 = 0.3;
const HALO_HAIR: f32 = 0.8;
const HALO_LIFT: f32 = 0.7;
const HALO_SHADE: f32 = 0.5;
const HALO_FIT: f32 = 2.5;
const HALO_CORE: f32 = 1.2;
const HALO_INSET: f32 = 3.0;
const HALO_VEIL: f32 = 0.8;
const HALO_BLEND: f32 = 0.3;
const KEY_EL: f32 = 32.0 * PI / 180.0;
const MISS_N: usize = 96;
const MISS_STEP: f32 = 0.1;
const GAP_DEEP: f32 = 0.28;
const GAP_SCATTER: f32 = 0.42;
const TIP_TINT: f32 = 0.6;
const W_MEAN: f32 = 0.25;
const W_SD: f32 = 0.133_630_78;

fn smooth(a: f32, b: f32, v: f32) -> f32 {
    let t = if v <= a {
        0.0
    } else if v >= b {
        1.0
    } else {
        (v - a) / (b - a)
    };
    t * t * (3.0 - 2.0 * t)
}

fn norm3(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l <= 1e-8 {
        [0.0, 0.0, 1.0]
    } else {
        [v[0] / l, v[1] / l, v[2] / l]
    }
}

fn bilerp(f: &[f32], n: usize, x: f32, y: f32) -> f32 {
    let last = (n - 1) as f32;
    let fx = x.clamp(0.0, last);
    let fy = y.clamp(0.0, last);
    let x0 = (fx as usize).min(n - 2);
    let y0 = (fy as usize).min(n - 2);
    let tx = fx - x0 as f32;
    let ty = fy - y0 as f32;
    let i = y0 * n + x0;
    (f[i] * (1.0 - tx) + f[i + 1] * tx) * (1.0 - ty)
        + (f[i + n] * (1.0 - tx) + f[i + n + 1] * tx) * ty
}

fn hash2(x: i32, y: i32) -> f32 {
    let mut h = (x.wrapping_mul(374_761_393)).wrapping_add(y.wrapping_mul(668_265_263));
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    ((h ^ (h >> 16)) as u32) as f32 / 4_294_967_296.0
}

fn hash3(x: i32, y: i32, z: i32) -> f32 {
    let mut h = x
        .wrapping_mul(374_761_393)
        .wrapping_add(y.wrapping_mul(668_265_263))
        .wrapping_add(z.wrapping_mul(1_274_126_177));
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    ((h ^ (h >> 16)) as u32) as f32 / 4_294_967_296.0
}

struct Rng(u32);

impl Rng {
    fn new(seed: usize) -> Self {
        Self(seed as u32)
    }

    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x6d2b_79f5);
        let mut t = (self.0 ^ (self.0 >> 15)).wrapping_mul(1 | self.0);
        t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t));
        (t ^ (t >> 14)) as f32 / 4_294_967_296.0
    }
}

struct ValueNoise {
    scale: f32,
    seed: i32,
}

impl ValueNoise {
    fn new(scale: f32, seed: i32) -> Self {
        Self { scale, seed }
    }

    fn at(&self, x: f32, y: f32) -> f32 {
        let fx = x / self.scale;
        let fy = y / self.scale;
        let x0 = fx.floor();
        let y0 = fy.floor();
        let tx = fx - x0;
        let ty = fy - y0;
        let sx = tx * tx * (3.0 - 2.0 * tx);
        let sy = ty * ty * (3.0 - 2.0 * ty);
        let h = |i: i32, j: i32| hash2(i + self.seed * 131, j - self.seed * 71);
        let (x0, y0) = (x0 as i32, y0 as i32);
        let a = h(x0, y0);
        let b = h(x0 + 1, y0);
        let c = h(x0, y0 + 1);
        let d = h(x0 + 1, y0 + 1);
        (a + (b - a) * sx) * (1.0 - sy) + (c + (d - c) * sx) * sy
    }
}

struct ValueNoise3 {
    scale: f32,
    seed: i32,
}

impl ValueNoise3 {
    fn new(scale: f32, seed: i32) -> Self {
        Self { scale, seed }
    }

    fn at(&self, x: f32, y: f32, z: f32) -> f32 {
        let fx = x / self.scale;
        let fy = y / self.scale;
        let fz = z / self.scale;
        let x0 = fx.floor() as i32;
        let y0 = fy.floor() as i32;
        let z0 = fz.floor() as i32;
        let tx = fx - x0 as f32;
        let ty = fy - y0 as f32;
        let tz = fz - z0 as f32;
        let sx = tx * tx * (3.0 - 2.0 * tx);
        let sy = ty * ty * (3.0 - 2.0 * ty);
        let sz = tz * tz * (3.0 - 2.0 * tz);
        let h = |i: i32, j: i32, k: i32| {
            hash3(i + self.seed * 131, j - self.seed * 71, k + self.seed * 17)
        };
        let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let c00 = lerp(h(x0, y0, z0), h(x0 + 1, y0, z0), sx);
        let c10 = lerp(h(x0, y0 + 1, z0), h(x0 + 1, y0 + 1, z0), sx);
        let c01 = lerp(h(x0, y0, z0 + 1), h(x0 + 1, y0, z0 + 1), sx);
        let c11 = lerp(h(x0, y0 + 1, z0 + 1), h(x0 + 1, y0 + 1, z0 + 1), sx);
        lerp(lerp(c00, c10, sy), lerp(c01, c11, sy), sz)
    }
}

#[derive(Clone)]
struct Alpha {
    w: usize,
    h: usize,
    a: Vec<u8>,
}

impl Alpha {
    fn new(w: usize, h: usize) -> Self {
        Self {
            w,
            h,
            a: vec![0; w * h],
        }
    }

    fn from_vec(w: usize, h: usize, a: Vec<u8>) -> Self {
        Self { w, h, a }
    }

    fn sample(&self, x: f32, y: f32) -> f32 {
        // Canvas samples a sprite with transparent borders, so anything
        // past its half-texel edge contributes nothing. Clamping instead
        // would smear the border texel across the rest of the canvas.
        if x < -0.5 || y < -0.5 || x > self.w as f32 - 0.5 || y > self.h as f32 - 0.5 {
            return 0.0;
        }
        let x = x.clamp(0.0, self.w as f32 - 1.001);
        let y = y.clamp(0.0, self.h as f32 - 1.001);
        let x0 = x as usize;
        let y0 = y as usize;
        let tx = x - x0 as f32;
        let ty = y - y0 as f32;
        let a = f32::from(self.a[y0 * self.w + x0]) / 255.0;
        let b = f32::from(self.a[y0 * self.w + x0 + 1]) / 255.0;
        let c = f32::from(self.a[(y0 + 1) * self.w + x0]) / 255.0;
        let d = f32::from(self.a[(y0 + 1) * self.w + x0 + 1]) / 255.0;
        (a * (1.0 - tx) + b * tx) * (1.0 - ty) + (c * (1.0 - tx) + d * tx) * ty
    }

    fn stroke(&mut self, path: &Path, width: f32, opacity: f32, out: bool) {
        if opacity <= 0.0 {
            return;
        }
        let stroke = Stroke {
            width,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default()
        };
        let Some(stroked) = path.stroke(&stroke, 1.0) else {
            return;
        };
        let Some(mut mask) = Mask::new(self.w as u32, self.h as u32) else {
            return;
        };
        mask.fill_path(&stroked, FillRule::Winding, true, Transform::identity());
        for (a, &m) in self.a.iter_mut().zip(mask.data()) {
            let c = f32::from(m) / 255.0 * opacity;
            let v = f32::from(*a) / 255.0;
            let v = if out {
                v * (1.0 - c)
            } else {
                v + c * (1.0 - v)
            };
            *a = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
}

/// The pile's layers, all alpha only at their own resolution.
#[derive(Clone)]
struct Fur {
    r: usize,
    mask: Alpha,
    haze: Alpha,
    core: Alpha,
    soft: Alpha,
    dark: Alpha,
    light: Alpha,
    glint: Alpha,
    back: Alpha,
    back_soft: Alpha,
    fringe: Option<Arc<Path>>,
    tufts: Option<Arc<Path>>,
    fringe_width: f32,
    tuft_width: f32,
}

fn fur_cache() -> &'static Mutex<HashMap<String, Arc<Fur>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Arc<Fur>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn style_id(s: &FurStyle) -> String {
    format!(
        "{:.2},{:.2},{:.2},{:.2},{:.2},{:.2}",
        s.length, s.density, s.fuzz, s.clumps, s.curl, s.gravity
    )
}

fn light_bin(lx: f32, ly: f32) -> i32 {
    (ly.atan2(lx) / (PI / 12.0)).round() as i32
}

fn fur_tier(cap_px: f32) -> usize {
    if cap_px <= 200.0 {
        192
    } else if cap_px <= 360.0 {
        320
    } else {
        512
    }
}

fn fur_for(form: &Form, r: usize, lx: f32, ly: f32, style: FurStyle) -> Arc<Fur> {
    let key = format!(
        "{}|{}|{}|{}",
        form.id,
        r,
        light_bin(lx, ly),
        style_id(&style)
    );
    if let Ok(c) = fur_cache().lock()
        && let Some(f) = c.get(&key)
    {
        return Arc::clone(f);
    }
    let fur = Arc::new(build_fur(form, r, lx, ly, style));
    if let Ok(mut c) = fur_cache().lock() {
        if c.len() > 8 {
            c.clear();
        }
        c.insert(key, Arc::clone(&fur));
    }
    fur
}

// ── the outline's rim, and the halo's geometry ─────────────────────────

struct RimCache {
    rim: Vec<f32>,
}

fn rims() -> &'static Mutex<HashMap<usize, Arc<RimCache>>> {
    static CACHE: OnceLock<Mutex<HashMap<usize, Arc<RimCache>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn rim_of(form: &Form) -> Arc<RimCache> {
    if let Ok(c) = rims().lock()
        && let Some(r) = c.get(&form.id)
    {
        return Arc::clone(r);
    }
    let n = form.n;
    let u = SPAN / n as f32;
    let mut rim = Vec::new();
    for j in 0..n {
        for x in 0..n {
            let i = j * n + x;
            if form.sd[i] >= 0.0 && form.sd[i] < 2.0 * u {
                rim.push((x as f32 + 0.5) * u - PAD);
                rim.push((j as f32 + 0.5) * u - PAD);
                rim.push(form.sd[i]);
            }
        }
    }
    let cache = Arc::new(RimCache { rim });
    if let Ok(mut c) = rims().lock() {
        if c.len() > 24 {
            c.clear();
        }
        c.insert(form.id, Arc::clone(&cache));
    }
    cache
}

fn reach(rim: &[f32], ex: f32, ey: f32) -> f32 {
    let mut m = f32::NEG_INFINITY;
    for p in rim.chunks_exact(3) {
        let v = p[0] * ex + p[1] * ey + p[2];
        if v > m {
            m = v;
        }
    }
    m
}

struct ReachAt {
    k: usize,
    far: bool,
}

fn stack_reach(
    rel: &Relief,
    k: usize,
    ex: f32,
    ey: f32,
    dl1: f32,
    vx: f32,
    vy: f32,
    h: f32,
    at: Option<&mut ReachAt>,
) -> f32 {
    let v0 = vx * rel.cx[0] + vy * rel.cy[0];
    let ve = (vx * ex + vy * ey) * dl1;
    let mut best = h;
    let mut kb = 0;
    let mut far = false;
    for level in 1..k {
        let r = vx * rel.cx[level] + vy * rel.cy[level] + rel.scale[level] * (h - v0);
        if r + rel.z[level] * ve > best {
            best = r + rel.z[level] * ve;
            kb = level;
            far = false;
        }
        if r - rel.z[level] * ve > best {
            best = r - rel.z[level] * ve;
            kb = level;
            far = true;
        }
    }
    if let Some(at) = at {
        at.k = kb;
        at.far = far;
    }
    best
}

fn halo_stretch(form: &Form, rel: &Relief, k: usize, ex: f32, ey: f32, dl1: f32) -> [f32; 6] {
    if dl1 <= 1e-4 {
        return [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    }
    let cache = rim_of(form);
    let hp = reach(&cache.rim, ex, ey);
    let hn = reach(&cache.rim, -ex, -ey);
    let up = stack_reach(rel, k, ex, ey, dl1, ex, ey, hp, None);
    let un = stack_reach(rel, k, ex, ey, dl1, -ex, -ey, hn, None);
    let s1 = (up + un) / (hp + hn).max(1.0) - 1.0;
    let t = up - (s1 + 1.0) * hp;
    [
        1.0 + s1 * ex * ex,
        s1 * ex * ey,
        s1 * ex * ey,
        1.0 + s1 * ey * ey,
        ex * t,
        ey * t,
    ]
}

struct HaloRays {
    inset: Vec<f32>,
    dark: Vec<f32>,
}

fn halo_rays(
    form: &Form,
    rel: &Relief,
    k: usize,
    ex: f32,
    ey: f32,
    dl1: f32,
    s: &[f32; 6],
    back_shade: f32,
) -> HaloRays {
    let cache = rim_of(form);
    let mut inset = vec![0.0f32; 24];
    let mut dark = vec![0.0f32; 24];
    let s1 = s[0] + s[3] - 2.0;
    for (i, (ins, dk)) in inset.iter_mut().zip(dark.iter_mut()).enumerate() {
        let phi = (i as f32 / 24.0) * 2.0 * PI;
        let (ux, uy) = (phi.cos(), phi.sin());
        let ue = ux * ex + uy * ey;
        let q = if (1.0 + s1).abs() > 1e-6 {
            s1 / (1.0 + s1)
        } else {
            0.0
        };
        let mut vx = ux - q * ue * ex;
        let mut vy = uy - q * ue * ey;
        let vl = (vx * vx + vy * vy).sqrt().max(1e-6);
        vx /= vl;
        vy /= vl;
        let mut at = ReachAt { k: 0, far: false };
        stack_reach(
            rel,
            k,
            ex,
            ey,
            dl1,
            vx,
            vy,
            reach(&cache.rim, vx, vy),
            Some(&mut at),
        );
        *ins = HALO_INSET.max(rel.inset[at.k]);
        *dk = if at.far {
            HALO_SHADE * back_shade * (1.0 - (-2.4 * (at.k as f32 / k as f32)).exp())
        } else {
            0.0
        };
    }
    HaloRays { inset, dark }
}

fn miss_cache() -> &'static Mutex<HashMap<String, [f32; 2]>> {
    static CACHE: OnceLock<Mutex<HashMap<String, [f32; 2]>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn halo_miss(
    form: &Form,
    path: &Path,
    rel: &Relief,
    k: usize,
    s: &[f32; 6],
    dxs: f32,
    dys: f32,
) -> [f32; 2] {
    let key = format!(
        "{}|{}|{:.2}|{}|{}",
        form.id,
        k,
        rel.q,
        (dxs / MISS_STEP).round() as i32,
        (dys / MISS_STEP).round() as i32
    );
    if let Ok(c) = miss_cache().lock()
        && let Some(v) = c.get(&key)
    {
        return *v;
    }
    let value = measure_miss(form, path, rel, k, s, dxs, dys);
    if let Ok(mut c) = miss_cache().lock() {
        if c.len() > 256 {
            c.clear();
        }
        c.insert(key, value);
    }
    value
}

fn measure_miss(
    form: &Form,
    path: &Path,
    rel: &Relief,
    k: usize,
    s: &[f32; 6],
    dxs: f32,
    dys: f32,
) -> [f32; 2] {
    let Some(mut mask) = Mask::new(MISS_N as u32, MISS_N as u32) else {
        return [0.0, 0.0];
    };
    let u = SPAN / MISS_N as f32;
    let det = s[0] * s[3] - s[1] * s[2];
    let (ia, ib, ic, id) = if det.abs() > 1e-9 {
        (s[3] / det, -s[1] / det, -s[2] / det, s[0] / det)
    } else {
        (1.0, 0.0, 0.0, 1.0)
    };
    let ie = -(ia * s[4] + ic * s[5]);
    let jf = -(ib * s[4] + id * s[5]);
    for level in 0..k {
        let zs: [f32; 2] = if level == 0 {
            [0.0, f32::NAN]
        } else {
            [rel.z[level], -rel.z[level]]
        };
        for z in zs {
            if z.is_nan() {
                continue;
            }
            let sk = if level == 0 { 1.0 } else { rel.scale[level] };
            let tx = if level == 0 {
                0.0
            } else {
                rel.cx[level] - sk * rel.cx[0]
            };
            let ty = if level == 0 {
                0.0
            } else {
                rel.cy[level] - sk * rel.cy[0]
            };
            let m = Affine::scale(sk, sk)
                .then(Affine::translate(tx + z * dxs, ty + z * dys))
                .then(Affine([ia, ib, ic, id, ie, jf]))
                .then(Affine::scale(MISS_N as f32 / SPAN, MISS_N as f32 / SPAN))
                .then(Affine::translate(
                    PAD * MISS_N as f32 / SPAN,
                    PAD * MISS_N as f32 / SPAN,
                ));
            mask.fill_path(path, FillRule::Winding, true, m.transform());
        }
    }
    let data = mask.data();
    let mut on = vec![0u8; MISS_N * MISS_N];
    for i in 0..on.len() {
        on[i] = u8::from(data[i] > 127);
    }
    let (dist, _) = form::edt2d(&on, 1, MISS_N);
    let mut cut = 0.0f32;
    let mut over = 0.0f32;
    for y in 0..MISS_N {
        for x in 0..MISS_N {
            let i = y * MISS_N + x;
            let d = bilerp(
                &form.sd,
                form.n,
                ((x as f32 + 0.5) / MISS_N as f32) * form.n as f32 - 0.5,
                ((y as f32 + 0.5) / MISS_N as f32) * form.n as f32 - 0.5,
            );
            if on[i] == 1 && d < 0.0 {
                cut = cut.max(-d);
            }
            if on[i] == 0 && d > 0.0 {
                over = over.max(dist[i].sqrt() * u);
            }
        }
    }
    [cut, over]
}

// ── the pile's bake ────────────────────────────────────────────────────

fn box_blur(src: &[f32], dst: &mut [f32], rb: usize, r: usize, k: usize, stride: usize) {
    let step = if stride == 1 { rb } else { 1 };
    let inv = 1.0 / (2 * r + 1) as f32;
    let mut line = vec![0.0f32; rb];
    let o = k * step;
    for t in 0..rb {
        line[t] = src[o + t * stride];
    }
    let at = |t: isize| -> f32 { line[t.clamp(0, rb as isize - 1) as usize] };
    let mut acc = 0.0;
    for t in -(r as isize)..=(r as isize) {
        acc += at(t);
    }
    for t in 0..rb {
        dst[o + t * stride] = acc * inv;
        acc += at(t as isize + r as isize + 1) - at(t as isize - r as isize);
    }
}

fn build_fur(form: &Form, r: usize, lx: f32, ly: f32, style: FurStyle) -> Fur {
    let n = form.n;
    let px = r as f32 / SPAN;
    let o = PAD + FILM_M;
    let rb = ((r as f32 * FILM_SPAN) / SPAN).round() as usize;
    let mut rand = Rng::new(n * 7919 + 17);
    let lean = ValueNoise::new(3.4, 21);
    let tone = ValueNoise::new(22.0, 9);
    let swirl = ValueNoise::new(13.0, 57);
    let k_len = style.length;
    let k_den = style.density;
    let fuzz = style.fuzz;
    let clumps = style.clumps;
    let curl = style.curl;
    let gravity = style.gravity;
    let ll = (lx * lx + ly * ly).sqrt().max(1e-6);
    let lx = lx / ll;
    let ly = ly / ll;
    let uf = [lx, ly, 0.0];
    let vf = [0.0, 0.0, 1.0];
    let df = [0.0, 1.0, 0.0];
    let k3 = {
        let c = KEY_EL.cos();
        let s = KEY_EL.sin();
        norm3([
            uf[0] * c + s * 0.0,
            uf[1] * c + s * 0.0,
            uf[2] * c + s * vf[2],
        ])
    };
    let b3 = norm3([
        0.94 * lx - 0.34 * vf[0],
        0.94 * ly - 0.34 * vf[1],
        -0.34 * vf[2],
    ]);
    let h3 = norm3([k3[0], k3[1], k3[2] + 1.0]);
    let f3 = norm3([
        -0.55 * uf[0] + 0.85 * vf[0] + 0.15 * df[0],
        -0.55 * uf[1] + 0.85 * vf[1] + 0.15 * df[1],
        -0.55 * uf[2] + 0.85 * vf[2] + 0.15 * df[2],
    ]);
    let at = |f: &[f32], x: f32, y: f32| {
        bilerp(
            f,
            n,
            ((x + PAD) / SPAN) * n as f32 - 0.5,
            ((y + PAD) / SPAN) * n as f32 - 0.5,
        )
    };
    let sd_at = |x: f32, y: f32| at(&form.sd, x, y);
    let h_at = |x: f32, y: f32| at(&form.h, x, y);
    let mut crown_y = 12.0;
    let mut y = -PAD;
    while y < 60.0 {
        if sd_at(50.0, y) > 0.5 {
            crown_y = y + 1.0;
            break;
        }
        y += 0.5;
    }
    crown_y -= 18.0;
    let crown_x = 50.0;
    let outward = |x: f32, y: f32| -> (f32, f32) {
        let e = 0.8;
        let ox = sd_at(x - e, y) - sd_at(x + e, y);
        let oy = sd_at(x, y - e) - sd_at(x, y + e);
        let ol = (ox * ox + oy * oy).sqrt().max(1e-6);
        (ox / ol, oy / ol)
    };
    let flow = |x: f32, y: f32| -> (f32, f32) {
        let mut rx = x - crown_x;
        let mut ry = y - crown_y;
        let rl = (rx * rx + ry * ry).sqrt().max(1e-6);
        rx /= rl;
        ry /= rl;
        let fall = gravity * (1.0 - 0.6 * (-rl / 8.0).exp());
        let mut dx = rx * (1.0 - 0.8 * fall);
        let mut dy = ry * (1.0 - 0.8 * fall) + fall;
        let d0 = sd_at(x, y);
        if d0 < 5.0 {
            let (ox, oy) = outward(x, y);
            let up = (-oy).max(0.0);
            let ex = ox * (1.0 - gravity * up);
            let ey = oy * (1.0 - gravity * up) + 1.2 * gravity;
            let w = (1.0 - d0 / 5.0).clamp(0.0, 1.0) * 0.75;
            dx = dx * (1.0 - w) + ex * w * 1.6;
            dy = dy * (1.0 - w) + ey * w * 1.6;
        }
        let a = (lean.at(x, y) - 0.5) * (0.5 + 1.6 * curl) + (swirl.at(x, y) - 0.5) * (0.6 + curl);
        let (ca, sa) = (a.cos(), a.sin());
        let qx = dx * ca - dy * sa;
        let qy = dx * sa + dy * ca;
        let m = (qx * qx + qy * qy).sqrt().max(1e-6);
        (qx / m, qy / m)
    };
    let slope_at = |x: f32, y: f32| -> (f32, f32, f32) {
        let e = 0.6;
        let gx = (h_at(x + e, y) - h_at(x - e, y)) / (2.0 * e);
        let gy = (h_at(x, y + e) - h_at(x, y - e)) / (2.0 * e);
        let m = (gx * gx + gy * gy).sqrt();
        (
            if m > 1e-6 { -gx / m } else { 0.0 },
            if m > 1e-6 { -gy / m } else { 0.0 },
            m / (1.0 + m * m).sqrt(),
        )
    };

    let g = 112usize;
    let gu = FILM_SPAN / g as f32;
    let mut flow_x = vec![0.0f32; g * g];
    let mut flow_y = vec![0.0f32; g * g];
    let mut tilt_g = vec![0.0f32; g * g];
    let mut nx_g = vec![0.0f32; g * g];
    let mut ny_g = vec![0.0f32; g * g];
    let mut nz_g = vec![0.0f32; g * g];
    let mut on_g = vec![0.0f32; g * g];
    let mut pf_x = vec![0.0f32; g * g];
    let mut pf_y = vec![0.0f32; g * g];
    let mut fore_g = vec![0.0f32; g * g];
    let mut h_g = vec![0.0f32; g * g];
    let mut crease_g = vec![0.0f32; g * g];
    let mut tone_g = vec![0.0f32; g * g];
    for j in 0..g {
        for x in 0..g {
            let i = j * g + x;
            let xx = (x as f32 + 0.5) * gu - o;
            let yy = (j as f32 + 0.5) * gu - o;
            let (mut fx, mut fy) = flow(xx, yy);
            let (dx, dy, tilt) = slope_at(xx, yy);
            let w = tilt * (0.25 + 0.5 * (1.0 - gravity)) * (1.0 - gravity * (-dy).max(0.0));
            fx = fx * (1.0 - w) + dx * w;
            fy = fy * (1.0 - w) + (dy + 0.35 * gravity) * w;
            let fl = (fx * fx + fy * fy).sqrt().max(1e-6);
            fx /= fl;
            fy /= fl;
            flow_x[i] = fx;
            flow_y[i] = fy;
            tilt_g[i] = tilt;
            let mut nx = dx * tilt;
            let mut ny = dy * tilt;
            let mut nz = (1.0 - tilt * tilt).max(0.0).sqrt();
            let d0 = sd_at(xx, yy);
            if d0 < 2.0 {
                let (ox, oy) = outward(xx, yy);
                let w = if d0 >= 0.0 {
                    0.7 * (1.0 - d0 / 2.0)
                } else {
                    (0.7 - 0.2 * d0).min(1.0)
                };
                nx += w * (ox - nx);
                ny += w * (oy - ny);
                nz -= w * nz;
                let nl = (nx * nx + ny * ny + nz * nz).sqrt().max(1e-6);
                nx /= nl;
                ny /= nl;
                nz /= nl;
            }
            nx_g[i] = nx;
            ny_g[i] = ny;
            nz_g[i] = nz;
            on_g[i] = smooth(-3.5, -1.5, d0);
            tone_g[i] = tone.at(xx, yy);
            h_g[i] = if d0 > 0.0 { h_at(xx, yy) } else { 0.0 };
            let s = (nx * nx + ny * ny).sqrt();
            let (gx, gy) = if s > 1e-4 {
                (nx / s, ny / s)
            } else {
                (0.0, 0.0)
            };
            let a = fx * gx + fy * gy;
            let k = (1.0 - nz.max(0.0)) * a;
            let mut qx = PILE_COS * (fx - k * gx) + PILE_SIN * s * gx;
            let mut qy = PILE_COS * (fy - k * gy) + PILE_SIN * s * gy;
            let att = smooth(-0.5, 1.5, d0);
            qx = fx + (qx - fx) * att;
            qy = fy + (qy - fy) * att;
            let ql = (qx * qx + qy * qy).sqrt().max(1e-6);
            pf_x[i] = qx / ql;
            pf_y[i] = qy / ql;
            fore_g[i] = 1.0 + (ql.min(1.0) - 1.0) * att;
        }
    }
    {
        let mut tmp = vec![0.0f32; g * g];
        let mut avg = h_g.clone();
        for _ in 0..3 {
            for j in 0..g {
                for x in 0..g {
                    let mut acc = 0.0;
                    for t in -2..=2 {
                        let xx = (x as i32 + t).clamp(0, g as i32 - 1) as usize;
                        acc += avg[j * g + xx];
                    }
                    tmp[j * g + x] = acc / 5.0;
                }
            }
            for j in 0..g {
                for x in 0..g {
                    let mut acc = 0.0;
                    for t in -2..=2 {
                        let yy = (j as i32 + t).clamp(0, g as i32 - 1) as usize;
                        acc += tmp[yy * g + x];
                    }
                    avg[j * g + x] = acc / 5.0;
                }
            }
        }
        for j in 0..g {
            for x in 0..g {
                let i = j * g + x;
                let inside = smooth(
                    0.2,
                    2.0,
                    sd_at((x as f32 + 0.5) * gu - o, (j as f32 + 0.5) * gu - o),
                );
                let valley = smooth(0.3, 3.5, avg[i] - h_g[i]) * inside;
                let under = smooth(0.15, 0.85, ny_g[i]) * (1.0 - nz_g[i]);
                crease_g[i] = (valley + 0.45 * under).min(1.0) * on_g[i];
            }
        }
    }
    let grid_at = |f: &[f32], x: f32, y: f32| bilerp(f, g, (x + o) / gu - 0.5, (y + o) / gu - 0.5);
    let dir_at = |x: f32, y: f32| -> (f32, f32) {
        let ux = grid_at(&pf_x, x, y);
        let uy = grid_at(&pf_y, x, y);
        let ul = (ux * ux + uy * uy).sqrt();
        if ul > 1e-4 {
            (ux / ul, uy / ul)
        } else {
            (0.0, 1.0)
        }
    };
    let free_dir_at = |x: f32, y: f32| -> (f32, f32) {
        let ux = grid_at(&flow_x, x, y);
        let uy = grid_at(&flow_y, x, y);
        let ul = (ux * ux + uy * uy).sqrt();
        if ul > 1e-4 {
            (ux / ul, uy / ul)
        } else {
            (0.0, 1.0)
        }
    };

    // locks
    let rl = rb >> 1;
    let pl = rl as f32 / FILM_SPAN;
    let mut lock = vec![0.0f32; rl * rl];
    let mut lock_lit = vec![0.0f32; rl * rl];
    let lock_at = |f: &[f32], x: f32, y: f32| bilerp(f, rl, (x + o) * pl - 0.5, (y + o) * pl - 0.5);
    let bump = ValueNoise::new(0.7 + 0.35 * k_len, 61);
    let swell = ValueNoise::new(2.2 + 0.6 * k_len, 67);
    let tuft_k = (0.2 + 0.5 * fuzz) * k_len.min(1.8);
    let tuft_at = |lock: &[f32], x: f32, y: f32| {
        let v = 0.3 * lock_at(lock, x, y).clamp(-1.6, 1.6) / 1.6
            + 0.25 * (bump.at(x, y) - 0.5) * 2.4
            + 0.45 * (swell.at(x, y) - 0.5) * 2.4;
        tuft_k * v.clamp(-1.0, 1.0)
    };
    let lock_w = 0.8 + 0.45 * k_len;
    let lock_wd = lock_w * (0.65 + 0.4 * clumps);
    let lock_ld = 1.5 + 1.6 * k_len;
    let h_max = 0.32 * lock_wd * (0.15 + 0.85 * clumps);
    let mut pile_h = vec![-0.25 * h_max; rb * rb];
    let mut own = vec![-1i32; rb * rb];
    let cell = ((0.6 * lock_wd * lock_ld) / LOCK_COVER / LOCK_DENSE).sqrt();
    let gn = (FILM_SPAN / cell).ceil() as usize;
    let mut locks: Vec<f32> = Vec::with_capacity(gn * gn * 16);
    let stand0 = 0.45 - 0.2 * gravity;
    let tuft_n = ValueNoise3::new(1.6 * lock_ld, 79);
    let size_n = ValueNoise::new(9.0, 73);
    let tuft_h = 1.5 * h_max * (0.25 + clumps);
    for gy in 0..gn {
        for gx in 0..gn {
            let x = (gx as f32 + rand.next()) * cell - o;
            let y = (gy as f32 + rand.next()) * cell - o;
            let r1 = rand.next();
            let r2 = rand.next();
            let r3 = rand.next();
            let r4 = rand.next();
            let r5 = rand.next();
            let r6 = rand.next();
            let r7 = rand.next();
            let mut nx = grid_at(&nx_g, x, y);
            let mut ny = grid_at(&ny_g, x, y);
            let mut nz = grid_at(&nz_g, x, y);
            let nl = (nx * nx + ny * ny + nz * nz).sqrt().max(1e-6);
            nx /= nl;
            ny /= nl;
            nz /= nl;
            let (fx, fy) = free_dir_at(x, y);
            let ja = (r1 - 0.5) * (0.2 + 0.45 * curl);
            let (cj, sj) = (ja.cos(), ja.sin());
            let ux = fx * cj - fy * sj;
            let uy = fx * sj + fy * cj;
            let a = ux * nx + uy * ny;
            let mut tx = ux - a * nx;
            let mut ty = uy - a * ny;
            let mut tz = -a * nz;
            let mut tl = (tx * tx + ty * ty + tz * tz).sqrt();
            if tl < 0.05 {
                tx = nz * nx;
                ty = nz * ny;
                tz = nz * nz - 1.0;
                tl = (tx * tx + ty * ty + tz * tz).sqrt().max(1e-6);
            }
            tx /= tl;
            ty /= tl;
            tz /= tl;
            let mut bx = ny * tz - nz * ty;
            let mut by = nz * tx - nx * tz;
            let mut bz = nx * ty - ny * tx;
            let st = stand0 + 0.2 * r2;
            let (cs, sn) = (st.cos(), st.sin());
            let grow = 0.8 + 0.45 * size_n.at(x, y);
            let wd = lock_wd * grow * (0.8 + 0.4 * r3);
            let ld = lock_ld * grow * (0.75 + 0.5 * r4);
            let mut ax = (tx * cs + nx * sn) * ld * px;
            let mut ay = (ty * cs + ny * sn) * ld * px;
            let mut la = (ax * ax + ay * ay).sqrt();
            if la < 1.5 {
                ax = ux;
                ay = uy;
                la = 1.5;
            } else {
                ax /= la;
                ay /= la;
            }
            let mut across = -bx * ay + by * ax;
            if across < 0.0 {
                bx = -bx;
                by = -by;
                bz = -bz;
                across = -across;
            }
            let _ = bz;
            let wa = (0.5 * wd * px * across.max(0.15)).max(0.6);
            let seen = (la / (ld * px)) * (wa / (0.5 * wd * px));
            let tn = tuft_n.at(x, y, h_at(x, y));
            if r5
                > (LOCK_DENSE / LOCK_DENSE.max(seen))
                    * (1.0 - LOCK_PART * clumps * (1.0 - smooth(0.25, 0.65, tn)))
            {
                continue;
            }
            locks.extend_from_slice(&[
                (x + o) * px,
                (y + o) * px,
                ax,
                ay,
                la,
                wa,
                0.1 * h_max * r6 + tuft_h * tn,
                (r7 - 0.5) * 1.2 * curl,
                tx,
                ty,
                tz,
                bx,
                by,
                bz,
                ld,
                wd,
            ]);
        }
    }
    let n_locks = locks.len() / 16;
    let lock_along = |s: f32| (if s < 0.25 { 4.0 * s } else { 1.0 }) * (1.0 - 0.5 * s);
    for k in 0..n_locks {
        let b = k * 16;
        let ox = locks[b];
        let oy = locks[b + 1];
        let ax = locks[b + 2];
        let ay = locks[b + 3];
        let la = locks[b + 4];
        let wa = locks[b + 5];
        let z0 = locks[b + 6];
        let bend = locks[b + 7];
        let ext = wa * (1.0 + bend.abs()) + 1.0;
        let ex = ox + ax * la;
        let ey = oy + ay * la;
        let xa = ((ox.min(ex) - ext).floor() as i32).max(0) as usize;
        let xb = ((ox.max(ex) + ext).ceil() as i32).min(rb as i32 - 1) as usize;
        let ya = ((oy.min(ey) - ext).floor() as i32).max(0) as usize;
        let yb = ((oy.max(ey) + ext).ceil() as i32).min(rb as i32 - 1) as usize;
        let ila = 1.0 / la;
        let iwa = 1.0 / wa;
        for y in ya..=yb {
            let dy = y as f32 + 0.5 - oy;
            for x in xa..=xb {
                let i = y * rb + x;
                let dx = x as f32 + 0.5 - ox;
                let s = (dx * ax + dy * ay) * ila;
                if s <= 0.0 || s >= 1.0 {
                    continue;
                }
                let w = LOCK_TAPER * s.sqrt() * (1.0 - s);
                let v = (dy * ax - dx * ay) * iwa - bend * s * s;
                if v >= w || v <= -w {
                    continue;
                }
                let q = v / w;
                let h = z0 + h_max * lock_along(s) * (1.0 - q * q);
                if h > pile_h[i] {
                    pile_h[i] = h;
                    own[i] = k as i32;
                }
            }
        }
    }
    let mut pile_b = vec![0.0f32; rb * rb];
    let r_b = ((0.3 * lock_wd * px).round() as usize).max(1);
    for y in 0..rb {
        box_blur(&pile_h, &mut pile_b, rb, r_b, y, 1);
    }
    for _ in 0..2 {
        let src = pile_b.clone();
        for y in 0..rb {
            box_blur(&src, &mut pile_b, rb, r_b, y, 1);
        }
        let src = pile_b.clone();
        for x in 0..rb {
            box_blur(&src, &mut pile_b, rb, r_b, x, rb);
        }
    }
    let mut pile_h2 = pile_h.clone();
    for y in 0..rb {
        box_blur(&pile_h, &mut pile_h2, rb, 1, y, 1);
    }
    pile_h = pile_h2;
    let mut pile_h3 = pile_h.clone();
    for x in 0..rb {
        box_blur(&pile_h, &mut pile_h3, rb, 1, x, rb);
    }
    pile_h = pile_h3;

    // fleece
    let mut dark = vec![0u8; rb * rb];
    let mut light = vec![0u8; rb * rb];
    let mut glint = vec![0u8; rb * rb];
    let mut back = vec![0u8; rb * rb];
    let mut soft_back = vec![0u8; rb * rb];
    let mut white = vec![0.0f32; rb * rb];
    for y in 0..rb {
        for x in 0..rb {
            let h = hash2(x as i32 + 911, y as i32 + 37);
            white[y * rb + x] = h * h * h;
        }
    }
    let fibre_light = |x: f32, y: f32, ux: f32, uy: f32, stand: f32| -> [f32; 11] {
        let fx = ((x + o) / gu - 0.5).clamp(0.0, g as f32 - 1.0);
        let fy = ((y + o) / gu - 0.5).clamp(0.0, g as f32 - 1.0);
        let x0 = (fx as usize).min(g - 2);
        let y0 = (fy as usize).min(g - 2);
        let u = fx - x0 as f32;
        let v = fy - y0 as f32;
        let j = y0 * g + x0;
        let w00 = (1.0 - u) * (1.0 - v);
        let w10 = u * (1.0 - v);
        let w01 = (1.0 - u) * v;
        let w11 = u * v;
        let read = |f: &[f32]| f[j] * w00 + f[j + 1] * w10 + f[j + g] * w01 + f[j + g + 1] * w11;
        let mut nx = read(&nx_g);
        let mut ny = read(&ny_g);
        let mut nz = read(&nz_g);
        let nn = (nx * nx + ny * ny + nz * nz).sqrt().max(1e-6);
        nx /= nn;
        ny /= nn;
        nz /= nn;
        let nk = nx * k3[0] + ny * k3[1] + nz * k3[2];
        let mut fib = [0.0f32; 11];
        fib[4] = nx;
        fib[5] = ny;
        fib[6] = nz;
        fib[7] = nk;
        fib[8] = read(&crease_g);
        let on = read(&on_g);
        fib[9] = on;
        fib[10] = read(&tone_g);
        if on <= 0.0 {
            fib[2] = 0.7;
            return fib;
        }
        let key = smooth(-0.2, 0.5, nk);
        let along = ux * nx + uy * ny;
        let mut tx = ux - along * nx;
        let mut ty = uy - along * ny;
        let mut tz = -along * nz;
        let tl = (tx * tx + ty * ty + tz * tz).sqrt();
        if tl > 1e-3 {
            tx /= tl;
            ty /= tl;
            tz /= tl;
        } else {
            tx = 0.0;
            ty = 0.0;
            tz = 1.0;
        }
        let (cs, sn) = (stand.cos(), stand.sin());
        tx = tx * cs + nx * sn;
        ty = ty * cs + ny * sn;
        tz = tz * cs + nz * sn;
        let th = tx * h3[0] + ty * h3[1] + tz * h3[2];
        let nh = nx * h3[0] + ny * h3[1] + nz * h3[2];
        let a1 = th - 0.12 * nh;
        let a2 = th + 0.2 * nh;
        let b1 = (1.0 - a1 * a1).max(0.0);
        let b2 = b1 * b1;
        let b4 = b2 * b2;
        let b8 = b4 * b4;
        let b16 = b8 * b8;
        let c1 = (1.0 - a2 * a2).max(0.0);
        let c2 = c1 * c1;
        fib[0] = key * (0.85 * b16 * b16 * b4 + 0.15 * c2 * c2);
        fib[0] *= (0.4 + 0.6 * (1.0 - smooth(0.35, 0.85, nk))) * on;
        let side = (1.0 - tz * tz).max(0.0).sqrt();
        fib[1] = smooth(-0.3, 0.6, nx * b3[0] + ny * b3[1] + nz * b3[2])
            * (1.0 - nz).powf(1.6)
            * (0.4 + 0.6 * side)
            * (1.0 - 0.6 * ny.max(0.0))
            * on;
        fib[3] = smooth(-0.5, 0.75, nx * b3[0] + ny * b3[1] + nz * b3[2])
            * (1.0 - nz).powf(0.85)
            * (1.0 - 0.6 * ny.max(0.0))
            * on;
        fib[2] = key * on + 0.7 * (1.0 - on);
        fib
    };
    let strand_half = ((0.35 + 0.4 * k_len) * px).round().max(1.0) as usize;
    let mut strand_w: Vec<Vec<f32>> = Vec::new();
    let mut strand_mean = vec![0.0f32; 2 * strand_half + 1];
    let mut strand_sd = vec![0.0f32; 2 * strand_half + 1];
    for l in 0..=2 * strand_half {
        let mut w = vec![0.0f32; l + 1];
        let mut m1 = 0.0f32;
        let mut v2 = 0.0f32;
        for (t, wt) in w.iter_mut().enumerate() {
            let tau = if l > 0 { t as f32 / l as f32 } else { 0.0 };
            let p = if tau <= STRAND_SHORT {
                1.0
            } else {
                (1.0 - tau) / (1.0 - STRAND_SHORT)
            };
            *wt = 1.0 - STRAND_TAPER * tau;
            m1 += *wt * p * W_MEAN;
            v2 += *wt * *wt * (p / 7.0 - W_MEAN * W_MEAN * p * p);
        }
        strand_w.push(w);
        strand_mean[l] = m1;
        strand_sd[l] = v2.max(1e-6).sqrt();
    }
    let strand_k =
        0.12 * (if r >= 512 {
            1.0
        } else if r >= 320 {
            0.75
        } else {
            0.4
        }) * k_den.mul_add(0.35, 0.55).min(1.4);
    let shadow_reach = (1.5 * lock_wd * px).max(1.0);
    let n_sh = (shadow_reach.round() as usize).clamp(1, 5);
    let mut sh_x = vec![0i32; n_sh];
    let mut sh_y = vec![0i32; n_sh];
    let mut sh_d = vec![0.0f32; n_sh];
    for t in 0..n_sh {
        let d = ((t as f32 + 1.0) / n_sh as f32) * shadow_reach;
        sh_x[t] = (lx * d).round() as i32;
        sh_y[t] = (ly * d).round() as i32;
        sh_d[t] = ((sh_x[t] * sh_x[t] + sh_y[t] * sh_y[t]) as f32).sqrt() / px;
    }
    let key_on = |nk: f32| -> f32 {
        let q = (nk + PILE_WRAP) / (1.0 + PILE_WRAP);
        if q <= 0.0 {
            0.0
        } else if q >= 1.0 {
            1.0
        } else {
            q.powf(1.35)
        }
    };
    let depth_k = if r >= 512 {
        1.0
    } else if r >= 320 {
        0.8
    } else {
        0.55
    };
    let lock_seen = |x: f32, y: f32, xx: f32, yy: f32, k: i32| -> [f32; 4] {
        if k < 0 {
            let (ux, uy) = dir_at(xx, yy);
            return [ux, uy, 0.5, 0.0];
        }
        let b = k as usize * 16;
        let ax = locks[b + 2];
        let ay = locks[b + 3];
        let la = locks[b + 4];
        let wa = locks[b + 5];
        let bend = locks[b + 7];
        let dx = x + 0.5 - locks[b];
        let dy = y + 0.5 - locks[b + 1];
        let s = ((dx * ax + dy * ay) / la).clamp(0.005, 0.995);
        let ss = s.sqrt();
        let w = LOCK_TAPER * ss * (1.0 - s);
        let dw = LOCK_TAPER * ((0.5 * (1.0 - s)) / ss - ss);
        let q = (((dy * ax - dx * ay) / wa - bend * s * s) / w).clamp(-1.0, 1.0);
        let sl = (q * dw + 2.0 * bend * s).clamp(-1.2, 1.2);
        let ux = la * ax - sl * wa * ay;
        let uy = la * ay + sl * wa * ax;
        let ul = (ux * ux + uy * uy).sqrt().max(1e-6);
        [ux / ul, uy / ul, s, lock_along(s) * (1.0 - q * q)]
    };
    let mut strand_buf = vec![0.0f32; rb * rb];
    for y in 0..rb {
        let yy = (y as f32 + 0.5) / px - o;
        for x in 0..rb {
            let xx = (x as f32 + 0.5) / px - o;
            let i = y * rb + x;
            let seen = lock_seen(x as f32, y as f32, xx, yy, own[i]);
            let (ux, uy) = (seen[0], seen[1]);
            let tilt = grid_at(&tilt_g, xx, yy);
            let fore = grid_at(&fore_g, xx, yy);
            let l = (2.0
                * (strand_half as f32 * (0.5 + 0.5 * tilt) * fore / PILE_COS)
                    .round()
                    .clamp(1.0, strand_half as f32)) as usize;
            let sw = &strand_w[l];
            let mut acc = 0.0f32;
            let mut acc_r = 0.0f32;
            for (t, &swt) in sw.iter().enumerate() {
                let sx = (x as f32 - ux * t as f32).round() as i32;
                let sy = (y as f32 - uy * t as f32).round() as i32;
                if sx < 0 || sy < 0 || sx >= rb as i32 || sy >= rb as i32 {
                    acc += strand_mean[l] / (l + 1) as f32;
                    continue;
                }
                let m = white[sy as usize * rb + sx as usize];
                let f = m * 53.7;
                let ls = l as f32 * (STRAND_SHORT + (1.0 - STRAND_SHORT) * (f - f.floor()));
                if t as f32 <= ls {
                    acc += m * swt;
                    acc_r += m * swt * t as f32 / ls.max(1.0);
                }
            }
            let strand = (acc - strand_mean[l]) / strand_sd[l];
            let along = if acc > 1e-4 { acc_r / acc } else { 0.5 };
            strand_buf[i] = strand + STRAND_RISE * strand.clamp(0.0, 2.5) * (along - 0.45);
        }
    }
    let gam = |v: f32| -> f32 {
        if v <= 0.0 {
            0.0
        } else if v >= 1.0 {
            1.0
        } else {
            1.055 * v.powf(1.0 / 2.2) - 0.055
        }
    };
    let mut lsum = 0.0f32;
    let mut lsum2 = 0.0f32;
    for y in 0..rb {
        let yy = (y as f32 + 0.5) / px - o;
        for x in 0..rb {
            let xx = (x as f32 + 0.5) / px - o;
            let i = y * rb + x;
            let k = own[i];
            let seen = lock_seen(x as f32, y as f32, xx, yy, k);
            let (ux, uy, s, hgt) = (seen[0], seen[1], seen[2], seen[3]);
            let fib = fibre_light(xx, yy, ux, uy, 0.25 + 0.2 * hgt);
            let nx = fib[4];
            let ny = fib[5];
            let nz = fib[6];
            let nkb = fib[7];
            let xl0 = if x > 0 { i - 1 } else { i };
            let xr0 = if x < rb - 1 { i + 1 } else { i };
            let yu0 = if y > 0 { i - rb } else { i };
            let yd0 = if y < rb - 1 { i + rb } else { i };
            let gx = ((pile_h[xr0] - pile_h[xl0]) * px) / (xr0 - xl0) as f32;
            let gy = ((pile_h[yd0] - pile_h[yu0]) * px * rb as f32) / (yd0 - yu0) as f32;
            let l1 = (nx * nx + ny * ny).sqrt();
            let (t1x, t1y) = if l1 > 1e-3 {
                (-ny / l1, nx / l1)
            } else {
                (1.0, 0.0)
            };
            let (t2x, t2y, t2z) = if l1 > 1e-3 {
                (-nz * nx / l1, -nz * ny / l1, l1)
            } else {
                (0.0, 1.0, 0.0)
            };
            let g1 = (gx * t1x + gy * t1y).clamp(-1.5, 1.5);
            let g2 = (gx * t2x + gy * t2y).clamp(-1.5, 1.5);
            let mut mx = nx - g1 * t1x - g2 * t2x;
            let mut my = ny - g1 * t1y - g2 * t2y;
            let mut mz = nz - g2 * t2z;
            let ml = (mx * mx + my * my + mz * mz).sqrt().max(1e-6);
            mx /= ml;
            my /= ml;
            mz /= ml;
            let mut kk = 1.0f32;
            if k >= 0 {
                let b = k as usize * 16;
                let tx = locks[b + 8];
                let ty = locks[b + 9];
                let tz = locks[b + 10];
                let fx = 0.9 * tx + 0.44 * nx;
                let fy = 0.9 * ty + 0.44 * ny;
                let fz = 0.9 * tz + 0.44 * nz;
                let fk = (fx * k3[0] + fy * k3[1] + fz * k3[2])
                    / (fx * fx + fy * fy + fz * fz).sqrt().max(1e-6);
                kk = 1.0 - PILE_KK + PILE_KK * (1.0 - fk * fk).max(0.0).sqrt().min(1.2) / 0.85;
            }
            let nkp = mx * k3[0] + my * k3[1] + mz * k3[2];
            let h0 = pile_h[i];
            let mut sh = 1.0f32;
            if nkb > -0.25 {
                let sin_e = nkb.clamp(0.05, 0.97);
                let tan_e = sin_e / (1.0 - sin_e * sin_e).max(0.0).sqrt();
                let mut occ = 0.0f32;
                for t in 0..n_sh {
                    let sx = x as i32 + sh_x[t];
                    let sy = y as i32 + sh_y[t];
                    if sx < 0 || sy < 0 || sx >= rb as i32 || sy >= rb as i32 {
                        break;
                    }
                    let o = pile_h[sy as usize * rb + sx as usize] - h0 - sh_d[t] * tan_e;
                    if o > occ {
                        occ = o;
                    }
                }
                sh = 1.0 - PILE_SHADOW * smooth(0.0, 0.35 * h_max, occ);
            }
            let mut ao = 1.0 - PILE_AO * smooth(0.0, 0.6 * h_max, pile_b[i] - h0);
            if k >= 0 {
                ao *= 1.0 - PILE_ROOT * (1.0 - smooth(0.0, 0.35, s));
            }
            let fill_p = PILE_AMB + PILE_FILL * (mx * f3[0] + my * f3[1] + mz * f3[2]).max(0.0);
            let fill_b = PILE_AMB + PILE_FILL * (nx * f3[0] + ny * f3[1] + nz * f3[2]).max(0.0);
            let mut rr =
                (ao * (fill_p + key_on(nkp) * sh * kk) * PILE_GAIN) / (fill_b + key_on(nkb));
            if k >= 0 {
                rr *= (1.0 + 0.15 * s * s) * (1.0 + 0.08 * (hash2(k, 5) - 0.5));
            }
            let crease = fib[8];
            rr *=
                1.0 - PILE_CREASE * crease * (0.6 + 0.4 * smooth(0.0, 0.6 * h_max, pile_b[i] - h0));
            rr = 1.0 + (rr - 1.0) * depth_k;
            let on = fib[9];
            let lq = (fill_b + key_on(nkb)) / PILE_LIT;
            let lbs = (if lq >= 1.0 { 1.0 } else { gam(lq) }) * on + 0.75 * (1.0 - on);
            let mut m0 = if rr > 2.4 { 2.4f32.sqrt() } else { rr.sqrt() };
            if m0 > 1.0 {
                m0 = 1.0 + (m0 - 1.0) / (1.0 + 2.5 * lbs * (m0 - 1.0));
            }
            let strand = strand_buf[i];
            let mut cast = 0.0f32;
            if nkb > 0.0 {
                let kx = k3[0] - nkb * nx;
                let ky = k3[1] - nkb * ny;
                let kt = (1.0 - nkb * nkb).max(0.0).sqrt();
                let reach_s = if kt > 1e-3 {
                    STRAND_CAST_H * px * (kt / nkb.max(0.2)).min(3.0) / kt
                } else {
                    0.0
                };
                let cx = (x as f32 + kx * reach_s).clamp(0.0, rb as f32 - 1.001);
                let cy = (y as f32 + ky * reach_s).clamp(0.0, rb as f32 - 1.001);
                let cx0 = cx as usize;
                let cy0 = cy as usize;
                let tx = cx - cx0 as f32;
                let ty = cy - cy0 as f32;
                let ci = cy0 * rb + cx0;
                let caster = (strand_buf[ci] * (1.0 - tx) + strand_buf[ci + 1] * tx) * (1.0 - ty)
                    + (strand_buf[ci + rb] * (1.0 - tx) + strand_buf[ci + rb + 1] * tx) * ty;
                cast = (caster - strand.max(0.0)).max(0.0) * smooth(0.0, 0.3, nkb);
            }
            let mut m = m0
                * (1.0
                    + strand_k
                        * (strand * (0.45 + 0.55 * fib[2]) * (0.75 + 0.5 * s)
                            - STRAND_CAST * cast))
                + 0.04 * (fib[10] - 0.5)
                + PILE_BIAS * depth_k * on;
            let edge_d = (xx + o)
                .min(FILM_SPAN - o - xx)
                .min(yy + o)
                .min(FILM_SPAN - o - yy);
            let fade_e = if edge_d >= 8.0 {
                1.0
            } else if edge_d <= 0.0 {
                0.0
            } else {
                let t = edge_d / 8.0;
                t * t * (3.0 - 2.0 * t)
            };
            m = 1.0 + (m - 1.0) * fade_e;
            if m < 1.0 {
                dark[i] =
                    (255.0 * (((1.0 - m) * lbs) / (lbs - GAP_REF).max(0.4)).min(1.0)).round() as u8;
            } else {
                light[i] = (255.0
                    * ((PILE_LIFT * (TIP_TOP - 1.0) * lbs) / (TIP_TOP - lbs)).min(1.0)
                    * (m - 1.0)
                    * (1.0 - 0.7 * crease))
                    .clamp(0.0, 1.0)
                    .round() as u8;
            }
            let lit = (4.0 * (m0 - 1.0)).clamp(-2.0, 2.0);
            let xl = x >> 1;
            let yl = y >> 1;
            if xl < rl && yl < rl {
                let j = yl * rl + xl;
                lock[j] += 0.25 * h0;
                lock_lit[j] += 0.25 * lit;
            }
            let tip_k = smooth(-0.4, 1.0, lit) * (0.65 + 0.25 * strand).clamp(0.0, 1.4);
            glint[i] = (255.0 * (fib[0] * tip_k).min(1.0)).round() as u8;
            back[i] =
                (255.0 * (fib[1] * (0.5 + 0.4 * strand).clamp(0.0, 1.6)).min(1.0)).round() as u8;
            let on_pile = (0.85 + 0.15 * strand + 0.12 * lit).clamp(0.55, 1.3);
            soft_back[i] = (255.0 * (0.85 * fib[3] * on_pile).min(1.0)).round() as u8;
        }
    }
    {
        let count = (rl * rl) as f32;
        for &v in &lock {
            lsum += v;
            lsum2 += v * v;
        }
        let mean = lsum / count;
        let sdv = (lsum2 / count - mean * mean).max(1e-9).sqrt();
        for v in lock.iter_mut() {
            *v = ((*v - mean) / sdv).clamp(-2.5, 2.5);
        }
    }

    // hair strokes
    struct Kind {
        count: f32,
        len: f32,
        vary: f32,
        end_on: f32,
        width: f32,
        spread: f32,
        alpha: f32,
        deep: f32,
        lift: f32,
        light: f32,
        dark: f32,
        bend: f32,
        clump: [f32; 2],
        clump_w: f32,
        shadow: f32,
        shadow_w: f32,
        over: f32,
        crowd: bool,
        short: f32,
    }
    let fine = if r >= 512 {
        1.0
    } else if r >= 320 {
        0.45
    } else {
        0.0
    };
    let kinds = [
        Kind {
            count: 0.12,
            len: 1.2,
            vary: 1.0,
            end_on: 0.6,
            width: 0.09,
            spread: 30.0,
            alpha: 0.4,
            deep: 1.0,
            lift: 0.25,
            light: 0.46,
            dark: 0.34,
            bend: 0.93,
            clump: [1.0, 0.0],
            clump_w: 0.0,
            shadow: 0.0,
            shadow_w: 1.0,
            over: 0.0,
            crowd: false,
            short: 0.0,
        },
        Kind {
            count: 0.4 * fine,
            len: 0.9,
            vary: 1.2,
            end_on: 0.35,
            width: 0.13,
            spread: 100.0,
            alpha: 0.26,
            deep: 0.2,
            lift: 0.18,
            light: 0.55,
            dark: 0.06,
            bend: 1.2,
            clump: [1.0, (1.0 + 2.0 * clumps).round()],
            clump_w: 0.08 + 0.07 * clumps,
            shadow: 0.25,
            shadow_w: 2.0,
            over: 0.25,
            crowd: true,
            short: 1.0,
        },
    ];
    let shade_of =
        |v: f32| -> usize { (((v.clamp(-1.0, 1.0) + 1.0) / 2.0) * 15.0).round() as usize };
    let glint_of = |v: f32| -> usize { ((v * 6.0) as usize).min(5) };
    let area = FILM_SPAN * FILM_SPAN;
    let mut film_dark = Alpha::from_vec(rb, rb, dark);
    let mut film_light = Alpha::from_vec(rb, rb, light);
    let mut film_glint = Alpha::from_vec(rb, rb, glint);
    let mut film_back = Alpha::from_vec(rb, rb, back);
    let film_soft = Alpha::from_vec(rb, rb, soft_back);
    for kind in &kinds {
        if kind.count <= 0.0 {
            continue;
        }
        let mut buckets: Vec<PathBuilder> = (0..16).map(|_| PathBuilder::new()).collect();
        let mut glints: Vec<PathBuilder> = (0..6).map(|_| PathBuilder::new()).collect();
        let mut backs: Vec<PathBuilder> = (0..6).map(|_| PathBuilder::new()).collect();
        let mut shades: Vec<PathBuilder> = (0..3).map(|_| PathBuilder::new()).collect();
        let mut over = PathBuilder::new();
        let count =
            ((area * kind.count * k_den * if kind.crowd { 2.0 } else { 1.0 }).round()) as usize;
        let mut hx = [0.0f32; 5];
        let mut hy = [0.0f32; 5];
        for n in 0..count {
            let xx = rand.next() * FILM_SPAN - o;
            let yy = rand.next() * FILM_SPAN - o;
            let r0 = rand.next();
            let r1 = rand.next();
            let r2 = rand.next();
            let r3 = rand.next();
            let r4 = rand.next();
            let r5 = rand.next();
            if r5 * 8.0
                > (xx + o)
                    .min(FILM_SPAN - o - xx)
                    .min(yy + o)
                    .min(FILM_SPAN - o - yy)
            {
                continue;
            }
            if kind.crowd
                && r5
                    > (if sd_at(xx, yy) > 0.0 {
                        0.5 / grid_at(&nz_g, xx, yy).max(0.25)
                    } else {
                        0.5
                    })
            {
                continue;
            }
            let (fx, fy) = dir_at(xx, yy);
            let tilt_s = grid_at(&tilt_g, xx, yy);
            let vl = grid_at(&fore_g, xx, yy) / PILE_COS;
            let ang = (r1 - 0.5) * (0.6 + 0.8 * curl) * (1.0 - 0.5 * tilt_s);
            let (ca, sa) = (ang.cos(), ang.sin());
            let ux = fx * ca - fy * sa;
            let uy = fx * sa + fy * ca;
            let longer = if r2 < 0.1 { 1.35 + 0.6 * r3 } else { 1.0 };
            let len = (kind.len + kind.vary * r3 * r3)
                * px
                * k_len
                * longer
                * vl
                * (1.0 - kind.end_on + 1.25 * kind.end_on * tilt_s);
            let fib = fibre_light(xx, yy, ux, uy, 0.1 + 0.5 * hash2(n as i32, 7));
            let mut v = 0.22 * lock_at(&lock, xx, yy) + 0.3 * lock_at(&lock_lit, xx, yy);
            let lit = r0 < kind.light * (0.4 + 0.6 * fib[2]);
            if lit {
                v += 0.2 + 0.28 * r4;
            } else if r0 < kind.light + kind.dark {
                v -= 0.2 + 0.28 * r4;
            }
            v += kind.lift;
            v += 0.45 * (fib[2] - 0.5) - 0.35 * fib[8];
            if v > 0.0 {
                v *= 0.3 + 0.7 * fib[2];
            }
            let deepen = 0.15 * (1.0 - fib[2]);
            let x0 = (xx + PAD) * px;
            let y0 = (yy + PAD) * px;
            let b1 = (rand.next() - 0.5) * kind.bend * curl * len;
            let b2 = if r4 < 0.3 {
                -0.6 * b1
            } else {
                b1 * (0.6 + 0.8 * rand.next())
            };
            let hairs = kind.clump[0] + (rand.next() * (kind.clump[1] + 1.0)) as i32 as f32;
            let w_c = kind.clump_w * px * k_len.min(1.5);
            for k in 0..hairs as i32 {
                let oo = if hairs > 1.0 { rand.next() - 0.5 } else { 0.0 };
                let f = if hairs > 1.0 {
                    0.7 + 0.3 * rand.next()
                } else {
                    1.0
                };
                let j = if hairs > 1.0 {
                    (rand.next() - 0.5) * 0.3
                } else {
                    0.0
                };
                let rx = x0 - uy * oo * w_c;
                let ry = y0 + ux * oo * w_c;
                let lk = len * f;
                let e1 = b1 + j * lk;
                let e2 = b2 + j * lk - oo * w_c * 0.7;
                let c1x = rx + ux * lk * 0.33 - uy * e1;
                let c1y = ry + uy * lk * 0.33 + ux * e1;
                let c2x = rx + ux * lk * 0.67 - uy * e2;
                let c2y = ry + uy * lk * 0.67 + ux * e2;
                let x1 = rx + ux * lk - uy * (b2 - oo * w_c * 0.75);
                let y1 = ry + uy * lk + ux * (b2 - oo * w_c * 0.75);
                let sg = if lk < 4.0 {
                    2
                } else if lk < 10.0 {
                    3
                } else {
                    4
                };
                for s in 0..=sg {
                    let t = s as f32 / sg as f32;
                    let q = 1.0 - t;
                    let a = q * q * q;
                    let b = 3.0 * q * q * t;
                    let c = 3.0 * q * t * t;
                    let d = t * t * t;
                    hx[s] = a * rx + b * c1x + c * c2x + d * x1;
                    hy[s] = a * ry + b * c1y + c * c2y + d * y1;
                }
                let vh = v + if hairs > 1.0 {
                    (rand.next() - 0.5) * 0.25
                } else {
                    0.0
                };
                let rise = 0.3 + 0.15 * fib[2];
                for s in 0..sg {
                    let idx = shade_of(
                        vh - 0.3 - deepen * (1.0 - s as f32 / sg as f32)
                            + rise * (s as f32 + 0.5) / sg as f32,
                    );
                    buckets[idx].move_to(hx[s], hy[s]);
                    buckets[idx].line_to(hx[s + 1], hy[s + 1]);
                }
                if lit && kind.over > 0.0 {
                    over.move_to(hx[1], hy[1]);
                    for s in 2..=sg {
                        over.line_to(hx[s], hy[s]);
                    }
                }
                if k == 0 && kind.shadow > 0.0 && fib[2] > 0.25 && v > -0.2 {
                    let kk = kind.shadow * px * k_len.min(1.0);
                    let sh = &mut shades[((fib[2] - 0.25) * 4.0).min(2.0) as usize];
                    for s in 1..=sg {
                        let t = s as f32 / sg as f32;
                        let ox = -lx * kk * t;
                        let oy = -ly * kk * t + 0.3 * kk * t;
                        if s == 1 {
                            sh.move_to(hx[0], hy[0]);
                        }
                        sh.line_to(hx[s] + ox, hy[s] + oy);
                    }
                }
                let h = sg >> 1;
                if fib[0] > 0.3 {
                    let gb = &mut glints[glint_of(fib[0])];
                    gb.move_to(hx[h], hy[h]);
                    for s in h + 1..=sg {
                        gb.line_to(hx[s], hy[s]);
                    }
                }
                if fib[1] > 0.1 {
                    let bb = &mut backs[glint_of(fib[1])];
                    bb.move_to(hx[h], hy[h]);
                    for s in h + 1..=sg {
                        bb.line_to(hx[s], hy[s]);
                    }
                }
            }
        }
        let short_k = k_len.min(1.0).powf(kind.short);
        let lw = (kind.width * px * (0.5 + 0.5 * short_k)).max(0.45);
        if kind.over > 0.0
            && let Some(p) = over.finish()
        {
            film_dark.stroke(&p, lw, kind.over * short_k, true);
        }
        for (i, shade) in shades.into_iter().enumerate() {
            if kind.shadow > 0.0
                && let Some(p) = shade.finish()
            {
                film_dark.stroke(
                    &p,
                    lw * kind.shadow_w,
                    0.03 * (i as f32 + 1.0) * short_k,
                    true,
                );
            }
        }
        for (i, bucket) in buckets.into_iter().enumerate() {
            let Some(p) = bucket.finish() else { continue };
            let v = FUR_MID + (((i as f32 / 15.0) * 2.0) - 1.0) * kind.spread;
            if v < FUR_MID {
                let a = kind.alpha * short_k * kind.deep * (1.0 - v / FUR_MID);
                film_dark.stroke(&p, lw, a, false);
            } else {
                let a = kind.alpha * short_k * (v - FUR_MID) / (255.0 - FUR_MID);
                film_light.stroke(&p, lw, a, false);
            }
        }
        for (i, p) in glints.into_iter().enumerate() {
            if let Some(p) = p.finish() {
                film_glint.stroke(&p, lw, 0.4 * (i as f32 + 0.5) / 6.0, false);
            }
        }
        for (i, p) in backs.into_iter().enumerate() {
            if let Some(p) = p.finish() {
                film_back.stroke(&p, lw, 0.8 * (i as f32 + 0.5) / 6.0, false);
            }
        }
    }

    // fringe and tufts
    let mut cells: Vec<usize> = Vec::new();
    for i in 0..n * n {
        if form.sd[i] > -1.6 && form.sd[i] < 2.2 {
            cells.push(i);
        }
    }
    let cu = SPAN / n as f32;
    let edge_tries = ((cells.len() as f32
        * cu
        * cu
        * 24.0
        * (0.3 + 1.4 * fuzz)
        * k_den.sqrt()
        * (0.5 + 0.5 * (r as f32 / 512.0).min(1.0))
        * (1.0 - 0.55 * fine))
        .round()) as usize;
    let mut fringe = PathBuilder::new();
    let mut fringe_back: Vec<PathBuilder> = (0..6).map(|_| PathBuilder::new()).collect();
    for _ in 0..edge_tries {
        let c = cells[(rand.next() * cells.len() as f32) as usize];
        let xx = ((c % n) as f32 + rand.next()) * cu - PAD;
        let yy = (((c / n) as f32) + rand.next()) * cu - PAD;
        let d0 = sd_at(xx, yy);
        if !(-1.2..=1.8).contains(&d0) {
            continue;
        }
        let d = d0 + tuft_at(&lock, xx, yy);
        if !(-0.2..=1.2).contains(&d) {
            continue;
        }
        let (fx, fy) = flow(xx, yy);
        let (ox, oy) = outward(xx, yy);
        let ang = (rand.next() - 0.5) * 1.1;
        let (ca, sa) = (ang.cos(), ang.sin());
        let mut ux = fx * ca - fy * sa;
        let mut uy = fx * sa + fy * ca;
        let r1 = rand.next();
        let hang = 1.0 + 0.5 * gravity * oy.max(0.0) - 0.35 * gravity * (-oy).max(0.0);
        let mut fl = (0.3 + 1.5 * r1 * r1 * r1) * px * k_len * (0.5 + fuzz) * hang;
        let x0 = (xx + PAD) * px;
        let y0 = (yy + PAD) * px;
        let mut bend = (rand.next() - 0.5) * 1.17 * curl * fl;
        if oy < -0.3 && rand.next() < 0.06 * (0.3 + fuzz) {
            ux = ox * 0.8 + (rand.next() - 0.5) * 0.6;
            uy = oy * 0.8;
            fl *= 1.2;
            bend = if rand.next() < 0.5 { -1.0 } else { 1.0 } * (0.25 + 0.3 * gravity) * fl;
        }
        fringe.move_to(x0, y0);
        let qx = x0 + ux * fl * 0.5 - uy * bend;
        let qy = y0 + uy * fl * 0.5 + ux * bend;
        let ex = x0 + ux * fl;
        let ey = y0 + uy * fl + gravity * fl * 0.25;
        fringe.quad_to(qx, qy, ex, ey);
        let fib = fibre_light(xx, yy, ux, uy, 0.3);
        if fib[1] > 0.08 {
            let fb = &mut fringe_back[glint_of(fib[1])];
            fb.move_to(x0, y0);
            fb.quad_to(qx, qy, ex, ey);
        }
    }
    let mut tufts = PathBuilder::new();
    let mut tuft_tips: Vec<PathBuilder> = (0..3).map(|_| PathBuilder::new()).collect();
    let mut tuft_back: Vec<PathBuilder> = (0..6).map(|_| PathBuilder::new()).collect();
    {
        let band = cells.len() as f32 * cu * cu;
        let around = band / 3.8;
        let count = ((around / (0.45 + 0.3 * k_len))
            * (0.3 + 0.8 * fuzz)
            * k_den.sqrt()
            * (1.0 + 0.6 * fine))
            .round() as usize;
        let mut n_done = 0usize;
        let mut tries = 0usize;
        while n_done < count && tries < 6 * count {
            tries += 1;
            let c = cells[(rand.next() * cells.len() as f32) as usize];
            let xx = ((c % n) as f32 + rand.next()) * cu - PAD;
            let yy = ((c / n) as f32 + rand.next()) * cu - PAD;
            let r0 = rand.next();
            let r1 = rand.next();
            let r2 = rand.next();
            let r3 = rand.next();
            let d0 = sd_at(xx, yy);
            if !(-0.5..=1.6).contains(&d0) {
                continue;
            }
            let d = d0 + tuft_at(&lock, xx, yy);
            if !(0.0..=0.9).contains(&d) {
                continue;
            }
            if lock_at(&lock, xx, yy) < -0.4 + 0.8 * r3 {
                continue;
            }
            n_done += 1;
            let (fx, fy) = flow(xx, yy);
            let (ox, oy) = outward(xx, yy);
            let a = (r0 - 0.5) * 0.8 * (0.5 + curl);
            let (ca, sa) = (a.cos(), a.sin());
            let mut sx = 0.75 * ox + 0.25 * fx;
            let mut sy = 0.75 * oy + 0.25 * fy;
            let mut ex = 0.55 * fx + 0.45 * ox;
            let mut ey = 0.55 * fy + 0.45 * oy + 0.35 * gravity;
            let sl = (sx * sx + sy * sy).sqrt().max(1e-6);
            let el = (ex * ex + ey * ey).sqrt().max(1e-6);
            let (nsx, nsy) = ((sx * ca - sy * sa) / sl, (sx * sa + sy * ca) / sl);
            let (nex, ney) = ((ex * ca - ey * sa) / el, (ex * sa + ey * ca) / el);
            sx = nsx;
            sy = nsy;
            ex = nex;
            ey = ney;
            let hang = 1.0 + 0.5 * gravity * oy.max(0.0) - 0.35 * gravity * (-oy).max(0.0);
            let lt = (0.5 + 1.1 * r1 * r1) * px * k_len * (0.5 + fuzz) * hang;
            let wt = (0.3 + 0.35 * r2) * px * k_len.min(1.5);
            let x0 = (xx + PAD) * px;
            let y0 = (yy + PAD) * px;
            let cx = x0 + sx * lt * 0.5;
            let cy = y0 + sy * lt * 0.5;
            let tx = x0 + (sx * 0.4 + ex * 0.6) * lt;
            let ty = y0 + (sy * 0.4 + ey * 0.6) * lt;
            let qx = -sy;
            let qy = sx;
            let fib = fibre_light(xx, yy, ex, ey, 0.4);
            let fb_idx = if fib[1] > 0.08 {
                Some(glint_of(fib[1]))
            } else {
                None
            };
            let tt = ((fib[2] * 3.0) as usize).min(2);
            let hairs = 4 + (rand.next() * 5.0) as i32;
            for _ in 0..hairs {
                let oo = rand.next() - 0.5;
                let f = 0.7 + 0.3 * rand.next();
                let w = (rand.next() - 0.5) * 0.35;
                let hx0 = x0 + qx * oo * wt;
                let hy0 = y0 + qy * oo * wt;
                let hcx = cx + qx * oo * wt * 0.6;
                let hcy = cy + qy * oo * wt * 0.6;
                let htx = hx0 + (tx + qx * (oo * 0.25 + w) * wt - hx0) * f;
                let hty = hy0 + (ty + qy * (oo * 0.25 + w) * wt - hy0) * f;
                tufts.move_to(hx0, hy0);
                tufts.quad_to(hcx, hcy, htx, hty);
                let mx = 0.25 * hx0 + 0.5 * hcx + 0.25 * htx;
                let my = 0.25 * hy0 + 0.5 * hcy + 0.25 * hty;
                tuft_tips[tt].move_to(mx, my);
                tuft_tips[tt].quad_to(0.5 * (hcx + htx), 0.5 * (hcy + hty), htx, hty);
                if let Some(idx) = fb_idx {
                    tuft_back[idx].move_to(mx, my);
                    tuft_back[idx].quad_to(0.5 * (hcx + htx), 0.5 * (hcy + hty), htx, hty);
                }
            }
        }
    }
    let fringe_w = (0.08 * px).max(0.35);
    let tuft_w = (0.11 * px).max(0.4);
    let fringe_light_alpha = 0.5 * ((140.0 - FUR_MID) / (255.0 - FUR_MID));
    let fringe_path = fringe.finish().map(Arc::new);
    let tufts_path = tufts.finish().map(Arc::new);
    let tuft_tip_paths: Vec<Option<Arc<Path>>> = tuft_tips
        .into_iter()
        .map(|p| p.finish().map(Arc::new))
        .collect();
    let fringe_back_paths: Vec<Option<Arc<Path>>> = fringe_back
        .into_iter()
        .map(|p| p.finish().map(Arc::new))
        .collect();
    let tuft_back_paths: Vec<Option<Arc<Path>>> = tuft_back
        .into_iter()
        .map(|p| p.finish().map(Arc::new))
        .collect();
    // the halo
    let h_out = 0.5 + 1.2 * fuzz;
    let reach_out = h_out + tuft_k;
    let halo_a = HALO_A * (1.0 - 0.6 * fine);
    let halo_streak = HALO_STREAK * (1.0 + 1.2 * fine);
    let hn = ((HALO_HAIR * k_len * px).round() as usize).max(1);
    let hs1 = (hn + 1) as f32;
    let hsd = (W_SD * (1.0 + (hn as f32 * (2.0 * hn as f32 + 1.0)) / (3.0 * hs1)).sqrt()) / hs1;
    let off = (FILM_M * px).round() as i32;
    let mut mi = vec![0u8; r * r];
    let mut zi = vec![0u8; r * r];
    let mut ci = vec![0u8; r * r];
    for y in 0..r {
        let yy = (y as f32 + 0.5) / px - PAD;
        for x in 0..r {
            let xx = (x as f32 + 0.5) / px - PAD;
            let k = y * r + x;
            let d0 = sd_at(xx, yy);
            if d0 >= tuft_k + HALO_CORE {
                mi[k] = 255;
                ci[k] = 255;
                continue;
            }
            let d = if d0 <= -reach_out {
                d0
            } else {
                d0 + tuft_at(&lock, xx, yy)
            };
            if d >= 0.0 {
                mi[k] = 255;
                ci[k] = (255.0 * smooth(0.25 * HALO_CORE, HALO_CORE, d)).round() as u8;
                continue;
            }
            let z = (-d / (2.0 * h_out)).min(1.0);
            let nb = grid_at(&nx_g, xx, yy) * b3[0] + grid_at(&ny_g, xx, yy) * b3[1];
            zi[k] = (255.0
                * HALO_LIFT
                * z
                * z
                * (3.0 - 2.0 * z)
                * (0.45 + 0.55 * smooth(-0.3, 0.6, nb)))
            .round() as u8;
            if d <= -h_out {
                continue;
            }
            let oo = 1.0 + d / h_out;
            let f = oo * oo * (3.0 - 2.0 * oo);
            let (ux, uy) = free_dir_at(xx, yy);
            let mut acc = 0.0f32;
            for t in -(hn as i32)..=(hn as i32) {
                let sx = x as i32 + off + (ux * t as f32).round() as i32;
                let sy = y as i32 + off + (uy * t as f32).round() as i32;
                let w = if sx < 0 || sy < 0 || sx >= rb as i32 || sy >= rb as i32 {
                    W_MEAN
                } else {
                    white[sy as usize * rb + sx as usize]
                };
                acc += w * (1.0 - t.abs() as f32 / hs1);
            }
            let strand = (acc / hs1 - W_MEAN) / hsd;
            mi[k] = (255.0 * (f * (halo_a + halo_streak * strand)).clamp(0.0, 1.0)).round() as u8;
        }
    }
    let soft_mask = {
        let n2 = form.n;
        let mut a = Alpha::new(r, r);
        for y in 0..r {
            for x in 0..r {
                let d = bilerp(
                    &form.sd,
                    n2,
                    (((x as f32 + 0.5) / px) / SPAN) * n2 as f32 - 0.5,
                    (((y as f32 + 0.5) / px) / SPAN) * n2 as f32 - 0.5,
                );
                let t = (d + 0.2) / 0.6;
                a.a[y * r + x] = (255.0
                    * if t >= 1.0 {
                        1.0
                    } else if t <= 0.0 {
                        0.0
                    } else {
                        t * t * (3.0 - 2.0 * t)
                    })
                .round() as u8;
            }
        }
        a
    };
    let mut mask = Alpha::from_vec(r, r, mi);
    if let Some(p) = &fringe_path {
        mask.stroke(p, fringe_w, 0.45, false);
        film_dark.stroke(p, fringe_w, 0.7, true);
        film_light.stroke(p, fringe_w, fringe_light_alpha, false);
    }
    if let Some(p) = &tufts_path {
        mask.stroke(p, tuft_w, 0.8, false);
    }
    for (i, p) in tuft_tip_paths.iter().enumerate().take(3) {
        if let Some(p) = p {
            film_dark.stroke(p, tuft_w, 0.6, true);
            film_light.stroke(p, tuft_w, 0.08 + 0.11 * i as f32, false);
        }
    }
    for (i, p) in tuft_back_paths.iter().enumerate() {
        if let Some(p) = p {
            film_back.stroke(p, tuft_w, (i as f32 + 0.5) / 6.0, false);
        }
    }
    for (i, p) in fringe_back_paths.iter().enumerate() {
        if let Some(p) = p {
            film_back.stroke(p, fringe_w, (i as f32 + 0.5) / 6.0, false);
        }
    }
    Fur {
        r,
        mask,
        haze: Alpha::from_vec(r, r, zi),
        core: Alpha::from_vec(r, r, ci),
        soft: soft_mask,
        dark: film_dark,
        light: film_light,
        glint: film_glint,
        back: film_back,
        back_soft: film_soft,
        fringe: fringe_path,
        tufts: tufts_path,
        fringe_width: fringe_w,
        tuft_width: tuft_w,
    }
}

// ── drawing ────────────────────────────────────────────────────────────

struct Rays {
    d: Vec<Vec<f32>>,
    c: Vec<Vec<[u8; 3]>>,
}

fn rays_of(tex: &[u8], form: &Form, c0: (f32, f32)) -> Rays {
    let n = form.n;
    let to_g = |v: f32| ((v + PAD) / SPAN) * n as f32 - 0.5;
    let mut d_out = Vec::with_capacity(24);
    let mut c_out = Vec::with_capacity(24);
    for s in 0..24 {
        let phi = (s as f32 / 24.0) * 2.0 * PI;
        let (ux, uy) = (phi.cos(), phi.sin());
        let mut ds = Vec::new();
        let mut cs = Vec::new();
        let mut r = 0.0f32;
        while r < 80.0 {
            let x = c0.0 + ux * r;
            let y = c0.1 + uy * r;
            let d = bilerp(&form.sd, n, to_g(x), to_g(y));
            if d < 0.0 {
                break;
            }
            let gx = to_g(x).round() as i32;
            let gy = to_g(y).round() as i32;
            if gx < 0 || gy < 0 || gx >= n as i32 || gy >= n as i32 {
                break;
            }
            let k = (gy as usize * n + gx as usize) * 4;
            ds.push(d);
            cs.push([tex[k], tex[k + 1], tex[k + 2]]);
            r += 0.5;
        }
        d_out.push(ds);
        c_out.push(cs);
    }
    Rays { d: d_out, c: c_out }
}

fn ray_color(rays: &Rays, angle: f32, inset: f32, dark: f32) -> [f32; 3] {
    let f = ((angle / (2.0 * PI)).rem_euclid(1.0)) * 24.0;
    let s0 = f.floor() as usize % 24;
    let s1 = (s0 + 1) % 24;
    let t = f - f.floor();
    let color = |s: usize| -> [f32; 3] {
        let d = &rays.d[s];
        let c = &rays.c[s];
        if c.is_empty() {
            return [0.0, 0.0, 0.0];
        }
        let mut j = 0;
        for (i, &v) in d.iter().enumerate() {
            if v >= inset {
                j = i;
            }
        }
        let k = 1.0 - dark;
        [
            f32::from(c[j][0]) * k,
            f32::from(c[j][1]) * k,
            f32::from(c[j][2]) * k,
        ]
    };
    let a = color(s0);
    let b = color(s1);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn ray_color_ray(rays: &Rays, s: usize, inset: f32, dark: f32) -> [f32; 3] {
    let d = &rays.d[s % 24];
    let c = &rays.c[s % 24];
    if c.is_empty() {
        return [0.0, 0.0, 0.0];
    }
    let mut j = 0;
    for (i, &v) in d.iter().enumerate() {
        if v >= inset {
            j = i;
        }
    }
    let k = 1.0 - dark;
    [
        f32::from(c[j][0]) * k,
        f32::from(c[j][1]) * k,
        f32::from(c[j][2]) * k,
    ]
}

fn brightest(rays: &Rays) -> [u8; 3] {
    let mut best = -1.0f32;
    let mut out = [0u8; 3];
    for c in &rays.c {
        for p in c {
            let l = 0.2126 * f32::from(p[0]) + 0.7152 * f32::from(p[1]) + 0.0722 * f32::from(p[2]);
            if l > best {
                best = l;
                out = *p;
            }
        }
    }
    let q = |v: u8| ((f32::from(v) / 4.0).round() * 4.0).min(255.0) as u8;
    [q(out[0]), q(out[1]), q(out[2])]
}

fn haze_color(rays: &Rays, base: Rgba, rim: f32, vivid: f32) -> [f32; 3] {
    let b = brightest(rays);
    let lin = base.linear();
    let e = [
        linear_to_srgb(lin[0]) * 255.0,
        linear_to_srgb(lin[1]) * 255.0,
        linear_to_srgb(lin[2]) * 255.0,
    ];
    let mx = e[0].max(e[1]).max(e[2]).max(1.0);
    let k = (0.75 * rim).min(0.85);
    let pale = 0.45 + 0.55 * vivid.min(1.0);
    let q = |i: usize| {
        f32::from(b[i]) + (255.0 + ((255.0 * e[i]) / mx - 255.0) * pale - f32::from(b[i])) * k
    };
    [q(0), q(1), q(2)]
}

fn linear_to_srgb(v: f64) -> f32 {
    if v <= 0.0 {
        0.0
    } else if v >= 1.0 {
        1.0
    } else if v <= 0.003_130_8 {
        (12.92 * v) as f32
    } else {
        (1.055 * v.powf(1.0 / 2.4) - 0.055) as f32
    }
}

fn blend_over_px(pixmap: &mut Pixmap, x: usize, y: usize, color: [f32; 3], alpha: f32) {
    if alpha <= 0.0 {
        return;
    }
    let idx = (y * pixmap.width() as usize + x) * 4;
    let data = pixmap.data_mut();
    let a = alpha.clamp(0.0, 1.0);
    for ch in 0..3 {
        let dst = f32::from(data[idx + ch]) / 255.0;
        let src = color[ch] / 255.0 * a;
        data[idx + ch] = ((src + dst * (1.0 - a)).clamp(0.0, 1.0) * 255.0).round() as u8;
    }
    let da = f32::from(data[idx + 3]) / 255.0;
    data[idx + 3] = ((a + da * (1.0 - a)).clamp(0.0, 1.0) * 255.0).round() as u8;
}

#[allow(clippy::too_many_arguments)]
fn fill_angular(
    body: &mut Pixmap,
    path: &Path,
    inverse: Affine,
    rays: &Rays,
    inset: f32,
    dark: f32,
    c0: (f32, f32),
    soft: Option<(&Alpha, f32)>,
) {
    let Some(mut mask) = Mask::new(body.width(), body.height()) else {
        return;
    };
    mask.fill_path(path, FillRule::Winding, true, Transform::identity());
    let bounds = path.bounds();
    let w = body.width() as usize;
    let h = body.height() as usize;
    let x0 = (bounds.left().floor() as i32).clamp(0, w as i32) as usize;
    let x1 = (bounds.right().ceil() as i32).clamp(0, w as i32) as usize;
    let y0 = (bounds.top().floor() as i32).clamp(0, h as i32) as usize;
    let y1 = (bounds.bottom().ceil() as i32).clamp(0, h as i32) as usize;
    let mdata = mask.data();
    for y in y0..y1 {
        for x in x0..x1 {
            let cov = f32::from(mdata[y * w + x]) / 255.0;
            if cov <= 0.0 {
                continue;
            }
            let (dx, dy) = inverse.point(x as f32 + 0.5, y as f32 + 0.5);
            let mut a = cov;
            if let Some((sm, scale)) = soft {
                // soft mask covers [-PAD, 100 + PAD] at its own resolution
                let sx = ((dx + PAD) / SPAN * sm.w as f32 * scale) - 0.5;
                let sy = ((dy + PAD) / SPAN * sm.h as f32 * scale) - 0.5;
                a *= sm.sample(sx, sy);
            }
            if a <= 0.0 {
                continue;
            }
            let angle = (dy - c0.1).atan2(dx - c0.0);
            let color = ray_color(rays, angle, inset, dark);
            blend_over_px(body, x, y, color, a);
        }
    }
}

fn apply_dark_ramp(
    body: &mut Pixmap,
    inv: Affine,
    ex: f32,
    ey: f32,
    hn: f32,
    span: f32,
    back_shade: f32,
) {
    let w = body.width() as usize;
    let h = body.height() as usize;
    let data = body.data_mut();
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) * 4;
            let a = data[idx + 3];
            if a == 0 {
                continue;
            }
            let (dx, dy) = inv.point(x as f32 + 0.5, y as f32 + 0.5);
            let t = ((hn - (dx * ex + dy * ey)) / span).clamp(0.0, 1.0);
            let alpha = back_shade * (1.0 - (-2.4 * t).exp()) * t * t * (3.0 - 2.0 * t);
            if alpha <= 0.0 {
                continue;
            }
            let f = 1.0 - alpha;
            for ch in 0..3 {
                data[idx + ch] = (f32::from(data[idx + ch]) * f).round() as u8;
            }
        }
    }
}

fn gap_film(base: Rgba, vivid: f32) -> ([f32; 3], f32) {
    let lin = base.linear();
    let e = [
        linear_to_srgb(lin[0]),
        linear_to_srgb(lin[1]),
        linear_to_srgb(lin[2]),
    ];
    let mx = e[0].max(e[1]).max(e[2]).max(1e-3);
    let lu = 0.2126 * e[0] + 0.7152 * e[1] + 0.0722 * e[2];
    let pale = e[0].min(e[1]).min(e[2]) / mx;
    let kd = GAP_DEEP + GAP_SCATTER * lu * lu * pale;
    let vq = ((vivid.min(1.0) * 10.0).round() / 10.0).clamp(0.0, 1.0);
    let ch = |v: f32| 255.0 * kd * v * (v / mx).powf(3.2 + 3.0 * vq);
    let color = [ch(e[0]), ch(e[1]), ch(e[2])];
    let deep = (0.2126 * color[0] + 0.7152 * color[1] + 0.0722 * color[2]) / 255.0 / lu.max(1e-3);
    let scale = ((1.0 - GAP_REF) / (1.0 - deep).max(0.05)).sqrt().min(1.0);
    (color, scale)
}

fn tip_film(base: Rgba, vivid: f32) -> [f32; 3] {
    let lin = base.linear();
    let e = [
        linear_to_srgb(lin[0]),
        linear_to_srgb(lin[1]),
        linear_to_srgb(lin[2]),
    ];
    let mx = e[0].max(e[1]).max(e[2]).max(1.0);
    let v = (TIP_TINT + (1.0 - TIP_TINT) * vivid.clamp(0.0, 1.0)).min(1.0);
    let v = (v * 10.0).round() / 10.0;
    [
        255.0 + (255.0 * e[0] / mx - 255.0) * v,
        255.0 + (255.0 * e[1] / mx - 255.0) * v,
        255.0 + (255.0 * e[2] / mx - 255.0) * v,
    ]
}

fn sheen_film(base: Rgba, vivid: f32, p: &Material) -> Option<Vec<([f32; 3], f32)>> {
    let hk = (0.45 * p.highlight * p.shine).min(1.0);
    let rk = (1.1 * p.rim).min(1.0);
    if hk <= 0.01 && rk <= 0.01 {
        return None;
    }
    let vivid = vivid.min(1.0);
    let soft = ((p.back_soft * 20.0).round() / 20.0).clamp(0.0, 1.0);
    let lin = base.linear();
    let e = [
        linear_to_srgb(lin[0]) * 255.0,
        linear_to_srgb(lin[1]) * 255.0,
        linear_to_srgb(lin[2]) * 255.0,
    ];
    let mx = e[0].max(e[1]).max(e[2]).max(1.0);
    let mix = |k: f32| {
        [
            255.0 + (255.0 * e[0] / mx - 255.0) * k,
            255.0 + (255.0 * e[1] / mx - 255.0) * k,
            255.0 + (255.0 * e[2] / mx - 255.0) * k,
        ]
    };
    Some(vec![
        (mix(0.65 + 0.35 * vivid), rk * (1.0 - 0.85 * soft)),
        (mix(0.7 + 0.3 * vivid), rk * soft),
        (mix(0.45 + 0.55 * vivid), hk),
    ])
}

#[allow(clippy::too_many_arguments)]
fn composite_film(
    body: &mut Pixmap,
    alpha: &Alpha,
    color: [f32; 3],
    alpha_scale: f32,
    to_design: Affine,
) {
    let w = body.width() as usize;
    let h = body.height() as usize;
    let inv = to_design.inverse();
    let data = body.data_mut();
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) * 4;
            let da = f32::from(data[idx + 3]) / 255.0;
            if da <= 0.0 {
                continue;
            }
            let (dx, dy) = inv.point(x as f32 + 0.5, y as f32 + 0.5);
            let fx = ((dx + PAD + FILM_M) / FILM_SPAN * alpha.w as f32) - 0.5;
            let fy = ((dy + PAD + FILM_M) / FILM_SPAN * alpha.h as f32) - 0.5;
            let a = alpha.sample(fx, fy) * alpha_scale;
            if a <= 0.0 {
                continue;
            }
            for ch in 0..3 {
                let dst = f32::from(data[idx + ch]) / 255.0;
                let src = color[ch] / 255.0 * a;
                data[idx + ch] =
                    ((src * da + dst * (1.0 - a)).clamp(0.0, 1.0) * 255.0).round() as u8;
            }
            data[idx + 3] = (da * 255.0).round() as u8;
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw(
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
    half_depth: f32,
    dev: f32,
) -> bool {
    let n = tier_for(dev);
    let Some(form) = form::form_for(&cfg.type_key, &cfg.path, n, half_depth) else {
        return false;
    };
    let (bx, by) = match cfg.back_light {
        Some(deg) if deg.is_finite() => {
            let a = deg.to_radians();
            (Some(a.sin()), Some(-a.cos()))
        }
        _ => (None, None),
    };
    let rig = Rig {
        cy,
        sy,
        cp,
        sp,
        facing,
        roll: 0.0,
        lx,
        ly,
        bx,
        by,
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
    let linear = |c: Rgba| {
        let l = c.linear();
        [l[0] as f32, l[1] as f32, l[2] as f32]
    };
    let mc = build_fabric_matcap(linear(cfg.color), &frame, &material);
    let ao_k = (0.3 + 0.8 * cfg.shadow).min(1.2);
    let mut ao_mul = [0.0f32; 256];
    for (a, v) in ao_mul.iter_mut().enumerate() {
        *v = (1.0 - ao_k * (1.0 - a as f32 / 255.0)).max(0.0);
    }
    let lift = edge_lift(&form);
    let mut tex = vec![0u8; n * n * 4];
    shade_texels(&form, &mc, &ao_mul, Some(&lift), &mut tex);
    let cap_px = (SPAN * dev / 100.0).ceil();
    let fur = fur_for(&form, fur_tier(cap_px), lx, ly, cfg.fur);
    let levels = if dev <= 100.0 {
        14
    } else if dev <= 224.0 {
        20
    } else {
        26
    };
    let rel = relief_for(&form, levels, cfg.roundness);
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
    let near = if facing >= 0.0 { 1.0 } else { -1.0 };
    let dxs = if cy.abs() > 1e-5 { sy / cy } else { 0.0 };
    let dys = if (cy * cp).abs() > 1e-5 {
        -sp / (cy * cp)
    } else {
        0.0
    };
    let dl1 = (dxs * dxs + dys * dys).sqrt();
    let mut lead = 50.0f32;
    for k in 1..levels {
        lead = lead.max(50.0 * rel.scale[k] + rel.z[k] * dl1);
    }
    let (ex, ey) = if dl1 > 1e-6 {
        (near * dxs / dl1, near * dys / dl1)
    } else {
        (1.0, 0.0)
    };
    let back_shade = (0.55 * (cfg.shadow / 0.35).min(1.6)).min(0.7);
    let halos = halo_stretch(&form, &rel, levels, ex, ey, dl1);
    let center = (rel.cx[0], rel.cy[0]);
    let rays = rays_of(&tex, &form, center);
    let stretch = (lead + 50.0) / 100.0;
    let shift = (lead - 50.0) / 2.0;
    let a = 1.0 + (stretch - 1.0) * ex * ex;
    let b = (stretch - 1.0) * ex * ey;
    let d = 1.0 + (stretch - 1.0) * ey * ey;
    let cap_m = Affine([
        a,
        b,
        b,
        d,
        shift * ex - 50.0 * a - 50.0 * b,
        shift * ey - 50.0 * b - 50.0 * d,
    ])
    .then(rot)
    .then(body);

    let Some(mut buffer) = Pixmap::new(pixmap.width(), pixmap.height()) else {
        return false;
    };
    let slice_path = |k: usize, z: f32| -> Option<(Path, Affine)> {
        let sk = rel.scale[k];
        let local = Affine([
            sk,
            0.0,
            0.0,
            sk,
            rel.cx[k] - sk * rel.cx[0],
            rel.cy[k] - sk * rel.cy[0],
        ]);
        let full = if k == 0 {
            slice_at(z).then(body)
        } else {
            local.then(slice_at(z)).then(body)
        };
        cfg.path
            .as_ref()
            .clone()
            .transform(full.transform())
            .map(|p| (p, full))
    };
    // far half
    for k in (1..levels).rev() {
        let z = -near * rel.z[k];
        if let Some((path, full)) = slice_path(k, z) {
            let inset = HALO_INSET.max(rel.inset[k]);
            fill_angular(
                &mut buffer,
                &path,
                full.inverse(),
                &rays,
                inset,
                0.0,
                center,
                None,
            );
        }
    }
    // fabric's far ramp
    if dl1 > 1e-3 {
        let cache = rim_of(&form);
        let hn = reach(&cache.rim, -ex, -ey);
        let span = 1.3 * rel.top * dl1;
        apply_dark_ramp(
            &mut buffer,
            slice_at(0.0).then(body).inverse(),
            ex,
            ey,
            hn,
            span,
            back_shade,
        );
    }
    // the equator, soft-edged
    if let Some((path, full)) = slice_path(0, 0.0) {
        let inset = HALO_INSET.max(rel.inset[0]);
        fill_angular(
            &mut buffer,
            &path,
            full.inverse(),
            &rays,
            inset,
            0.0,
            center,
            Some((&fur.soft, 1.0)),
        );
    }
    // near half
    for k in 1..levels {
        let z = near * rel.z[k];
        if let Some((path, full)) = slice_path(k, z) {
            let inset = HALO_INSET.max(rel.inset[k]);
            fill_angular(
                &mut buffer,
                &path,
                full.inverse(),
                &rays,
                inset,
                0.0,
                center,
                None,
            );
        }
    }
    // the front
    let face = facing.abs();
    let front_a = if face >= 0.72 {
        1.0
    } else if face <= 0.34 {
        0.0
    } else {
        ((face - 0.34) / 0.38).powf(1.5)
    };
    if front_a > 0.01
        && let Some(front) = to_pixmap(tex.clone(), n)
    {
        let tex_scale = SPAN / n as f32;
        let sprite_m = Affine::scale(tex_scale, tex_scale)
            .then(Affine::translate(-PAD, -PAD))
            .then(cap_m);
        buffer.draw_pixmap(
            0,
            0,
            front.as_ref(),
            &PixmapPaint {
                opacity: front_a,
                blend_mode: BlendMode::SourceOver,
                quality: FilterQuality::Bicubic,
            },
            sprite_m.transform(),
            None,
        );
    }

    // the halo
    let spx = cap_px.round().max(1.0) as usize;
    let dev_halo = halo_dev(&fur, spx);
    let [miss_cut, miss_over] = if dl1 > 1e-4 {
        halo_miss(
            &form,
            &cfg.path,
            &rel,
            levels,
            &halos,
            near * dxs,
            near * dys,
        )
    } else {
        [0.0, 0.0]
    };
    let miss = miss_cut.max(miss_over);
    let back = smooth(HALO_FIT / 2.0, 2.0 * HALO_FIT, miss);
    let gone = smooth(HALO_FIT / 2.0, HALO_FIT, miss_over);
    let hr = halo_rays(&form, &rel, levels, ex, ey, dl1, &halos, back_shade);
    let haze_col = haze_color(&rays, pal.base, cfg.rim, cfg.vivid);
    // The halo is laid on the same frame as the slices: `at(0)` (which
    // centres the shape on the body) then the halo's own stretch then the
    // body. Mapping through `body` alone puts it fifty units down-right.
    let s_aff = Affine(halos).then(slice_at(0.0)).then(body);
    let s_inv = s_aff.inverse();
    let c0 = (rel.cx[0], rel.cy[0]);
    let halo_sample = |x: f32, y: f32| -> (f32, f32, f32, [f32; 3]) {
        // x,y already in the halo's design space (canvas -> design is done
        // by the caller through `s_inv`); returns alpha, core, haze alpha
        // and colour
        let ha = dev_halo.sample(
            (x + PAD) * (spx as f32 / SPAN) - 0.5,
            (y + PAD) * (spx as f32 / SPAN) - 0.5,
        );
        let hz = fur.haze.sample(
            (x + PAD) * (fur.r as f32 / SPAN) - 0.5,
            (y + PAD) * (fur.r as f32 / SPAN) - 0.5,
        );
        let co = fur.core.sample(
            (x + PAD) * (fur.r as f32 / SPAN) - 0.5,
            (y + PAD) * (fur.r as f32 / SPAN) - 0.5,
        );
        let angle = (y - c0.1).atan2(x - c0.0);
        let f = (angle / (2.0 * PI)).rem_euclid(1.0) * 24.0;
        let s0 = (f.floor() as usize) % 24;
        let s1 = (s0 + 1) % 24;
        let t = f - f.floor();
        let a = ray_color_ray(&rays, s0, hr.inset[s0], hr.dark[s0]);
        let b = ray_color_ray(&rays, s1, hr.inset[s1], hr.dark[s1]);
        let base = [
            a[0] + (b[0] - a[0]) * t,
            a[1] + (b[1] - a[1]) * t,
            a[2] + (b[2] - a[2]) * t,
        ];
        (ha, co, hz, base)
    };
    let laid = |buf: &mut Pixmap| {
        let w = buf.width() as usize;
        let h = buf.height() as usize;
        let data = buf.data_mut();
        for y in 0..h {
            for x in 0..w {
                let idx = (y * w + x) * 4;
                let da = f32::from(data[idx + 3]) / 255.0;
                let (dx, dy) = s_inv.point(x as f32 + 0.5, y as f32 + 0.5);
                let (ha, _co, hz, base) = halo_sample(dx, dy);
                if ha <= 0.0 {
                    continue;
                }
                // haze over base
                let color = [
                    haze_col[0] * hz + base[0] * (1.0 - hz),
                    haze_col[1] * hz + base[1] * (1.0 - hz),
                    haze_col[2] * hz + base[2] * (1.0 - hz),
                ];
                // destination-atop
                for ch in 0..3 {
                    let dst = f32::from(data[idx + ch]) / 255.0;
                    let src = color[ch] / 255.0 * ha;
                    data[idx + ch] = ((dst * ha + src * (1.0 - da)).clamp(0.0, 1.0) * 255.0) as u8;
                }
                data[idx + 3] = ((da * ha + ha * (1.0 - da)).clamp(0.0, 1.0) * 255.0) as u8;
            }
        }
        // the veil: the halo's band over the body's own edge
        let veil = HALO_VEIL * (dl1 / 0.2).min(1.0);
        if veil > 0.01 {
            for y in 0..h {
                for x in 0..w {
                    let idx = (y * w + x) * 4;
                    let da = f32::from(data[idx + 3]) / 255.0;
                    if da <= 0.0 {
                        continue;
                    }
                    let (dx, dy) = s_inv.point(x as f32 + 0.5, y as f32 + 0.5);
                    let (ha, co, _hz, base) = halo_sample(dx, dy);
                    let a = ha * (1.0 - co) * veil;
                    if a <= 0.0 {
                        continue;
                    }
                    for ch in 0..3 {
                        let dst = f32::from(data[idx + ch]) / 255.0;
                        let src = base[ch] / 255.0 * a;
                        data[idx + ch] =
                            ((src * da + dst * (1.0 - a)).clamp(0.0, 1.0) * 255.0) as u8;
                    }
                }
            }
        }
    };
    let lay = 1.0 - smooth(0.0, HALO_BLEND, back);
    let mut temp: Option<Pixmap> = None;
    if lay > 0.0
        && lay < 1.0
        && let Some(mut t) = Pixmap::new(pixmap.width(), pixmap.height())
    {
        t.draw_pixmap(
            0,
            0,
            buffer.as_ref(),
            &PixmapPaint {
                opacity: 1.0,
                blend_mode: BlendMode::Source,
                quality: FilterQuality::Nearest,
            },
            Transform::identity(),
            None,
        );
        laid(&mut t);
        temp = Some(t);
    }
    if lay >= 1.0 || (temp.is_none() && lay >= 0.5) {
        laid(&mut buffer);
    } else {
        // cut the body back, fill the halo behind, then cross-fade the laid copy
        {
            let w = buffer.width() as usize;
            let h = buffer.height() as usize;
            let data = buffer.data_mut();
            let f = 1.0 + (back * (miss + HALO_FIT)) / 25.0;
            let cxx = rel.cx[0];
            let cyy = rel.cy[0];
            let complement = |dx: f32, dy: f32| -> f32 {
                // dx,dy are already in the halo's design space (the caller
                // mapped the pixel through `s_inv`); grow about the centre,
                // then sample the cut sprite there.
                let px = cxx + (dx - cxx) / f;
                let py = cyy + (dy - cyy) / f;
                let ha = dev_halo.sample(
                    (px + PAD) * (spx as f32 / SPAN) - 0.5,
                    (py + PAD) * (spx as f32 / SPAN) - 0.5,
                );
                1.0 - ha
            };
            for y in 0..h {
                for x in 0..w {
                    let idx = (y * w + x) * 4;
                    if data[idx + 3] == 0 {
                        continue;
                    }
                    let (dx, dy) = s_inv.point(x as f32 + 0.5, y as f32 + 0.5);
                    let k = 1.0 - complement(dx, dy);
                    for ch in 0..4 {
                        data[idx + ch] = (f32::from(data[idx + ch]) * k).round() as u8;
                    }
                }
            }
            // destination-over the halo
            if gone < 1.0 {
                for y in 0..h {
                    for x in 0..w {
                        let idx = (y * w + x) * 4;
                        let da = f32::from(data[idx + 3]) / 255.0;
                        let (dx, dy) = s_inv.point(x as f32 + 0.5, y as f32 + 0.5);
                        let (ha, _co, hz, base) = halo_sample(dx, dy);
                        let ha = ha * (1.0 - gone);
                        if ha <= 0.0 {
                            continue;
                        }
                        let color = [
                            haze_col[0] * hz + base[0] * (1.0 - hz),
                            haze_col[1] * hz + base[1] * (1.0 - hz),
                            haze_col[2] * hz + base[2] * (1.0 - hz),
                        ];
                        for ch in 0..3 {
                            let dst = f32::from(data[idx + ch]) / 255.0;
                            let src = color[ch] / 255.0 * ha;
                            data[idx + ch] =
                                ((src * (1.0 - da) + dst).clamp(0.0, 1.0) * 255.0) as u8;
                        }
                        data[idx + 3] = ((ha * (1.0 - da) + da).clamp(0.0, 1.0) * 255.0) as u8;
                    }
                }
            }
            // cross-fade
            if let Some(t) = &temp {
                let td = t.data();
                for (i, d) in data.iter_mut().enumerate() {
                    *d = ((f32::from(*d) * (1.0 - lay) + f32::from(td[i]) * lay).clamp(0.0, 255.0))
                        as u8;
                }
            }
        }
    }

    // the pile's film
    let (gap_color, gap_scale) = gap_film(pal.base, cfg.vivid);
    let tip_color = tip_film(pal.base, cfg.vivid);
    let lin = pal.base.linear();
    let lum = (0.2126 * lin[0] + 0.7152 * lin[1] + 0.0722 * lin[2]).powf(1.0 / 2.2) as f32;
    // Turned far round, the front's projection squeezes into a strip that
    // no longer covers the body; the film then gives way to a square lay
    // over the whole stack (below), so its edge never shows as a seam.
    let side_u = if face >= 0.62 {
        0.0
    } else if face <= 0.36 {
        1.0
    } else {
        (0.62 - face) / 0.26
    };
    let cap_a = 1.0 - side_u;
    if cap_a > 0.001 {
        composite_film(&mut buffer, &fur.dark, gap_color, gap_scale * cap_a, cap_m);
        composite_film(
            &mut buffer,
            &fur.light,
            tip_color,
            (0.25 + 0.75 * lum) * cap_a,
            cap_m,
        );
    }
    let sheen_a = if dev < 160.0 {
        0.0
    } else if face >= 0.88 {
        1.0
    } else if face <= 0.55 {
        0.0
    } else {
        (face - 0.55) / 0.33
    };
    if sheen_a > 0.01
        && let Some(layers) = sheen_film(pal.base, cfg.vivid, &material)
    {
        let lift = near * PILE_H * cfg.fur.length;
        let cap_t = Affine::translate(lift * sy, -lift * cy * sp).then(cap_m);
        if layers[0].1 > 0.01 {
            composite_film(
                &mut buffer,
                &fur.back,
                layers[0].0,
                layers[0].1 * sheen_a * cap_a,
                cap_t,
            );
        }
        if layers[1].1 > 0.01 {
            composite_film(
                &mut buffer,
                &fur.back_soft,
                layers[1].0,
                layers[1].1 * sheen_a * cap_a,
                cap_m,
            );
        }
        if layers[2].1 > 0.01 {
            composite_film(
                &mut buffer,
                &fur.glint,
                layers[2].0,
                layers[2].1 * sheen_a * cap_a,
                cap_m,
            );
        }
    }
    // a far turn lays the pile square to the view
    if side_u > 0.0 {
        let k = dev / 100.0;
        let (cxs, cys) = stack_center(&rel, body);
        let square = Affine::scale(k, k).then(Affine::translate(cxs - 50.0 * k, cys - 50.0 * k));
        composite_film(
            &mut buffer,
            &fur.dark,
            gap_color,
            gap_scale * side_u,
            square,
        );
        composite_film(
            &mut buffer,
            &fur.light,
            tip_color,
            (0.25 + 0.75 * lum) * side_u,
            square,
        );
    }
    pixmap.draw_pixmap(
        0,
        0,
        buffer.as_ref(),
        &PixmapPaint {
            opacity: 1.0,
            blend_mode: BlendMode::SourceOver,
            quality: FilterQuality::Nearest,
        },
        Transform::identity(),
        None,
    );
    true
}

fn stack_center(rel: &Relief, body: Affine) -> (f32, f32) {
    let mut x0 = f32::INFINITY;
    let mut y0 = f32::INFINITY;
    let mut x1 = f32::NEG_INFINITY;
    let mut y1 = f32::NEG_INFINITY;
    for x in [-PAD, 100.0 + PAD] {
        for y in [-PAD, 100.0 + PAD] {
            let (px, py) = body.point(x, y);
            x0 = x0.min(px);
            x1 = x1.max(px);
            y0 = y0.min(py);
            y1 = y1.max(py);
        }
    }
    let _ = rel;
    ((x0 + x1) * 0.5, (y0 + y1) * 0.5)
}

fn halo_dev(fur: &Fur, spx: usize) -> Alpha {
    let mut out = Alpha::new(spx, spx);
    let k = spx as f32 / fur.r as f32;
    for y in 0..spx {
        for x in 0..spx {
            let sx = (x as f32 + 0.5) / k - 0.5;
            let sy = (y as f32 + 0.5) / k - 0.5;
            out.a[y * spx + x] = (fur.mask.sample(sx, sy) * 255.0).round() as u8;
        }
    }
    if let Some(p) = &fur.fringe
        && let Some(p) = p.as_ref().clone().transform(Transform::from_scale(k, k))
    {
        out.stroke(&p, fur.fringe_width * k, 0.45, false);
    }
    if let Some(p) = &fur.tufts
        && let Some(p) = p.as_ref().clone().transform(Transform::from_scale(k, k))
    {
        out.stroke(&p, fur.tuft_width * k, 0.8, false);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_follow_the_reference() {
        assert_eq!(fur_tier(100.0), 192);
        assert_eq!(fur_tier(200.0), 192);
        assert_eq!(fur_tier(201.0), 320);
        assert_eq!(fur_tier(400.0), 512);
    }

    #[test]
    fn noise_is_stable_and_bounded() {
        let n = ValueNoise::new(3.0, 7);
        let a = n.at(12.5, -3.0);
        assert!((0.0..=1.0).contains(&a));
        assert!((a - n.at(12.5, -3.0)).abs() < 1e-9);
    }

    #[test]
    fn halo_stretch_is_identity_face_on() {
        let path = crate::widgets::bot_avatar::path::parse_svg_path(
            "M20 50 C20 33 33 20 50 20 C67 20 80 33 80 50 C80 67 67 80 50 80 C33 80 20 67 20 50 Z",
        )
        .unwrap();
        let form = form::form_for("test-halo", &path, 32, 8.0).unwrap();
        let rel = relief_for(&form, 14, 1.0);
        let s = halo_stretch(&form, &rel, 14, 1.0, 0.0, 0.0);
        assert_eq!(s, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn a_small_pile_is_made_and_opaque_in_the_middle() {
        let path = crate::widgets::bot_avatar::path::parse_svg_path(
            "M20 50 C20 33 33 20 50 20 C67 20 80 33 80 50 C80 67 67 80 50 80 C33 80 20 67 20 50 Z",
        )
        .unwrap();
        let form = form::form_for("test-pile", &path, 32, 8.0).unwrap();
        let fur = build_fur(&form, 64, -1.0, 0.0, FurStyle::default());
        let mid = fur.mask.sample(32.0, 32.0);
        assert!(mid > 0.9, "middle of the pile should be solid, got {mid}");
        assert!(fur.dark.sample(32.0, 32.0) >= 0.0);
    }
}
