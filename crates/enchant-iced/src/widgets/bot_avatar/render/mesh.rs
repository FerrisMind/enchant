//! A small surface renderer for the things the bots wear.
//!
//! Ported from `mesh.ts`. A worn object is a handful of parametric surfaces
//! (a dome, a band, a cone, a tube along a curve, a revolved profile), each
//! sampled into a grid in its own space once and kept; every frame the grid
//! is projected through the head's rotation and lit from the body's light
//! with a material: felt, knit, paper, gold, silver, velvet, satin, soft
//! plastic, leather, pearl, a cut gem.
//!
//! It is painted a strip at a time: the visible stretch of each row of the
//! grid is one shape, filled with a gradient that carries the light along
//! the row from the corners' own normals, so a dome is a dozen fills rather
//! than hundreds of quads and its light runs smoothly round it; the rows
//! step it the other way, finely enough not to show. Strips go back to
//! front across all the surfaces of an object. Cut facets are painted a
//! cell at a time, flat. Small spheres — pearls, beads, rivets — are single
//! radial fills.

use std::collections::HashMap;
use std::f32::consts::TAU;
use std::rc::Rc;
use std::sync::{Arc, Mutex, OnceLock};

use tiny_skia::{
    FillRule, GradientStop, LineCap, LineJoin, Mask, Paint, Path, PathBuilder, Pixmap,
    RadialGradient, SpreadMode, Stroke, Transform,
};

use super::super::canvas::{paint, stroke};
use super::super::color::Rgba;

/// A point or vector in an object's own 3D space.
pub(crate) type V3 = [f32; 3];
/// A point on screen, in the space the context's transform maps from.
pub(crate) type P2 = [f32; 2];
/// A colour, 0–255 a channel.
pub(crate) type Rgb = [f32; 3];

/// A colouring: a normal in view space to a colour.
pub(crate) type Shade = Rc<dyn Fn(V3) -> Rgb>;

/// How the worn object is seen: a linear map from the object's space (about
/// the origin `o`) to view space — rows for screen x (right), screen y
/// (down) and depth (toward the viewer) — the light, and the scale.
#[derive(Clone, Copy, Debug)]
pub(crate) struct View {
    /// The nine matrix entries, row-major.
    pub m: [f32; 9],
    /// The origin in object space.
    pub o: V3,
    /// Toward the light, in view space.
    pub l: V3,
    /// The half vector with the view, in view space.
    pub h: V3,
    /// Device pixels per unit of this space.
    pub px: f32,
}

impl View {
    /// A point in object space, on screen.
    #[must_use]
    pub fn to_screen(self, p: V3) -> P2 {
        let x = p[0] - self.o[0];
        let y = p[1] - self.o[1];
        let z = p[2] - self.o[2];
        let m = self.m;
        [
            m[0] * x + m[1] * y + m[2] * z,
            m[3] * x + m[4] * y + m[5] * z,
        ]
    }

    /// How far a point sits toward the viewer.
    #[must_use]
    pub fn depth(self, p: V3) -> f32 {
        let m = self.m;
        m[6] * (p[0] - self.o[0]) + m[7] * (p[1] - self.o[1]) + m[8] * (p[2] - self.o[2])
    }

    /// A direction in the object's space, turned into view space, unit length.
    #[must_use]
    pub fn dir(self, v: V3) -> V3 {
        let m = self.m;
        norm([
            m[0] * v[0] + m[1] * v[1] + m[2] * v[2],
            m[3] * v[0] + m[4] * v[1] + m[5] * v[2],
            m[6] * v[0] + m[7] * v[1] + m[8] * v[2],
        ])
    }
}

/// Make a view from its matrix, origin, light and half vector.
#[must_use]
pub(crate) fn make_view(m: [f32; 9], o: V3, l: V3, h: V3, px: f32) -> View {
    View { m, o, l, h, px }
}

/// Subtract two vectors.
#[must_use]
pub(crate) fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// The cross product.
#[must_use]
pub(crate) fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The dot product.
#[must_use]
pub(crate) fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// A unit vector.
#[must_use]
pub(crate) fn norm(v: V3) -> V3 {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l <= 1e-12 {
        [0.0, 0.0, 1.0]
    } else {
        [v[0] / l, v[1] / l, v[2] / l]
    }
}

/// About one segment per `quad` device pixels, within `lo … hi`.
#[must_use]
pub(crate) fn segs(view: View, len: f32, lo: usize, hi: usize, quad: f32) -> usize {
    let n = (len * view.px / quad).round();
    (n as usize).clamp(lo, hi)
}

/// A surface sampled on a grid: positions, the cells' normals and, for
/// smooth shading, the corners'.
#[derive(Clone, Debug)]
pub(crate) struct Mesh {
    /// Samples across.
    pub nu: usize,
    /// Samples down.
    pub nv: usize,
    /// Closed round u: the first and last columns are one.
    pub closed: bool,
    /// Positions, three floats a corner.
    pub pos: Vec<f32>,
    /// The cells' normals, three floats a cell.
    pub fnv: Vec<f32>,
    /// The corners' normals, three floats a corner.
    pub vn: Option<Vec<f32>>,
}

