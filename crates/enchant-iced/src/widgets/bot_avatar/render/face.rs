//! The face, printed on a sphere behind the front cap.
//!
//! Ported from `drawFace` in `draw.ts`. Every feature is placed by longitude
//! and latitude on a sphere of radius [`FACE_R`], so a turn slides it round
//! the head, narrows it at the edge, and hides it once it goes past the side.
//! The eyes are one stroked hairpin curve whose numbers morph between open,
//! shut, laughing and sleeping, so a state change is a real morph rather than
//! a cross-fade. The mouth is a single closed outline built from two cubic
//! edges and round caps, blending between a smile, an open mouth and a
//! sleeping "o".

use tiny_skia::{Mask, PathBuilder, Pixmap};

use super::super::canvas::{Affine, fill, paint_alpha, stroke};
use super::super::color::Rgba;
use super::super::engine::Pose;
use super::super::types::BotAvatarFace;

/// Radius of the sphere the face is drawn on, in body units.
pub(crate) const FACE_R: f32 = 30.0;

/// The distance between the eyes' centres, in body units.
pub(crate) const EYE_GAP: f32 = 25.0;

/// The eyes' radius, in body units.
const EYE_RX: f32 = 6.3;

/// The eyes' vertical place for each face kind.
fn eye_y(face: BotAvatarFace) -> f32 {
    match face {
        BotAvatarFace::Eyes => 1.0,
        BotAvatarFace::Mouth => -3.5,
    }
}

/// Where an eye sits on the face, for anything worn over it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct EyeSpot {
    /// Screen offset on the sphere.
    pub x: f32,
    /// Screen offset on the sphere.
    pub y: f32,
    /// Horizontal foreshortening.
    pub sx: f32,
    /// Vertical foreshortening.
    pub sy: f32,
    /// How much the spot faces the viewer.
    pub z: f32,
    /// The design position, before the head's turn.
    pub ox: f32,
    /// The design position, before the head's turn.
    pub oy: f32,
    /// The sphere's depth at the design position.
    pub fz: f32,
}

/// A point on the face sphere.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SpherePoint {
    pub x: f32,
    pub y: f32,
    pub sx: f32,
    pub sy: f32,
    pub z: f32,
}

/// Put a design point `(x, y)` on the face sphere and turn it with the head.
pub(crate) fn on_sphere(x: f32, y: f32, yaw: f32, pitch: f32) -> SpherePoint {
    let lon = (x / FACE_R).clamp(-1.0, 1.0).asin() + yaw;
    let lat = (-y / FACE_R).clamp(-1.0, 1.0).asin() + pitch;
    let cl = lat.cos();
    SpherePoint {
        x: FACE_R * lon.sin() * cl,
        y: -FACE_R * lat.sin(),
        sx: lon.cos(),
        sy: cl,
        z: lon.cos() * cl,
    }
}

impl EyeSpot {
    fn from_sphere(x: f32, y: f32, q: SpherePoint) -> Self {
        Self {
            x: q.x,
            y: q.y,
            sx: q.sx,
            sy: q.sy,
            z: q.z,
            ox: x,
            oy: y,
            fz: (FACE_R * FACE_R - x * x - y * y).max(0.0).sqrt(),
        }
    }
}

