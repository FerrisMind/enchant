//! The material evaluated for every normal, and the per-texel shading pass.
//!
//! Ported from `buildMatcap`, `sampleMatcap` and `shadeTexels` in
//! `plastic.ts`: a clear coat with a broad key highlight, a tighter second
//! lobe, a sky/floor gradient in the reflection, a window reflection and a
//! Fresnel rim. The body's own light only paints the colour; the coat's
//! reflections stay white.

use super::form::{Form, Lift, MATCAP};

/// Material knobs shared by plastic and fabric.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Material {
    pub shadow: f32,
    pub highlight: f32,
    pub spread: f32,
    pub rim: f32,
    pub vivid: f32,
    /// Fabric: how far round to the front the key sits, radians.
    pub front: f32,
    /// Fabric: the strength of the light along single fibres.
    pub shine: f32,
    /// Fabric: 0–1, how soft the back light is.
    pub back_soft: f32,
    /// Fabric: the smooth satin light over the body's edge.
    pub sheen: f32,
}

const WARM: [f32; 3] = [1.0, 0.98, 0.95];
const ENV: [f32; 3] = [0.92, 0.96, 1.0];

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

fn soft(w: f32, s: f32, x: f32) -> f32 {
    1.0 - smooth(w - s, w + s, x)
}

fn tone(v: f32) -> f32 {
    if v <= 0.0 {
        return 0.0;
    }
    if v >= 2.5 {
        return 255.0;
    }
    let y = if v <= 0.75 {
        v
    } else {
        0.75 + 0.25 * (1.0 - (-(v - 0.75) / 0.25).exp())
    };
    let s = if y <= 0.003_130_8 {
        12.92 * y
    } else {
        1.055 * y.powf(1.0 / 2.4) - 0.055
    };
    255.0 * s
}

/// Raise the chroma of a lit colour about its luminance, as far as the gamut
/// allows: a fully saturated body keeps its colour in the light instead of
/// paling toward white.
fn vivify(c: &mut [f32; 3], v: f32) {
    if v <= 0.0 {
        return;
    }
    let y = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
    let mn = c[0].min(c[1]).min(c[2]);
    let mx = c[0].max(c[1]).max(c[2]);
    let mut k = 1.0 + 3.0 * v;
    if mn < y {
        k = k.min(y / (y - mn).max(1e-6));
    }
    if mx > y {
        k = k.min((1.0 - y) / (mx - y).max(1e-6)).max(1.0);
    }
    for ch in c.iter_mut() {
        *ch = y + (*ch - y) * k;
    }
}

