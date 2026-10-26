//! Cursor / tilt liquid dent — `useMetalBend` / `MetalBend.swift`.
//!
//! Two gaussian blobs at a contact point: a directional spring displacement
//! and a divergent "liquid" push that thins the stroke under the contact.

use super::bend_config::MetalBendConfig;

/// Evaluated bend field for one frame (`MetalBendField`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MetalBendField {
    pub cx: f32,
    pub cy: f32,
    pub dir_x: f32,
    pub dir_y: f32,
    pub rad_k: f32,
    pub s2_blob: f32,
    pub s2_liquid: f32,
    /// Max travel any point can take (overscan).
    pub reach: f32,
}

impl MetalBendField {
    /// Displaces a box-local point.
    #[must_use]
    pub fn deform(&self, p: [f32; 2]) -> [f32; 2] {
        let dx = p[0] - self.cx;
        let dy = p[1] - self.cy;
        let q = dx * dx + dy * dy;
        let gb = (-q / self.s2_blob).exp();
        let gl = (-q / self.s2_liquid).exp();
        [
            p[0] + self.dir_x * gb + dx * self.rad_k * gl,
            p[1] + self.dir_y * gb + dy * self.rad_k * gl,
        ]
    }

    /// Builds a field from a contact, directional displacement and liquid amp.
    #[must_use]
    pub fn new(
        contact: [f32; 2],
        dir: [f32; 2],
        liquid_amp: f32,
        cfg: &MetalBendConfig,
    ) -> Self {
        let blob = cfg.blob.max(0.5);
        let liquid_blob = cfg.liquid_blob.max(0.5);
        Self {
            cx: contact[0],
            cy: contact[1],
            dir_x: dir[0],
            dir_y: dir[1],
            rad_k: liquid_amp / (liquid_blob * liquid_blob),
            s2_blob: 2.0 * blob * blob,
            s2_liquid: 2.0 * liquid_blob * liquid_blob,
            reach: dir[0].hypot(dir[1]).max(liquid_amp.abs()) + blob.max(liquid_blob),
        }
    }
}

/// Spring-integrated bend driver (pointer or tilt).
#[derive(Clone, Debug)]
pub struct MetalBendState {
    contact: [f32; 2],
    /// Displacement spring state (x, y, vx, vy).
    disp: [f32; 4],
    /// Liquid amplitude spring (value, velocity).
    liquid: [f32; 2],
    /// Smoothed targets.
    target_disp: [f32; 2],
    target_liquid: f32,
    /// Envelope 0..=1 after fade in/out.
    envelope: f32,
    active: bool,
    last_ms: f64,
}

impl Default for MetalBendState {
    fn default() -> Self {
        Self::new()
    }
}

impl MetalBendState {
    /// Rest state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            contact: [0.0; 2],
            disp: [0.0; 4],
            liquid: [0.0; 2],
            target_disp: [0.0; 2],
            target_liquid: 0.0,
            envelope: 0.0,
            active: false,
            last_ms: 0.0,
        }
    }

    /// Advances the springs toward `target_disp` / `target_liquid` with the
    /// contact tracking `follow`. Returns the field when the envelope is live.
    pub fn tick(
        &mut self,
        now_ms: f64,
        contact: [f32; 2],
        target_disp: [f32; 2],
        target_liquid: f32,
        active: bool,
        cfg: &MetalBendConfig,
    ) -> Option<MetalBendField> {
        if !cfg.enabled {
            *self = Self::new();
            return None;
        }
        let dt_ms = if self.last_ms <= 0.0 {
            16.0
        } else {
            (now_ms - self.last_ms).clamp(1.0, 50.0)
        };
        self.last_ms = now_ms;
        let dt = (dt_ms / 1000.0) as f32;

        // Contact follow (time-based lerp calibrated at 60 Hz).
        let follow = 1.0 - (1.0 - cfg.follow.clamp(0.0, 1.0)).powf(dt * 60.0);
        self.contact[0] += (contact[0] - self.contact[0]) * follow;
        self.contact[1] += (contact[1] - self.contact[1]) * follow;

        // Smooth targets.
        let smooth_a = if cfg.smooth_ms > 1.0 {
            1.0 - (-dt_ms as f32 / cfg.smooth_ms).exp()
        } else {
            1.0
        };
        self.target_disp[0] += (target_disp[0] - self.target_disp[0]) * smooth_a;
        self.target_disp[1] += (target_disp[1] - self.target_disp[1]) * smooth_a;
        self.target_liquid += (target_liquid - self.target_liquid) * smooth_a;

        spring2(
            &mut self.disp,
            self.target_disp,
            cfg.stiffness,
            cfg.damping,
            cfg.mass.max(0.01),
            dt,
        );
        spring1(
            &mut self.liquid,
            self.target_liquid,
            cfg.liquid_stiffness,
            cfg.liquid_damping,
            1.0,
            dt,
        );

        // Envelope fade.
        let fade = if active {
            cfg.fade_in_ms.max(1.0)
        } else {
            cfg.fade_out_ms.max(1.0)
        };
        let step = dt_ms as f32 / fade;
        if active {
            self.envelope = (self.envelope + step).min(1.0);
            self.active = true;
        } else {
            self.envelope = (self.envelope - step).max(0.0);
            if self.envelope <= 0.0 {
                self.active = false;
            }
        }

        if self.envelope <= 1e-4 {
            return None;
        }
        let k = self.envelope * cfg.strength;
        let dir = [self.disp[0] * k, self.disp[1] * k];
        let liquid = self.liquid[0] * k;
        // Cap displacement magnitude.
        let mag = dir[0].hypot(dir[1]);
        let dir = if mag > cfg.max_disp && mag > 1e-6 {
            let s = cfg.max_disp / mag;
            [dir[0] * s, dir[1] * s]
        } else {
            dir
        };
        Some(MetalBendField::new(self.contact, dir, liquid, cfg))
    }
}