/// Draw the face and return the two eyes' spots in face space.
///
/// `transform` maps face-local coordinates (centred on the face) to the
/// device. `clip` is the front cap's outline, so a feature that would reach
/// past the body's foreshortened silhouette is cut off at it.
pub(crate) fn draw_face(
    pixmap: &mut Pixmap,
    pose: &Pose,
    face: BotAvatarFace,
    ink: Rgba,
    transform: Affine,
    clip: Option<&Mask>,
) -> [EyeSpot; 2] {
    let [wd, ww, ws] = pose.w;
    let ey = eye_y(face);
    let half = EYE_GAP / 2.0;
    let look_x = pose.look_x;
    let look_y = pose.look_y;

    let at = |x: f32, y: f32, alpha: f32, f: &mut dyn FnMut(f32, Affine)| {
        let q = on_sphere(x, y, pose.yaw, pose.pitch);
        if q.z <= 0.02 || alpha <= 0.01 {
            return;
        }
        let a = alpha * (q.z * 5.0).min(1.0);
        let m = Affine::translate(q.x, q.y)
            .then(Affine::scale(q.sx.max(0.02), q.sy.max(0.02)))
            .then(transform);
        f(a, m);
    };

    // The head's aim shapes the eyes: taller looking up, shorter looking
    // down, a little wider from the corner of a sideways glance.
    let past = |v: f32, d: f32| {
        if v.abs() <= d {
            0.0
        } else {
            (v.signum() * (v.abs() - d)) / (1.0 - d)
        }
    };
    let clamp1 = |v: f32| v.clamp(-1.0, 1.0);
    let up = past(clamp1(-pose.pitch / 0.26 - pose.look_y / 7.0), 0.34);
    let side = past(clamp1(pose.look_x / 4.5), 0.4).abs();
    let tall = (1.0 + 0.55 * up - 0.1 * side).max(0.3);
    let wide = 1.0 - 0.05 * up + 0.12 * side;
    let open = wd + ww * (1.0 - pose.laugh);
    let laugh = ww * pose.laugh;
    let lift = (-pose.y).max(0.0) / 26.0;
    let sag = 0.5 + 0.5 * pose.breath;

    for side in [-1.0_f32, 1.0] {
        let lid = if side < 0.0 {
            pose.blink_l
        } else {
            pose.blink_r
        };
        let e = (pose.eye_open * (1.0 - lid)).clamp(0.0, 1.0);
        let k_open = open * e;
        let k_shut = open * (1.0 - e);
        let k_laugh = laugh;
        let k_sleep = ws;
        let x0 = k_open * 0.01 + k_shut * 5.4 + k_laugh * 6.2 + k_sleep * 6.0;
        let y0 = k_open * 1.1 * tall
            + k_shut * 0.6
            + k_laugh * (2.2 - lift * 1.5)
            + k_sleep * (-1.4 + sag);
        let cy = k_open * -3.3 * tall
            + k_shut * 0.6
            + k_laugh * (-11.4 - 4.0 * lift)
            + k_sleep * (5.4 + 2.0 * sag);
        let w = k_open * EYE_RX * 2.0 * wide + k_shut * 2.8 + k_laugh * 4.4 + k_sleep * 4.0;
        let dx = look_x * (k_open + 0.5 * (k_shut + k_laugh));
        let dy = look_y * (k_open + 0.5 * k_shut);
        at(side * half + dx, ey + dy, 1.0, &mut |a, m| {
            if let Some(path) = eye_path(x0, y0, cy) {
                let p = paint_alpha(ink, a);
                stroke(pixmap, &path, &p, w, m, clip);
            }
        });
    }

    if face == BotAvatarFace::Mouth {
        let mx = look_x * 0.35;
        let kd = (0.6 + 0.4 * wd) * (1.0 + 0.06 * pose.breath);
        let kw = (0.6 + 0.4 * ww) * (1.0 + (0.25 * (-pose.y).max(0.0)) / 26.0);
        let r = 2.7 * ws * (1.0 + 0.25 * pose.breath);
        let b = |d: f32, w: f32, s: f32| wd * d + ww * w + ws * s;
        let m = Mouth {
            hw: b(6.5 * kd, 9.5 * kw, r),
            t0: b(1.9, 0.0, 0.0),
            a: b(2.0 / 3.0, 2.0 / 3.0, 0.0),
            yt: b(3.53 * kd, 1.6 * kw, (-4.0 * r) / 3.0),
            ab: b(2.0 / 3.0, 0.0, 0.0),
            yb: b(3.53 * kd, 17.3 * kw, (4.0 * r) / 3.0),
        };
        let place = b(12.5, 11.6, 15.5);
        at(mx, place, 1.0, &mut |a, t| {
            if let Some(path) = mouth_path(m) {
                let p = paint_alpha(ink, a);
                fill(pixmap, &path, &p, t, clip);
            }
        });
    }

    let spot = |x: f32| {
        let q = on_sphere(x, ey, pose.yaw, pose.pitch);
        EyeSpot::from_sphere(x, ey, q)
    };
    [spot(-half), spot(half)]
}

/// The eyes' coordinates after the state blend, exposed for tests.
#[cfg(test)]
fn eye_numbers(pose: &Pose, face: BotAvatarFace) -> Vec<(f32, f32, f32, f32, f32)> {
    let [wd, ww, ws] = pose.w;
    let ey = eye_y(face);
    let open = wd + ww * (1.0 - pose.laugh);
    let laugh = ww * pose.laugh;
    let mut out = Vec::new();
    for side in [-1.0_f32, 1.0] {
        let lid = if side < 0.0 {
            pose.blink_l
        } else {
            pose.blink_r
        };
        let e = (pose.eye_open * (1.0 - lid)).clamp(0.0, 1.0);
        let k_open = open * e;
        let k_shut = open * (1.0 - e);
        let k_laugh = laugh;
        let k_sleep = ws;
        out.push((
            k_open * 0.01 + k_shut * 5.4 + k_laugh * 6.2 + k_sleep * 6.0,
            k_open * 1.1 + k_shut * 0.6 + k_laugh * 2.2 + k_sleep * -1.4,
            k_open * -3.3 + k_shut * 0.6 + k_laugh * -11.4 + k_sleep * 5.4,
            k_open * EYE_RX * 2.0 + k_shut * 2.8 + k_laugh * 4.4 + k_sleep * 4.0,
            ey,
        ));
    }
    out
}