/// Build the lit sphere for body colour `c` (linear).
pub(crate) fn build_matcap(c: [f32; 3], frame: &super::form::Frame, p: &Material) -> Vec<f32> {
    let mut out = vec![0.0f32; MATCAP * MATCAP * 3];
    let mx = c[0].max(c[1]).max(c[2]).max(0.05);
    let tint = [c[0] / mx, c[1] / mx, c[2] / mx];
    let amb = (0.30 - 0.15 * p.shadow).max(0.03);
    let wrap = 0.15 + 0.14 * p.spread;
    let kd = 0.85;
    let e1 = (110.0 / p.spread.powf(1.3)).round().clamp(1.0, 90.0);
    let e2 = (8.0 / p.spread).round().max(2.0);
    let ks1 = 0.45 * p.highlight;
    let ks2 = 0.10 * p.highlight;
    let win_k = 0.11 * p.highlight;
    let rim_k = 0.30 * p.rim;
    let amb_t = [amb * tint[0], amb * tint[1], amb * tint[2]];
    for j in 0..MATCAP {
        for i in 0..MATCAP {
            let mut nx = (i as f32 / (MATCAP - 1) as f32) * 2.0 - 1.0;
            let mut ny = (j as f32 / (MATCAP - 1) as f32) * 2.0 - 1.0;
            let mut r2 = nx * nx + ny * ny;
            if r2 > 1.14 {
                continue;
            }
            if r2 > 1.0 {
                let s = 1.0 / r2.sqrt();
                nx *= s;
                ny *= s;
                r2 = 1.0;
            }
            let nz = (1.0 - r2).max(0.0).sqrt();
            let nl = nx * frame.l[0] + ny * frame.l[1] + nz * frame.l[2];
            let nv = (nx * frame.v[0] + ny * frame.v[1] + nz * frame.v[2]).max(0.0);
            let nh = (nx * frame.h[0] + ny * frame.h[1] + nz * frame.h[2]).max(0.0);
            let dif = ((nl + wrap) / (1.0 + wrap)).clamp(0.0, 1.0);
            let q = 1.0 - nv;
            let q2 = q * q;
            let f3 = q2 * q;
            let f5 = f3 * q2;
            let spec = (ks1 * nh.powf(e1) + ks2 * nh.powf(e2)) * (1.0 + 3.0 * f5);
            let rx = 2.0 * nv * nx - frame.v[0];
            let ry = 2.0 * nv * ny - frame.v[1];
            let rz = 2.0 * nv * nz - frame.v[2];
            let sky = 0.45 + 0.55 * smooth(-0.4, 0.6, rx * frame.u[0] + ry * frame.u[1] + rz * frame.u[2]);
            let rw = rx * frame.w[0] + ry * frame.w[1] + rz * frame.w[2];
            let mut win = 0.0;
            if rw > 0.5 {
                let ra = (rx * frame.a[0] + ry * frame.a[1] + rz * frame.a[2]) / rw;
                let rb = (rx * frame.b[0] + ry * frame.b[1] + rz * frame.b[2]) / rw;
                win = soft(0.34, 0.12, ra.abs()) * soft(0.12, 0.06, rb.abs());
            }
            let env = rim_k * f3 * sky + win_k * win;
            let mut px = [
                c[0] * (amb_t[0] + kd * dif),
                c[1] * (amb_t[1] + kd * dif),
                c[2] * (amb_t[2] + kd * dif),
            ];
            vivify(&mut px, p.vivid);
            let i3 = (j * MATCAP + i) * 3;
            out[i3] = tone(px[0] + spec * WARM[0] + env * ENV[0]);
            out[i3 + 1] = tone(px[1] + spec * WARM[1] + env * ENV[1]);
            out[i3 + 2] = tone(px[2] + spec * WARM[2] + env * ENV[2]);
        }
    }
    out
}

