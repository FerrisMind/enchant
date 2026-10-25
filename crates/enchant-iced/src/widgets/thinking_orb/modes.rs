//! The nine geometry modes behind the nine orb states.
//!
//! Each mode is a pure function `(size, t, options) -> OrbFrame`: no canvas, no
//! theme, no clock. They are line-for-line ports of the reference engine so a
//! frame matches it to the digit.

use std::f64::consts::PI;

use super::frame::{Dot, Line, OrbFrame};
use super::math::{
    Fdlibm, Projector, angle_delta, below, fib_dir, frac, hash_d, lerp, radius_scale, upto, vnoise,
};
use super::options::{OrbKnob as K, OrbOptions};

/// A mode's geometry function.
pub(crate) type ModeFrame = fn(f64, f64, &OrbOptions) -> OrbFrame;

fn r_min(o: &OrbOptions) -> f64 {
    o.or(K::RMin, 0.3)
}

// --- orbits: particles on tilted orbits ("working") ---------------------

pub(crate) fn frame_orbits(size: f64, t: f64, o: &OrbOptions) -> OrbFrame {
    let cx = size / 2.0;
    let cy = size / 2.0;
    let big_r = (size / 2.0) * 0.82;
    let pt = Projector::new(t * 0.12, 0.3, cx, cy, 1.0);
    let rs = radius_scale(size, o.or(K::RsPow, 0.6));

    let orbit_n = o.count(K::OrbitN, 12.0);
    let ghost_n = o.count(K::GhostN, 40.0);
    let particles = o.count(K::Particles, 3.0);

    let mut dots = Vec::new();
    for orb in below(orbit_n) {
        let h1 = hash_d(orb, 1.7);
        let h2 = hash_d(orb, 5.2);
        let h3 = hash_d(orb, 8.9);
        let ro = big_r * (0.45 + 0.52 * h1);
        let th = h1 * 2.0 * PI;
        let phi = (2.0 * h2 - 1.0).acos();
        // orbit plane basis (u, v perpendicular to normal n)
        let nx = phi.fsin() * th.fcos();
        let ny = phi.fcos();
        let nz = phi.fsin() * th.fsin();
        let mut ux = -ny;
        let mut uy = nx;
        let uz = 0.0;
        let ul = (ux * ux + uy * uy).sqrt().max(1e-6);
        ux /= ul;
        uy /= ul;
        let vx = ny * uz - nz * uy;
        let vy = nz * ux - nx * uz;
        let vz = nx * uy - ny * ux;
        let speed = (0.25 + 0.55 * h3) * if h3 > 0.5 { 1.0 } else { -1.0 };

        let on_orbit = |a: f64| {
            pt.project(
                (ux * a.fcos() + vx * a.fsin()) * ro,
                (uy * a.fcos() + vy * a.fsin()) * ro,
                (uz * a.fcos() + vz * a.fsin()) * ro,
            )
        };

        // ghost path
        for k in below(ghost_n) {
            let a = (k / ghost_n) * 2.0 * PI;
            let (px, py, z) = on_orbit(a);
            let depth = (z / ro + 1.0) / 2.0;
            dots.push(
                Dot::new(px, py, z, o.or(K::GhostR, 0.9) * rs, 0.72)
                    .with_alpha(o.or(K::GhostA, 0.5) * (0.4 + 0.6 * depth)),
            );
        }
        // the particles doing the work
        for m in below(particles) {
            let a = t * speed + (m / particles) * 2.0 * PI + h2 * 6.0;
            let (px, py, z) = on_orbit(a);
            let depth = (z / ro + 1.0) / 2.0;
            dots.push(Dot::new(
                px,
                py,
                z,
                (o.or(K::PartR, 1.2) + o.or(K::PartRDepth, 1.6) * depth) * rs,
                0.3 - 0.22 * depth,
            ));
        }
    }
    OrbFrame::finalize(dots, Vec::new(), r_min(o))
}

// --- the sphere-lattice modes: globe, rubik, wave ------------------------

/// One quarter-turn of a band of the rubik sphere.
struct Move {
    axis: u8,
    lo: f64,
    hi: f64,
    ang: f64,
}

struct SolveCycle {
    amount: Vec<f64>,
    active: Option<usize>,
}

