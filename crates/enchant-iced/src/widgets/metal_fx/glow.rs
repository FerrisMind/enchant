//! The glow state machine, from `engine/glow/glow.ts` (`updateGlow`).
//!
//! A luminance-driven halo that tracks the brightest point on the ring's
//! perimeter, plus a tight catch-light. [`GlowState::tick`] runs one update:
//!
//! 1. luminance is sampled at 16 points around the perimeter;
//! 2. a hotspot state machine holds a point for at least `min_dwell_ms`, and a
//!    brighter rival then steals it through a full fade-out at the old spot and
//!    a full fade-in at the new one (the halo never slides);
//! 3. the halo wanders a little along the outline and is tinted to the colour
//!    the material has at the hotspot.
//!
//! The result is a [`GlowFrame`]: positions, angle, opacities, envelope and
//! tints, which the renderer draws with the baked sprites.
//!
//! Masked text/glyph glow ("point mode") and the cursor-light tracking mode
//! belong to the text and cursor-light features and are not part of this
//! module yet.

use super::color::{hsv_to_rgb, rgb_to_hsv};
use super::geometry::{
    MetalFxVariant, rr_perimeter, sample_at_arc, shape_perimeter, smoothstep, tangent_angle_at_arc,
};
use super::glow_config::MetalGlowConfig;
use super::material::{MetalMaterial, SHEET_RESOLUTION, SheetMapping};
use super::preset::MetalFxResolvedTheme;
use super::tween::{Ease, Tween};

const RELOCATE_DELTA: f32 = 0.05;
/// Wander retarget period. Was 120 ticks at the 15 fps shader rate.
const WANDER_RETARGET_MS: f64 = 120.0 * (1000.0 / 15.0);
/// Per-tick rates in the config are defined at the 15 fps shader rate.
const RATE_TICK_MS: f64 = 1000.0 / 15.0;
const TINT_HOLD_MS: f64 = 2000.0;
const TINT_FADE_MS: f64 = 400.0;
const LT_SAT_BOOST: f32 = 2.625;
const LT_VAL_MULT: f32 = 1.008;
const LT_MIN_VAL: f32 = 0.31;
const REF_W: f32 = 140.0;
const REF_H: f32 = 40.0;
const REF_R: f32 = 20.0;
/// Longest a single tick may advance the fade envelope, ms (~2 frames).
const ENV_MAX_STEP_MS: f64 = 34.0;
/// Points sampled around the perimeter to find the brightest hotspot.
pub const PERIMETER_SAMPLES: usize = 16;
/// Half-window (in sheet texels) luminance and colour are averaged over.
const SAMPLE_HALF_WINDOW: i32 = 2;
/// The luminance hunt runs at the web's shader rate (`FRAME_INTERVAL_MS`);
/// the envelope and wander tick at the caller's rate.
const HUNT_INTERVAL_MS: f64 = 66.0;

/// Samples the material at box-local points for the glow.
pub trait GlowSampler {
    /// Mean luminance (`0..=1`) around `p`.
    fn luminance(&self, p: [f32; 2]) -> f32;
    /// Mean colour (`0..=1` per channel) around `p`.
    fn rgb(&self, p: [f32; 2]) -> [f32; 3];
    /// The most saturated colour around `p` (light theme tint).
    fn rgb_chromatic(&self, p: [f32; 2]) -> [f32; 3];
}

/// A [`GlowSampler`] over the CPU material: what the web reads back from the
/// shared GL canvas (`sampling.ts`), computed directly.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::{
///     GlowSampler, MaterialSampler, MetalFxPreset, MetalFxResolvedTheme, MetalMaterial, SheetMapping,
/// };
///
/// let material = MetalMaterial::new(MetalFxPreset::Silver.mode(MetalFxResolvedTheme::Dark));
/// let sampler = MaterialSampler::new(material, SheetMapping::new(140.0, 40.0, 1.6), 0.0);
/// let lum = sampler.luminance([70.0, 1.5]);
/// assert!((0.0..=1.0).contains(&lum));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct MaterialSampler {
    material: MetalMaterial,
    mapping: SheetMapping,
    u_time: f32,
}

