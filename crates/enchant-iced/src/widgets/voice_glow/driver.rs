//! The voice driver: a pure, deterministic, time-stepped state machine.
//!
//! This is `voiceDriver.ts` with the DOM taken out. Each step it reads the
//! level source, shapes the raw level (gain, noise gate, soft saturation),
//! follows it with an attack / release envelope, advances the flow (the lobes
//! sliding sideways at a speed set by the level), gathers the lobes into a
//! beam where the motion asks for it (the processing sweep), folds in the idle
//! breathing and the slow hue drift, and returns the result as a
//! [`VoiceGlowFrame`].

use std::time::Duration;

use super::audio::{LevelSource, SpectrumAnalyser};
use super::config::VoiceGlowConfig;
use super::frame::{
    BandInput, DistortionFrame, HostSize, LayerOpacity, LobeFrame, VoiceGlowFrame, band_paint,
    band_points,
};
use super::geometry::{
    LOBE_SPAN, LOBES, corner_lift, edge_envelope, painted_radius, ping_pong, shape, wrap_x,
};
use super::options::{Callback, ResolvedTheme, VoiceGlow};
use super::palette::Rgb;
use super::processing::{MotionSource, VoiceGlowMotion, VoiceProcessing, follow};

/// The longest step the driver integrates (`Math.min(0.05, ...)`).
pub const MAX_STEP: f64 = 0.05;
/// The step upstream's first frame takes, and the step [`VoiceGlow::frame_at`]
/// replays at (a 60 Hz display).
pub const FRAME_STEP: f64 = 1.0 / 60.0;

/// The distortion settles out in ~a quarter second once the beam gathers...
const WARP_OUT_TAU: f64 = 0.06;
/// ...and eases back over roughly a second once it releases.
const WARP_IN_TAU: f64 = 0.35;
/// Below this share the warp layers are taken out of the paint altogether.
const WARP_OFF_BELOW: f64 = 0.03;
/// The fade-in animation, seconds (`vb-fade-in`, `0.6s ease`).
const FADE_IN: f64 = 0.6;
/// The fade-out animation, seconds (`vb-fade-out`, `0.5s ease`).
const FADE_OUT: f64 = 0.5;