// Rapid eased moves scramble, then replay in reverse (palindrome) so
// everything clicks back to solved, rests, repeats.
fn solve_cycle(time: f64, count: usize, slot_dur: f64, rest: f64) -> SolveCycle {
    let cyc = 2.0 * count as f64 * slot_dur + rest;
    let tc = time % cyc;
    let mut amount = vec![0.0; count];
    let mut active = None;
    if tc < 2.0 * count as f64 * slot_dur {
        let slot = (tc / slot_dur).floor() as usize;
        let p = (tc - slot as f64 * slot_dur) / slot_dur;
        let cl = (p / 0.7).min(1.0);
        let ep = 1.0 - (1.0 - cl).fpowf(3.0); // machine ease-out
        if slot < count {
            amount[..slot].fill(1.0);
            amount[slot] = ep;
            active = Some(slot);
        } else {
            let u = 2 * count - 1 - slot;
            amount[..u].fill(1.0);
            amount[u] = 1.0 - ep;
            active = Some(u);
        }
    }
    SolveCycle { amount, active }
}

fn make_moves(count: usize) -> Vec<Move> {
    (0..count)
        .map(|i| {
            let i = i as f64;
            let axis = ((hash_d(i, 2.3) * 3.0).floor().min(2.0)) as u8;
            let lo = -1.0 + 0.5 * (hash_d(i, 5.9) * 4.0).floor().min(3.0);
            let dir = if hash_d(i, 7.7) < 0.5 { 1.0 } else { -1.0 };
            Move {
                axis,
                lo,
                hi: lo + 0.5,
                ang: dir * PI / 2.0,
            }
        })
        .collect()
}

fn apply_moves(point: [f64; 3], moves: &[Move], cycle: &SolveCycle) -> ([f64; 3], bool) {
    let [mut x, mut y, mut z] = point;
    let mut in_active = false;
    for (i, mv) in moves.iter().enumerate() {
        if cycle.amount[i] <= 0.0 {
            continue;
        }
        let coord = match mv.axis {
            0 => x,
            1 => y,
            _ => z,
        };
        if coord < mv.lo || coord >= mv.hi {
            continue;
        }
        if cycle.active == Some(i) {
            in_active = true;
        }
        let a = mv.ang * cycle.amount[i];
        let (ca, sa) = (a.fcos(), a.fsin());
        match mv.axis {
            0 => {
                let y2 = y * ca - z * sa;
                z = y * sa + z * ca;
                y = y2;
            }
            1 => {
                let x2 = x * ca + z * sa;
                z = -x * sa + z * ca;
                x = x2;
            }
            _ => {
                let x2 = x * ca - y * sa;
                y = x * sa + y * ca;
                x = x2;
            }
        }
    }
    ([x, y, z], in_active)
}

/// Globe: a lat/long field with a sweeping scan meridian ("searching").
pub(crate) fn frame_globe(size: f64, t: f64, o: &OrbOptions) -> OrbFrame {
    let spin = 0.5;
    let cx = size / 2.0;
    let cy = size / 2.0;
    let radius = (size / 2.0) * 0.82;
    let tilt = 0.4 + 0.06 * (t * 0.35).fsin();
    let pt = Projector::new(t * spin, tilt, cx, cy, radius);
    // scan sweeps relative to the spin; scanMul scales that relative rate
    let scan = t * (spin + (1.7 - spin) * o.or(K::ScanMul, 1.0));
    let rs = radius_scale(size, o.or(K::RsPow, 0.6));
    let dim_base = o.or(K::DimBase, 1.0);

    let lat_rings = o.count(K::LatRings, 17.0);
    let lon_density = o.count(K::LonDensity, 44.0);
    let mut dots = Vec::new();
    for li in upto(lat_rings) {
        let lat = -PI / 2.0 + (li / lat_rings) * PI;
        let cos_lat = lat.fcos();
        let sin_lat = lat.fsin();
        let lon_count = (cos_lat.abs() * lon_density).round().max(1.0);
        for lj in below(lon_count) {
            let lon = (lj / lon_count) * 2.0 * PI;
            let (px, py, z) = pt.project(cos_lat * lon.fcos(), sin_lat, cos_lat * lon.fsin());
            let depth = (z + 1.0) / 2.0;
            // the scan: a moving meridian read as a size ripple, not a shine
            let d = angle_delta(lon + t * spin, scan);
            let boost = (-(d * d) / 0.18).fexp() * z.max(0.0);
            dots.push(
                Dot::new(
                    px,
                    py,
                    z,
                    (o.or(K::RBase, 0.6)
                        + o.or(K::RDepth, 1.7) * depth
                        + o.or(K::RBoost, 1.0) * boost)
                        * rs,
                    o.or(K::InkFar, 0.62) - o.or(K::InkSpan, 0.54) * depth,
                )
                // dimBase < 1 fades un-scanned dots so the meridian reads clearly
                .with_alpha(dim_base + (1.0 - dim_base) * boost.min(1.0)),
            );
        }
    }
    OrbFrame::finalize(dots, Vec::new(), r_min(o))
}