impl MaterialSampler {
    /// Samples `material` through `mapping` at `u_time` (speed already applied).
    #[must_use]
    pub fn new(material: MetalMaterial, mapping: SheetMapping, u_time: f32) -> Self {
        Self {
            material,
            mapping,
            u_time,
        }
    }

    fn window(&self, p: [f32; 2]) -> impl Iterator<Item = [f32; 4]> + '_ {
        let centre = self.mapping.uv(p);
        // Snap to the sheet's pixel grid like `mapToGlowBuf`.
        let snap = |v: f32| ((v * SHEET_RESOLUTION).round() + 0.5) / SHEET_RESOLUTION;
        let (cx, cy) = (snap(centre[0]), snap(centre[1]));
        (-SAMPLE_HALF_WINDOW..=SAMPLE_HALF_WINDOW).flat_map(move |j| {
            (-SAMPLE_HALF_WINDOW..=SAMPLE_HALF_WINDOW).map(move |i| {
                let s = [
                    cx + i as f32 / SHEET_RESOLUTION,
                    cy + j as f32 / SHEET_RESOLUTION,
                ];
                self.material.shade(s, self.u_time, None)
            })
        })
    }
}

impl GlowSampler for MaterialSampler {
    fn luminance(&self, p: [f32; 2]) -> f32 {
        let (sum, n) = self.window(p).fold((0.0, 0.0), |(s, n), c| {
            (s + 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2], n + 1.0)
        });
        sum / n
    }

    fn rgb(&self, p: [f32; 2]) -> [f32; 3] {
        let (sum, n) = self.window(p).fold(([0.0; 3], 0.0), |(s, n), c| {
            ([s[0] + c[0], s[1] + c[1], s[2] + c[2]], n + 1.0)
        });
        [sum[0] / n, sum[1] / n, sum[2] / n]
    }

    fn rgb_chromatic(&self, p: [f32; 2]) -> [f32; 3] {
        let mut best = (-1.0, [1.0; 3]);
        for c in self.window(p) {
            let (r, g, b) = (
                c[0].clamp(0.0, 1.0),
                c[1].clamp(0.0, 1.0),
                c[2].clamp(0.0, 1.0),
            );
            let max = r.max(g).max(b);
            let min = r.min(g).min(b);
            let sat = if max > 0.0 { (max - min) / max } else { 0.0 };
            let score = sat * (0.35 + 0.65 * max);
            if score > best.0 {
                best = (score, [r, g, b]);
            }
        }
        best.1
    }
}

/// What to draw for one glow tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlowFrame {
    /// Halo centre, box-local CSS px.
    pub position: [f32; 2],
    /// Catch-light centre, box-local CSS px.
    pub extra_position: [f32; 2],
    /// Outline tangent at the hotspot, radians.
    pub tangent: f32,
    /// Halo opacity (`0..=1`), strength applied.
    pub halo_opacity: f32,
    /// Catch-light opacity (`0..=1`), strength applied.
    pub extra_opacity: f32,
    /// Appear/disappear envelope (`0..=1`).
    pub envelope: f32,
    /// Halo tint, `0..=1` per channel.
    pub halo_tint: [f32; 3],
    /// Catch-light tint, `0..=1` per channel.
    pub extra_tint: [f32; 3],
    /// Whether an envelope is mid-fade (callers should tick every frame).
    pub animating: bool,
}

/// Inputs of [`GlowState::tick`].
#[derive(Clone, Copy)]
pub struct GlowTick<'a> {
    /// Clock, milliseconds (monotonic; only differences matter).
    pub now_ms: f64,
    /// `strength × glowGain` multiplier (clamped to `0..=1`).
    pub strength: f32,
    /// Resolved theme (light mode tints from the most saturated pixel).
    pub theme: MetalFxResolvedTheme,
    /// Material sampler.
    pub sampler: &'a dyn GlowSampler,
    /// Tuning.
    pub config: &'a MetalGlowConfig,
}

impl std::fmt::Debug for GlowTick<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlowTick")
            .field("now_ms", &self.now_ms)
            .field("strength", &self.strength)
            .field("theme", &self.theme)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Geometry {
    width: f32,
    height: f32,
    radius: f32,
    kind: MetalFxVariant,
    scale: f32,
    inset: f32,
}