/// CSS `ease`: `cubic-bezier(0.25, 0.1, 0.25, 1)`.
fn css_ease(progress: f64) -> f64 {
    let p = progress.clamp(0.0, 1.0);
    if p <= 0.0 || p >= 1.0 {
        return p;
    }
    let (x1, y1, x2, y2) = (0.25, 0.1, 0.25, 1.0);
    let bez = |a: f64, b: f64, t: f64| {
        3.0 * a * (1.0 - t).powi(2) * t + 3.0 * b * (1.0 - t) * t * t + t * t * t
    };
    let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
    for _ in 0..60 {
        let mid = (lo + hi) / 2.0;
        if bez(x1, x2, mid) < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    bez(y1, y2, (lo + hi) / 2.0)
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Fade {
    Off,
    In(f64),
    On,
    Out(f64),
}

impl Fade {
    fn opacity(self) -> f64 {
        match self {
            Self::Off => 0.0,
            Self::In(e) => css_ease(e / FADE_IN),
            Self::On => 1.0,
            Self::Out(e) => 1.0 - css_ease(e / FADE_OUT),
        }
    }
}

/// The envelope state, kept across [`VoiceGlowEngine::reconfigure`] so a prop
/// change does not reset the glow to zero mid-sentence.
#[derive(Clone, Debug)]
struct State {
    level: f64,
    bands: [f64; 3],
    /// Flow phase in px, `0 <= phase < span`.
    phase: f64,
    /// The motion last read, held while paused.
    motion: VoiceGlowMotion,
    /// The instance's own clock in seconds: advances only while running.
    t: f64,
    /// The distortion's share, 0-1.
    warp: f64,
}

impl Default for State {
    fn default() -> Self {
        Self {
            level: 0.0,
            bands: [0.0; 3],
            phase: 0.0,
            motion: VoiceGlowMotion::default(),
            t: 0.0,
            warp: 1.0,
        }
    }
}

/// The stateful driver of one voice glow.
///
/// Feed it elapsed time with [`step`](Self::step); it returns a
/// [`VoiceGlowFrame`] each time. The engine is deterministic: the same
/// config, level source and step sequence always give the same frames.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::{HostSize, ResolvedTheme, VoiceGlow, VoiceGlowEngine};
///
/// let glow = VoiceGlow::new().level(0.8);
/// let mut engine = VoiceGlowEngine::new(&glow, ResolvedTheme::Dark);
/// let host = HostSize::new(350.0, 56.0);
/// let mut frame = engine.step(1.0 / 60.0, host);
/// for _ in 0..120 {
///     frame = engine.step(1.0 / 60.0, host);
/// }
/// assert!(frame.level > 0.8);
/// assert_eq!(frame.band_line.len(), 57);
/// ```
#[derive(Clone, Debug)]
pub struct VoiceGlowEngine {
    config: VoiceGlowConfig,
    level_source: LevelSource,
    motion_source: MotionSource,
    on_level: Option<Callback<f64>>,
    on_activate: Option<Callback<()>>,
    on_deactivate: Option<Callback<()>>,
    state: State,
    analyser: SpectrumAnalyser,
    processing: VoiceProcessing,
    fade: Fade,
    /// Whether the current config has been painted once (a paused engine
    /// repaints only when the config changes).
    painted: bool,
}

impl VoiceGlowEngine {
    /// An engine for `glow`, with `Auto` themes resolved against `system`.
    /// An active glow starts fading in.
    #[must_use]
    pub fn new(glow: &VoiceGlow, system: ResolvedTheme) -> Self {
        let config = glow.config(system);
        let fade = if config.active {
            Fade::In(0.0)
        } else {
            Fade::Off
        };
        Self {
            config,
            level_source: glow.level.clone(),
            motion_source: glow.motion.clone(),
            on_level: glow.on_level.clone(),
            on_activate: glow.on_activate.clone(),
            on_deactivate: glow.on_deactivate.clone(),
            state: State::default(),
            analyser: SpectrumAnalyser::new(),
            processing: VoiceProcessing::new(),
            fade,
            painted: false,
        }
    }

    /// Applies changed props to a running engine: the envelope, flow phase,
    /// clocks and fade keep their state (upstream keeps it per element across
    /// re-registrations); a paused engine repaints once.
    pub fn reconfigure(&mut self, glow: &VoiceGlow, system: ResolvedTheme) {
        self.config = glow.config(system);
        self.level_source = glow.level.clone();
        self.motion_source = glow.motion.clone();
        self.on_level = glow.on_level.clone();
        self.on_activate = glow.on_activate.clone();
        self.on_deactivate = glow.on_deactivate.clone();
        self.painted = false;
        match (self.config.active, self.fade) {
            (true, Fade::Off) => self.fade = Fade::In(0.0),
            // Faded out while still `active`: the end of the fade-out
            // re-activates it (see `step_fade`).
            (false, Fade::In(_) | Fade::On) => self.fade = Fade::Out(0.0),
            _ => {}
        }
    }

    /// The resolved configuration in use.
    #[must_use]
    pub fn config(&self) -> &VoiceGlowConfig {
        &self.config
    }

    /// `true` while the effect is on screen (fading in, on, or fading out).
    /// While `false` the audio analysis is stopped and steps do nothing.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.fade != Fade::Off
    }

    /// `data-active`: the effect is on screen (the same as
    /// [`is_running`](Self::is_running)).
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.is_running()
    }

    /// `data-fading`: the fade-out is under way.
    #[must_use]
    pub fn is_fading(&self) -> bool {
        matches!(self.fade, Fade::Out(_))
    }

    /// `data-paused`: the frame is held.
    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.config.paused
    }

    /// `data-voice-warp="off"`: the distortion has settled out (a gathered
    /// beam drops it) and its layers are not painted.
    #[must_use]
    pub fn is_warp_off(&self) -> bool {
        self.config.distortion > 0.0 && self.state.warp < WARP_OFF_BELOW
    }

    /// The smoothed level (0-1) the glow is showing.
    #[must_use]
    pub fn level(&self) -> f64 {
        self.state.level
    }

    /// The engine's own clock in seconds: advances only while running and not
    /// paused, so a pause holds every drift and resumes without a jump.
    #[must_use]
    pub fn clock(&self) -> f64 {
        self.state.t
    }

    /// Advances by `dt` seconds (clamped to `0..=0.05` like upstream) and
    /// returns the frame for a host of size `host`.
    pub fn step(&mut self, dt: f64, host: HostSize) -> VoiceGlowFrame {
        let dt = if dt.is_finite() {
            dt.clamp(0.0, MAX_STEP)
        } else {
            0.0
        };
        if !self.is_running() {
            // Off: the resting state at zero opacity, nothing read.
            return self.tick(0.0, false, host);
        }
        let paused = self.config.paused;
        // Paused: the frame holds. It is repainted once per config change so
        // the controls still shape the frozen frame; nothing advances.
        let first_paint = !self.painted;
        let dt = if paused { 0.0 } else { dt };
        self.step_fade(dt);
        let frame = self.tick(dt, !paused, host);
        if (!paused || first_paint)
            && let Some(cb) = &self.on_level
        {
            cb.call(self.state.level);
        }
        self.painted = true;
        frame
    }

    fn step_fade(&mut self, dt: f64) {
        self.fade = match self.fade {
            Fade::In(e) if e + dt >= FADE_IN => {
                if let Some(cb) = &self.on_activate {
                    cb.call(());
                }
                Fade::On
            }
            Fade::In(e) => Fade::In(e + dt),
            Fade::Out(e) if e + dt >= FADE_OUT => {
                if let Some(cb) = &self.on_deactivate {
                    cb.call(());
                }
                if self.config.active {
                    Fade::In(0.0)
                } else {
                    Fade::Off
                }
            }
            Fade::Out(e) => Fade::Out(e + dt),
            other => other,
        };
        // A step that ended the fade-out of a glow switched back on restarts
        // the fade-in, exactly as React's effect does after `isActive` flips.
        if self.fade == Fade::Off && self.config.active {
            self.fade = Fade::In(0.0);
        }
    }

    /// Reads the source and advances every envelope by `dt`.
    fn tick(&mut self, dt: f64, read: bool, host: HostSize) -> VoiceGlowFrame {
        let c = self.config.clone();
        self.state.t += dt;
        let t = self.state.t;

        // ── Raw level and bands from the source ─────────────────────────
        if read {
            let (level, bands) = match &self.level_source {
                LevelSource::Pcm(stream) => {
                    let r = self.analyser.read(stream, c.sensitivity);
                    (r.level, r.bands)
                }
                LevelSource::Constant(_) | LevelSource::Getter(_) => {
                    let raw = match &self.level_source {
                        LevelSource::Constant(v) => *v,
                        LevelSource::Getter(f) => f(),
                        LevelSource::Pcm(_) => 0.0,
                    };
                    let raw = if raw.is_finite() {
                        raw.clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    // No spectrum to read, so give the bands a little
                    // independent life: slow, out-of-phase wobbles scaled by
                    // the level, so the lobes still ripple.
                    (
                        raw,
                        [
                            raw,
                            raw * (0.72 + 0.28 * (t * 9.1).sin()),
                            raw * (0.6 + 0.4 * (t * 13.7 + 2.0).sin()),
                        ],
                    )
                }
            };
            // ── Shape and follow ────────────────────────────────────────
            let target = shape(level, c.threshold);
            self.state.level = follow(self.state.level, target, dt, c.attack, c.release);
            for (b, raw) in bands.iter().enumerate() {
                let bt = shape(*raw, c.threshold * 0.6);
                self.state.bands[b] =
                    follow(self.state.bands[b], bt, dt, c.attack, c.release * 1.15);
            }
        }

        // ── Motion: the lobes gathered into one beam, and where it sits ──
        let span = LOBE_SPAN * c.lobe_spacing;
        if read {
            let sweep = if c.processing || self.processing.is_engaged() {
                Some(
                    self.processing
                        .step(c.processing, &c.processing_tuning, c.reduced_motion, dt),
                )
            } else {
                None
            };
            self.state.motion = self
                .motion_source
                .sample()
                .or(sweep)
                .map_or_else(VoiceGlowMotion::default, VoiceGlowMotion::sanitized);
        }
        let motion = self.state.motion;
        let morph = motion.gather;
        let (cw, ch) = (host.width, host.height);
        let cx = if c.reduced_motion {
            0.0
        } else {
            motion.offset * span / 2.0
        };
        // The cluster: lobes pulled to 40% of their resting spread, the
        // visible range narrowed to match, a little wider while it moves.
        let gather = 1.0 - morph * 0.6;
        let mask_width = 1.0 - morph * 0.45;
        let pass_width = 1.0 + morph * 0.3 * motion.stretch;

        // ── Idle breathing folded under the voice ────────────────────────
        let breathe = if c.reduced_motion {
            0.5
        } else {
            0.5 + 0.5 * (std::f64::consts::TAU * t / c.breathe_duration).sin()
        };
        let level = self.state.level;
        let voiced = level + (1.0 - level) * c.idle * breathe;
        // The held level waits for the first quarter of the morph, the time
        // the tail hooks take to collapse, so they never light up with it.
        let held_t = ((morph - 0.25) / 0.75).clamp(0.0, 1.0);
        let held = held_t * held_t * (3.0 - 2.0 * held_t);
        let eff = voiced.max(motion.held_level * held);

        let glow = 0.15 + 0.85 * eff;
        let h = 0.5 + c.reach * eff;
        let w = (0.85 + c.spread * eff) * pass_width;

        // ── Flow: the spectrum slides sideways as the voice comes in ─────
        if c.flow != 0.0 && !c.reduced_motion {
            self.state.phase = ((self.state.phase + c.flow * eff * dt) % span + span) % span;
        }

        // ── Bend: the glow's ceiling humps up at the centre ──────────────
        let lift = c.bend * eff;
        let bend_height = lift.max(0.0);
        let bend_strength = if c.bend > 0.0 {
            (lift / c.bend).min(1.0)
        } else {
            0.0
        };

        // ── Corner arcs ─────────────────────────────────────────────────
        let beam_abs_x = cw / 2.0 + cx * w;
        let lobe_reach = 30.0 * c.scale * w;
        let arc_radius = painted_radius(c.radius, cw, ch);
        let corner_blend = morph * motion.corner_follow;
        let corner = -corner_lift(beam_abs_x, cw, arc_radius, lobe_reach * 1.4) * corner_blend;

        // ── Distortion: the glow under the band warps sideways ───────────
        // A gathered beam drops it: the warp settles out fast and, once it is
        // gone, the layers leave the paint.
        self.state.warp = follow(
            self.state.warp,
            if morph > 0.02 { 0.0 } else { 1.0 },
            dt,
            WARP_IN_TAU,
            WARP_OUT_TAU,
        );
        let warp = self.state.warp;
        let warp_off = c.distortion > 0.0 && warp < WARP_OFF_BELOW;
        let distortion = (c.distortion > 0.0 && !warp_off).then(|| DistortionFrame {
            amount: if c.reduced_motion {
                0.0
            } else {
                c.distortion * 120.0 * c.scale * (0.15 + 0.85 * eff) * warp
            },
            noise_offset: (
                8.0 * c.scale * (t * 0.9).sin(),
                4.0 * c.scale * (t * 0.6 + 1.3).sin(),
            ),
        });

        // ── Each lobe: its offset along the flow, and its amplitude ──────
        let phase = self.state.phase;
        let mut lobes = [LobeFrame {
            x: 0.0,
            amplitude: 0.0,
            lift: 0.0,
            center_x: 0.0,
            color: Rgb::default(),
            band: 0,
            base_width: 0,
            base_height: 0,
        }; 7];
        for (i, lobe) in LOBES.iter().enumerate() {
            let x = wrap_x(f64::from(lobe.x) * c.lobe_spacing + phase, span);
            // The band it follows lifts the lobe between 0.6x and 1.3x of the
            // shared height; the edge envelope fades it toward the wrap.
            let band_lift = if c.bands {
                0.6 + 0.7 * self.state.bands[lobe.band]
            } else {
                1.0
            };
            let lobe_abs_x = cw / 2.0 + (cx + x * gather) * w;
            lobes[i] = LobeFrame {
                x: x * gather,
                amplitude: band_lift * edge_envelope(x, span),
                lift: -corner_lift(lobe_abs_x, cw, arc_radius, lobe_reach) * corner_blend,
                center_x: lobe_abs_x,
                color: c.style.colors[i],
                band: lobe.band,
                base_width: lobe.w,
                base_height: lobe.h,
            };
        }

        // ── Hue drift ────────────────────────────────────────────────────
        let hue_drift = if c.static_colors || c.reduced_motion || c.hue_range == 0.0 {
            0.0
        } else {
            -c.hue_range + 2.0 * c.hue_range * ping_pong(t / c.hue_duration)
        };

        // ── The band ─────────────────────────────────────────────────────
        let (band_line, band) = if host.is_empty() {
            (Vec::new(), None)
        } else {
            let input = BandInput {
                cx,
                w,
                h,
                mw: mask_width,
                lift,
                corner: morph,
            };
            (
                band_points(&c, &input, host),
                band_paint(&c, level, bend_strength),
            )
        };

        let fade = self.fade.opacity();
        let layers = LayerOpacity {
            stroke: fade * glow * c.style.stroke_opacity * c.strength,
            inner: fade * glow * c.style.inner_opacity * c.strength,
            bloom: fade * glow * c.style.bloom_opacity * c.strength,
            band: fade * c.band_opacity * c.strength,
            core_light: c.style.core_light.map_or(0.0, |core| {
                fade * (glow * core.opacity_base * core.opacity_gain).min(1.0)
                    * c.core_light_opacity
            }),
        };

        VoiceGlowFrame {
            host,
            opacity: fade,
            level,
            drive: eff,
            glow,
            height_scale: h,
            width_scale: w,
            center_offset: cx,
            mask_width,
            bend_height,
            bend_strength,
            corner_lift: corner,
            gather: morph,
            hue_drift,
            hue_degrees: c.style.hue_base + hue_drift,
            warp,
            distortion,
            lobes,
            band_line,
            band,
            layers,
            range: (c.range_width, c.range_height),
        }
    }
}