/// Rubik: bands twist in quarter turns, scramble then solve ("solving").
pub(crate) fn frame_rubik(size: f64, t: f64, o: &OrbOptions) -> OrbFrame {
    let cx = size / 2.0;
    let cy = size / 2.0;
    let big_r = (size / 2.0) * 0.82;
    let pt = Projector::new(t * 0.55, 0.35 + 0.1 * (t * 0.9).fsin(), cx, cy, big_r);
    let rs = radius_scale(size, o.or(K::RsPow, 0.6));
    let move_count = o.count(K::MoveCount, 14.0) as usize;
    let moves = make_moves(move_count);
    let cycle = solve_cycle(t, move_count, 0.42, 1.2);

    let lat_rings = o.count(K::LatRings, 15.0);
    let lon_density = o.count(K::LonDensity, 40.0);
    let mut dots = Vec::new();
    for li in upto(lat_rings) {
        let lat = -PI / 2.0 + (li / lat_rings) * PI;
        let cos_lat = lat.fcos();
        let sin_lat = lat.fsin();
        let lon_count = (cos_lat.abs() * lon_density).round().max(1.0);
        for lj in below(lon_count) {
            let lon = (lj / lon_count) * 2.0 * PI;
            let ([x, y, z], in_active) = apply_moves(
                [cos_lat * lon.fcos(), sin_lat, cos_lat * lon.fsin()],
                &moves,
                &cycle,
            );
            let (px, py, zr) = pt.project(x, y, z);
            let depth = (zr + 1.0) / 2.0;
            // the band being turned inks a touch darker: the "hand"
            let active_r = if in_active {
                o.or(K::RActive, 0.3)
            } else {
                0.0
            };
            let active_ink = if in_active { 0.14 } else { 0.0 };
            dots.push(Dot::new(
                px,
                py,
                zr,
                (o.or(K::RBase, 0.6) + o.or(K::RDepth, 1.7) * depth + active_r) * rs,
                o.or(K::InkFar, 0.62) - o.or(K::InkSpan, 0.54) * depth - active_ink,
            ));
        }
    }
    OrbFrame::finalize(dots, Vec::new(), r_min(o))
}

/// Wave: a waveform rolls through the rings ("listening").
pub(crate) fn frame_wave(size: f64, t: f64, o: &OrbOptions) -> OrbFrame {
    let cx = size / 2.0;
    let cy = size / 2.0;
    // 0.76 base x 1.15: the undulation pulls the sphere inward, so wave read
    // ~15% smaller than the other lattice modes; scaled up to match them
    let big_r = (size / 2.0) * 0.874;
    let pt = Projector::new(t * 0.18, 0.38, cx, cy, 1.0);
    let rs = radius_scale(size, o.or(K::RsPow, 0.6));

    let rings = o.count(K::Rings, 15.0);
    let lon_density = o.count(K::LonDensity, 40.0);
    let mut dots = Vec::new();
    for ri in upto(rings) {
        let lat = -PI / 2.0 + (ri / rings) * PI;
        let cos_lat = lat.fcos();
        let sin_lat = lat.fsin();
        // two waves, different tempi: organic, never quite repeating
        let w = 0.62 * (t * 2.1 - ri * 0.52).fsin() + 0.38 * (t * 1.27 + ri * 0.83).fsin();
        let rr = big_r * (0.88 + 0.105 * w);
        let lon_count = (cos_lat.abs() * lon_density).round().max(1.0);
        for lj in below(lon_count) {
            let lon = (lj / lon_count) * 2.0 * PI;
            let (px, py, z) = pt.project(
                cos_lat * lon.fcos() * rr,
                sin_lat * rr,
                cos_lat * lon.fsin() * rr,
            );
            let depth = (z / big_r + 1.0) / 2.0;
            let crest = w.max(0.0);
            dots.push(Dot::new(
                px,
                py,
                z,
                (o.or(K::RBase, 0.6) + o.or(K::RDepth, 1.7) * depth) * (1.0 + 0.4 * crest) * rs,
                0.66 - 0.56 * depth - 0.1 * crest,
            ));
        }
    }
    OrbFrame::finalize(dots, Vec::new(), r_min(o))
}