fn spring1(state: &mut [f32; 2], target: f32, k: f32, d: f32, mass: f32, dt: f32) {
    let x = state[0];
    let v = state[1];
    let a = (-k * (x - target) - d * v) / mass;
    state[1] = v + a * dt;
    state[0] = x + state[1] * dt;
}

fn spring2(state: &mut [f32; 4], target: [f32; 2], k: f32, d: f32, mass: f32, dt: f32) {
    for i in 0..2 {
        let x = state[i];
        let v = state[i + 2];
        let a = (-k * (x - target[i]) - d * v) / mass;
        state[i + 2] = v + a * dt;
        state[i] = x + state[i + 2] * dt;
    }
}

/// Pointer press/pull target from signed distance to the ring edge.
///
/// `signed_dist` < 0 inside the edge, > 0 outside. Returns
/// `(dir_toward_center_scaled, liquid_amp)`.
#[must_use]
pub fn pointer_targets(
    signed_dist: f32,
    toward_center: [f32; 2],
    toward_cursor: [f32; 2],
    speed_px_s: f32,
    cfg: &MetalBendConfig,
) -> ([f32; 2], f32) {
    let press = if signed_dist < 0.0 {
        (-signed_dist) * cfg.press_gain + cfg.press
    } else {
        0.0
    };
    let pull = if signed_dist > 0.0 && signed_dist < cfg.reach {
        signed_dist * cfg.pull_gain
    } else {
        0.0
    };
    let kick = (speed_px_s / 100.0) * cfg.gain;
    let mut dir = [
        toward_center[0] * press + toward_cursor[0] * (pull + kick),
        toward_center[1] * press + toward_cursor[1] * (pull + kick),
    ];
    let mag = dir[0].hypot(dir[1]);
    if mag > cfg.max_disp && mag > 1e-6 {
        let s = cfg.max_disp / mag;
        dir = [dir[0] * s, dir[1] * s];
    }
    let liquid_pressure = if signed_dist.abs() < cfg.liquid_reach {
        1.0 - signed_dist.abs() / cfg.liquid_reach.max(1e-3)
    } else {
        0.0
    };
    (dir, cfg.liquid * liquid_pressure.max(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_moves_the_contact_point() {
        let cfg = MetalBendConfig::DEFAULTS;
        let f = MetalBendField::new([50.0, 20.0], [4.0, 0.0], 3.0, &cfg);
        let p = f.deform([50.0, 20.0]);
        assert!(p[0] > 50.0, "directional dent should push contact: {p:?}");
    }

    #[test]
    fn spring_settles_toward_target() {
        let cfg = MetalBendConfig::DEFAULTS;
        let mut s = MetalBendState::new();
        let mut field = None;
        for i in 1..=120 {
            field = s.tick(
                i as f64 * 16.0,
                [40.0, 0.0],
                [5.0, 0.0],
                4.0,
                true,
                &cfg,
            );
        }
        let f = field.expect("envelope up");
        assert!(f.dir_x > 2.0, "spring should approach target, got {}", f.dir_x);
    }

    #[test]
    fn release_fades_out() {
        let cfg = MetalBendConfig::DEFAULTS;
        let mut s = MetalBendState::new();
        for i in 1..=60 {
            let _ = s.tick(i as f64 * 16.0, [40.0, 0.0], [5.0, 0.0], 4.0, true, &cfg);
        }
        let mut last = None;
        for i in 61..=200 {
            last = s.tick(i as f64 * 16.0, [40.0, 0.0], [0.0, 0.0], 0.0, false, &cfg);
        }
        assert!(last.is_none(), "fade-out should clear the field");
    }
}