impl VoiceGlow {
    /// One deterministic instant: a fresh engine replayed at 60 Hz up to
    /// `elapsed` (rounded down to whole frames), without callbacks.
    ///
    /// Use it for static renders, golden tests and previews. `elapsed` of
    /// zero returns the first frame, before anything has been read.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use enchant_iced::widgets::voice_glow::{HostSize, ResolvedTheme, VoiceGlow};
    ///
    /// let glow = VoiceGlow::new().level(1.0);
    /// let host = HostSize::new(350.0, 56.0);
    /// let early = glow.frame_at(Duration::from_millis(100), host, ResolvedTheme::Dark);
    /// let late = glow.frame_at(Duration::from_secs(3), host, ResolvedTheme::Dark);
    /// assert!(late.level > early.level);
    /// assert_eq!(late, glow.frame_at(Duration::from_secs(3), host, ResolvedTheme::Dark));
    /// ```
    #[must_use]
    pub fn frame_at(
        &self,
        elapsed: Duration,
        host: HostSize,
        system: ResolvedTheme,
    ) -> VoiceGlowFrame {
        let mut quiet = self.clone();
        quiet.on_level = None;
        quiet.on_activate = None;
        quiet.on_deactivate = None;
        let mut engine = VoiceGlowEngine::new(&quiet, system);
        let steps = (elapsed.as_secs_f64() * 60.0 + 1e-9).floor() as u64;
        if steps == 0 {
            return engine.step(0.0, host);
        }
        let mut frame = engine.step(FRAME_STEP, host);
        for _ in 1..steps {
            frame = engine.step(FRAME_STEP, host);
        }
        frame
    }