/// The open eye's hairpin curve, sampled into a polyline with round joins.
///
/// The original strokes a quadratic directly; Chrome and Firefox round its
/// cusp but Safari's stroker does not, so both the web package and this port
/// stroke the sampled polyline instead.
fn eye_path(x0: f32, y0: f32, control_y: f32) -> Option<tiny_skia::Path> {
    const STEPS: usize = 8;
    let x = round50(x0);
    let y = round50(y0);
    let c = round50(control_y);
    let mut b = PathBuilder::new();
    b.move_to(-x, y);
    for i in 1..=STEPS {
        let t = i as f32 / STEPS as f32;
        let mt = 1.0 - t;
        let px = mt * mt * -x + t * t * x;
        let py = (mt * mt + t * t) * y + 2.0 * mt * t * c;
        b.line_to(px, py);
    }
    b.finish()
}

fn round50(v: f32) -> f32 {
    (v * 50.0).round() / 50.0
}

#[derive(Clone, Copy, Debug)]
struct Mouth {
    hw: f32,
    t0: f32,
    a: f32,
    yt: f32,
    ab: f32,
    yb: f32,
}

fn mouth_path(m: Mouth) -> Option<tiny_skia::Path> {
    const SIN: f32 = 0.632_029_6; // sin(0.684)
    const COS: f32 = 0.774_944_4; // cos(0.684)
    let hw = round50(m.hw);
    let t0 = round50(m.t0);
    let a = round50(m.a);
    let yt = round50(m.yt);
    let ab = round50(m.ab);
    let yb = round50(m.yb);
    let nx = t0 * SIN;
    let ny = t0 * COS;
    let ltx = -hw + nx;
    let lty = -ny;
    let rtx = hw - nx;
    let rty = -ny;
    let lbx = -hw - nx;
    let lby = ny;
    let rbx = hw + nx;
    let rby = ny;
    let cx = (4.0 / 3.0) * t0 * COS;
    let cy = (4.0 / 3.0) * t0 * SIN;
    let mut b = PathBuilder::new();
    b.move_to(ltx, lty);
    b.cubic_to(-hw + a * hw, yt - t0, hw - a * hw, yt - t0, rtx, rty);
    b.cubic_to(rtx + cx, rty - cy, rbx + cx, rby - cy, rbx, rby);
    b.cubic_to(hw - ab * hw, yb + t0, -hw + ab * hw, yb + t0, lbx, lby);
    b.cubic_to(lbx - cx, lby - cy, ltx - cx, lty - cy, ltx, lty);
    b.close();
    b.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::bot_avatar::engine::{BotAvatarSim, rest_pose};
    use crate::widgets::bot_avatar::types::BotAvatarState;

    #[test]
    fn sphere_positions_foreshorten_at_the_edges() {
        let q = on_sphere(0.0, 0.0, 0.0, 0.0);
        assert!((q.z - 1.0).abs() < 1e-5);
        let side = on_sphere(20.0, 0.0, 0.0, 0.0);
        assert!(side.sx < 0.8);
        let turned = on_sphere(20.0, 0.0, 1.4, 0.0);
        assert!(turned.z < 0.1);
    }

    #[test]
    fn eyes_morph_closed_in_sleep() {
        let mut idle = BotAvatarSim::new(0.4, BotAvatarState::Idle);
        let mut sleep = BotAvatarSim::new(0.4, BotAvatarState::Sleeping);
        for _ in 0..60 {
            idle.update(1.0 / 60.0);
            sleep.update(1.0 / 60.0);
        }
        let open = eye_numbers(&idle.pose(), BotAvatarFace::Eyes);
        let shut = eye_numbers(&sleep.pose(), BotAvatarFace::Eyes);
        assert!(open[0].3 > shut[0].3, "{open:?} {shut:?}");
        assert!((open[0].2 - shut[0].2).abs() > 1.0, "{open:?} {shut:?}");
        let rest = rest_pose(BotAvatarState::Sleeping);
        assert_eq!(rest.w[2], 1.0);
    }

    #[test]
    fn eye_and_mouth_paths_are_finite() {
        for (x, y, c) in [(3.0, 1.1, -3.3), (5.4, 0.6, 0.6), (6.0, -1.4, 5.4)] {
            let p = eye_path(x, y, c).unwrap();
            assert!(p.bounds().width() > 0.0);
        }
        let m = mouth_path(Mouth {
            hw: 6.5,
            t0: 1.9,
            a: 2.0 / 3.0,
            yt: 3.53,
            ab: 2.0 / 3.0,
            yb: 3.53,
        })
        .unwrap();
        assert!(m.bounds().height() > 0.0);
    }
}