/// Sample `surf(u, v)` on an `nu × nv` grid.
///
/// `inside` gives, for each cell, a point inside the object (or on a tube's
/// centre line); every normal is turned away from it, so a surface needs no
/// care over its winding. With `smooth`, each corner gets the mean of its
/// cells' normals — a surface closed round u, or pinched to a pole, is
/// joined up.
pub(crate) fn make_mesh<P, I>(
    surf: P,
    nu: usize,
    nv: usize,
    inside: I,
    smooth: bool,
) -> Mesh
where
    P: Fn(f32, f32) -> V3,
    I: Fn(f32, f32) -> V3,
{
    let w = nu + 1;
    let n = w * (nv + 1);
    let mut pos = vec![0.0f32; n * 3];
    let mut ext = 0.0f32;
    let mut k = 0;
    for j in 0..=nv {
        for i in 0..=nu {
            let p = surf(i as f32 / nu as f32, j as f32 / nv as f32);
            pos[k * 3] = p[0];
            pos[k * 3 + 1] = p[1];
            pos[k * 3 + 2] = p[2];
            ext = ext.max(
                (p[0] - pos[0]).abs() + (p[1] - pos[1]).abs() + (p[2] - pos[2]).abs(),
            );
            k += 1;
        }
    }
    let at = |k: usize| -> V3 { [pos[k * 3], pos[k * 3 + 1], pos[k * 3 + 2]] };
    let mut fnv = vec![0.0f32; nu * nv * 3];
    for j in 0..nv {
        for i in 0..nu {
            let a = j * w + i;
            let pa = at(a);
            let pb = at(a + 1);
            let pc = at(a + w + 1);
            let pd = at(a + w);
            let mut n = cross(sub(pc, pa), sub(pd, pb));
            let mid = [
                (pa[0] + pb[0] + pc[0] + pd[0]) * 0.25,
                (pa[1] + pb[1] + pc[1] + pd[1]) * 0.25,
                (pa[2] + pb[2] + pc[2] + pd[2]) * 0.25,
            ];
            let ins = inside((i as f32 + 0.5) / nu as f32, (j as f32 + 0.5) / nv as f32);
            if dot(n, sub(mid, ins)) < 0.0 {
                n = [-n[0], -n[1], -n[2]];
            }
            n = norm(n);
            let f = (j * nu + i) * 3;
            fnv[f] = n[0];
            fnv[f + 1] = n[1];
            fnv[f + 2] = n[2];
        }
    }
    let eps = 1e-6 + ext * 1e-5;
    let same = |p: usize, q: usize| -> bool {
        (pos[p * 3] - pos[q * 3]).abs()
            + (pos[p * 3 + 1] - pos[q * 3 + 1]).abs()
            + (pos[p * 3 + 2] - pos[q * 3 + 2]).abs()
            < eps
    };
    let mut closed = true;
    for j in 0..=nv {
        if !same(j * w, j * w + nu) {
            closed = false;
            break;
        }
    }
    let mut vn = None;
    if smooth {
        let mut acc = vec![0.0f32; n * 3];
        for j in 0..nv {
            for i in 0..nu {
                let f = (j * nu + i) * 3;
                let a = j * w + i;
                for k in [a, a + 1, a + w + 1, a + w] {
                    acc[k * 3] += fnv[f];
                    acc[k * 3 + 1] += fnv[f + 1];
                    acc[k * 3 + 2] += fnv[f + 2];
                }
            }
        }
        let join = |acc: &mut [f32], ks: &[usize]| {
            let mut x = 0.0;
            let mut y = 0.0;
            let mut z = 0.0;
            for &k in ks {
                x += acc[k * 3];
                y += acc[k * 3 + 1];
                z += acc[k * 3 + 2];
            }
            for &k in ks {
                acc[k * 3] = x;
                acc[k * 3 + 1] = y;
                acc[k * 3 + 2] = z;
            }
        };
        if closed {
            for j in 0..=nv {
                join(&mut acc, &[j * w, j * w + nu]);
            }
        }
        for j in [0, nv] {
            let mut pole = true;
            for i in 1..=nu {
                if !same(j * w, j * w + i) {
                    pole = false;
                    break;
                }
            }
            if pole {
                let ks: Vec<usize> = (0..w).map(|i| j * w + i).collect();
                join(&mut acc, &ks);
            }
        }
        for k in 0..n {
            let l = (acc[k * 3] * acc[k * 3]
                + acc[k * 3 + 1] * acc[k * 3 + 1]
                + acc[k * 3 + 2] * acc[k * 3 + 2])
                .sqrt();
            let l = if l <= 1e-12 { 1.0 } else { l };
            acc[k * 3] /= l;
            acc[k * 3 + 1] /= l;
            acc[k * 3 + 2] /= l;
        }
        vn = Some(acc);
    }
    Mesh {
        nu,
        nv,
        closed,
        pos,
        fnv,
        vn,
    }
}

/// A tube along a curve: `path(u)` its centre line, `up` the direction its
/// cross-section is squared to, `prof(u, φ)` the cross-section's offset at
/// angle φ — along the up direction and across it.
pub(crate) fn make_tube<P, F>(
    path: P,
    up: V3,
    prof: F,
    nu: usize,
    nv: usize,
    closed: bool,
) -> Mesh
where
    P: Fn(f32) -> V3,
    F: Fn(f32, f32) -> [f32; 2],
{
    let e = 1e-3;
    let wrap = |u: f32| -> f32 {
        if closed {
            u.rem_euclid(1.0)
        } else {
            u.clamp(0.0, 1.0)
        }
    };
    let surf = |u: f32, v: f32| -> V3 {
        let c = path(u);
        let t = norm(sub(path(wrap(u + e)), path(wrap(u - e))));
        let k = dot(up, t);
        let nn = norm([up[0] - t[0] * k, up[1] - t[1] * k, up[2] - t[2] * k]);
        let bn = cross(t, nn);
        let ab = prof(u, v * TAU);
        [
            c[0] + nn[0] * ab[0] + bn[0] * ab[1],
            c[1] + nn[1] * ab[0] + bn[1] * ab[1],
            c[2] + nn[2] * ab[0] + bn[2] * ab[1],
        ]
    };
    let inside = |u: f32, _v: f32| path(u);
    make_mesh(surf, nu, nv, inside, true)
}

/// A cross-section: an ellipse (`p` = 1) squared toward a rounded box as `p`
/// falls, `a` along the up direction and `b` across it.
#[must_use]
pub(crate) fn section(a: f32, b: f32, p: f32) -> impl Fn(f32, f32) -> [f32; 2] {
    move |_u, ph| {
        let c = ph.cos();
        let s = ph.sin();
        [
            a * c.signum() * c.abs().powf(p),
            b * s.signum() * s.abs().powf(p),
        ]
    }
}

/// A mesh as this frame sees it: every corner on screen with its depth and
/// its normal in view space, and every cell's normal and depth.
#[derive(Clone, Debug)]
pub(crate) struct Proj {
    pub nu: usize,
    pub nv: usize,
    pub closed: bool,
    pub x: Vec<f32>,
    pub y: Vec<f32>,
    pub d: Vec<f32>,
    /// The corners' normals, three a corner; `None` for a faceted mesh.
    pub n: Option<Vec<f32>>,
    /// The cells' normals, three a cell.
    pub f: Vec<f32>,
    /// The cells' mean depths.
    pub cd: Vec<f32>,
}