    /// A [`VoiceGlowEngine`] for this glow.
    #[must_use]
    pub fn engine(&self, system: ResolvedTheme) -> VoiceGlowEngine {
        VoiceGlowEngine::new(self, system)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::widgets::voice_glow::{PcmStream, VoiceGlowType};

    const HOST: HostSize = HostSize {
        width: 350.0,
        height: 56.0,
    };

    fn run(engine: &mut VoiceGlowEngine, steps: usize) -> VoiceGlowFrame {
        let mut frame = engine.step(FRAME_STEP, HOST);
        for _ in 1..steps {
            frame = engine.step(FRAME_STEP, HOST);
        }
        frame
    }

    #[test]
    fn envelope_rises_with_the_attack_time_constant() {
        // Level 1 gives target 1; after one attack (0.325 s) the follower is
        // 1 - 1/e of the way up (the first step is 1/60 s, so 0.325 s is 19.5
        // steps: use 0.3 s = 18 steps and compare to the closed form).
        let glow = VoiceGlow::new().level(1.0);
        let mut engine = VoiceGlowEngine::new(&glow, ResolvedTheme::Dark);
        let frame = run(&mut engine, 18);
        let want = 1.0 - (-(18.0 / 60.0) / 0.325_f64).exp();
        assert!(
            (frame.level - want).abs() < 1e-12,
            "{} vs {want}",
            frame.level
        );
    }

    #[test]
    fn envelope_releases_slower_than_it_attacks() {
        let glow = VoiceGlow::new().level(1.0);
        let mut engine = VoiceGlowEngine::new(&glow, ResolvedTheme::Dark);
        let peak = run(&mut engine, 240).level;
        engine.reconfigure(&glow.clone().level(0.0), ResolvedTheme::Dark);
        let after = run(&mut engine, 18).level;
        let want = peak * (-(18.0 / 60.0) / 0.86_f64).exp();
        assert!((after - want).abs() < 1e-12);
        assert!(after > 0.7 * peak, "release is slow");
    }

    #[test]
    fn noise_gate_keeps_quiet_levels_at_silence() {
        let glow = VoiceGlow::new().level(0.01);
        let mut engine = VoiceGlowEngine::new(&glow, ResolvedTheme::Dark);
        assert_eq!(run(&mut engine, 60).level, 0.0);
    }

    #[test]
    fn idle_breathing_keeps_the_beam_alive_in_silence() {
        let glow = VoiceGlow::new();
        let mut engine = VoiceGlowEngine::new(&glow, ResolvedTheme::Dark);
        let (mut lo, mut hi) = (f64::MAX, f64::MIN);
        for _ in 0..400 {
            let f = engine.step(FRAME_STEP, HOST);
            lo = lo.min(f.drive);
            hi = hi.max(f.drive);
        }
        assert!(lo >= 0.0 && hi <= 0.18 + 1e-9);
        assert!(hi - lo > 0.1, "it breathes: {lo}..{hi}");
        let still = VoiceGlow::new().idle(0.0);
        let mut engine = VoiceGlowEngine::new(&still, ResolvedTheme::Dark);
        assert_eq!(run(&mut engine, 100).drive, 0.0);
    }

    #[test]
    fn flow_advances_the_phase_and_wraps_into_the_ring() {
        let glow = VoiceGlow::new().level(1.0);
        let mut engine = VoiceGlowEngine::new(&glow, ResolvedTheme::Dark);
        let span = LOBE_SPAN * engine.config().lobe_spacing;
        let first = run(&mut engine, 1);
        let later = run(&mut engine, 600);
        assert_ne!(first.lobes[0].x, later.lobes[0].x);
        for lobe in later.lobes {
            // `x` is gathered (x1) but at gather 0 it is the raw wrapped offset.
            assert!(lobe.x >= -span / 2.0 && lobe.x < span / 2.0, "{}", lobe.x);
        }
        let held = VoiceGlow::new().level(1.0).flow(0.0);
        let mut engine = VoiceGlowEngine::new(&held, ResolvedTheme::Dark);
        let f = run(&mut engine, 200);
        assert!(f.lobes[0].x.abs() < 1e-9);
        assert!((f.lobes[1].x - -36.0 * 0.85).abs() < 1e-9);
    }

    #[test]
    fn paused_engine_freezes_every_clock_and_resumes_where_it_stopped() {
        let live = VoiceGlow::new().level(0.9);
        let mut engine = VoiceGlowEngine::new(&live, ResolvedTheme::Dark);
        run(&mut engine, 90);
        let clock = engine.clock();
        engine.reconfigure(&live.clone().paused(true), ResolvedTheme::Dark);
        let frozen = engine.step(FRAME_STEP, HOST);
        let later = run(&mut engine, 120);
        assert_eq!(frozen, later);
        assert_eq!(engine.clock(), clock);
        // Resuming continues from the held state, with no jump.
        engine.reconfigure(&live, ResolvedTheme::Dark);
        let resumed = engine.step(FRAME_STEP, HOST);
        assert!((resumed.level - frozen.level).abs() < 0.02);
        assert!(engine.clock() > clock);
    }

    #[test]
    fn paused_engine_never_reads_the_source_and_reports_level_once() {
        let reads = Arc::new(AtomicUsize::new(0));
        let reported = Arc::new(AtomicUsize::new(0));
        let r = Arc::clone(&reads);
        let p = Arc::clone(&reported);
        let glow = VoiceGlow::new()
            .level_source(LevelSource::getter(move || {
                r.fetch_add(1, Ordering::SeqCst);
                0.5
            }))
            .on_level(move |_| {
                p.fetch_add(1, Ordering::SeqCst);
            })
            .paused(true);
        let mut engine = VoiceGlowEngine::new(&glow, ResolvedTheme::Dark);
        run(&mut engine, 30);
        assert_eq!(reads.load(Ordering::SeqCst), 0);
        assert_eq!(
            reported.load(Ordering::SeqCst),
            1,
            "one repaint per config change"
        );
    }

    #[test]
    fn reduced_motion_stops_flow_hue_distortion_and_travel() {
        let glow = VoiceGlow::new()
            .level(1.0)
            .reduced_motion(true)
            .processing(true);
        let mut engine = VoiceGlowEngine::new(&glow, ResolvedTheme::Dark);
        let frame = run(&mut engine, 400);
        assert_eq!(frame.center_offset, 0.0);
        assert_eq!(frame.hue_drift, 0.0);
        assert!(
            frame.lobes.iter().any(|l| l.x.abs() > 1.0),
            "lobes keep resting spots"
        );
        // No flow: the centre lobe never leaves 0.
        assert!(frame.lobes[0].x.abs() < 1e-9);
        // The breath is a still 0.5.
        let at_rest = VoiceGlow::new().reduced_motion(true);
        let mut engine = VoiceGlowEngine::new(&at_rest, ResolvedTheme::Dark);
        let a = run(&mut engine, 10).drive;
        let b = run(&mut engine, 200).drive;
        assert_eq!(a, b);
        assert_eq!(a, 0.18 * 0.5);
    }

    #[test]
    fn hue_drift_ping_pongs_across_the_range() {
        let glow = VoiceGlow::new();
        let mut engine = VoiceGlowEngine::new(&glow, ResolvedTheme::Dark);
        let (mut lo, mut hi) = (f64::MAX, f64::MIN);
        for _ in 0..(12 * 60 + 5) {
            let h = engine.step(FRAME_STEP, HOST).hue_drift;
            lo = lo.min(h);
            hi = hi.max(h);
        }
        assert!(
            (lo + 24.0).abs() < 0.05 && (hi - 24.0).abs() < 0.05,
            "{lo}..{hi}"
        );
        let fixed = VoiceGlow::new().static_colors(true);
        let mut engine = VoiceGlowEngine::new(&fixed, ResolvedTheme::Dark);
        assert_eq!(run(&mut engine, 120).hue_drift, 0.0);
    }

    #[test]
    fn processing_gathers_the_lobes_and_drops_the_distortion() {
        let glow = VoiceGlow::new()
            .kind(VoiceGlowType::Mobile)
            .processing(true);
        let mut engine = VoiceGlowEngine::new(&glow, ResolvedTheme::Dark);
        let host = HostSize::new(400.0, 300.0);
        let mut frame = engine.step(FRAME_STEP, host);
        assert!(frame.distortion.is_some());
        for _ in 0..240 {
            frame = engine.step(FRAME_STEP, host);
        }
        assert!(frame.gather > 0.99);
        assert!((frame.mask_width - 0.55).abs() < 0.01);
        assert!(frame.distortion.is_none(), "warp settled out");
        assert!(frame.drive >= 0.35 - 1e-9, "held at the tuning's level");
        // Switching it off releases everything.
        engine.reconfigure(&glow.clone().processing(false), ResolvedTheme::Dark);
        for _ in 0..600 {
            frame = engine.step(FRAME_STEP, host);
        }
        assert!(frame.gather < 1e-3);
        assert!(frame.distortion.is_some());
    }

    #[test]
    fn fade_in_and_out_fire_callbacks_once() {
        let on = Arc::new(AtomicUsize::new(0));
        let off = Arc::new(AtomicUsize::new(0));
        let (a, b) = (Arc::clone(&on), Arc::clone(&off));
        let glow = VoiceGlow::new()
            .on_activate(move || {
                a.fetch_add(1, Ordering::SeqCst);
            })
            .on_deactivate(move || {
                b.fetch_add(1, Ordering::SeqCst);
            });
        let mut engine = VoiceGlowEngine::new(&glow, ResolvedTheme::Dark);
        let early = engine.step(FRAME_STEP, HOST);
        assert!(early.opacity < 0.1);
        let faded_in = run(&mut engine, 60);
        assert_eq!(faded_in.opacity, 1.0);
        assert_eq!(
            (on.load(Ordering::SeqCst), off.load(Ordering::SeqCst)),
            (1, 0)
        );

        engine.reconfigure(&glow.clone().active(false), ResolvedTheme::Dark);
        let fading = engine.step(FRAME_STEP, HOST);
        assert!(
            fading.opacity < 1.0 && fading.opacity > 0.9,
            "fade-out starts from full"
        );
        assert!(engine.is_running());
        let gone = run(&mut engine, 40);
        assert_eq!(gone.opacity, 0.0);
        assert!(!engine.is_running());
        assert_eq!(
            (on.load(Ordering::SeqCst), off.load(Ordering::SeqCst)),
            (1, 1)
        );

        // Switched back on: fades in again from nothing.
        engine.reconfigure(&glow, ResolvedTheme::Dark);
        assert!(engine.is_running());
        assert!(engine.step(FRAME_STEP, HOST).opacity < 0.1);
    }

    #[test]
    fn inactive_engine_stops_reading_and_shows_nothing() {
        let reads = Arc::new(AtomicUsize::new(0));
        let r = Arc::clone(&reads);
        let glow = VoiceGlow::new()
            .level_source(LevelSource::getter(move || {
                r.fetch_add(1, Ordering::SeqCst);
                1.0
            }))
            .active(false);
        let mut engine = VoiceGlowEngine::new(&glow, ResolvedTheme::Dark);
        let frame = run(&mut engine, 30);
        assert_eq!(reads.load(Ordering::SeqCst), 0);
        assert_eq!(frame.opacity, 0.0);
        assert_eq!(frame.layers.stroke, 0.0);
    }

    #[test]
    fn css_ease_matches_the_keyword() {
        assert_eq!(css_ease(0.0), 0.0);
        assert!((css_ease(1.0) - 1.0).abs() < 1e-12);
        // cubic-bezier(.25,.1,.25,1) at x = 0.5 is about 0.8024.
        assert!((css_ease(0.5) - 0.8024).abs() < 1e-3);
    }

    #[test]
    fn pcm_stream_drives_level_and_bands() {
        let stream = PcmStream::new(48_000.0);
        let glow = VoiceGlow::new().level_source(stream.clone());
        let mut engine = VoiceGlowEngine::new(&glow, ResolvedTheme::Dark);
        let silent = run(&mut engine, 30);
        assert_eq!(silent.level, 0.0);
        let tone: Vec<f32> = (0..4096)
            .map(|n| {
                (0.3 * (2.0 * std::f64::consts::PI * 1000.0 * f64::from(n) / 48_000.0).sin()) as f32
            })
            .collect();
        stream.push(&tone);
        let loud = run(&mut engine, 60);
        assert!(loud.level > 0.5, "{}", loud.level);
        // A 1 kHz tone is the mids: the neighbours of the centre outrun the
        // highs' lobes in amplitude, relative to their envelope.
        let amp = |i: usize| {
            loud.lobes[i].amplitude
                / edge_envelope(loud.lobes[i].x, LOBE_SPAN * engine.config().lobe_spacing)
        };
        assert!(amp(1) > amp(3));
    }

    #[test]
    fn frame_at_is_deterministic_and_silent() {
        let calls = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&calls);
        let glow = VoiceGlow::new().level(0.7).on_level(move |_| {
            c.fetch_add(1, Ordering::SeqCst);
        });
        let a = glow.frame_at(Duration::from_millis(1234), HOST, ResolvedTheme::Dark);
        let b = glow.frame_at(Duration::from_millis(1234), HOST, ResolvedTheme::Dark);
        assert_eq!(a, b);
        assert_eq!(calls.load(Ordering::SeqCst), 0, "frame_at strips callbacks");
        let zero = glow.frame_at(Duration::ZERO, HOST, ResolvedTheme::Dark);
        assert_eq!(zero.level, 0.0);
    }

