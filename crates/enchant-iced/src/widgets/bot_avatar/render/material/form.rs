//! The baked form of one outline: a cushion height field, its normals as
//! matcap cells, and its baked horizon occlusion.
//!
//! Ported from `buildForm` and `reliefFor` in `plastic.ts`. The height field
//! is the torsion function of the shape (`Δφ = −1` inside, `φ = 0` outside),
//! whose square root is an elliptical dome over a disc, a round tube along a
//! thin ray, and a smooth cushion over a square. It has no crease along the
//! medial axis, which a plain distance-to-outline field would show as a seam
//! down the middle of the body.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use tiny_skia::{FillRule, Pixmap};

use super::super::super::canvas::Affine;

/// Padding round the body in the texture, in design units.
pub(crate) const PAD: f32 = 3.0;
/// The texture covers `[-PAD, 100 + PAD]`.
pub(crate) const SPAN: f32 = 100.0 + 2.0 * PAD;
/// The matcap's side, in cells.
pub(crate) const MATCAP: usize = 64;
const INF: f32 = 1e12;

const E_XY: f32 = 0.669_130_6;
const E_Z: f32 = 0.743_144_8;

#[derive(Clone, Debug)]
pub(crate) struct Form {
    pub n: usize,
    pub id: usize,
    /// Matcap cell index of the top-left corner.
    pub i00: Vec<u16>,
    /// Weight inside the cell, 0–255.
    pub wx: Vec<u8>,
    /// Weight inside the cell, 0–255.
    pub wy: Vec<u8>,
    /// Baked occlusion, 0 = not drawn.
    pub ao: Vec<u8>,
    /// Signed distance to the outline in design units, positive inside.
    pub sd: Vec<f32>,
    /// Pillow height in design units.
    pub h: Vec<f32>,
    /// The surface's highest point.
    pub top: f32,
}

/// A level set of the height field, used to draw the body's side.
#[derive(Clone, Debug)]
pub(crate) struct Relief {
    pub top: f32,
    pub z: Vec<f32>,
    pub scale: Vec<f32>,
    pub cx: Vec<f32>,
    pub cy: Vec<f32>,
    pub inset: Vec<f32>,
    pub q: f32,
}

struct Cache {
    forms: HashMap<String, Arc<Form>>,
    any: HashMap<String, Arc<Form>>,
    reliefs: HashMap<(usize, u32, u16), Arc<Relief>>,
    lifts: HashMap<usize, Arc<Lift>>,
}

/// The baked edge factor undone, for a pile's sheen: the occlusion the form
/// carries, and a weight that removes the highlight the edge baked in.
#[derive(Clone, Debug)]
pub(crate) struct Lift {
    pub ao: Vec<u8>,
    pub k: Vec<f32>,
}

fn cache() -> &'static Mutex<Cache> {
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    CACHE.get_or_init(|| {
        Mutex::new(Cache {
            forms: HashMap::new(),
            any: HashMap::new(),
            reliefs: HashMap::new(),
            lifts: HashMap::new(),
        })
    })
}