/// Project a mesh through a view.
#[must_use]
pub(crate) fn project_mesh(view: View, mesh: &Mesh) -> Proj {
    let (nu, nv) = (mesh.nu, mesh.nv);
    let w = nu + 1;
    let n = w * (nv + 1);
    let c = nu * nv;
    let m = view.m;
    let o = view.o;
    let mut x = vec![0.0f32; n];
    let mut y = vec![0.0f32; n];
    let mut d = vec![0.0f32; n];
    for k in 0..n {
        let px = mesh.pos[k * 3] - o[0];
        let py = mesh.pos[k * 3 + 1] - o[1];
        let pz = mesh.pos[k * 3 + 2] - o[2];
        x[k] = m[0] * px + m[1] * py + m[2] * pz;
        y[k] = m[3] * px + m[4] * py + m[5] * pz;
        d[k] = m[6] * px + m[7] * py + m[8] * pz;
    }
    let turn = |src: &[f32], dst: &mut [f32], count: usize| {
        for k in 0..count {
            let a = src[k * 3];
            let b = src[k * 3 + 1];
            let cc = src[k * 3 + 2];
            let xx = m[0] * a + m[1] * b + m[2] * cc;
            let yy = m[3] * a + m[4] * b + m[5] * cc;
            let zz = m[6] * a + m[7] * b + m[8] * cc;
            let l = (xx * xx + yy * yy + zz * zz).sqrt();
            let l = if l <= 1e-12 { 1.0 } else { l };
            dst[k * 3] = xx / l;
            dst[k * 3 + 1] = yy / l;
            dst[k * 3 + 2] = zz / l;
        }
    };
    let nout = mesh.vn.as_ref().map(|vn| {
        let mut out = vec![0.0f32; n * 3];
        turn(vn, &mut out, n);
        out
    });
    let mut f = vec![0.0f32; c * 3];
    turn(&mesh.fnv, &mut f, c);
    let mut cd = vec![0.0f32; c];
    for j in 0..nv {
        for i in 0..nu {
            let ci = j * nu + i;
            let a = j * w + i;
            cd[ci] = (d[a] + d[a + 1] + d[a + w] + d[a + w + 1]) * 0.25;
        }
    }
    Proj {
        nu,
        nv,
        closed: mesh.closed,
        x,
        y,
        d,
        n: nout,
        f,
        cd,
    }
}

/// The screen bounds of some projected meshes.
#[must_use]
pub(crate) fn bounds_of(ps: &[&Proj]) -> (f32, f32, f32, f32) {
    let (mut x0, mut y0, mut x1, mut y1) = (f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    for p in ps {
        for k in 0..p.x.len() {
            x0 = x0.min(p.x[k]);
            y0 = y0.min(p.y[k]);
            x1 = x1.max(p.x[k]);
            y1 = y1.max(p.y[k]);
        }
    }
    (x0, y0, x1, y1)
}

/// One painted surface group.
pub(crate) struct Layer {
    pub parts: Vec<Proj>,
    pub shade: Shade,
    /// Keep the cells facing away (thin plates seen from behind).
    pub cull: bool,
    /// A cell at a time, one colour each: cut facets.
    pub flat: bool,
    /// A nudge toward the viewer in the depth sort.
    pub bias: f32,
    /// Keep only the cells whose depth passes: one side of a split.
    pub keep: Option<Rc<dyn Fn(f32) -> bool>>,
}

impl Layer {
    /// A layer with the reference's defaults.
    pub fn new(parts: Vec<Proj>, shade: Shade) -> Self {
        Self {
            parts,
            shade,
            cull: true,
            flat: false,
            bias: 0.0,
            keep: None,
        }
    }
}

struct Piece {
    d: f32,
    layer: usize,
    part: usize,
    j: usize,
    cols: Vec<usize>,
    cells: Vec<usize>,
}

fn monotone(mx: &[f32], my: &[f32], s: usize, e: usize) -> bool {
    let ax = mx[e] - mx[s];
    let ay = my[e] - my[s];
    let mut prev = f32::NEG_INFINITY;
    for k in s..=e {
        let t = (mx[k] - mx[s]) * ax + (my[k] - my[s]) * ay;
        if t < prev - 1e-9 {
            return false;
        }
        prev = t;
    }
    true
}

fn clamp_byte(v: f32) -> u8 {
    v.clamp(0.0, 255.0).round() as u8
}

/// A colour as tiny-skia's paint understands it.
fn rgba(c: Rgb, alpha: f32) -> Rgba {
    Rgba::from_rgb(clamp_byte(c[0]), clamp_byte(c[1]), clamp_byte(c[2])).with_alpha(alpha.clamp(0.0, 1.0))
}

/// Options for one paint call.
#[derive(Clone, Copy)]
pub(crate) struct PaintOpts<'a> {
    /// A device-space clip.
    pub mask: Option<&'a Mask>,
    /// A multiplier on every colour's alpha.
    pub alpha: f32,
}

impl Default for PaintOpts<'_> {
    fn default() -> Self {
        Self {
            mask: None,
            alpha: 1.0,
        }
    }
}