    #[test]
    fn empty_host_has_no_band_and_never_panics() {
        let glow = VoiceGlow::new().level(1.0);
        let mut engine = VoiceGlowEngine::new(&glow, ResolvedTheme::Dark);
        let f = engine.step(FRAME_STEP, HostSize::new(0.0, 0.0));
        assert!(f.band_line.is_empty() && f.band.is_none());
        let g = engine.step(f64::NAN, HostSize::new(f64::NAN, -5.0));
        assert!(g.level.is_finite());
    }

    #[test]
    fn mask_ellipse_grows_with_the_bend() {
        let glow = VoiceGlow::new().level(1.0);
        let mut engine = VoiceGlowEngine::new(&glow, ResolvedTheme::Dark);
        let rest = engine.step(0.0, HOST);
        let loud = run(&mut engine, 300);
        let r = rest.mask_ellipse(170.0, 64.0);
        let l = loud.mask_ellipse(170.0, 64.0);
        assert!(l.2 > r.2 && l.3 > r.3);
        assert_eq!(l.0, HOST.width / 2.0);
    }

    #[test]
    fn the_css_hook_opacities_scale_their_layers() {
        let base = VoiceGlow::new().theme(crate::widgets::voice_glow::VoiceGlowTheme::Light);
        let half = base.clone().band_opacity(0.5).core_light_opacity(0.5);
        let mut a = VoiceGlowEngine::new(&base, ResolvedTheme::Light);
        let mut b = VoiceGlowEngine::new(&half, ResolvedTheme::Light);
        let (fa, fb) = (run(&mut a, 120), run(&mut b, 120));
        assert!(fa.layers.band > 0.0 && fa.layers.core_light > 0.0);
        assert!((fb.layers.band - fa.layers.band * 0.5).abs() < 1e-12);
        assert!((fb.layers.core_light - fa.layers.core_light * 0.5).abs() < 1e-12);
        // The strokes are untouched by the band hook.
        assert_eq!(fa.layers.stroke, fb.layers.stroke);
    }