/// Build the lit sphere for the pile: a dye-deepened shade, a saturated
/// terminator band, a fibre sheen and a back light that rims the silhouette.
pub(crate) fn build_fabric_matcap(
    c: [f32; 3],
    frame: &super::form::Frame,
    p: &Material,
) -> Vec<f32> {
    let mut out = vec![0.0f32; MATCAP * MATCAP * 3];
    let vivid = p.vivid;
    let l = fabric_key(frame.u, frame.v, p.front);
    let lb = fabric_back(frame.k, frame.v);
    let amb = (0.2 - 0.15 * p.shadow).max(0.03);
    let fill_k = (0.24 - 0.13 * p.shadow).max(0.04);
    let wrap = (0.05 + 0.12 * p.spread).min(0.7);
    let kd = 1.3;
    let floor_k = 0.34 * (p.shadow / 0.35).min(1.6);
    let f_dir = norm3([
        -0.55 * frame.u[0] + 0.85 * frame.v[0] + 0.15 * frame.d[0],
        -0.55 * frame.u[1] + 0.85 * frame.v[1] + 0.15 * frame.d[1],
        -0.55 * frame.u[2] + 0.85 * frame.v[2] + 0.15 * frame.d[2],
    ]);
    let satin = p.sheen;
    let back_soft = p.back_soft.clamp(0.0, 1.0);
    let sheen_k = 0.16 * p.highlight * satin;
    let rim_k = 1.15 * p.rim * satin;
    let sss_k = 0.2 + 0.25 * vivid;
    let mx = c[0].max(c[1]).max(c[2]).max(0.02);
    let cb = [c[0] / mx, c[1] / mx, c[2] / mx];
    let w_r = 0.45 * (1.0 - vivid);
    let w_s = 0.25 * (1.0 - vivid);
    let rim_out = [
        w_r + (1.0 - w_r) * cb[0],
        w_r + (1.0 - w_r) * cb[1],
        w_r + (1.0 - w_r) * cb[2],
    ];
    let sheen_c = [
        w_s + (1.0 - w_s) * cb[0],
        w_s + (1.0 - w_s) * cb[1],
        w_s + (1.0 - w_s) * cb[2],
    ];
    let y_of = |v: [f32; 3]| 0.2126 * v[0] + 0.7152 * v[1] + 0.0722 * v[2];
    let de = 1.8 + 2.2 * vivid;
    let d0 = [c[0].powf(de), c[1].powf(de), c[2].powf(de)];
    let dk = y_of(c) / y_of(d0).max(1e-4);
    let deep = [d0[0] * dk, d0[1] * dk, d0[2] * dk];
    let sc = [
        cb[0] * cb[0] * c[0],
        cb[1] * cb[1] * c[1],
        cb[2] * cb[2] * c[2],
    ];
    for j in 0..MATCAP {
        for i in 0..MATCAP {
            let mut nx = (i as f32 / (MATCAP - 1) as f32) * 2.0 - 1.0;
            let mut ny = (j as f32 / (MATCAP - 1) as f32) * 2.0 - 1.0;
            let mut r2 = nx * nx + ny * ny;
            if r2 > 1.14 {
                continue;
            }
            if r2 > 1.0 {
                let s = 1.0 / r2.sqrt();
                nx *= s;
                ny *= s;
                r2 = 1.0;
            }
            let nz = (1.0 - r2).max(0.0).sqrt();
            let nl = nx * l[0] + ny * l[1] + nz * l[2];
            let nv = (nx * frame.v[0] + ny * frame.v[1] + nz * frame.v[2]).max(0.0);
            let nd = nx * frame.d[0] + ny * frame.d[1] + nz * frame.d[2];
            let nb = nx * lb[0] + ny * lb[1] + nz * lb[2];
            let q = 1.0 - nv;
            let w = wrap + (0.45 - wrap) * q * q;
            let dif = ((nl + w) / (1.0 + w)).clamp(0.0, 1.0).powf(1.35);
            let fill = fill_k * (nx * f_dir[0] + ny * f_dir[1] + nz * f_dir[2]).max(0.0);
            let floor = 1.0 - floor_k * nd.max(0.0) * (0.6 + 0.4 * q);
            let light = (amb + kd * dif + fill) * floor;
            let t = (1.0 - light).clamp(0.0, 1.0);
            let sb = (1.0 - ((nl + 0.08) * (nl + 0.08)) / 0.18).max(0.0);
            let sss = sss_k * sb * sb * floor;
            let lit = ((nl + 0.2) / 0.8).clamp(0.0, 1.0);
            let sheen = sheen_k * q * q * lit;
            let bl = smooth(-0.3, 0.6, nb);
            let rq = smooth(0.18 - 0.12 * back_soft, 0.62 + 0.18 * back_soft, q);
            let rim =
                rim_k * rq * rq.sqrt() * (0.3 + 0.7 * bl) * (1.0 - 0.7 * nd.max(0.0));
            let hot = rq * rq * rq.sqrt() * (1.0 - 0.5 * back_soft);
            let mut rgb = [0.0f32; 3];
            for ch in 0..3 {
                let base = c[ch] + (deep[ch] - c[ch]) * t * 0.9;
                rgb[ch] = base * light
                    + sss * sc[ch]
                    + sheen * sheen_c[ch]
                    + rim * (cb[ch] + (rim_out[ch] - cb[ch]) * hot);
            }
            let m = rgb[0].max(rgb[1]).max(rgb[2]);
            if m > 0.8 {
                let kk = (0.8 + 0.2 * (1.0 - (-(m - 0.8) / 0.2).exp())) / m;
                for ch in rgb.iter_mut() {
                    *ch *= kk;
                }
            }
            if vivid > 0.0 {
                vivify(&mut rgb, vivid);
            }
            let i3 = (j * MATCAP + i) * 3;
            out[i3] = srgb_byte(rgb[0]);
            out[i3 + 1] = srgb_byte(rgb[1]);
            out[i3 + 2] = srgb_byte(rgb[2]);
        }
    }
    out
}

fn srgb_byte(v: f32) -> f32 {
    if v <= 0.0 {
        0.0
    } else if v >= 1.0 {
        255.0
    } else if v <= 0.003_130_8 {
        255.0 * 12.92 * v
    } else {
        255.0 * (1.055 * v.powf(1.0 / 2.4) - 0.055)
    }
}

fn norm3(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l <= 1e-8 {
        [0.0, 0.0, 1.0]
    } else {
        [v[0] / l, v[1] / l, v[2] / l]
    }
}