fn next_id() -> usize {
    static NEXT: AtomicUsize = AtomicUsize::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

fn depth_key(half_depth: f32) -> String {
    format!("{half_depth:.2}")
}

/// Get (or bake) the form of `key` at texture size `n` and depth `half_depth`.
pub(crate) fn form_for(
    key: &str,
    path: &tiny_skia::Path,
    n: usize,
    half_depth: f32,
) -> Option<Arc<Form>> {
    let id = format!("{key}|{n}|{}", depth_key(half_depth));
    if let Some(f) = cache().lock().ok()?.forms.get(&id) {
        return Some(Arc::clone(f));
    }
    let cov = rasterize(path, n)?;
    let form = Arc::new(build_form(&cov, n, half_depth));
    let mut c = cache().lock().ok()?;
    if c.forms.len() > 24 {
        c.forms.clear();
        c.any.clear();
        c.reliefs.clear();
        c.lifts.clear();
    }
    c.any.insert(
        format!("{key}|{}", depth_key(half_depth)),
        Arc::clone(&form),
    );
    c.forms.insert(id, Arc::clone(&form));
    Some(form)
}

/// How high the surface stands at a design point, on whichever form of the
/// outline is kept.
pub(crate) fn surface_at(key: &str, half_depth: f32, x: f32, y: f32, round: f32) -> Option<f32> {
    let lookup = format!("{key}|{}", depth_key(half_depth));
    let form = {
        let c = cache().lock().ok()?;
        Arc::clone(c.any.get(&lookup)?)
    };
    let n = form.n;
    let u = SPAN / n as f32;
    let fx = ((x + PAD) / u - 0.5).clamp(0.0, n as f32 - 1.0);
    let fy = ((y + PAD) / u - 0.5).clamp(0.0, n as f32 - 1.0);
    let x0 = (fx as usize).min(n - 2);
    let y0 = (fy as usize).min(n - 2);
    let tx = fx - x0 as f32;
    let ty = fy - y0 as f32;
    let i = y0 * n + x0;
    let v = (form.h[i] * (1.0 - tx) + form.h[i + 1] * tx) * (1.0 - ty)
        + (form.h[i + n] * (1.0 - tx) + form.h[i + n + 1] * tx) * ty;
    let top = form.top;
    if top <= 0.0 {
        return Some(0.0);
    }
    let q = round.clamp(0.05, 1.0);
    Some(top * (v.max(0.0) / top).powf(q))
}

/// The level sets used to draw the side at `k` levels.
pub(crate) fn relief_for(form: &Form, k: usize, round: f32) -> Arc<Relief> {
    let q = round.clamp(0.05, 1.0);
    let key = (form.id, k as u32, (q * 100.0).round() as u16);
    if let Ok(c) = cache().lock()
        && let Some(r) = c.reliefs.get(&key)
    {
        return Arc::clone(r);
    }
    let relief = Arc::new(build_relief(form, k, q));
    if let Ok(mut c) = cache().lock() {
        if c.reliefs.len() > 64 {
            c.reliefs.clear();
        }
        c.reliefs.insert(key, Arc::clone(&relief));
    }
    relief
}

/// Undo the edge factor the material bakes into the form's occlusion, so a
/// pile's sheen can be laid over its edge without a second shadow.
pub(crate) fn edge_lift(form: &Form) -> Arc<Lift> {
    if let Ok(c) = cache().lock()
        && let Some(l) = c.lifts.get(&form.id)
    {
        return Arc::clone(l);
    }
    let mut ao = vec![0u8; form.n * form.n];
    let mut k = vec![0.0f32; form.n * form.n];
    for i in 0..form.n * form.n {
        if form.ao[i] == 0 {
            continue;
        }
        let dd = form.sd[i].max(0.0);
        let e = 1.0 - (dd / 3.0).min(1.0);
        let edge = 1.0 - 0.2 * e * e;
        let lin = ((form.ao[i] as f32 / 255.0).powf(2.2) / edge).min(1.0);
        ao[i] = (255.0 * lin.powf(1.0 / 2.2)).max(1.0).round() as u8;
        k[i] = 1.0;
    }
    let lift = Arc::new(Lift { ao, k });
    if let Ok(mut c) = cache().lock() {
        if c.lifts.len() > 24 {
            c.lifts.clear();
        }
        c.lifts.insert(form.id, Arc::clone(&lift));
    }
    lift
}

fn build_relief(form: &Form, k: usize, q: f32) -> Relief {
    let n = form.n;
    let u = SPAN / n as f32;
    let mut z = vec![0.0f32; k];
    let mut scale = vec![1.0f32; k];
    let mut cx = vec![50.0f32; k];
    let mut cy = vec![50.0f32; k];
    let mut inset = vec![1.2f32; k];
    let measure = |inside: &dyn Fn(usize) -> bool| -> (usize, f32, f32) {
        let mut count = 0usize;
        let mut sx = 0.0f64;
        let mut sy = 0.0f64;
        for j in 0..n {
            for x in 0..n {
                let i = j * n + x;
                if inside(i) {
                    count += 1;
                    sx += x as f64;
                    sy += j as f64;
                }
            }
        }
        if count == 0 {
            (0, 50.0, 50.0)
        } else {
            (
                count,
                ((sx / count as f64 + 0.5) as f32 * u) - PAD,
                ((sy / count as f64 + 0.5) as f32 * u) - PAD,
            )
        }
    };
    let (a0, x0, y0) = measure(&|i| form.sd[i] >= 0.0);
    cx[0] = x0;
    cy[0] = y0;
    for level in 1..k {
        let s = level as f32 / k as f32;
        z[level] = form.top * s;
        let tau = form.top * s.powf(1.0 / q);
        let (ak, xk, yk) = measure(&|i| form.sd[i] >= 0.0 && form.h[i] >= tau);
        scale[level] = scale[level - 1].min(if a0 > 0 {
            (ak as f32 / a0 as f32).sqrt()
        } else {
            1.0
        });
        cx[level] = xk;
        cy[level] = yk;
        let band = form.top / (2.0 * k as f32);
        let mut count = 0usize;
        let mut sum = 0.0f64;
        for i in 0..n * n {
            if form.sd[i] > 0.0 && (form.h[i] - tau).abs() < band {
                sum += f64::from(form.sd[i]);
                count += 1;
            }
        }
        inset[level] = inset[level - 1].max(if count > 0 {
            (sum / count as f64) as f32
        } else {
            inset[level - 1]
        });
    }
    Relief {
        top: form.top,
        z,
        scale,
        cx,
        cy,
        inset,
        q,
    }
}

/// Rasterise a path into an `n × n` coverage buffer over `[-PAD, 100 + PAD]`.
pub(crate) fn rasterize(path: &tiny_skia::Path, n: usize) -> Option<Vec<u8>> {
    let mut pixmap = Pixmap::new(n as u32, n as u32)?;
    let scale = n as f32 / SPAN;
    let transform = Affine::translate(PAD, PAD).then(Affine::scale(scale, scale));
    let mut paint = tiny_skia::Paint::default();
    paint.set_color_rgba8(255, 255, 255, 255);
    paint.anti_alias = true;
    pixmap.fill_path(path, &paint, FillRule::Winding, transform.transform(), None);
    Some(pixmap.pixels().iter().map(|p| p.alpha()).collect())
}

fn build_form(cov: &[u8], n: usize, half_depth: f32) -> Form {
    let nn = n * n;
    let u = SPAN / n as f32;
    let mut mask = vec![0u8; nn];
    for i in 0..nn {
        mask[i] = if cov[i] >= 128 { 1 } else { 0 };
    }
    let (d_in, _) = edt2d(&mask, 0, n);
    let (d_out, near_in) = edt2d(&mask, 1, n);
    let mut sd = vec![0.0f32; nn];
    let mut tmp = vec![0.0f32; nn];
    for i in 0..nn {
        let a = f32::from(cov[i]) / 255.0;
        sd[i] = u * if a > 0.0 && a < 1.0 {
            a - 0.5
        } else if mask[i] != 0 {
            d_in[i].sqrt() - 0.5
        } else {
            0.5 - d_out[i].sqrt()
        };
    }
    blur5(&mut sd, n, &mut tmp);
    blur5(&mut sd, n, &mut tmp);

    let mut phi = poisson(cov, n, u);
    let mut phi_max = 0.0f32;
    for &v in &phi {
        if v > phi_max {
            phi_max = v;
        }
    }
    let r_in = 2.0 * phi_max.sqrt();
    let h_max = (1.5 * half_depth + 0.32 * r_in).min(1.4 * r_in);
    let kh = if phi_max > 0.0 {
        h_max / phi_max.sqrt()
    } else {
        0.0
    };
    let mut h = vec![0.0f32; nn];
    for i in 0..nn {
        h[i] = if phi[i] > 0.0 {
            kh * phi[i].sqrt()
        } else {
            0.0
        };
    }
    phi.clear();
    blur5(&mut h, n, &mut tmp);

    let mut top = 0.0f32;
    for &v in &h {
        if v > top {
            top = v;
        }
    }

    let mut form = Form {
        n,
        id: next_id(),
        i00: vec![0; nn],
        wx: vec![0; nn],
        wy: vec![0; nn],
        ao: vec![0; nn],
        sd,
        h,
        top,
    };
    let steps: &[usize] = if n <= 64 {
        &[1, 2, 3, 5, 8]
    } else if n <= 96 {
        &[1, 2, 4, 7, 11]
    } else {
        &[1, 2, 4, 7, 11, 15]
    };
    // Outside texels within a few texels borrow their nearest inside texel,
    // so a fringe can use the same normals and never reads transparent black.
    let halo = 24.0 * n as f32 / 128.0;
    let last = n - 1;
    let dx = [1i32, 1, 0, -1, -1, -1, 0, 1];
    let dy = [0i32, 1, 1, 1, 0, -1, -1, -1];
    let dl = [
        1.0,
        std::f32::consts::SQRT_2,
        1.0,
        std::f32::consts::SQRT_2,
        1.0,
        std::f32::consts::SQRT_2,
        1.0,
        std::f32::consts::SQRT_2,
    ];
    for y in 0..n {
        for x in 0..n {
            let i = y * n + x;
            let src = if mask[i] != 0 {
                i
            } else if d_out[i].sqrt() <= halo {
                near_in[i].max(0) as usize
            } else {
                usize::MAX
            };
            if src == usize::MAX {
                continue;
            }
            let sx = src % n;
            let sy = (src - sx) / n;
            let xl = if sx > 0 { sx - 1 } else { 0 };
            let xr = if sx < last { sx + 1 } else { last };
            let yu = if sy > 0 { sy - 1 } else { 0 };
            let yd = if sy < last { sy + 1 } else { last };
            let mut nx = -(form.h[sy * n + xr] - form.h[sy * n + xl]) / (2.0 * u);
            let mut ny = -(form.h[yd * n + sx] - form.h[yu * n + sx]) / (2.0 * u);
            let mut nz = 1.0;
            let mut len = (nx * nx + ny * ny + 1.0).sqrt();
            nx /= len;
            ny /= len;
            nz /= len;
            let dd = form.sd[src].max(0.0);
            if dd < 2.0 {
                let mut gx = form.sd[sy * n + xr] - form.sd[sy * n + xl];
                let mut gy = form.sd[yd * n + sx] - form.sd[yu * n + sx];
                let gl = (gx * gx + gy * gy).sqrt().max(1e-6);
                gx /= gl;
                gy /= gl;
                let w = 0.7 * (1.0 - dd / 2.0);
                nx += w * (-gx - nx);
                ny += w * (-gy - ny);
                nz += w * (0.0 - nz);
                len = (nx * nx + ny * ny + nz * nz).sqrt().max(1e-6);
                nx /= len;
                ny /= len;
            }
            let h0 = form.h[src];
            let mut occ = 0.0f32;
            for d in 0..8 {
                let mut m = 0.0f32;
                for &r in steps {
                    let qx = (sx as i32 + dx[d] * r as i32).clamp(0, last as i32) as usize;
                    let qy = (sy as i32 + dy[d] * r as i32).clamp(0, last as i32) as usize;
                    let t = (form.h[qy * n + qx] - h0) / (r as f32 * u * dl[d]);
                    if t > m {
                        m = t;
                    }
                }
                occ += m / (1.0 + m * m).sqrt();
            }
            let e = 1.0 - (dd / 3.0).min(1.0);
            let edge = 1.0 - 0.2 * e * e;
            let ao_lin = (1.0 - 0.9 * occ / 8.0).powf(1.5) * edge;
            form.ao[i] = (255.0 * ao_lin.powf(1.0 / 2.2)).round().max(1.0) as u8;
            let fx = (nx * 0.5 + 0.5) * (MATCAP - 1) as f32;
            let fy = (ny * 0.5 + 0.5) * (MATCAP - 1) as f32;
            let cx = (fx as usize).min(MATCAP - 2);
            let cy = (fy as usize).min(MATCAP - 2);
            form.i00[i] = (cy * MATCAP + cx) as u16;
            form.wx[i] = (255.0 * (fx - cx as f32).clamp(0.0, 1.0)).round() as u8;
            form.wy[i] = (255.0 * (fy - cy as f32).clamp(0.0, 1.0)).round() as u8;
        }
    }
    form
}

fn edt1d(f: &[f32], d: &mut [f32], near: &mut [i32]) {
    let n = f.len();
    let mut v = vec![0i32; n];
    let mut z = vec![0.0f32; n + 1];
    let mut k: isize = 0;
    z[0] = -1e30;
    z[1] = 1e30;
    for q in 1..n {
        loop {
            let vk = v[k.max(0) as usize] as usize;
            let x =
                (f[q] + (q * q) as f32 - f[vk] - (vk * vk) as f32) / (2.0 * (q as f32 - vk as f32));
            if x > z[k.max(0) as usize] {
                k += 1;
                v[k as usize] = q as i32;
                z[k as usize] = x;
                z[k as usize + 1] = 1e30;
                break;
            }
            if k == 0 {
                v[0] = q as i32;
                z[0] = x;
                z[1] = 1e30;
                break;
            }
            k -= 1;
        }
    }
    let mut k: usize = 0;
    for q in 0..n {
        while z[k + 1] < q as f32 {
            k += 1;
        }
        let vk = v[k] as usize;
        d[q] = ((q as f32 - vk as f32) * (q as f32 - vk as f32)) + f[vk];
        near[q] = vk as i32;
    }
}

pub(crate) fn edt2d(mask: &[u8], site: u8, n: usize) -> (Vec<f32>, Vec<i32>) {
    let mut out = vec![0.0f32; n * n];
    let mut near = vec![0i32; n * n];
    let mut f = vec![0.0f32; n];
    let mut d = vec![0.0f32; n];
    let mut s = vec![0i32; n];
    let mut g = vec![0.0f32; n * n];
    let mut row = vec![0i32; n * n];
    for x in 0..n {
        for y in 0..n {
            f[y] = if mask[y * n + x] == site { 0.0 } else { INF };
        }
        edt1d(&f, &mut d, &mut s);
        for y in 0..n {
            g[y * n + x] = d[y];
            row[y * n + x] = s[y];
        }
    }
    for y in 0..n {
        f.copy_from_slice(&g[y * n..y * n + n]);
        edt1d(&f, &mut d, &mut s);
        for x in 0..n {
            out[y * n + x] = d[x];
            near[y * n + x] = row[y * n + s[x] as usize] * n as i32 + s[x];
        }
    }
    (out, near)
}

fn blur5(a: &mut [f32], n: usize, tmp: &mut [f32]) {
    for y in 0..n {
        let o = y * n;
        for x in 0..n {
            let x0 = x.saturating_sub(2);
            let x1 = x.saturating_sub(1);
            let x3 = (x + 1).min(n - 1);
            let x4 = (x + 2).min(n - 1);
            tmp[o + x] =
                (a[o + x0] + 4.0 * a[o + x1] + 6.0 * a[o + x] + 4.0 * a[o + x3] + a[o + x4])
                    * 0.0625;
        }
    }
    for x in 0..n {
        for y in 0..n {
            let y0 = y.saturating_sub(2);
            let y1 = y.saturating_sub(1);
            let y3 = (y + 1).min(n - 1);
            let y4 = (y + 2).min(n - 1);
            a[y * n + x] = (tmp[y0 * n + x]
                + 4.0 * tmp[y1 * n + x]
                + 6.0 * tmp[y * n + x]
                + 4.0 * tmp[y3 * n + x]
                + tmp[y4 * n + x])
                * 0.0625;
        }
    }
}

struct Level {
    n: usize,
    mask: Vec<u8>,
    phi: Vec<f32>,
}

fn poisson(cov: &[u8], n: usize, u: f32) -> Vec<f32> {
    let mut levels: Vec<Level> = Vec::new();
    let mut size = n;
    let mut factor = 1usize;
    loop {
        if factor != 1 && !(factor <= 4 && size.is_multiple_of(2)) {
            break;
        }
        let mut mask = vec![0u8; size * size];
        for y in 0..size {
            for x in 0..size {
                let mut sum = 0u32;
                for j in 0..factor {
                    for i in 0..factor {
                        sum += u32::from(cov[(y * factor + j) * n + x * factor + i]);
                    }
                }
                mask[y * size + x] = if sum >= 128 * (factor * factor) as u32 {
                    1
                } else {
                    0
                };
            }
        }
        levels.push(Level {
            n: size,
            mask,
            phi: vec![0.0; size * size],
        });
        if factor == 4 {
            break;
        }
        factor *= 2;
        size /= 2;
    }
    for l in (0..levels.len()).rev() {
        if l + 1 < levels.len() {
            let (cn, coarse) = {
                let c = &levels[l + 1];
                (c.n, c.phi.clone())
            };
            let cur_n = levels[l].n;
            let mut next = vec![0.0f32; cur_n * cur_n];
            for y in 0..cur_n {
                let fy = ((y as f32 + 0.5) / 2.0 - 0.5).clamp(0.0, cn as f32 - 1.0);
                let y0 = fy as usize;
                let y1 = (y0 + 1).min(cn - 1);
                let ty = fy - y0 as f32;
                for x in 0..cur_n {
                    let i = y * cur_n + x;
                    if levels[l].mask[i] == 0 {
                        continue;
                    }
                    let fx = ((x as f32 + 0.5) / 2.0 - 0.5).clamp(0.0, cn as f32 - 1.0);
                    let x0 = fx as usize;
                    let x1 = (x0 + 1).min(cn - 1);
                    let tx = fx - x0 as f32;
                    next[i] = (coarse[y0 * cn + x0] * (1.0 - tx) + coarse[y0 * cn + x1] * tx)
                        * (1.0 - ty)
                        + (coarse[y1 * cn + x0] * (1.0 - tx) + coarse[y1 * cn + x1] * tx) * ty;
                }
            }
            levels[l].phi = next;
        }
        let ln = levels[l].n;
        let om = (2.0 / (1.0 + (std::f32::consts::PI / ln as f32).sin()) - 0.05).min(1.9);
        sweep(&mut levels[l], u * (1 << l) as f32, 4, 1.0);
        sweep(
            &mut levels[l],
            u * (1 << l) as f32,
            if l == 2 {
                100
            } else if l == 1 {
                30
            } else {
                16
            },
            om,
        );
        sweep(&mut levels[l], u * (1 << l) as f32, 8, 1.0);
    }
    levels.remove(0).phi
}

fn sweep(level: &mut Level, s: f32, iters: usize, om: f32) {
    let n = level.n;
    let s2 = s * s;
    for _ in 0..iters {
        for y in 1..n - 1 {
            let o = y * n;
            for x in 1..n - 1 {
                let i = o + x;
                if level.mask[i] == 0 {
                    continue;
                }
                let v = (level.phi[i - 1]
                    + level.phi[i + 1]
                    + level.phi[i - n]
                    + level.phi[i + n]
                    + s2)
                    * 0.25;
                level.phi[i] += om * (v - level.phi[i]);
            }
        }
    }
}

/// Sample a float field bilinearly.
#[allow(dead_code)]
pub(crate) fn bilerp(field: &[f32], n: usize, x: f32, y: f32) -> f32 {
    let fx = x.clamp(0.0, n as f32 - 1.0);
    let fy = y.clamp(0.0, n as f32 - 1.0);
    let x0 = (fx as usize).min(n - 2);
    let y0 = (fy as usize).min(n - 2);
    let tx = fx - x0 as f32;
    let ty = fy - y0 as f32;
    let i = y0 * n + x0;
    (field[i] * (1.0 - tx) + field[i + 1] * tx) * (1.0 - ty)
        + (field[i + n] * (1.0 - tx) + field[i + n + 1] * tx) * ty
}

/// The screen-space light frame for the cap.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Frame {
    pub l: [f32; 3],
    pub v: [f32; 3],
    pub h: [f32; 3],
    pub u: [f32; 3],
    pub w: [f32; 3],
    pub a: [f32; 3],
    pub b: [f32; 3],
    pub d: [f32; 3],
    pub k: [f32; 3],
}