// --- web: a constellation wires itself ("connecting") --------------------

/// Nodes drift on the sphere under slow value noise; any pair closer than
/// `thr` grows an edge, and bright packets run along re-picked node pairs.
pub(crate) fn frame_web(size: f64, t: f64, o: &OrbOptions) -> OrbFrame {
    let cx = size / 2.0;
    let cy = size / 2.0;
    let big_r = (size / 2.0) * 0.8 * o.or(K::Spread, 1.0);
    // the projector carries the radius as its scale, so node vectors stay
    // unit-length and distances below are in unit-sphere space
    let pt = Projector::new(t * 0.12, 0.32, cx, cy, big_r);
    let rs = radius_scale(size, o.or(K::RsPow, 0.6));

    let node_n = o.count(K::NodeN, 30.0);
    let thr = o.or(K::Thr, 0.72);
    let node_r = o.or(K::NodeR, 1.4);
    let node_r_depth = o.or(K::NodeRDepth, 1.8);

    // nodes: fib lattice + slow noise wander, renormalised to the surface
    let nodes: Vec<[f64; 3]> = below(node_n)
        .map(|i| {
            let d = fib_dir(i, node_n);
            let x = d[0] + 0.3 * (vnoise(i * 0.31 + 9.0, t * 0.24) - 0.5) * 2.0;
            let y = d[1] + 0.3 * (vnoise(i * 0.53 + 27.0, t * 0.21) - 0.5) * 2.0;
            let z = d[2] + 0.3 * (vnoise(i * 0.77 + 55.0, t * 0.27) - 0.5) * 2.0;
            let l = (x * x + y * y + z * z).sqrt();
            [x / l, y / l, z / l]
        })
        .collect();

    let mut lines = Vec::new();
    let mut dots = Vec::new();

    // edges between close neighbours, alpha by proximity + depth
    for i in 0..nodes.len() {
        for j in (i + 1)..nodes.len() {
            let dx = nodes[i][0] - nodes[j][0];
            let dy = nodes[i][1] - nodes[j][1];
            let dz = nodes[i][2] - nodes[j][2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            if dist >= thr {
                continue;
            }
            let (x1, y1, z1) = pt.project(nodes[i][0], nodes[i][1], nodes[i][2]);
            let (x2, y2, z2) = pt.project(nodes[j][0], nodes[j][1], nodes[j][2]);
            let depth = ((z1 + z2) / 2.0 + 1.0) / 2.0;
            lines.push(
                Line::new(x1, y1, x2, y2, 0.42, (o.or(K::LineW, 0.8) * rs).max(0.6))
                    .with_alpha((1.0 - dist / thr) * (0.3 + 0.55 * depth)),
            );
        }
    }

    for (i, node) in nodes.iter().enumerate() {
        let (px, py, z) = pt.project(node[0], node[1], node[2]);
        let depth = (z + 1.0) / 2.0;
        let pulse = 1.0 + 0.25 * (t * 1.4 + i as f64 * 2.7).fsin();
        dots.push(Dot::new(
            px,
            py,
            z,
            (node_r + node_r_depth * depth) * pulse * rs,
            0.55 - 0.45 * depth,
        ));
    }

    // signals: bright packets running between paired nodes
    let signals = o.count(K::Signals, 5.0);
    if !nodes.is_empty() {
        for s in below(signals) {
            let seg = (t * 0.55 + s * 7.31).floor();
            let a = (hash_d(seg, s * 3.1 + 1.7) * node_n).floor() as usize;
            let b = (hash_d(seg, s * 5.7 + 4.2) * node_n).floor() as usize;
            if a == b || a >= nodes.len() || b >= nodes.len() {
                continue;
            }
            let f = frac(t * 0.55 + s * 7.31);
            let x = lerp(nodes[a][0], nodes[b][0], f);
            let y = lerp(nodes[a][1], nodes[b][1], f);
            let z = lerp(nodes[a][2], nodes[b][2], f);
            let l = (x * x + y * y + z * z).sqrt().max(1e-6);
            let (px, py, zr) = pt.project(x / l, y / l, z / l);
            let depth = (zr + 1.0) / 2.0;
            dots.push(
                Dot::new(px, py, zr, (node_r * 1.5 + node_r_depth * depth) * rs, 0.05)
                    .with_alpha(0.5 + 0.5 * depth),
            );
        }
    }

    OrbFrame::finalize(dots, lines, r_min(o))
}

// --- braid: three strands plait around the sphere ("weaving") ------------

pub(crate) fn frame_braid(size: f64, t: f64, o: &OrbOptions) -> OrbFrame {
    let cx = size / 2.0;
    let cy = size / 2.0;
    let big_r = (size / 2.0) * 0.76;
    let pt = Projector::new(t * 0.4, 0.3, cx, cy, 1.0);
    let rs = radius_scale(size, o.or(K::RsPow, 0.6));

    let mut dots = Vec::new();
    let ghost_n = o.count(K::GhostN, 150.0);
    for i in below(ghost_n) {
        let d = fib_dir(i, ghost_n);
        let (px, py, z) = pt.project(d[0] * big_r, d[1] * big_r, d[2] * big_r);
        let depth = (z / big_r + 1.0) / 2.0;
        dots.push(Dot::new(px, py, z, 0.8 * rs, 0.78).with_alpha(0.1 + 0.22 * depth));
    }

    let strand_n = o.count(K::StrandN, 52.0);
    let turns = o.or(K::Turns, 3.0);
    for s in 0..3 {
        let phase = (f64::from(s) / 3.0) * 2.0 * PI;
        for i in below(strand_n) {
            // u walks pole to pole; the frac() drift slides the whole strand along
            let u = (frac(i / strand_n + t * 0.045) * 2.0 - 1.0) * 0.96;
            let surf = (1.0 - u * u).max(0.0).sqrt();
            let end_fade = ((1.0 - u.abs()) / 0.1).min(1.0);
            let a = u * PI * turns + phase;
            // radial breathing: strands trade places, the over/under of a plait
            let weave = 1.0 + 0.075 * (u * PI * turns * 2.0 + phase * 2.0 + t * 0.8).fsin();
            let rr = surf * big_r * weave;
            let (px, py, zr) = pt.project(a.fcos() * rr, u * big_r * weave, a.fsin() * rr);
            let depth = (zr / big_r + 1.0) / 2.0;
            dots.push(
                Dot::new(
                    px,
                    py,
                    zr,
                    (o.or(K::RBase, 1.2) + o.or(K::RDepth, 1.8) * depth) * rs,
                    0.55 - 0.45 * depth,
                )
                .with_alpha(end_fade * (0.45 + 0.55 * depth)),
            );
        }
    }
    OrbFrame::finalize(dots, Vec::new(), r_min(o))
}

// --- ribbon: an undulating sash on a great circle ("composing"/"breathing")

/// The same painter drives "breathing" (ring) through the `faceOn` knob: a
/// face-on circle whose radius, not its out-of-plane offset, undulates.
pub(crate) fn frame_ribbon(size: f64, t: f64, o: &OrbOptions) -> OrbFrame {
    let cx = size / 2.0;
    let cy = size / 2.0;
    let big_r = (size / 2.0) * 0.78;
    // spin scales the 3D tumble; spin=0 freezes the band's orientation,
    // leaving only the traveling undulation
    let spin = o.or(K::Spin, 1.0);
    let cam_tilt = 0.3;
    let pt = Projector::new(t * 0.1 * spin, cam_tilt, cx, cy, 1.0);
    let rs = radius_scale(size, o.or(K::RsPow, 0.6));
    let face_on = o.get(K::FaceOn).is_some_and(|v| v != 0.0);

    let mut dots = Vec::new();
    let ghost_n = o.count(K::GhostN, 150.0);
    for i in below(ghost_n) {
        let d = fib_dir(i, ghost_n);
        let (px, py, z) = pt.project(d[0] * big_r, d[1] * big_r, d[2] * big_r);
        let depth = (z / big_r + 1.0) / 2.0;
        dots.push(Dot::new(px, py, z, 0.8 * rs, 0.78).with_alpha(0.1 + 0.22 * depth));
    }

    // The band plane, precessing (frozen when spin=0). The projection squashes
    // the band's great circle vertically by cos(ta + camTilt); face-on sets
    // ta = -camTilt so that term is 1 and the band reads as a true circle.
    let ya = t * 0.24 * spin;
    let ta = if face_on {
        -cam_tilt
    } else {
        0.55 + 0.3 * (t * 0.18).fsin() * spin
    };
    let ux = ya.fcos();
    let uy = 0.0;
    let uz = ya.fsin();
    let vx = -uz * ta.fsin();
    let vy = ta.fcos();
    let vz = ux * ta.fsin();
    // plane normal n = u x v
    let nx = uy * vz - uz * vy;
    let ny = uz * vx - ux * vz;
    let nz = ux * vy - uy * vx;

    // Radial lobes swell past R, so pull the base radius in by (most of) the
    // wobble amplitude. The silhouette then stays inside the frame however far
    // the deformation is pushed.
    let wob_mul = o.or(K::WobMul, 1.0);
    let wob_amp = 0.23 * wob_mul;
    let base_r = if face_on {
        big_r / (1.0 + 0.85 * wob_amp)
    } else {
        big_r
    };

    let base_lanes = o.or(K::Lanes, 5.0);
    let segs = o.count(K::Segs, 88.0);
    let lanes = (base_lanes * o.or(K::BandMul, 1.0))
        .round()
        .clamp(1.0, 1024.0);
    for w in below(lanes) {
        let lane_off = (w - (lanes - 1.0) / 2.0) * 0.075;
        let edge = (w - (lanes - 1.0) / 2.0).abs() / ((lanes - 1.0) / 2.0).max(1.0);
        for k in below(segs) {
            let a = (k / segs) * 2.0 * PI;
            // the undulation: two traveling waves along the band; wobMul
            // scales the deformation, 0 is a clean band
            let wob = (0.16 * (a * 3.0 - t * 1.7 + w * 0.22).fsin()
                + 0.07 * (a * 5.0 + t * 1.1).fsin())
                * wob_mul;
            // Face-on modulates the in-plane RADIUS, so lobes genuinely swell
            // and pinch; ribbon keeps the out-of-plane sash wobble.
            let radial = if face_on { 1.0 + wob } else { 1.0 };
            let off = if face_on { lane_off } else { lane_off + wob };
            let x = ux * a.fcos() + vx * a.fsin() + nx * off;
            let y = uy * a.fcos() + vy * a.fsin() + ny * off;
            let z = uz * a.fcos() + vz * a.fsin() + nz * off;
            let l = (x * x + y * y + z * z).sqrt();
            let rr = base_r * radial;
            let (px, py, zr) = pt.project((x / l) * rr, (y / l) * rr, (z / l) * rr);
            let depth = (zr / big_r + 1.0) / 2.0;
            dots.push(
                Dot::new(
                    px,
                    py,
                    zr,
                    (o.or(K::RBase, 1.1) + o.or(K::RDepth, 1.7) * depth) * (1.0 - 0.25 * edge) * rs,
                    0.52 - 0.44 * depth + 0.18 * edge,
                )
                .with_alpha(0.4 + 0.6 * depth),
            );
        }
    }
    OrbFrame::finalize(dots, Vec::new(), r_min(o))
}

// --- morph: a dotted outline cycling circle, triangle, square ("shaping") --

#[derive(Clone, Copy)]
enum Shape {
    Circle,
    Triangle,
    Square,
}

const TRIANGLE: &[(f64, f64)] = &[(0.0, -0.26), (0.24, 0.16), (-0.24, 0.16)];
// 5-vertex walk so the path STARTS at top-centre like the other shapes
const SQUARE: &[(f64, f64)] = &[
    (0.0, -0.2),
    (0.2, -0.2),
    (0.2, 0.2),
    (-0.2, 0.2),
    (-0.2, -0.2),
];
const CYCLE: [Shape; 3] = [Shape::Circle, Shape::Triangle, Shape::Square];

fn smooth_e(x: f64) -> f64 {
    x * x * (3.0 - 2.0 * x)
}

fn poly_path(verts: &[(f64, f64)], f: f64) -> (f64, f64) {
    let count = verts.len();
    let lengths: Vec<f64> = (0..count)
        .map(|i| {
            let a = verts[i];
            let b = verts[(i + 1) % count];
            (b.0 - a.0).hypot(b.1 - a.1)
        })
        .collect();
    let total: f64 = lengths.iter().sum();
    let mut target = f * total;
    let mut i = 0;
    while target > lengths[i] && i < count - 1 {
        target -= lengths[i];
        i += 1;
    }
    let a = verts[i];
    let b = verts[(i + 1) % count];
    let ff = if lengths[i] != 0.0 {
        (target / lengths[i]).min(1.0)
    } else {
        0.0
    };
    (a.0 + (b.0 - a.0) * ff, a.1 + (b.1 - a.1) * ff)
}

/// A point of `shape`'s closed outline at arc-length fraction `f`.
fn shape_path(shape: Shape, f: f64) -> (f64, f64) {
    match shape {
        Shape::Circle => {
            let a = -PI / 2.0 + f * 2.0 * PI;
            (a.fcos() * 0.24, a.fsin() * 0.24)
        }
        Shape::Triangle => poly_path(TRIANGLE, f),
        Shape::Square => poly_path(SQUARE, f),
    }
}

// low floor keeps sparse outlines possible while never degenerating
fn morph_n(d: f64) -> f64 {
    (34.0 * d).round().max(6.0)
}

const HOLD: f64 = 1.4;
const MORPH: f64 = 0.9;
const SEG: f64 = HOLD + MORPH;

/// Each frame blends the two neighbouring paths, then lays the dots EVENLY
/// along the blended outline, so spacing stays uniform at every instant.
pub(crate) fn frame_morph(size: f64, t: f64, o: &OrbOptions) -> OrbFrame {
    let k_count = CYCLE.len();
    let tc = t % (SEG * k_count as f64);
    // `shape` 0 / 1 / 2 holds one outline instead of cycling; anything else
    // keeps the tuned circle, triangle, square loop
    let held = o
        .get(K::Shape)
        .filter(|s| *s >= 0.0 && *s < k_count as f64)
        .map(|s| s.floor() as usize);
    let k = held.unwrap_or_else(|| (tc / SEG).floor() as usize);
    let local = if held.is_some() {
        t % SEG
    } else {
        tc - k as f64 * SEG
    };
    let m = if held.is_some() {
        0.0
    } else if local > HOLD {
        smooth_e((local - HOLD) / MORPH)
    } else {
        0.0
    };
    let sprd = o.or(K::Spread, 1.0);

    // blend the two shape PATHS at m, then measure the blended outline
    let path_a = CYCLE[k];
    let path_b = if held.is_some() {
        path_a
    } else {
        CYCLE[(k + 1) % k_count]
    };
    const SAMPLES: usize = 160;
    let pts: Vec<(f64, f64)> = (0..SAMPLES)
        .map(|i| {
            let f = i as f64 / SAMPLES as f64;
            let a = shape_path(path_a, f);
            let b = shape_path(path_b, f);
            (
                (a.0 + (b.0 - a.0) * m) * sprd,
                (a.1 + (b.1 - a.1) * m) * sprd,
            )
        })
        .collect();
    let lengths: Vec<f64> = (0..SAMPLES)
        .map(|i| {
            let a = pts[i];
            let b = pts[(i + 1) % SAMPLES];
            (b.0 - a.0).hypot(b.1 - a.1)
        })
        .collect();
    let total: f64 = lengths.iter().sum();

    // dot radius depends ONLY on rDot (the size knob); the count sets the gaps.
    // Formed shapes breathe a little (uniform pulse).
    let n = morph_n(o.or(K::IconD, 1.0)).min(1024.0);
    let re = o.or(K::RDot, 0.021) * 1.35 * sprd;
    let pulse = 1.0 + 0.02 * (local * 3.1).fsin();

    let c2 = size / 2.0;
    let mut dots = Vec::new();
    let mut seg = 0;
    let mut acc = 0.0;
    for k2 in below(n) {
        let target = (k2 / n) * total;
        while acc + lengths[seg] < target && seg < SAMPLES - 1 {
            acc += lengths[seg];
            seg += 1;
        }
        let a = pts[seg];
        let b = pts[(seg + 1) % SAMPLES];
        let f = if lengths[seg] != 0.0 {
            ((target - acc) / lengths[seg]).min(1.0)
        } else {
            0.0
        };
        let x = (a.0 + (b.0 - a.0) * f) * pulse;
        let y = (a.1 + (b.1 - a.1) * f) * pulse;
        dots.push(Dot::new(
            c2 + x * size,
            c2 + y * size,
            0.0,
            (re * size).max(0.35),
            0.1,
        ));
    }
    OrbFrame::finalize(dots, Vec::new(), o.or(K::RMin, 0.3))
}