/// Paint several surfaces as one, back to front, the parts facing away
/// dropped. Returns the corners painted, for an outline.
pub(crate) fn paint_layers(
    pixmap: &mut Pixmap,
    layers: &[Layer],
    px: f32,
    space: super::super::canvas::Affine,
    opts: PaintOpts,
) -> Vec<P2> {
    let mut pieces: Vec<Piece> = Vec::new();
    let mut corners: Vec<P2> = Vec::new();
    for (li, l) in layers.iter().enumerate() {
        for (pi, p) in l.parts.iter().enumerate() {
            let (nu, nv) = (p.nu, p.nv);
            let w = nu + 1;
            let vis = |c: usize| -> bool {
                (!l.cull || p.f[c * 3 + 2] > -0.04)
                    && l.keep.as_ref().is_none_or(|keep| keep(p.cd[c]))
            };
            for j in 0..nv {
                if l.flat {
                    for i in 0..nu {
                        let c = j * nu + i;
                        if !vis(c) {
                            continue;
                        }
                        pieces.push(Piece {
                            d: p.cd[c] + l.bias,
                            layer: li,
                            part: pi,
                            j,
                            cols: vec![i, i + 1],
                            cells: vec![c],
                        });
                        corners.push([p.x[j * w + i], p.y[j * w + i]]);
                        corners.push([p.x[(j + 1) * w + i + 1], p.y[(j + 1) * w + i + 1]]);
                    }
                    continue;
                }
                let mut runs: Vec<Vec<usize>> = Vec::new();
                let mut open = false;
                for i in 0..nu {
                    if vis(j * nu + i) {
                        if !open {
                            runs.push(Vec::new());
                            open = true;
                        }
                        if let Some(r) = runs.last_mut() {
                            r.push(i);
                        }
                    } else {
                        open = false;
                    }
                }
                if p.closed
                    && runs.len() > 1
                    && runs[0].first() == Some(&0)
                    && runs.last().and_then(|r| r.last()) == Some(&(nu - 1))
                {
                    let last = runs.pop().unwrap_or_default();
                    let mut merged = last;
                    merged.extend_from_slice(&runs[0]);
                    runs[0] = merged;
                }
                for run in runs {
                    let mut cols = run.clone();
                    if let Some(last) = run.last() {
                        cols.push(last + 1);
                    }
                    let mut mx = Vec::with_capacity(cols.len());
                    let mut my = Vec::with_capacity(cols.len());
                    for (k, &col) in cols.iter().enumerate() {
                        let t = j * w + col;
                        let b = t + w;
                        mx.push((p.x[t] + p.x[b]) * 0.5);
                        my.push((p.y[t] + p.y[b]) * 0.5);
                        if k == 0 || k == cols.len() - 1 {
                            corners.push([p.x[t], p.y[t]]);
                            corners.push([p.x[b], p.y[b]]);
                        } else if j == 0 {
                            corners.push([p.x[t], p.y[t]]);
                        } else if j == nv - 1 {
                            corners.push([p.x[b], p.y[b]]);
                        }
                    }
                    let mut s = 0;
                    while s + 1 < cols.len() {
                        let mut e = s + 1;
                        while e + 1 < cols.len() && monotone(&mx, &my, s, e + 1) {
                            e += 1;
                        }
                        let mut dd = 0.0;
                        for k in s..e {
                            if let Some(&r) = run.get(k) {
                                dd += p.cd[j * nu + r];
                            }
                        }
                        pieces.push(Piece {
                            d: dd / (e - s) as f32 + l.bias,
                            layer: li,
                            part: pi,
                            j,
                            cols: cols[s..=e].to_vec(),
                            cells: run[s..e].to_vec(),
                        });
                        s = e;
                    }
                }
            }
        }
    }
    pieces.sort_by(|a, b| a.d.total_cmp(&b.d));
    let grow = 0.5 / px;
    for piece in &pieces {
        let l = &layers[piece.layer];
        let p = &l.parts[piece.part];
        let (j, cols, cells) = (piece.j, &piece.cols, &piece.cells);
        let w = p.nu + 1;
        let nc = cols.len();
        let mut tx = Vec::with_capacity(nc);
        let mut ty = Vec::with_capacity(nc);
        let mut bx = Vec::with_capacity(nc);
        let mut by = Vec::with_capacity(nc);
        let t0 = j * w + cols[0];
        let tn = j * w + cols[nc - 1];
        let mut ax = (p.x[tn] + p.x[tn + w]) * 0.5 - (p.x[t0] + p.x[t0 + w]) * 0.5;
        let mut ay = (p.y[tn] + p.y[tn + w]) * 0.5 - (p.y[t0] + p.y[t0 + w]) * 0.5;
        let al = (ax * ax + ay * ay).sqrt();
        let (ux, uy) = if al > 1e-9 {
            ax /= al;
            ay /= al;
            (ax, ay)
        } else {
            (0.0, 0.0)
        };
        for (k, &col) in cols.iter().enumerate() {
            let t = j * w + col;
            let b = t + w;
            let mut dx = p.x[t] - p.x[b];
            let mut dy = p.y[t] - p.y[b];
            let dl = (dx * dx + dy * dy).sqrt();
            if dl > 1e-9 {
                dx /= dl;
                dy /= dl;
            } else {
                dx = 0.0;
                dy = 0.0;
            }
            let e = if k == 0 {
                -grow
            } else if k == nc - 1 {
                grow
            } else {
                0.0
            };
            tx.push(p.x[t] + dx * grow + ux * e);
            ty.push(p.y[t] + dy * grow + uy * e);
            bx.push(p.x[b] - dx * grow + ux * e);
            by.push(p.y[b] - dy * grow + uy * e);
        }
        let colour = |k: usize| -> Rgb {
            if let Some(n) = p.n.as_ref().filter(|_| !l.flat) {
                let t = (j * w + cols[k]) * 3;
                let b = t + w * 3;
                (l.shade)(norm([
                    n[t] + n[b],
                    n[t + 1] + n[b + 1],
                    n[t + 2] + n[b + 2],
                ]))
            } else {
                let c0 = cells[k.saturating_sub(1).min(cells.len() - 1)] * 3;
                let c1 = cells[k.min(cells.len() - 1)] * 3;
                if l.flat {
                    (l.shade)([p.f[c1], p.f[c1 + 1], p.f[c1 + 2]])
                } else {
                    (l.shade)(norm([
                        p.f[c0] + p.f[c1],
                        p.f[c0 + 1] + p.f[c1 + 1],
                        p.f[c0 + 2] + p.f[c1 + 2],
                    ]))
                }
            }
        };
        let mut builder = PathBuilder::new();
        builder.move_to(tx[0], ty[0]);
        for k in 1..nc {
            builder.line_to(tx[k], ty[k]);
        }
        for k in (0..nc).rev() {
            builder.line_to(bx[k], by[k]);
        }
        builder.close();
        let Some(path) = builder.finish() else {
            continue;
        };
        let mut pnt = paint(Rgba::from_rgb(0, 0, 0));
        pnt.anti_alias = true;
        if l.flat || al < 1e-6 {
            let c = rgba(colour(0), opts.alpha);
            pnt.set_color_rgba8(c.r, c.g, c.b, c.a);
        } else {
            let m0x = (p.x[t0] + p.x[t0 + w]) * 0.5;
            let m0y = (p.y[t0] + p.y[t0 + w]) * 0.5;
            let mut stops: Vec<GradientStop> = Vec::with_capacity(nc);
            let mut last: Option<Rgb> = None;
            for k in 0..nc {
                let c = colour(k);
                if let Some(prev) = last {
                    if k < nc - 1
                        && (c[0] - prev[0]).abs()
                            + (c[1] - prev[1]).abs()
                            + (c[2] - prev[2]).abs()
                            < 5.0
                    {
                        continue;
                    }
                }
                let t = j * w + cols[k];
                let off = (((p.x[t] + p.x[t + w]) * 0.5 - m0x) * ax
                    + ((p.y[t] + p.y[t + w]) * 0.5 - m0y) * ay)
                    / (al * al);
                let col = rgba(c, opts.alpha);
                stops.push(GradientStop::new(
                    off.clamp(0.0, 1.0),
                    tiny_skia::Color::from_rgba8(col.r, col.g, col.b, col.a),
                ));
                last = Some(c);
            }
            if stops.len() >= 2 {
                if let Some(shader) = tiny_skia::LinearGradient::new(
                    tiny_skia::Point::from_xy(m0x, m0y),
                    tiny_skia::Point::from_xy(
                        (p.x[tn] + p.x[tn + w]) * 0.5,
                        (p.y[tn] + p.y[tn + w]) * 0.5,
                    ),
                    stops,
                    SpreadMode::Pad,
                    Transform::identity(),
                ) {
                    pnt.shader = shader;
                } else {
                    let c = rgba(colour(0), opts.alpha);
                    pnt.set_color_rgba8(c.r, c.g, c.b, c.a);
                }
            } else {
                let c = rgba(colour(0), opts.alpha);
                pnt.set_color_rgba8(c.r, c.g, c.b, c.a);
            }
        }
        pixmap.fill_path(
            &path,
            &pnt,
            FillRule::Winding,
            space.transform(),
            opts.mask,
        );
    }
    corners
}