    #[test]
    fn hue_base_replaces_the_themes_resting_shift() {
        use crate::widgets::voice_glow::{Degrees, VoiceGlowTheme};
        let light = VoiceGlow::new().theme(VoiceGlowTheme::Light);
        assert_eq!(light.config(ResolvedTheme::Light).style.hue_base, 5.0);
        let set = light.clone().hue_base(Degrees::new(30.0));
        assert_eq!(set.config(ResolvedTheme::Light).style.hue_base, 30.0);
        let dark = VoiceGlow::new().hue_base(Degrees::new(-12.0));
        assert_eq!(dark.config(ResolvedTheme::Dark).style.hue_base, -12.0);
    }

    #[test]
    fn the_data_attribute_accessors_follow_the_state() {
        let glow = VoiceGlow::new().level(0.5);
        let mut engine = VoiceGlowEngine::new(&glow, ResolvedTheme::Dark);
        assert!(engine.is_active() && !engine.is_fading() && !engine.is_paused());
        run(&mut engine, 60);
        engine.reconfigure(&glow.clone().active(false), ResolvedTheme::Dark);
        assert!(engine.is_active() && engine.is_fading());
        run(&mut engine, 60);
        assert!(!engine.is_active() && !engine.is_fading());
        let paused = VoiceGlowEngine::new(&glow.clone().paused(true), ResolvedTheme::Dark);
        assert!(paused.is_paused());
    }

    #[test]
    fn a_gathered_beam_drops_the_warp() {
        let glow = VoiceGlow::new().processing(true);
        let mut engine = VoiceGlowEngine::new(&glow, ResolvedTheme::Dark);
        assert!(!engine.is_warp_off());
        run(&mut engine, 240);
        assert!(engine.is_warp_off());
    }

    #[test]
    fn stream_is_the_pcm_level_source() {
        let pcm = PcmStream::new(48_000.0);
        let glow = VoiceGlow::new().stream(pcm.clone());
        assert_eq!(
            glow,
            VoiceGlow::new().level_source(crate::widgets::voice_glow::LevelSource::pcm(pcm))
        );
    }
}