pub(crate) struct Rig {
    pub cy: f32,
    pub sy: f32,
    pub cp: f32,
    pub sp: f32,
    pub facing: f32,
    pub roll: f32,
    pub lx: f32,
    pub ly: f32,
    pub bx: Option<f32>,
    pub by: Option<f32>,
}

fn norm3(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
    [v[0] / l, v[1] / l, v[2] / l]
}

/// Where the back light comes from when the caller leaves it out: above the
/// toy, toward the side away from the key.
pub(crate) fn back_from_key(lx: f32, ly: f32) -> (f32, f32) {
    let l = (lx * lx + ly * ly).sqrt().max(1e-6);
    let (ux, uy) = (lx / l, ly / l);
    let bx = -uy + 0.35;
    let by = ux - 0.55;
    let l = (bx * bx + by * by).sqrt().max(1e-6);
    (bx / l, by / l)
}

pub(crate) fn cap_frame(rig: &Rig) -> Frame {
    let cr = rig.roll.cos();
    let sr = rig.roll.sin();
    let lx = cr * rig.lx + sr * rig.ly;
    let ly = -sr * rig.lx + cr * rig.ly;
    let mirror = if rig.facing < 0.0 { -1.0 } else { 1.0 };
    let zf = mirror * (rig.facing.abs() / 0.16).min(1.0);
    let (cy, sy, cp, sp) = (rig.cy, rig.sy, rig.cp, rig.sp);
    let local = |x: f32, y: f32, z: f32, zk: f32| {
        norm3([
            cy * x + sy * sp * y - sy * cp * z,
            cp * y + sp * z,
            zk * (sy * x - cy * sp * y + cy * cp * z),
        ])
    };
    let l = local(E_XY * lx, E_XY * ly, E_Z, zf);
    let v = local(0.0, 0.0, 1.0, mirror);
    let h = norm3([l[0] + v[0], l[1] + v[1], l[2] + v[2]]);
    let u = local(lx, ly, 0.0, mirror);
    let cw = 80.0f32.to_radians().cos();
    let sw = 80.0f32.to_radians().sin();
    let wx = cw * lx - sw * ly;
    let wy = sw * lx + cw * ly;
    let ws = norm3([0.55 * wx, 0.55 * wy, 0.83]);
    let as_ = norm3([ws[1], -ws[0], 0.0]);
    let bs = [
        ws[1] * as_[2] - ws[2] * as_[1],
        ws[2] * as_[0] - ws[0] * as_[2],
        ws[0] * as_[1] - ws[1] * as_[0],
    ];
    let w = local(ws[0], ws[1], ws[2], mirror);
    let a = local(as_[0], as_[1], as_[2], mirror);
    let b = local(bs[0], bs[1], bs[2], mirror);
    let d = local(sr, cr, 0.0, mirror);
    let (kx, ky) = match (rig.bx, rig.by) {
        (Some(bx), Some(by)) => (bx, by),
        _ => back_from_key(rig.lx, rig.ly),
    };
    let k = local(cr * kx + sr * ky, -sr * kx + cr * ky, 0.0, mirror);
    Frame {
        l,
        v,
        h,
        u,
        w,
        a,
        b,
        d,
        k,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::bot_avatar::data::bot_avatar_shape;
    use crate::widgets::bot_avatar::path::parse_svg_path;
    use crate::widgets::bot_avatar::types::BotAvatarType;

    #[test]
    fn form_has_a_positive_cushion_in_the_middle() {
        let path = parse_svg_path(bot_avatar_shape(BotAvatarType::Clover)).unwrap();
        let form = form_for("test-clover", &path, 64, 9.75).unwrap();
        let n = form.n;
        let mid = (n / 2) * n + n / 2;
        assert!(form.sd[mid] > 20.0, "{}", form.sd[mid]);
        assert!(form.h[mid] > 5.0, "{}", form.h[mid]);
        assert!(form.ao[mid] > 100);
        assert_eq!(form.h[0], 0.0);
    }

    #[test]
    fn surface_at_matches_the_form() {
        let path = parse_svg_path(bot_avatar_shape(BotAvatarType::Circle)).unwrap();
        let form = form_for("test-circle", &path, 64, 9.75).unwrap();
        let s = surface_at("test-circle", 9.75, 50.0, 50.0, 1.0).unwrap();
        assert!((s - form.top).abs() < 4.0, "{s} {}", form.top);
        assert!(surface_at("missing", 9.75, 50.0, 50.0, 1.0).is_none());
    }

    #[test]
    fn relief_levels_shrink_and_rise_toward_the_middle() {
        let path = parse_svg_path(bot_avatar_shape(BotAvatarType::Clover)).unwrap();
        let form = form_for("test-relief", &path, 64, 9.75).unwrap();
        let r = relief_for(&form, 14, 1.0);
        assert_eq!(r.z[0], 0.0);
        assert_eq!(r.scale[0], 1.0);
        for k in 1..14 {
            assert!(r.z[k] >= r.z[k - 1]);
            assert!(r.scale[k] <= r.scale[k - 1] + 1e-5);
        }
    }

    #[test]
    fn cap_frame_keeps_light_unit_length() {
        let frame = cap_frame(&Rig {
            cy: 1.0,
            sy: 0.0,
            cp: 1.0,
            sp: 0.0,
            facing: 1.0,
            roll: 0.0,
            lx: -0.85,
            ly: -0.53,
            bx: None,
            by: None,
        });
        let l =
            (frame.l[0] * frame.l[0] + frame.l[1] * frame.l[1] + frame.l[2] * frame.l[2]).sqrt();
        assert!((l - 1.0).abs() < 1e-4);
    }
}