/// The convex hull of some points, the way the reference builds outlines.
#[must_use]
pub(crate) fn hull(pts: &[P2]) -> Vec<P2> {
    let mut p = pts.to_vec();
    if p.len() < 3 {
        return p;
    }
    p.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    let cr = |o: P2, a: P2, b: P2| -> f32 {
        (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    };
    let mut lo: Vec<P2> = Vec::new();
    let mut up: Vec<P2> = Vec::new();
    for &q in &p {
        while lo.len() >= 2 && cr(lo[lo.len() - 2], lo[lo.len() - 1], q) <= 0.0 {
            lo.pop();
        }
        lo.push(q);
    }
    for &q in p.iter().rev() {
        while up.len() >= 2 && cr(up[up.len() - 2], up[up.len() - 1], q) <= 0.0 {
            up.pop();
        }
        up.push(q);
    }
    lo.pop();
    up.pop();
    lo.extend(up);
    lo
}

/// A closed polygon as a path.
#[must_use]
pub(crate) fn poly_path(points: &[P2]) -> Option<Path> {
    if points.len() < 2 {
        return None;
    }
    let mut b = PathBuilder::new();
    b.move_to(points[0][0], points[0][1]);
    for q in &points[1..] {
        b.line_to(q[0], q[1]);
    }
    b.close();
    b.finish()
}

/// A soft contact shadow cast on the body under an object's lower edge.
#[allow(clippy::too_many_arguments)]
pub(crate) fn contact(
    pixmap: &mut Pixmap,
    clip: Option<&Mask>,
    space: super::super::canvas::Affine,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    a: f32,
) {
    if rx <= 0.0 || ry <= 0.0 {
        return;
    }
    let mut b = PathBuilder::new();
    let n = 40;
    for i in 0..=n {
        let t = i as f32 / n as f32 * TAU;
        let x = cx + rx * t.cos();
        let y = cy + ry * t.sin();
        if i == 0 {
            b.move_to(x, y);
        } else {
            b.line_to(x, y);
        }
    }
    b.close();
    let Some(path) = b.finish() else {
        return;
    };
    let stops = vec![
        GradientStop::new(0.0, tiny_skia::Color::from_rgba8(0, 0, 0, (a * 255.0) as u8)),
        GradientStop::new(
            0.55,
            tiny_skia::Color::from_rgba8(0, 0, 0, (a * 0.45 * 255.0) as u8),
        ),
        GradientStop::new(1.0, tiny_skia::Color::from_rgba8(0, 0, 0, 0)),
    ];
    let Some(shader) = RadialGradient::new(
        tiny_skia::Point::from_xy(0.0, 0.0),
        tiny_skia::Point::from_xy(0.0, 0.0),
        1.0,
        stops,
        SpreadMode::Pad,
        Transform::from_row(1.0, 0.0, 0.0, ry / rx, cx, cy),
    ) else {
        return;
    };
    let mut p = paint(Rgba::from_rgb(0, 0, 0));
    p.shader = shader;
    pixmap.fill_path(
        &path,
        &p,
        FillRule::Winding,
        space.transform(),
        clip,
    );
}

/// A stable pseudo-random per index, so fuzz does not shimmer frame to frame.
pub(crate) fn rnd(i: i32, k: i32) -> f32 {
    let mut h = (i as u32)
        .wrapping_mul(2_654_435_761)
        .wrapping_add((k as u32).wrapping_mul(40_503));
    h = (h ^ (h >> 15)).wrapping_mul(2_246_822_507);
    h = (h ^ (h >> 13)).wrapping_mul(3_266_489_909);
    (h ^ (h >> 16)) as f32 / 4_294_967_296.0
}

fn bounds2(pts: &[P2]) -> (f32, f32, f32, f32, f32, f32, f32, f32) {
    let (mut x0, mut y0, mut x1, mut y1) = (f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    for p in pts {
        x0 = x0.min(p[0]);
        y0 = y0.min(p[1]);
        x1 = x1.max(p[0]);
        y1 = y1.max(p[1]);
    }
    (x0, y0, x1, y1, (x0 + x1) * 0.5, (y0 + y1) * 0.5, x1 - x0, y1 - y0)
}

fn grain_tile() -> &'static [u8; 48 * 48] {
    static TILE: OnceLock<[u8; 48 * 48]> = OnceLock::new();
    TILE.get_or_init(|| {
        let n = 48usize;
        let mut out = [0u8; 48 * 48];
        for i in 0..n * n {
            let x = i % n;
            let y = i / n;
            let mut v = 0.0;
            for k in -2i32..=2 {
                let xx = (x as i32 + k).rem_euclid(n as i32) as usize;
                let yy = (y as i32 + k).rem_euclid(n as i32) as usize;
                v += rnd((xx + yy * n) as i32, 5);
            }
            v /= 5.0;
            out[i] = (255.0 * (0.82 + 0.36 * v)).round().clamp(0.0, 255.0) as u8;
        }
        out
    })
}

/// The grain, laid over a painted outline; its pattern repeats in the
/// context's space.
pub(crate) fn grain_over(
    pixmap: &mut Pixmap,
    outline: &[P2],
    alpha: f32,
    scale_k: f32,
    space: super::super::canvas::Affine,
    mask: Option<&Mask>,
) {
    if outline.len() < 3 {
        return;
    }
    let tile = grain_tile();
    let Some(poly) = poly_path(outline) else {
        return;
    };
    let mut clip = Mask::new(pixmap.width(), pixmap.height()).expect("non-zero mask");
    clip.fill_path(
        &poly,
        FillRule::Winding,
        true,
        space.transform(),
    );
    let (mut sx0, mut sy0, mut sx1, mut sy1) = (f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    for p in outline {
        let (x, y) = space.point(p[0], p[1]);
        sx0 = sx0.min(x);
        sy0 = sy0.min(y);
        sx1 = sx1.max(x);
        sy1 = sy1.max(y);
    }
    let (px0, py0) = (
        sx0.floor().max(0.0) as u32,
        sy0.floor().max(0.0) as u32,
    );
    let (px1, py1) = (
        sx1.ceil().clamp(0.0, pixmap.width() as f32) as u32,
        sy1.ceil().clamp(0.0, pixmap.height() as f32) as u32,
    );
    let inv = space.inverse().0;
    let period = 48.0 * scale_k;
    let cw = pixmap.width() as usize;
    let data = pixmap.data_mut();
    let cm = clip.data();
    let am = alpha.clamp(0.0, 1.0);
    for py in py0..py1 {
        for px in px0..px1 {
            let idx = py as usize * cw + px as usize;
            if mask.is_some_and(|m| m.data()[idx] == 0) || cm[idx] == 0 {
                continue;
            }
            let fx = px as f32 + 0.5;
            let fy = py as f32 + 0.5;
            let ux = (inv[0] * fx + inv[2] * fy + inv[4]) / period;
            let uy = (inv[1] * fx + inv[3] * fy + inv[5]) / period;
            let tx = (ux.rem_euclid(1.0) * 48.0) as usize;
            let ty = (uy.rem_euclid(1.0) * 48.0) as usize;
            let m = f32::from(tile[ty.min(47) * 48 + tx.min(47)]) / 255.0;
            let f = 1.0 + (m - 1.0) * am;
            let o = idx * 4;
            data[o] = (data[o] as f32 * f).round().min(255.0) as u8;
            data[o + 1] = (data[o + 1] as f32 * f).round().min(255.0) as u8;
            data[o + 2] = (data[o + 2] as f32 * f).round().min(255.0) as u8;
        }
    }
}

/// Short hairs standing off an outline: a felt or pile edge. Two batched
/// strokes, a lighter and a darker set.
#[allow(clippy::too_many_arguments)]
pub(crate) fn fuzz(
    pixmap: &mut Pixmap,
    outline: &[P2],
    base: Rgb,
    seed: i32,
    len: f32,
    px: f32,
    space: super::super::canvas::Affine,
    mask: Option<&Mask>,
) {
    let n = outline.len();
    if n < 3 {
        return;
    }
    let mut total = 0.0;
    let mut seg: Vec<f32> = Vec::with_capacity(n);
    for i in 0..n {
        let a = outline[i];
        let c = outline[(i + 1) % n];
        let d = ((c[0] - a[0]).powi(2) + (c[1] - a[1]).powi(2)).sqrt();
        seg.push(d);
        total += d;
    }
    let count = (total * (px * 0.5).min(2.4))
        .round()
        .clamp(24.0, 360.0) as usize;
    let (_, _, _, _, cx, cy, ..) = bounds2(outline);
    let mut light = PathBuilder::new();
    let mut dark = PathBuilder::new();
    let mut si = 0usize;
    let mut acc = 0.0;
    for k in 0..count {
        let want = (k as f32 + rnd(k as i32, seed) * 0.8) * (total / count as f32);
        while si + 1 < n && acc + seg[si] < want {
            acc += seg[si];
            si += 1;
        }
        let a = outline[si % n];
        let c = outline[(si + 1) % n];
        let t = if seg[si % n] != 0.0 {
            (want - acc) / seg[si % n]
        } else {
            0.0
        };
        let x = a[0] + (c[0] - a[0]) * t;
        let y = a[1] + (c[1] - a[1]) * t;
        let mut ox = x - cx;
        let mut oy = y - cy;
        let ol = (ox * ox + oy * oy).sqrt();
        if ol <= 1e-6 {
            ox = 0.0;
            oy = 1.0;
        } else {
            ox /= ol;
            oy /= ol;
        }
        let ang = (rnd(k as i32, seed + 1) - 0.5) * 0.9;
        let (sa, ca) = ang.sin_cos();
        let hx = ox * ca - oy * sa;
        let hy = ox * sa + oy * ca;
        let l = len * (0.35 + rnd(k as i32, seed + 2));
        let p = if rnd(k as i32, seed + 3) > 0.5 {
            &mut light
        } else {
            &mut dark
        };
        p.move_to(x - hx * 0.4, y - hy * 0.4);
        p.line_to(x + hx * l, y + hy * l);
    }
    let light_c = super::super::color::shade(
        Rgba::from_rgb(clamp_byte(base[0]), clamp_byte(base[1]), clamp_byte(base[2])),
        f64::from(0.14),
        0.0,
    );
    let dark_c = super::super::color::shade(
        Rgba::from_rgb(clamp_byte(base[0]), clamp_byte(base[1]), clamp_byte(base[2])),
        f64::from(-0.06),
        0.0,
    );
    if let Some(p) = light.finish() {
        stroke(
            pixmap,
            &p,
            &super::super::canvas::paint_alpha(light_c, 0.55),
            0.4,
            space,
            mask,
        );
    }
    if let Some(p) = dark.finish() {
        stroke(
            pixmap,
            &p,
            &super::super::canvas::paint_alpha(dark_c, 0.55),
            0.4,
            space,
            mask,
        );
    }
}

/// A linear light gradient between two points, for a stroked pompom.
pub(crate) fn linear_paint(from: P2, to: P2, stops: &[(f32, Rgba)]) -> Paint<'static> {
    let mut p = paint(Rgba::from_rgb(0, 0, 0));
    if let Some(shader) = tiny_skia::LinearGradient::new(
        tiny_skia::Point::from_xy(from[0], from[1]),
        tiny_skia::Point::from_xy(to[0], to[1]),
        stops
            .iter()
            .map(|&(t, c)| GradientStop::new(t, tiny_skia::Color::from_rgba8(c.r, c.g, c.b, c.a)))
            .collect(),
        SpreadMode::Pad,
        Transform::identity(),
    ) {
        p.shader = shader;
    }
    p
}

/// Stroke a path with a paint that may carry a gradient.
pub(crate) fn stroke_path(
    pixmap: &mut Pixmap,
    path: &Path,
    paint: &Paint<'_>,
    width: f32,
    space: super::super::canvas::Affine,
    mask: Option<&Mask>,
) {
    let stroke = Stroke {
        width,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };
    pixmap.stroke_path(path, paint, &stroke, space.transform(), mask);
}

pub(crate) const SPRITE_GOLD: &[(f32, [u8; 3])] = &[
    (0.0, [255, 251, 232]),
    (0.08, [255, 231, 160]),
    (0.3, [238, 181, 70]),
    (0.62, [173, 114, 20]),
    (0.87, [90, 56, 6]),
    (1.0, [140, 92, 24]),
];
pub(crate) const SPRITE_SILVER: &[(f32, [u8; 3])] = &[
    (0.0, [255, 255, 255]),
    (0.08, [243, 245, 249]),
    (0.32, [198, 204, 214]),
    (0.64, [122, 128, 140]),
    (0.88, [61, 65, 73]),
    (1.0, [107, 113, 124]),
];
pub(crate) const SPRITE_PEARL: &[(f32, [u8; 3])] = &[
    (0.0, [255, 255, 255]),
    (0.1, [253, 249, 247]),
    (0.45, [235, 228, 230]),
    (0.8, [198, 188, 203]),
    (1.0, [168, 157, 181]),
];

/// A small sphere as one radial fill.
pub(crate) fn sphere_sprite(
    pixmap: &mut Pixmap,
    view: View,
    space: super::super::canvas::Affine,
    c: V3,
    rad: f32,
    kind: &[(f32, [u8; 3])],
) {
    let s = view.to_screen(c);
    let h = view.h;
    let stops: Vec<GradientStop> = kind
        .iter()
        .map(|&(t, col)| {
            GradientStop::new(
                t,
                tiny_skia::Color::from_rgba8(col[0], col[1], col[2], 255),
            )
        })
        .collect();
    let Some(shader) = RadialGradient::new(
        tiny_skia::Point::from_xy(0.0, 0.0),
        tiny_skia::Point::from_xy(0.0, 0.0),
        1.0,
        stops,
        SpreadMode::Pad,
        Transform::from_row(
            rad,
            0.0,
            0.0,
            rad,
            s[0] + h[0] * rad * 0.85,
            s[1] + h[1] * rad * 0.85,
        ),
    ) else {
        return;
    };
    let mut b = PathBuilder::new();
    b.push_circle(s[0], s[1], rad);
    let Some(path) = b.finish() else {
        return;
    };
    let mut p = paint(Rgba::from_rgb(0, 0, 0));
    p.shader = shader;
    pixmap.fill_path(
        &path,
        &p,
        FillRule::Winding,
        space.transform(),
        None,
    );
}

/// A small sphere in an arbitrary colour.
pub(crate) fn sphere_sprite_rgb(
    pixmap: &mut Pixmap,
    view: View,
    space: super::super::canvas::Affine,
    c: V3,
    rad: f32,
    col: Rgb,
) {
    let s = view.to_screen(c);
    let h = view.h;
    let stops = vec![
        GradientStop::new(0.0, tiny_skia::Color::from_rgba8(255, 255, 255, 255)),
        GradientStop::new(
            0.1,
            tiny_skia::Color::from_rgba8(
                clamp_byte(col[0] * 0.3 + 255.0 * 0.7),
                clamp_byte(col[1] * 0.3 + 255.0 * 0.7),
                clamp_byte(col[2] * 0.3 + 255.0 * 0.7),
                255,
            ),
        ),
        GradientStop::new(
            0.35,
            tiny_skia::Color::from_rgba8(clamp_byte(col[0]), clamp_byte(col[1]), clamp_byte(col[2]), 255),
        ),
        GradientStop::new(
            0.85,
            tiny_skia::Color::from_rgba8(
                clamp_byte(col[0] * 0.5),
                clamp_byte(col[1] * 0.5),
                clamp_byte(col[2] * 0.5),
                255,
            ),
        ),
        GradientStop::new(
            1.0,
            tiny_skia::Color::from_rgba8(
                clamp_byte(col[0] * 0.62),
                clamp_byte(col[1] * 0.62),
                clamp_byte(col[2] * 0.62),
                255,
            ),
        ),
    ];
    let Some(shader) = RadialGradient::new(
        tiny_skia::Point::from_xy(0.0, 0.0),
        tiny_skia::Point::from_xy(0.0, 0.0),
        1.0,
        stops,
        SpreadMode::Pad,
        Transform::from_row(
            rad,
            0.0,
            0.0,
            rad,
            s[0] + h[0] * rad * 0.85,
            s[1] + h[1] * rad * 0.85,
        ),
    ) else {
        return;
    };
    let mut b = PathBuilder::new();
    b.push_circle(s[0], s[1], rad);
    let Some(path) = b.finish() else {
        return;
    };
    let mut p = paint(Rgba::from_rgb(0, 0, 0));
    p.shader = shader;
    pixmap.fill_path(&path, &p, FillRule::Winding, space.transform(), None);
}

/// A cut stone set in a surface: an eight-sided girdle, a table on top and
/// the crown facets between, each lit by its own normal.
pub(crate) fn cut_gem(
    pixmap: &mut Pixmap,
    view: View,
    space: super::super::canvas::Affine,
    c: V3,
    n: V3,
    t: V3,
    rad: f32,
    col: Rgb,
) {
    let b = cross(n, t);
    let at = |r: f32, a: f32, h: f32| -> V3 {
        let x = r * a.cos();
        let y = r * a.sin();
        [
            c[0] + x * t[0] + y * b[0] + h * n[0],
            c[1] + x * t[1] + y * b[1] + h * n[1],
            c[2] + x * t[2] + y * b[2] + h * n[2],
        ]
    };
    let k = 8;
    let tr = 0.55;
    let th = 0.36 * rad;
    let mut girdle = Vec::with_capacity(k);
    let mut table = Vec::with_capacity(k);
    for i in 0..k {
        let a = (i as f32 + 0.5) / k as f32 * TAU;
        girdle.push(at(rad, a, 0.0));
        table.push(at(rad * tr, a, th));
    }
    let shade = gem(view, col);
    let mut poly = |pts: &[V3], nn: V3| {
        let nv = view.dir(nn);
        if nv[2] <= 0.02 {
            return;
        }
        let rgb = shade(nv);
        let mut bb = PathBuilder::new();
        for (i, &p) in pts.iter().enumerate() {
            let s = view.to_screen(p);
            if i == 0 {
                bb.move_to(s[0], s[1]);
            } else {
                bb.line_to(s[0], s[1]);
            }
        }
        bb.close();
        if let Some(path) = bb.finish() {
            let mut p = paint(rgba(rgb, 1.0));
            p.anti_alias = true;
            pixmap.fill_path(&path, &p, FillRule::Winding, space.transform(), None);
        }
    };
    for i in 0..k {
        let g0 = girdle[i];
        let g1 = girdle[(i + 1) % k];
        let t1 = table[(i + 1) % k];
        let t0 = table[i];
        let mut nn = cross(sub(t1, g0), sub(t0, g1));
        if dot(nn, n) < 0.0 {
            nn = [-nn[0], -nn[1], -nn[2]];
        }
        poly(&[g0, g1, t1, t0], norm(nn));
    }
    poly(&table, n);
    let nv = view.dir(n);
    if nv[2] > 0.1 {
        let l = view.l;
        let s0 = view.to_screen(at(0.0, 0.0, th));
        let base = (-l[1]).atan2(-l[0]);
        let s1 = view.to_screen(at(rad * tr * 0.8, base + 2.2, th));
        let s2 = view.to_screen(at(rad * tr * 0.9, base + std::f32::consts::PI, th));
        let s3 = view.to_screen(at(rad * tr * 0.8, base + std::f32::consts::PI + 0.94, th));
        let mut bb = PathBuilder::new();
        bb.move_to(s0[0], s0[1]);
        bb.line_to(s1[0], s1[1]);
        bb.line_to(s2[0], s2[1]);
        bb.line_to(s3[0], s3[1]);
        bb.close();
        if let Some(path) = bb.finish() {
            let mut p = paint(Rgba::from_rgb(255, 255, 255).with_alpha(0.3));
            p.anti_alias = true;
            pixmap.fill_path(&path, &p, FillRule::Winding, space.transform(), None);
        }
    }
}

/* ── materials ─────────────────────────────────────────────────────── */

fn clamp01(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

/// A mix of two colours.
#[must_use]
pub(crate) fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// A colour scaled.
#[must_use]
pub(crate) fn scale(a: Rgb, k: f32) -> Rgb {
    [a[0] * k, a[1] * k, a[2] * k]
}

/// Felt and knit: matte, the light wrapping softly round, a faint sheen
/// where the fibres lie side-on; a dark felt keeps a floor of light so it
/// reads charcoal on its lit side.
#[must_use]
pub(crate) fn felt(view: View, base: Rgb, k: f32) -> Shade {
    Rc::new(move |n: V3| {
        let nl = dot(n, view.l);
        let dif = clamp01((nl + 0.4) / 1.4).powf(1.35);
        let graze = (1.0 - clamp01(n[2])).powi(2);
        let lit = (0.22 + 0.95 * dif + 0.22 * graze * clamp01(nl + 0.6)) * k;
        let floor = 26.0 * dif * k;
        [
            base[0] * lit + floor,
            base[1] * lit + floor,
            base[2] * lit + floor * 1.05,
        ]
    })
}

/// Velvet: deep in its folds, its colour lifting toward the silhouette.
#[must_use]
pub(crate) fn velvet(view: View, base: Rgb) -> Shade {
    let sheen = mix(base, [255.0, 255.0, 255.0], 0.3);
    Rc::new(move |n: V3| {
        let nl = dot(n, view.l);
        let dif = clamp01((nl + 0.3) / 1.3);
        let graze = (1.0 - clamp01(n[2])).powf(1.6);
        mix(
            scale(base, 0.2 + 0.7 * dif),
            sheen,
            0.6 * graze * (0.4 + 0.6 * dif),
        )
    })
}

/// Paper: Lambert with a soft wide sheen.
#[must_use]
pub(crate) fn paper(view: View, base: Rgb) -> Shade {
    Rc::new(move |n: V3| {
        let dif = clamp01((dot(n, view.l) + 0.2) / 1.2);
        let spec = clamp01(dot(n, view.h)).powf(22.0) * 0.3;
        mix(
            scale(base, 0.36 + 0.7 * dif),
            [255.0, 255.0, 255.0],
            spec,
        )
    })
}

/// Soft plastic and leather: diffuse, a broad sheen and a tighter hot spot,
/// a thin rim of light at the silhouette.
#[must_use]
pub(crate) fn plastic(view: View, base: Rgb, gloss: f32) -> Shade {
    Rc::new(move |n: V3| {
        let dif = clamp01((dot(n, view.l) + 0.25) / 1.25);
        let nh = clamp01(dot(n, view.h));
        let spec = (nh.powf(16.0) * 0.2 + nh.powf(80.0) * 0.6) * gloss;
        let rim = (1.0 - clamp01(n[2])).powi(3) * 0.14 * gloss;
        mix(
            scale(base, 0.28 + 0.78 * dif),
            [255.0, 255.0, 255.0],
            clamp01(spec + rim),
        )
    })
}

/// Satin: a silk ribbon's light — a wide soft sheen that runs along the folds
/// and a deep, saturated shade between them.
#[must_use]
pub(crate) fn satin(view: View, base: Rgb) -> Shade {
    let deep = scale(base, 0.42);
    let sheen = mix(base, [255.0, 255.0, 255.0], 0.55);
    Rc::new(move |n: V3| {
        let dif = clamp01((dot(n, view.l) + 0.35) / 1.35);
        let nh = clamp01(dot(n, view.h));
        let s = nh.powf(6.0) * 0.5 + nh.powf(40.0) * 0.35;
        mix(mix(deep, base, dif), sheen, clamp01(s))
    })
}

/// Metal: the colour of what it reflects — a bright sky above, a dark floor
/// below, a bright horizon between — tinted, with a hot glint.
#[must_use]
pub(crate) fn metal(view: View, tint: Rgb, dark: Rgb, glint: Rgb) -> Shade {
    Rc::new(move |n: V3| {
        let rz = 2.0 * n[2] * n[2] - 1.0;
        let ry = 2.0 * n[2] * n[1];
        let up = -ry;
        let sky = clamp01(0.45 + up * 0.85 + rz * 0.1);
        let horizon = (-((up + 0.12) / 0.16).powi(2)).exp() * 0.55;
        let nl = clamp01(dot(n, view.l));
        let c = mix(dark, tint, clamp01(0.1 + sky * 0.72 + nl * 0.3));
        let spec = clamp01(dot(n, view.h)).powf(70.0) + horizon * 0.4;
        mix(c, glint, clamp01(spec))
    })
}

/// Gold.
#[must_use]
pub(crate) fn gold(view: View) -> Shade {
    metal(
        view,
        [255.0, 200.0, 88.0],
        [86.0, 50.0, 8.0],
        [255.0, 247.0, 220.0],
    )
}

/// Silver.
#[must_use]
pub(crate) fn silver(view: View) -> Shade {
    metal(
        view,
        [232.0, 236.0, 242.0],
        [58.0, 62.0, 72.0],
        [255.0, 255.0, 255.0],
    )
}

/// A pearl: soft white with a pink-blue play and a small bright glint.
#[must_use]
pub(crate) fn pearl(view: View) -> Shade {
    Rc::new(move |n: V3| {
        let dif = clamp01((dot(n, view.l) + 0.4) / 1.4);
        let base = mix(
            [238.0, 228.0, 228.0],
            [212.0, 222.0, 242.0],
            1.0 - clamp01(n[2]),
        );
        mix(
            scale(base, 0.62 + 0.42 * dif),
            [255.0, 255.0, 255.0],
            clamp01(dot(n, view.h)).powf(40.0) * 0.8,
        )
    })
}

/// A cut gem's facet: its colour deepening toward the edges, the facets
/// turned to the light bright, a white fire on the one that catches it.
#[must_use]
pub(crate) fn gem(view: View, base: Rgb) -> Shade {
    let deep = scale(base, 0.22);
    let clear = mix(base, [255.0, 255.0, 255.0], 0.3);
    Rc::new(move |n: V3| {
        let c = mix(
            deep,
            clear,
            clamp01(clamp01(n[2]) * 0.75 + dot(n, view.l) * 0.4),
        );
        mix(c, [255.0, 255.0, 255.0], clamp01(dot(n, view.h)).powf(24.0) * 0.95)
    })
}

/* ── mesh cache ───────────────────────────────────────────────────── */

/// A process-wide cache of sampled meshes, keyed by owner and grid.
pub(crate) fn keep_mesh<F>(key: &str, make: F) -> Arc<Mesh>
where
    F: FnOnce() -> Mesh,
{
    static CACHE: OnceLock<Mutex<HashMap<String, Arc<Mesh>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(c) = cache.lock() {
        if let Some(m) = c.get(key) {
            return Arc::clone(m);
        }
    }
    let mesh = Arc::new(make());
    if let Ok(mut c) = cache.lock() {
        if c.len() > 300 {
            c.clear();
        }
        c.insert(key.to_owned(), Arc::clone(&mesh));
    }
    mesh
}