#[derive(Clone, Copy, Debug)]
struct PerimPoint {
    arc: f32,
}

/// xorshift64*: deterministic wander without a dependency.
#[derive(Clone, Copy, Debug)]
struct Rng(u64);

impl Rng {
    fn next_unit(&mut self) -> f32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let v = self.0.wrapping_mul(0x2545_F491_4F6C_DD1D);
        ((v >> 40) as f32) / (1u64 << 24) as f32
    }
}

/// The hotspot state machine of one glow.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::{
///     GlowState, GlowTick, MaterialSampler, MetalFxPreset, MetalFxResolvedTheme, MetalFxVariant,
///     MetalGlowConfig, MetalMaterial, SheetMapping,
/// };
///
/// let theme = MetalFxResolvedTheme::Dark;
/// let material = MetalMaterial::new(MetalFxPreset::Chromatic.mode(theme));
/// let sampler = MaterialSampler::new(material, SheetMapping::new(140.0, 40.0, 1.6), 0.0);
/// let config = MetalGlowConfig::DEFAULTS;
///
/// let mut glow = GlowState::new(1);
/// glow.configure(140.0, 40.0, 20.0, MetalFxVariant::Button, 1.0, &config);
/// let frame = glow
///     .tick(&GlowTick { now_ms: 16.0, strength: 1.0, theme, sampler: &sampler, config: &config })
///     .expect("a non-empty perimeter always yields a frame");
/// assert_eq!(frame.envelope, 0.0, "the first appearance fades in from nothing");
/// ```
#[derive(Clone, Debug)]
pub struct GlowState {
    geometry: Option<Geometry>,
    perim: Vec<PerimPoint>,
    current_idx: usize,
    appeared_at: f64,
    glow_opacity: f32,
    reloc_tween: Option<Tween>,
    reloc_next_idx: usize,
    reloc_mul: f32,
    env_clock: f64,
    last_tick_ms: f64,
    wander_s: f32,
    wander_target_s: f32,
    wander_ms: f64,
    tint_from: [f32; 3],
    tint_target: [f32; 3],
    tint_tween: Option<Tween>,
    tint_hold_until: f64,
    lum_cache: Vec<f32>,
    last_hunt_ms: Option<f64>,
    rng: Rng,
}