fn fabric_key(u: [f32; 3], v: [f32; 3], front: f32) -> [f32; 3] {
    let c = front.cos();
    let s = front.sin();
    norm3([
        u[0] * c + v[0] * s,
        u[1] * c + v[1] * s,
        u[2] * c + v[2] * s,
    ])
}

fn fabric_back(k: [f32; 3], v: [f32; 3]) -> [f32; 3] {
    norm3([
        0.94 * k[0] - 0.34 * v[0],
        0.94 * k[1] - 0.34 * v[1],
        0.94 * k[2] - 0.34 * v[2],
    ])
}


pub(crate) fn sample_matcap(mc: &[f32], nx: f32, ny: f32) -> [f32; 3] {
    let fx = (nx * 0.5 + 0.5) * (MATCAP - 1) as f32;
    let fy = (ny * 0.5 + 0.5) * (MATCAP - 1) as f32;
    let cx = (fx as usize).min(MATCAP - 2);
    let cy = (fy as usize).min(MATCAP - 2);
    let x = (fx - cx as f32).clamp(0.0, 1.0);
    let y = (fy - cy as f32).clamp(0.0, 1.0);
    let b = (cy * MATCAP + cx) * 3;
    let r = MATCAP * 3;
    let mut out = [0.0f32; 3];
    for ch in 0..3 {
        out[ch] = mc[b + ch] * (1.0 - x) * (1.0 - y)
            + mc[b + 3 + ch] * x * (1.0 - y)
            + mc[b + r + ch] * (1.0 - x) * y
            + mc[b + r + 3 + ch] * x * y;
    }
    out
}

/// The width of the front's fade at the outline, in design units.
const FRONT_FADE: f32 = 5.0;

/// Shade the form's texels for the current matcap.
pub(crate) fn shade_texels(
    form: &Form,
    mc: &[f32],
    ao_mul: &[f32; 256],
    lift: Option<&Lift>,
    out: &mut [u8],
) {
    let n = form.n;
    for i in 0..n * n {
        let a = form.ao[i];
        if a == 0 {
            out[i * 4 + 3] = 0;
            continue;
        }
        let mut m = match lift {
            Some(l) => ao_mul[l.ao[i] as usize] * l.k[i],
            None => ao_mul[a as usize],
        };
        let b = form.i00[i] as usize * 3;
        let x = f32::from(form.wx[i]) / 255.0;
        let y = f32::from(form.wy[i]) / 255.0;
        if lift.is_some() {
            let g = (mc[b + 1] * (1.0 - x) + mc[b + 4] * x) * (1.0 - y)
                + (mc[b + MATCAP * 3 + 1] * (1.0 - x)
                    + mc[b + MATCAP * 3 + 4] * x)
                    * y;
            m = 1.0 - (1.0 - m) * (1.1 - 0.75 * (g / 235.0).min(1.0));
        }
        let w00 = (1.0 - x) * (1.0 - y) * m;
        let w10 = x * (1.0 - y) * m;
        let w01 = (1.0 - x) * y * m;
        let w11 = x * y * m;
        let r = MATCAP * 3;
        for ch in 0..3 {
            out[i * 4 + ch] = (mc[b + ch] * w00
                + mc[b + 3 + ch] * w10
                + mc[b + r + ch] * w01
                + mc[b + r + 3 + ch] * w11)
                .clamp(0.0, 255.0) as u8;
        }
        let t = form.sd[i] / FRONT_FADE;
        let alpha = if t >= 1.0 {
            255.0
        } else if t <= 0.0 {
            0.0
        } else {
            255.0 * t * t * (3.0 - 2.0 * t)
        };
        out[i * 4 + 3] = alpha as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matcap_is_opaque_and_lit_in_the_disc() {
        let frame = super::super::form::cap_frame(&super::super::form::Rig {
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
        let mc = build_matcap([0.05, 0.5, 1.0], &frame, &Material {
            shadow: 0.35,
            highlight: 1.3,
            spread: 1.55,
            rim: 0.5,
            vivid: 0.09,
            front: 32.0f32.to_radians(),
            shine: 1.0,
            back_soft: 1.0,
            sheen: 0.0,
        });
        let center = (MATCAP / 2 * MATCAP + MATCAP / 2) * 3;
        assert!(mc[center + 2] > 100.0);
        assert!(mc[0] == 0.0 && mc[1] == 0.0 && mc[2] == 0.0);
    }
}