impl GlowState {
    /// A fresh, invisible glow. `seed` drives the wander deterministically.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            geometry: None,
            perim: Vec::new(),
            current_idx: 0,
            appeared_at: 0.0,
            glow_opacity: 0.0,
            reloc_tween: None,
            reloc_next_idx: 0,
            reloc_mul: 0.0,
            env_clock: 0.0,
            last_tick_ms: 0.0,
            wander_s: 0.0,
            wander_target_s: 0.0,
            wander_ms: 0.0,
            tint_from: [255.0; 3],
            tint_target: [255.0; 3],
            tint_tween: None,
            tint_hold_until: 0.0,
            lum_cache: Vec::new(),
            last_hunt_ms: None,
            rng: Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1),
        }
    }

    /// (Re)builds the perimeter table for a new box. State carries over a
    /// resize like `carryGlowState`: the halo keeps its hotspot, brightness
    /// and fade instead of restarting from invisible.
    pub fn configure(
        &mut self,
        width: f32,
        height: f32,
        radius: f32,
        kind: MetalFxVariant,
        scale: f32,
        config: &MetalGlowConfig,
    ) {
        let geometry = Geometry {
            width,
            height,
            radius,
            kind,
            scale,
            inset: config.inset * scale,
        };
        if self.geometry == Some(geometry) {
            return;
        }
        let perim = shape_perimeter(width, height, radius, kind);
        self.perim = (0..PERIMETER_SAMPLES)
            .map(|i| PerimPoint {
                arc: (i as f32 / PERIMETER_SAMPLES as f32) * perim,
            })
            .collect();
        self.current_idx = self.current_idx.min(PERIMETER_SAMPLES - 1);
        self.geometry = Some(geometry);
        self.last_hunt_ms = None;
    }

    /// The perimeter sample points (box-local CSS px) luminance is read at.
    #[must_use]
    pub fn perimeter_points(&self) -> Vec<[f32; 2]> {
        let Some(g) = self.geometry else {
            return Vec::new();
        };
        self.perim
            .iter()
            .map(|p| sample_at_arc(p.arc, g.width, g.height, g.radius, g.inset, 0.0, g.kind))
            .collect()
    }

    /// Index of the current hotspot in [`perimeter_points`](Self::perimeter_points).
    #[must_use]
    pub fn hotspot_index(&self) -> usize {
        self.current_idx
    }

    /// The glow's current envelope (`0..=1`).
    #[must_use]
    pub fn envelope(&self) -> f32 {
        self.reloc_mul
    }

    /// Runs one update. `None` until [`configure`](Self::configure) has run.
    pub fn tick(&mut self, input: &GlowTick<'_>) -> Option<GlowFrame> {
        let g = self.geometry?;
        if self.perim.is_empty() {
            return None;
        }
        let cfg = input.config;
        let now = input.now_ms;
        let points = self.perimeter_points();

        let due = match self.last_hunt_ms {
            None => true,
            Some(last) => now < last || now - last >= HUNT_INTERVAL_MS,
        };
        if due || self.lum_cache.len() != points.len() {
            self.lum_cache = points.iter().map(|p| input.sampler.luminance(*p)).collect();
            self.last_hunt_ms = Some(now);
        }
        let mut max_lum = -1.0_f32;
        let mut max_idx = self.current_idx;
        for (i, lum) in self.lum_cache.iter().enumerate() {
            if *lum > max_lum {
                max_lum = *lum;
                max_idx = i;
            }
        }
        let cur_lum = self.lum_cache[self.current_idx];

        let dwell_active =
            self.appeared_at > 0.0 && now - self.appeared_at < f64::from(cfg.min_dwell_ms);
        let lit = |lum: f32| {
            cfg.base_op + (cfg.peak_op - cfg.base_op) * smoothstep(cfg.lum_lo, cfg.lum_hi, lum)
        };
        let target_op = lit(cur_lum);
        let rival_dominates = !dwell_active && max_lum - cur_lum > RELOCATE_DELTA;

        let dt_ms = if self.last_tick_ms > 0.0 {
            (now - self.last_tick_ms).clamp(0.5, 200.0)
        } else {
            RATE_TICK_MS
        };
        self.last_tick_ms = now;
        self.env_clock += dt_ms.min(ENV_MAX_STEP_MS);
        let rate =
            |per_tick: f32| 1.0 - (1.0 - f64::from(per_tick)).powf(dt_ms / RATE_TICK_MS) as f32;

        let fade_ms = f64::from(cfg.reloc_fade_ms).max(1.0);
        let fade_out_ms = f64::from(cfg.reloc_fade_out_ms).max(1.0);

        if let Some(t) = self.reloc_tween
            && t.is_done()
            && t.target() == 0.0
        {
            // Faded out at the old spot: switch, then fade in at the new one.
            self.current_idx = self.reloc_next_idx.min(points.len() - 1);
            self.glow_opacity = lit(self.lum_cache[self.current_idx]);
            self.fade_in(now, fade_ms);
        }
        let idle = self.reloc_tween.is_none_or(|t| t.is_done());
        if idle {
            if self.appeared_at == 0.0 {
                // First appearance: face the brightest point.
                self.current_idx = max_idx;
                self.glow_opacity = target_op;
                self.fade_in(now, fade_ms);
            } else if rival_dominates {
                self.reloc_next_idx = max_idx;
                let mut t = Tween::new(1.0, 0.0, fade_out_ms, Ease::Smoothstep);
                t.start(self.env_clock);
                self.reloc_tween = Some(t);
            }
        }

        // Luminance tracking runs continuously; the envelope handles appear/disappear.
        self.glow_opacity += (target_op - self.glow_opacity) * rate(cfg.fade_rate);
        self.glow_opacity = self.glow_opacity.clamp(0.0, 1.0);
        self.reloc_mul = match self.reloc_tween.as_mut() {
            Some(t) => t.tick(self.env_clock) as f32,
            None => 1.0,
        };

        let ratio = shape_perimeter(g.width, g.height, g.radius, g.kind)
            / rr_perimeter(REF_W, REF_H, REF_R);
        let wander_range = cfg.wander_range * ratio;
        self.wander_ms += dt_ms;
        if self.wander_ms >= WANDER_RETARGET_MS {
            self.wander_target_s = (self.rng.next_unit() * 2.0 - 1.0) * wander_range;
            self.wander_ms = 0.0;
        }
        self.wander_s += (self.wander_target_s - self.wander_s) * rate(cfg.wander_lerp);

        let arc = self.perim[self.current_idx].arc + self.wander_s;
        let blob = sample_at_arc(arc, g.width, g.height, g.radius, g.inset, 0.0, g.kind);
        let tangent = tangent_angle_at_arc(arc, g.width, g.height, g.radius, g.inset, g.kind);
        let extra_out = cfg.extra_outward * ratio * g.scale;
        let extra = sample_at_arc(arc, g.width, g.height, g.radius, g.inset, extra_out, g.kind);

        let light = input.theme.is_light();
        let sample = || {
            let c = if light {
                input.sampler.rgb_chromatic(blob)
            } else {
                input.sampler.rgb(blob)
            };
            [c[0] * 255.0, c[1] * 255.0, c[2] * 255.0]
        };
        self.update_tint(&sample, light, now);
        let ft = self.tint_tween.as_mut().map_or(1.0, |t| t.tick(now)) as f32;

        let lerp = |k: usize| self.tint_from[k] + (self.tint_target[k] - self.tint_from[k]) * ft;
        let tint = [lerp(0), lerp(1), lerp(2)];
        let halo_tint = if light {
            [
                tint[0].round() / 255.0,
                tint[1].round() / 255.0,
                tint[2].round() / 255.0,
            ]
        } else {
            let peak = tint[0].max(tint[1]).max(tint[2]);
            let peak = if peak == 0.0 { 1.0 } else { peak };
            [
                (255.0 * (tint[0] / peak)).round() / 255.0,
                (255.0 * (tint[1] / peak)).round() / 255.0,
                (255.0 * (tint[2] / peak)).round() / 255.0,
            ]
        };
        let extra_tint = if light {
            let [h, s, v] = rgb_to_hsv(
                halo_tint[0] * 255.0,
                halo_tint[1] * 255.0,
                halo_tint[2] * 255.0,
            );
            let [r, gc, b] = hsv_to_rgb(
                (h).clamp(0.0, 1.0),
                (s * LT_SAT_BOOST).min(1.0),
                (v * LT_VAL_MULT).max(LT_MIN_VAL),
            );
            [r / 255.0, gc / 255.0, b / 255.0]
        } else {
            [1.0; 3]
        };

        let m = input.strength.clamp(0.0, 1.0);
        let halo_opacity = (self.glow_opacity * cfg.halo_op_mul * m).min(1.0);
        let extra_opacity = (self.glow_opacity * cfg.extra_intensity * m).min(1.0);
        let animating = self.reloc_tween.is_some_and(|t| !t.is_done());

        Some(GlowFrame {
            position: blob,
            extra_position: extra,
            tangent,
            halo_opacity,
            extra_opacity,
            envelope: self.reloc_mul,
            halo_tint,
            extra_tint,
            animating,
        })
    }

    fn fade_in(&mut self, now: f64, fade_ms: f64) {
        self.appeared_at = now;
        self.wander_s = 0.0;
        self.wander_target_s = 0.0;
        self.wander_ms = 0.0;
        let mut t = Tween::new(0.0, 1.0, fade_ms, Ease::Smoothstep);
        t.start(self.env_clock);
        self.reloc_tween = Some(t);
    }

    fn update_tint(&mut self, sample: &dyn Fn() -> [f32; 3], light: bool, now: f64) {
        let restart = |from: [f32; 3], target: [f32; 3], now: f64, state: &mut Self| {
            state.tint_from = from;
            state.tint_target = target;
            let mut t = Tween::new(0.0, 1.0, TINT_FADE_MS, Ease::Linear);
            t.start(now);
            state.tint_tween = Some(t);
        };
        match self.tint_tween {
            None => {
                let sampled = sample();
                restart(sampled, sampled, now, self);
                self.tint_hold_until = if light { 0.0 } else { now + TINT_HOLD_MS };
            }
            Some(t) if t.is_done() => {
                if light {
                    let v = t.value() as f32;
                    let lerp = |k: usize| {
                        self.tint_from[k] + (self.tint_target[k] - self.tint_from[k]) * v
                    };
                    let from = [lerp(0), lerp(1), lerp(2)];
                    restart(from, sample(), now, self);
                } else if now >= self.tint_hold_until {
                    restart(self.tint_target, sample(), now, self);
                    self.tint_hold_until = now + TINT_HOLD_MS;
                }
            }
            Some(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::metal_fx::preset::MetalFxPreset;

    /// A sampler whose luminance is a function of the point, for hunting tests.
    struct Fn2<F: Fn([f32; 2]) -> f32>(F);

    impl<F: Fn([f32; 2]) -> f32> GlowSampler for Fn2<F> {
        fn luminance(&self, p: [f32; 2]) -> f32 {
            (self.0)(p)
        }
        fn rgb(&self, p: [f32; 2]) -> [f32; 3] {
            let l = (self.0)(p);
            [l, l * 0.5, 1.0]
        }
        fn rgb_chromatic(&self, p: [f32; 2]) -> [f32; 3] {
            self.rgb(p)
        }
    }

    fn state(kind: MetalFxVariant, w: f32, h: f32, r: f32, cfg: &MetalGlowConfig) -> GlowState {
        let mut s = GlowState::new(7);
        s.configure(w, h, r, kind, 1.0, cfg);
        s
    }

    fn tick<'a>(
        s: &mut GlowState,
        now: f64,
        sampler: &'a dyn GlowSampler,
        cfg: &'a MetalGlowConfig,
        theme: MetalFxResolvedTheme,
    ) -> GlowFrame {
        s.tick(&GlowTick {
            now_ms: now,
            strength: 1.0,
            theme,
            sampler,
            config: cfg,
        })
        .unwrap()
    }

    #[test]
    fn unconfigured_state_yields_nothing() {
        let cfg = MetalGlowConfig::DEFAULTS;
        let sampler = Fn2(|_| 0.5);
        let mut s = GlowState::new(1);
        assert!(
            s.tick(&GlowTick {
                now_ms: 1.0,
                strength: 1.0,
                theme: MetalFxResolvedTheme::Dark,
                sampler: &sampler,
                config: &cfg
            })
            .is_none()
        );
    }

    #[test]
    fn perimeter_table_has_sixteen_inset_points() {
        let cfg = MetalGlowConfig::DEFAULTS;
        let s = state(MetalFxVariant::Button, 140.0, 40.0, 20.0, &cfg);
        let pts = s.perimeter_points();
        assert_eq!(pts.len(), PERIMETER_SAMPLES);
        assert_eq!(pts[0], [20.0, 1.5]);
    }

    #[test]
    fn first_appearance_picks_the_brightest_point_and_fades_in() {
        let cfg = MetalGlowConfig::DEFAULTS;
        // Bright on the right-hand end of a pill.
        let sampler = Fn2(|p| if p[0] > 120.0 { 0.9 } else { 0.1 });
        let mut s = state(MetalFxVariant::Button, 140.0, 40.0, 20.0, &cfg);
        let f0 = tick(&mut s, 16.0, &sampler, &cfg, MetalFxResolvedTheme::Dark);
        assert_eq!(f0.envelope, 0.0);
        assert!(s.perimeter_points()[s.hotspot_index()][0] > 120.0);
        let mut last = 0.0;
        let mut now = 16.0;
        for _ in 0..40 {
            now += 16.0;
            last = tick(&mut s, now, &sampler, &cfg, MetalFxResolvedTheme::Dark).envelope;
        }
        assert!(
            (last - 1.0).abs() < 1e-6,
            "fade-in completes in 300 ms, got {last}"
        );
    }

    #[test]
    fn hotspot_holds_for_the_dwell_then_relocates_through_a_full_fade() {
        let cfg = MetalGlowConfig::DEFAULTS;
        let right = |p: [f32; 2]| if p[0] > 100.0 { 0.9 } else { 0.1 };
        let left = |p: [f32; 2]| if p[0] < 40.0 { 0.9 } else { 0.1 };
        let mut s = state(MetalFxVariant::Button, 140.0, 40.0, 20.0, &cfg);
        let first = Fn2(right);
        let mut now = 16.0;
        for _ in 0..60 {
            tick(&mut s, now, &first, &cfg, MetalFxResolvedTheme::Dark);
            now += 16.0;
        }
        let before = s.hotspot_index();
        // The rival appears; within the 1.5 s dwell nothing moves.
        let second = Fn2(left);
        let t0 = now;
        while now - t0 < 800.0 {
            tick(&mut s, now, &second, &cfg, MetalFxResolvedTheme::Dark);
            now += 16.0;
            assert_eq!(s.hotspot_index(), before, "dwell must hold the hotspot");
        }
        // After the dwell: it fades out (never below 0 jumps), then moves, then fades in.
        let mut min_env = 1.0_f32;
        let mut moved_at_env = None;
        while now - t0 < 4000.0 {
            let f = tick(&mut s, now, &second, &cfg, MetalFxResolvedTheme::Dark);
            min_env = min_env.min(f.envelope);
            if moved_at_env.is_none() && s.hotspot_index() != before {
                moved_at_env = Some(f.envelope);
            }
            now += 16.0;
        }
        assert!(
            min_env < 0.05,
            "the halo fully disappears before moving, min {min_env}"
        );
        assert!(
            s.perimeter_points()[s.hotspot_index()][0] < 40.0,
            "ends at the bright rival"
        );
        assert!(moved_at_env.unwrap() < 0.2, "it moves while invisible");
        assert!((s.envelope() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn opacity_tracks_luminance_between_base_and_peak() {
        let cfg = MetalGlowConfig::DEFAULTS;
        let theme = MetalFxResolvedTheme::Dark;
        let dark = Fn2(|_| 0.0);
        let bright = Fn2(|_| 1.0);
        let mut a = state(MetalFxVariant::Button, 140.0, 40.0, 20.0, &cfg);
        let mut b = state(MetalFxVariant::Button, 140.0, 40.0, 20.0, &cfg);
        let (mut fa, mut fb) = (None, None);
        let mut now = 16.0;
        for _ in 0..600 {
            fa = Some(tick(&mut a, now, &dark, &cfg, theme));
            fb = Some(tick(&mut b, now, &bright, &cfg, theme));
            now += 16.0;
        }
        let (fa, fb) = (fa.unwrap(), fb.unwrap());
        assert!((fa.halo_opacity - (cfg.base_op * cfg.halo_op_mul).min(1.0)).abs() < 0.01);
        assert!((fb.halo_opacity - (cfg.peak_op * cfg.halo_op_mul).min(1.0)).abs() < 0.01);
        assert!(fa.extra_opacity <= 1.0 && fb.extra_opacity <= 1.0);
    }

    #[test]
    fn strength_scales_the_opacities() {
        let cfg = MetalGlowConfig::DEFAULTS;
        let sampler = Fn2(|_| 0.2);
        let mut s = state(MetalFxVariant::Button, 140.0, 40.0, 20.0, &cfg);
        let half = s
            .tick(&GlowTick {
                now_ms: 16.0,
                strength: 0.25,
                theme: MetalFxResolvedTheme::Dark,
                sampler: &sampler,
                config: &cfg,
            })
            .unwrap();
        let mut t = state(MetalFxVariant::Button, 140.0, 40.0, 20.0, &cfg);
        let full = tick(&mut t, 16.0, &sampler, &cfg, MetalFxResolvedTheme::Dark);
        assert!(
            (half.halo_opacity * 4.0 - full.halo_opacity).abs() < 1e-4 || full.halo_opacity >= 0.99
        );
        assert!(half.halo_opacity < full.halo_opacity);
    }

    #[test]
    fn dark_tint_is_normalised_to_its_brightest_channel() {
        let cfg = MetalGlowConfig::DEFAULTS;
        let sampler = Fn2(|_| 0.4);
        let mut s = state(MetalFxVariant::Button, 140.0, 40.0, 20.0, &cfg);
        let f = tick(&mut s, 16.0, &sampler, &cfg, MetalFxResolvedTheme::Dark);
        let peak = f.halo_tint.iter().cloned().fold(0.0, f32::max);
        assert!((peak - 1.0).abs() < 1e-6);
        assert_eq!(f.extra_tint, [1.0; 3]);
    }

    #[test]
    fn light_theme_boosts_the_catch_light_colour() {
        let cfg = MetalGlowConfig::DEFAULTS;
        let sampler = Fn2(|_| 0.4);
        let mut s = state(MetalFxVariant::Button, 140.0, 40.0, 20.0, &cfg);
        let f = tick(&mut s, 16.0, &sampler, &cfg, MetalFxResolvedTheme::Light);
        let sat = |c: [f32; 3]| {
            let max = c[0].max(c[1]).max(c[2]);
            (max - c[0].min(c[1]).min(c[2])) / max
        };
        assert!(sat(f.extra_tint) >= sat(f.halo_tint) - 1e-3);
        assert_ne!(f.extra_tint, [1.0; 3]);
    }

    #[test]
    fn wander_is_bounded_and_deterministic() {
        let cfg = MetalGlowConfig::DEFAULTS;
        let sampler = Fn2(|_| 0.3);
        let run = |seed: u64| {
            let mut s = GlowState::new(seed);
            s.configure(140.0, 40.0, 20.0, MetalFxVariant::Button, 1.0, &cfg);
            let mut positions = Vec::new();
            let mut now = 16.0;
            for _ in 0..800 {
                let f = tick(&mut s, now, &sampler, &cfg, MetalFxResolvedTheme::Dark);
                positions.push(f.position);
                now += 16.0;
            }
            positions
        };
        let a = run(3);
        assert_eq!(a, run(3));
        let start = a[0];
        let far = a
            .iter()
            .map(|p| (p[0] - start[0]).hypot(p[1] - start[1]))
            .fold(0.0, f32::max);
        assert!(far > 0.5, "the halo wanders");
        assert!(
            far < 2.0 * cfg.wander_range * 1.2,
            "but stays within the range, got {far}"
        );
    }

    #[test]
    fn resize_carries_state() {
        let cfg = MetalGlowConfig::DEFAULTS;
        let sampler = Fn2(|p| if p[0] > 100.0 { 0.9 } else { 0.1 });
        let mut s = state(MetalFxVariant::Button, 140.0, 40.0, 20.0, &cfg);
        let mut now = 16.0;
        for _ in 0..40 {
            tick(&mut s, now, &sampler, &cfg, MetalFxResolvedTheme::Dark);
            now += 16.0;
        }
        let idx = s.hotspot_index();
        let env = s.envelope();
        s.configure(180.0, 40.0, 20.0, MetalFxVariant::Button, 1.0, &cfg);
        assert_eq!(s.hotspot_index(), idx);
        assert_eq!(s.envelope(), env);
    }

    #[test]
    fn circle_glow_rides_the_circumference() {
        let cfg = MetalGlowConfig::DEFAULTS;
        let sampler = Fn2(|_| 0.3);
        let mut s = state(MetalFxVariant::Circle, 36.0, 36.0, 18.0, &cfg);
        let f = tick(&mut s, 16.0, &sampler, &cfg, MetalFxResolvedTheme::Dark);
        let d = (f.position[0] - 18.0).hypot(f.position[1] - 18.0);
        assert!((d - (18.0 - cfg.inset)).abs() < 0.01, "radius {d}");
        assert!(f.extra_position[0].is_finite());
    }

    #[test]
    fn material_sampler_reads_the_material() {
        let theme = MetalFxResolvedTheme::Dark;
        let material = MetalMaterial::new(MetalFxPreset::Chromatic.mode(theme));
        let sampler = MaterialSampler::new(material, SheetMapping::new(140.0, 40.0, 1.6), 1.0);
        let lum = sampler.luminance([70.0, 20.0]);
        assert!((0.0..=1.0).contains(&lum));
        let rgb = sampler.rgb([70.0, 20.0]);
        let from_rgb = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2];
        assert!(
            (from_rgb - lum).abs() < 1e-4,
            "linear in colour: {from_rgb} vs {lum}"
        );
        let chroma = sampler.rgb_chromatic([70.0, 20.0]);
        assert!(chroma.iter().all(|c| (0.0..=1.0).contains(c)));
    }
}
