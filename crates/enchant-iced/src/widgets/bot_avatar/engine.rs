//! The animation engine behind the bot avatar.
//!
//! A faithful port of the Libraries.dev `bot-avatars` engine
//! (`packages/bot-avatars/src/engine.ts`). A [`Pose`] is the rig: head yaw,
//! pitch and roll, a position, squash, how open the eyes are, where they look,
//! the breathing cycle, the laugh, the whirl round a spinning body, and blend
//! weights for the three states. A [`BotAvatarSim`] advances that pose through
//! time: each state sets targets and wanders around them, runs its own events
//! (a flip, a hop, a nod, a blink), and a state change eases from one set of
//! targets to the next on a timed curve, so it starts and ends softly.
//!
//! Every formula, constant and random draw follows the TypeScript original in
//! order, so a given seed replays the same sequence of poses. Angles are in
//! radians internally; body units are the 100×100 design space.
//!
//! | here | upstream |
//! |------|----------|
//! | [`BotAvatarSim`] | `Sim` |
//! | [`Pose`] | `Pose` |
//! | [`JumpConfig`], [`JUMP_DEFAULTS`] | `JumpConfig`, `JUMP_DEFAULTS` |
//! | [`rest_pose`] | `restPose` |

use std::f32::consts::{PI, TAU};

use crate::widgets::bot_avatar::types::{BotAvatarSquashEase, BotAvatarState};

/// Degrees to radians.
const DEG: f32 = PI / 180.0;

/// One working hop's period, in seconds.
const HOP_T: f32 = 0.68;
/// The height of a spinning (every third) working hop.
const HOP_SPIN_H: f32 = 26.0;

/// A flip is that spinning hop on its own: a crouch before it (the squash
/// that, in the working state, the previous landing leaves), the hop, and the
/// landing's recovery after — measured in hop periods.
const FLIP_PRE: f32 = 0.2;
/// Declared by the upstream engine as part of the flip's shaping; it is not
/// read there either, and it is kept here for parity.
#[allow(dead_code)]
const FLIP_POST: f32 = 0.66;

/// The idle gaze: how far the head swings, and how long it stays. The sideways
/// reach is the big one — a look to a corner reads as a turn of the head, with
/// the rise and a slight tilt going along.
const GAZE_YAW: f32 = 35.0 * DEG;
const GAZE_PITCH: f32 = 14.0 * DEG;
const GAZE_ROLL: f32 = 3.2 * DEG;
const GAZE_HOLD_MIN: f32 = 2.6;
const GAZE_HOLD_MAX: f32 = 4.4;

/// How long a state change out of sleep takes.
const SWITCH_FROM_SLEEP: f32 = 1.0;

/// How long a switch takes. Settling down takes longer than getting to work:
/// coming back to idle the body has to let go of the hops, so it eases out over
/// more than a second.
fn switch_to(state: BotAvatarState) -> f32 {
    match state {
        BotAvatarState::Idle => 1.2,
        BotAvatarState::Working => 0.7,
        BotAvatarState::Sleeping => 1.4,
    }
}

/// The index of a state in the blend-weight array: idle, working, sleeping.
fn state_index(state: BotAvatarState) -> usize {
    match state {
        BotAvatarState::Idle => 0,
        BotAvatarState::Working => 1,
        BotAvatarState::Sleeping => 2,
    }
}

/// Resting targets per state. Everything the wanderers and events add sits on
/// top of these.
#[derive(Clone, Copy, Debug, Default)]
struct Rest {
    pitch: f32,
    roll: f32,
    y: f32,
    look_x: f32,
    look_y: f32,
}

fn rest_for(state: BotAvatarState) -> Rest {
    match state {
        BotAvatarState::Idle => Rest::default(),
        BotAvatarState::Working => Rest {
            pitch: 5.0 * DEG,
            ..Rest::default()
        },
        BotAvatarState::Sleeping => Rest {
            pitch: -16.0 * DEG,
            roll: 6.0 * DEG,
            y: 3.0,
            look_x: 0.0,
            look_y: 1.0,
        },
    }
}

/// Blend the states' rest targets by `weights`.
fn blend_rest(weights: [f32; 3]) -> Rest {
    let states = [
        BotAvatarState::Idle,
        BotAvatarState::Working,
        BotAvatarState::Sleeping,
    ];
    let mut rest = Rest::default();
    for (state, weight) in states.into_iter().zip(weights) {
        let r = rest_for(state);
        rest.pitch += r.pitch * weight;
        rest.roll += r.roll * weight;
        rest.y += r.y * weight;
        rest.look_x += r.look_x * weight;
        rest.look_y += r.look_y * weight;
    }
    rest
}

/// Deterministic per-instance randomness (mulberry32).
#[derive(Clone, Copy, Debug)]
struct Mulberry32 {
    state: u32,
}

impl Mulberry32 {
    /// Seed from the public avatar `seed`, matching
    /// `rng(Math.floor(seed * 1e6) + 1)` in the original.
    fn from_seed(seed: f32) -> Self {
        let step = (f64::from(seed) * 1e6).floor() + 1.0;
        let mixed = to_uint32(step * 2_654_435_761.0);
        Self {
            state: if mixed == 0 { 1 } else { mixed },
        }
    }

    /// The next uniform sample in `[0, 1)`.
    fn next(&mut self) -> f32 {
        self.state = self.state.wrapping_add(0x6d2b_79f5);
        let mut t = self.state;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        (f64::from(t ^ (t >> 14)) / 4_294_967_296.0) as f32
    }
}

/// JavaScript's `ToUint32`: truncate toward zero, then wrap into a `u32`.
fn to_uint32(value: f64) -> u32 {
    if !value.is_finite() {
        return 0;
    }
    let truncated = value.trunc();
    let wrapped = truncated % 4_294_967_296.0;
    let wrapped = if wrapped < 0.0 {
        wrapped + 4_294_967_296.0
    } else {
        wrapped
    };
    wrapped as u32
}

/// Exponential approach: `rate` per second, frame-rate independent.
fn approach(cur: f32, target: f32, rate: f32, dt: f32) -> f32 {
    cur + (target - cur) * (1.0 - (-rate * dt).exp())
}

/// A cubic S-curve: soft at both ends.
fn ease_in_out(p: f32) -> f32 {
    if p < 0.5 {
        4.0 * p * p * p
    } else {
        1.0 - (-2.0 * p + 2.0).powi(3) / 2.0
    }
}

/// Gentler at both ends than the cubic: for drifting off and waking.
fn ease_sine(p: f32) -> f32 {
    0.5 - 0.5 * (PI * p).cos()
}

/// A smoothstep from `a` to `b`, clamped outside the pair.
fn smooth(a: f32, b: f32, v: f32) -> f32 {
    let x = ((v - a) / (b - a)).clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Where in its time each easing reaches the deepest squash.
fn squash_peak(ease: BotAvatarSquashEase) -> f32 {
    match ease {
        BotAvatarSquashEase::Sharp => 0.0,
        BotAvatarSquashEase::Soft => 0.5,
        BotAvatarSquashEase::Bouncy => 0.144,
        _ => 2.0 / 7.0,
    }
}

/// The rise: how the body comes back from its deepest squash to its own shape,
/// 1 → 0 over the rise time. `sharp` lets go at once and eases in to rest,
/// `pulse` leaves quickly with a long settle, `soft` eases out of the squash
/// and into rest, `bouncy` passes rest into a slight stretch and settles back.
fn rise_pulse(v: f32, ease: BotAvatarSquashEase) -> f32 {
    if v <= 0.0 {
        return 1.0;
    }
    if v >= 1.0 {
        return 0.0;
    }
    match ease {
        BotAvatarSquashEase::Sharp => (1.0 - v) * (1.0 - v),
        BotAvatarSquashEase::Soft => 0.5 + 0.5 * (PI * v).cos(),
        BotAvatarSquashEase::Bouncy => (-3.2 * v).exp() * (5.4 * v).cos() - v * v * v * 0.026,
        _ => {
            let k = 4.2 * v;
            (1.0 + k) * (-k).exp() - v * v * v * 0.078
        }
    }
}

/// The weight settling through the hold, as a gain on the deepest squash: the
/// same four curves as the landing squash, pressing up to a quarter deeper and
/// back to the held depth, so the hold's ends never step.
fn ground_shape(v: f32, ease: BotAvatarSquashEase) -> f32 {
    1.0 + 0.25 * squash_pulse(v, ease)
}

/// The landing squash over its time, 0–1 → its depth, 0–1 at the peak: sharp is
/// all there at contact and eases off; pulse presses quickly and recovers
/// without a wobble (a critically damped spring under a short push); soft eases
/// in and out; bouncy overshoots into a stretch and settles. Each is faded out
/// over the last part so it ends at nothing.
fn squash_pulse(u: f32, ease: BotAvatarSquashEase) -> f32 {
    if u <= 0.0 || u >= 1.0 {
        return 0.0;
    }
    let x = match ease {
        BotAvatarSquashEase::Sharp => (1.0 - u) * (1.0 - u),
        BotAvatarSquashEase::Soft => (PI * u).sin().powi(2),
        BotAvatarSquashEase::Bouncy => (-3.15 * u).exp() * (8.43 * u).sin() / 0.596,
        _ => {
            let k = 7.0 * u;
            k * k * (2.0 - k).exp() / 4.0
        }
    };
    let tail = if u > 0.85 {
        1.0 - (u - 0.85) / 0.15
    } else {
        1.0
    };
    x * tail * tail * (3.0 - 2.0 * tail)
}

/// The hop's shaping: squash on the ground, stretch at the top.
fn hop_squash(a: f32) -> f32 {
    (-(a.abs().min((a - 1.0).abs()) / 0.11).powi(2)).exp()
}

/// The envelope of the whirl round a spinning body: not there until the turn
/// is visibly under way, gone before the landing.
fn whirl_envelope(q: f32) -> f32 {
    smooth(0.1, 0.26, q) * (1.0 - smooth(0.66, 0.9, q))
}

/// The whirl ring's own travel: fast and steady from the first frame, with a
/// little of the body's own easing on top, so it is moving the moment it shows.
fn whirl_ring_angle(q: f32) -> f32 {
    TAU * (1.5 * q + 0.9 * ease_in_out(q))
}

/// The crouch before a flip's take-off: a click's builds over its squash time,
/// an idle jump's is the hop's short bump.
fn flip_pre(j: JumpConfig, poked: bool) -> f32 {
    if poked {
        j.click_squash_time
    } else {
        FLIP_PRE * j.time
    }
}

/// How long a whole flip lasts, crouch to settled landing.
fn flip_duration(j: JumpConfig, poked: bool) -> f32 {
    flip_pre(j, poked)
        + j.time
        + squash_peak(j.squash_ease)
            * if poked {
                j.click_squash_time
            } else {
                j.squash_time
            }
        + j.ground_time.max(0.0)
        + j.rise_time
        + j.land.max(0.0)
        + 0.05
}

/// The jump's numbers: an idle flip's and a click's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JumpConfig {
    /// How high, in body units (the body is 100 tall).
    pub height: f32,
    /// Seconds in the air.
    pub time: f32,
    /// How much the body stretches in the air, 0–2.
    pub stretch: f32,
    /// How much it squashes on the ground, before take-off and on landing, 0–2.
    pub squash: f32,
    /// Seconds the landing squash takes, contact to recovered.
    pub squash_time: f32,
    /// The landing squash's shape.
    pub squash_ease: BotAvatarSquashEase,
    /// Seconds the body holds its deepest squash on the ground.
    pub ground_time: f32,
    /// How the weight settles through that hold.
    pub ground_ease: BotAvatarSquashEase,
    /// Seconds the body takes to rise from its deepest squash back to shape.
    pub rise_time: f32,
    /// How it rises.
    pub rise_ease: BotAvatarSquashEase,
    /// Seconds a click's jump takes for its landing squash.
    pub click_squash_time: f32,
    /// Whole turns in the air.
    pub spin: f32,
    /// Degrees of lean into it.
    pub lean: f32,
    /// Seconds between idle jumps, roughly (±40 %); 0 for none.
    pub every: f32,
    /// When the landing squash begins: seconds before (negative) or after
    /// touch-down; 0 is the moment of contact.
    pub land: f32,
}

impl Default for JumpConfig {
    fn default() -> Self {
        JUMP_DEFAULTS
    }
}

/// The default jump: an idle flip's numbers, and a click's squash and spin.
pub const JUMP_DEFAULTS: JumpConfig = JumpConfig {
    height: HOP_SPIN_H,
    time: HOP_T,
    stretch: 1.0,
    squash: 1.15,
    squash_time: 0.37,
    squash_ease: BotAvatarSquashEase::Pulse,
    ground_time: 0.11,
    ground_ease: BotAvatarSquashEase::Pulse,
    rise_time: 0.33,
    rise_ease: BotAvatarSquashEase::Pulse,
    click_squash_time: 0.24,
    spin: 1.0,
    lean: 6.0,
    every: 8.0,
    land: 0.0,
};

/// The rig's pose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    /// Radians; yaw > 0 turns the face to the viewer's right, pitch > 0 looks up.
    pub yaw: f32,
    /// Radians; positive looks up.
    pub pitch: f32,
    /// Radians; the head's tilt.
    pub roll: f32,
    /// Body-box units (the 100×100 design space).
    pub x: f32,
    /// Body-box units; negative is up.
    pub y: f32,
    /// Horizontal scale.
    pub sx: f32,
    /// Vertical scale.
    pub sy: f32,
    /// 0 shut … 1 open, before blinks.
    pub eye_open: f32,
    /// How far the left lid is down right now, 0 … 1.
    pub blink_l: f32,
    /// How far the right lid is down right now, 0 … 1.
    pub blink_r: f32,
    /// Where the eyes look, in eye-widths.
    pub look_x: f32,
    /// Where the eyes look, in eye-heights.
    pub look_y: f32,
    /// The breathing cycle, −1 … 1.
    pub breath: f32,
    /// Working only: how far the eyes have closed into a laugh, 0 … 1.
    pub laugh: f32,
    /// The cartoon whirl round a spinning body: strength 0 … 1.
    pub whirl: f32,
    /// Where the whirl's head is, in radians round the ring.
    pub whirl_angle: f32,
    /// Blend weights: idle, working, sleeping; they sum to 1.
    pub w: [f32; 3],
}

impl Default for Pose {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,
            x: 0.0,
            y: 0.0,
            sx: 1.0,
            sy: 1.0,
            eye_open: 1.0,
            blink_l: 0.0,
            blink_r: 0.0,
            look_x: 0.0,
            look_y: 0.0,
            breath: 0.0,
            laugh: 0.0,
            whirl: 0.0,
            whirl_angle: 0.0,
            w: [1.0, 0.0, 0.0],
        }
    }
}

/// A value that drifts: picks a new target inside its range every hold, and
/// eases toward it. Ranges change with the state; the value never jumps.
#[derive(Clone, Copy, Debug)]
struct Wander {
    value: f32,
    vel: f32,
    target: f32,
    next: f32,
    amp: f32,
    hold_min: f32,
    hold_max: f32,
    rate: f32,
    /// The head channels move as a lightly damped spring; the eyes dart with an
    /// exponential approach.
    spring: bool,
}

impl Wander {
    fn new(amp: f32, hold_min: f32, hold_max: f32, rate: f32, spring: bool) -> Self {
        Self {
            value: 0.0,
            vel: 0.0,
            target: 0.0,
            next: 0.0,
            amp,
            hold_min,
            hold_max,
            rate,
            spring,
        }
    }

    fn update(&mut self, t: f32, dt: f32, rand: &mut Mulberry32) {
        if t >= self.next {
            self.target = (rand.next() * 2.0 - 1.0) * self.amp;
            self.next = t + self.hold_min + rand.next() * (self.hold_max - self.hold_min);
        }
        if self.spring {
            let w = self.rate * 1.6;
            let z = 0.9;
            self.vel += (w * w * (self.target - self.value) - 2.0 * z * w * self.vel) * dt;
            self.value += self.vel * dt;
        } else {
            self.value = approach(self.value, self.target, self.rate, dt);
        }
    }

    /// Aim at a value and stay there: the rig picks the next one.
    fn aim(&mut self, v: f32) {
        self.target = v;
        self.next = f32::INFINITY;
    }

    fn set(&mut self, amp: f32, hold_min: f32, hold_max: f32, rate: f32) {
        self.amp = amp;
        self.hold_min = hold_min;
        self.hold_max = hold_max;
        self.rate = rate;
        self.next = 0.0;
    }
}

/// A one-shot event: progress 0 → 1 over its duration, then idle at -1.
#[derive(Clone, Copy, Debug)]
struct Event {
    p: f32,
    duration: f32,
}

impl Event {
    fn new(duration: f32) -> Self {
        Self { p: -1.0, duration }
    }

    fn fire(&mut self) {
        self.p = 0.0;
    }

    fn active(&self) -> bool {
        self.p >= 0.0
    }

    fn update(&mut self, dt: f32) {
        if self.p < 0.0 {
            return;
        }
        self.p += dt / self.duration;
        if self.p >= 1.0 {
            self.p = -1.0;
        }
    }
}

/// When the next idle jump is due: `every` seconds, give or take 40 %.
fn next_flip(jump: &JumpConfig, rand: &mut Mulberry32, t: f32, k: f32) -> f32 {
    if jump.every > 0.0 {
        t + jump.every * k * (0.625 + rand.next() * 0.75)
    } else {
        f32::INFINITY
    }
}

/// The animation state: a pose advanced through time by the rig's motion.
///
/// The simulation is deterministic per seed: two instances created with the
/// same seed and advanced with the same steps produce identical poses.
pub struct BotAvatarSim {
    pose: Pose,
    state: BotAvatarState,
    seed: f32,
    rand: Mulberry32,
    t: f32,

    /// A state change: the weights it started from and its progress.
    w_from: [f32; 3],
    tr: f32,
    tr_duration: f32,

    yaw_w: Wander,
    pitch_w: Wander,
    roll_w: Wander,
    look_x_w: Wander,
    look_y_w: Wander,

    blink: Event,
    blink_at: f32,
    blink_again: bool,

    /// −1 left eye only, 1 right eye only, 0 both.
    dart: Event,
    dart_at: f32,
    dart_x: f32,
    dart_y: f32,

    flip: Event,
    flip_poked: bool,
    jump: JumpConfig,
    flip_at: f32,
    flip_side: f32,

    nod: Event,
    nod_at: f32,

    hop_phase: f32,
    hop_count: u32,
    /// The hops' gain: the working weight while in the state, then held so a
    /// hop under way finishes whole when the state is left.
    hop_gain: f32,

    laugh_ev: Event,
    laugh_at: f32,

    /// The jelly: a damped spring driven by how fast the head turns, so a sweep
    /// stretches the body and it wobbles back.
    prev_yaw: f32,
    jelly: f32,
    jelly_v: f32,

    /// How far the eyes run ahead of a head turn.
    gaze_lead: f32,

    /// The idle gaze: where the head is looking, and when it moves on.
    gaze_dir: [f32; 2],
    gaze_at: f32,
    /// How far it turns to the side, 1 as the gaze has it.
    turn_k: f32,

    /// The breathing cycle's phase, in turns.
    breath_phase: f32,

    /// The pointer, as an offset from the head in head-widths, and how much to
    /// follow it — both smoothed.
    ptr_x: f32,
    ptr_y: f32,
    ptr_s: f32,
    ptr_target_x: f32,
    ptr_target_y: f32,
    ptr_target_s: f32,

    /// The smoothed yaw the head is turning to on its own.
    base_yaw: f32,
}

impl BotAvatarSim {
    /// Create a simulation at `state`, seeded so every instance starts
    /// somewhere else in its loops.
    pub fn new(seed: f32, state: BotAvatarState) -> Self {
        let mut rand = Mulberry32::from_seed(seed);

        // The wanderers are built first; they do not draw from the generator.
        let yaw_w = Wander::new(36.0 * DEG, 1.1, 2.6, 3.0, true);
        let pitch_w = Wander::new(10.0 * DEG, 1.1, 2.6, 2.6, true);
        let roll_w = Wander::new(5.0 * DEG, 1.6, 3.2, 2.0, true);
        let look_x_w = Wander::new(3.6, 0.5, 2.0, 14.0, false);
        let look_y_w = Wander::new(2.4, 0.5, 2.0, 14.0, false);

        // Every instance starts somewhere else in its loops.
        let t = rand.next() * 10.0;
        let hop_phase = rand.next();
        let breath_phase = rand.next();
        // Event timers count from that start, so nothing fires on the first tick.
        let blink_at = t + 1.0 + rand.next() * 3.0;
        let jump = JUMP_DEFAULTS;
        let flip_at = next_flip(&jump, &mut rand, t, 1.0);
        let nod_at = t + 3.0 + rand.next() * 4.0;
        let dart_at = t + 1.0 + rand.next() * 2.0;
        let laugh_at = t + 0.6 + rand.next() * 1.5;

        let mut sim = Self {
            pose: Pose::default(),
            state: BotAvatarState::Idle,
            seed,
            rand,
            t,
            w_from: [1.0, 0.0, 0.0],
            tr: 1.0,
            tr_duration: 1.2,
            yaw_w,
            pitch_w,
            roll_w,
            look_x_w,
            look_y_w,
            blink: Event::new(0.17),
            blink_at,
            blink_again: false,
            dart: Event::new(0.12),
            dart_at,
            dart_x: 0.0,
            dart_y: 0.0,
            flip: Event::new(flip_duration(JUMP_DEFAULTS, false)),
            flip_poked: false,
            jump,
            flip_at,
            flip_side: 1.0,
            nod: Event::new(1.7),
            nod_at,
            hop_phase,
            hop_count: 0,
            hop_gain: 0.0,
            laugh_ev: Event::new(0.8),
            laugh_at,
            prev_yaw: 0.0,
            jelly: 0.0,
            jelly_v: 0.0,
            gaze_lead: 0.0,
            gaze_dir: [0.0, 0.0],
            gaze_at: 0.0,
            turn_k: 1.0,
            breath_phase,
            ptr_x: 0.0,
            ptr_y: 0.0,
            ptr_s: 0.0,
            ptr_target_x: 0.0,
            ptr_target_y: 0.0,
            ptr_target_s: 0.0,
            base_yaw: 0.0,
        };
        sim.switch_state(state, true);
        sim
    }

    /// The pose at the last update.
    pub fn pose(&self) -> &Pose {
        &self.pose
    }

    /// The current state.
    pub fn state(&self) -> BotAvatarState {
        self.state
    }

    /// Ease to `state` on the state's switch curve.
    pub fn set_state(&mut self, state: BotAvatarState) {
        self.switch_state(state, false);
    }

    /// Snap to `state` at once, with its weights fully applied.
    pub fn set_state_immediate(&mut self, state: BotAvatarState) {
        self.switch_state(state, true);
    }

    fn switch_state(&mut self, next: BotAvatarState, immediate: bool) {
        if next == self.state && !immediate {
            return;
        }
        let from = self.state;
        self.state = next;
        if immediate {
            let idx = state_index(next);
            for (i, slot) in self.pose.w.iter_mut().enumerate() {
                *slot = if i == idx { 1.0 } else { 0.0 };
            }
            self.tr = 1.0;
        } else {
            self.w_from = self.pose.w;
            self.tr = 0.0;
            self.tr_duration = if from == BotAvatarState::Sleeping {
                SWITCH_FROM_SLEEP
            } else {
                switch_to(next)
            };
        }
        match next {
            BotAvatarState::Idle => {
                // The head takes its time: it holds a place, then swings to the
                // next over about a second — no short quick jabs. The gaze
                // picks where, so these carry the motion and their amplitude is
                // unused.
                self.yaw_w.set(GAZE_YAW, 2.6, 5.4, 2.0);
                self.pitch_w.set(GAZE_PITCH, 2.8, 5.8, 1.8);
                self.roll_w.set(GAZE_ROLL, 3.4, 6.6, 1.5);
                self.gaze_at = 0.0;
                self.gaze_dir = [0.0, 0.0];
                // The eyes keep their quickness: they dart, the head follows.
                self.look_x_w.set(3.6, 0.6, 2.2, 13.0);
                self.look_y_w.set(2.4, 0.6, 2.2, 13.0);
                self.flip_at = self.next_flip(self.t, 0.6);
            }
            BotAvatarState::Working => {
                self.yaw_w.set(16.0 * DEG, 0.9, 1.8, 4.0);
                self.pitch_w.set(3.0 * DEG, 1.2, 2.4, 3.0);
                self.roll_w.set(0.0, 1.0, 2.0, 3.0);
                self.look_x_w.set(2.0, 0.5, 1.2, 12.0);
                self.look_y_w.set(1.0, 0.5, 1.2, 12.0);
                self.hop_phase = 0.0;
                self.hop_count = 0;
                self.laugh_at = self.t + 0.5 + self.rand.next() * 1.2;
            }
            BotAvatarState::Sleeping => {
                self.yaw_w.set(7.0 * DEG, 3.0, 6.0, 0.7);
                self.pitch_w.set(3.0 * DEG, 3.0, 6.0, 0.7);
                self.roll_w.set(2.0 * DEG, 3.0, 6.0, 0.6);
                self.look_x_w.set(0.0, 2.0, 4.0, 2.0);
                self.look_y_w.set(0.0, 2.0, 4.0, 2.0);
                self.nod_at = self.t + 2.5 + self.rand.next() * 4.0;
            }
        }
    }

    /// Where the pointer is, relative to the head (−1 … 1 across a head
    /// width), and how strongly to follow it (0 lets go).
    pub fn set_pointer(&mut self, x: f32, y: f32, strength: f32) {
        self.ptr_target_x = x.clamp(-1.2, 1.2);
        self.ptr_target_y = y.clamp(-1.2, 1.2);
        self.ptr_target_s = strength.clamp(0.0, 1.0);
    }

    /// A hop and a full turn, right now, whatever the state.
    pub fn poke(&mut self) {
        if self.flip.active() && self.flip.p < 0.6 {
            return;
        }
        self.flip_poked = true;
        self.flip.duration = flip_duration(self.jump, true);
        self.flip_side = if self.rand.next() < 0.5 { -1.0 } else { 1.0 };
        self.flip.fire();
        self.flip_at = self.next_flip(self.t, 1.1);
    }

    /// How far the head turns to the side while idle: 1 as the gaze has it, 0
    /// keeps it facing forward.
    pub fn set_turn(&mut self, turn: f32) {
        let next = turn.max(0.0);
        if next == self.turn_k {
            return;
        }
        self.turn_k = next;
        // Take the new reach at once rather than at the next place.
        if self.state == BotAvatarState::Idle {
            self.gaze_at = 0.0;
        }
    }

    /// The jump's numbers. A config with a different `every` restarts the
    /// idle-jump clock at once, so `every: 0` stops the next jump that was
    /// already due and a first value starts the clock.
    pub fn set_jump(&mut self, jump: JumpConfig) {
        let every = self.jump.every;
        self.jump = jump;
        if jump.every != every {
            self.flip_at = self.next_flip(self.t, 1.0);
        }
    }

    /// Where the head looks next. From a corner it mostly swings straight
    /// across to the opposite one — top right, stay, bottom left — now and then
    /// only sideways, or back to the middle for a beat.
    fn next_gaze(&mut self) -> (f32, f32) {
        let (px, py) = (self.gaze_dir[0], self.gaze_dir[1]);
        if px != 0.0 || py != 0.0 {
            let p = self.rand.next();
            if p < 0.66 {
                return (-px, -py);
            }
            if p < 0.85 {
                return (-px, py);
            }
            return (0.0, 0.0);
        }
        const CORNERS: [(f32, f32); 4] = [(1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, 1.0)];
        let i = ((self.rand.next() * CORNERS.len() as f32).floor() as usize).min(CORNERS.len() - 1);
        CORNERS[i]
    }

    /// When the next idle jump is due: `every` seconds, give or take 40 %.
    fn next_flip(&mut self, t: f32, k: f32) -> f32 {
        next_flip(&self.jump, &mut self.rand, t, k)
    }

    /// Advance by `dt` seconds (already scaled by the speed).
    pub fn update(&mut self, dt: f32) {
        let dt = dt.min(0.05);
        self.t += dt;
        let t = self.t;

        let mut pose = self.pose;
        let mut w = pose.w;

        // The state change eases from the weights it started with to the new
        // state's on an S-curve: soft start, soft finish, no creeping tail.
        if self.tr < 1.0 {
            self.tr = (self.tr + dt / self.tr_duration).min(1.0);
            // A sine ease: no kick at either end of a switch.
            let e = ease_sine(self.tr);
            let idx = state_index(self.state);
            for (i, (slot, from)) in w.iter_mut().zip(self.w_from).enumerate() {
                let target = if i == idx { 1.0 } else { 0.0 };
                *slot = from + (target - from) * e;
            }
        }
        let wd = w[0];
        let ww = w[1];
        let ws = w[2];

        // Rest targets, blended.
        let rest = blend_rest(w);

        // The idle gaze: a place to look, a while to stay, then the swing to
        // the next — the wanderers carry it on their own springs.
        if self.state == BotAvatarState::Idle && t >= self.gaze_at {
            let (gx, gy) = self.next_gaze();
            self.gaze_dir = [gx, gy];
            let reach = 0.84 + self.rand.next() * 0.16;
            self.yaw_w.aim(gx * GAZE_YAW * reach * self.turn_k);
            self.pitch_w.aim(gy * GAZE_PITCH * reach);
            // The tilt goes with the turn, so it follows it.
            self.roll_w.aim(gx * GAZE_ROLL * reach * self.turn_k);
            self.gaze_at = t + GAZE_HOLD_MIN + self.rand.next() * (GAZE_HOLD_MAX - GAZE_HOLD_MIN);
        }

        // Wander.
        self.yaw_w.update(t, dt, &mut self.rand);
        self.pitch_w.update(t, dt, &mut self.rand);
        self.roll_w.update(t, dt, &mut self.rand);
        self.look_x_w.update(t, dt, &mut self.rand);
        self.look_y_w.update(t, dt, &mut self.rand);

        // Following the pointer: the eyes lead, the head turns after them, and
        // the wander quietens while it lasts.
        self.ptr_s = approach(self.ptr_s, self.ptr_target_s, 8.0, dt);
        self.ptr_x = approach(self.ptr_x, self.ptr_target_x, 14.0, dt);
        self.ptr_y = approach(self.ptr_y, self.ptr_target_y, 14.0, dt);
        let ps = self.ptr_s;
        let quiet = 1.0 - 0.75 * ps;

        // The base pose: the blended rest plus the smoothed wander and the
        // pointer's pull — every term is already continuous.
        self.base_yaw = approach(
            self.base_yaw,
            self.yaw_w.value * quiet + 22.0 * DEG * self.ptr_x * ps,
            5.0,
            dt,
        );
        let base_pitch = rest.pitch + self.pitch_w.value * quiet - 12.0 * DEG * self.ptr_y * ps;
        let base_roll = rest.roll + self.roll_w.value * quiet;
        let base_y = rest.y;
        let base_look_x = rest.look_x + self.look_x_w.value * quiet + 4.5 * self.ptr_x * ps;
        let base_look_y = rest.look_y + self.look_y_w.value * quiet + 3.0 * self.ptr_y * ps;

        // ── events ──
        let mut spin = 0.0;
        let mut hop_y = 0.0;
        let mut sx = 1.0;
        let mut sy = 1.0;
        let mut pitch_add = 0.0;
        let mut roll_add = 0.0;
        let mut blink_close = 0.0;
        let mut look_x_add = 0.0;
        let mut look_y_add = 0.0;
        let mut laugh: f32 = 0.0;
        let mut whirl: f32 = 0.0;
        let mut whirl_angle = 0.0;

        // Blinks: idle and working blink; a double blink now and then.
        if t >= self.blink_at && !self.blink.active() && wd + ww > 0.5 {
            self.blink.fire();
            // `!this.blinkAgain && rand() < 0.22`, with the same short circuit
            // (and therefore the same random draws) as the original.
            self.blink_again = !self.blink_again && self.rand.next() < 0.22;
            self.blink_at = if self.blink_again {
                t + 0.28
            } else {
                t + 2.2 + self.rand.next() * 2.6
            };
        }
        self.blink.update(dt);
        if self.blink.active() {
            blink_close = (PI * self.blink.p).sin();
        }

        // Eye darts: a quick glance to the side and back, between the slower
        // looks — the eyes have a life of their own.
        if t >= self.dart_at && !self.dart.active() && wd + ww > 0.5 {
            self.dart.fire();
            self.dart_x = (self.rand.next() * 2.0 - 1.0) * 4.0;
            self.dart_y = (self.rand.next() * 2.0 - 1.0) * 2.0;
            self.dart.duration = 0.25 + self.rand.next() * 0.45;
            self.dart_at = t + 1.2 + self.rand.next() * 2.6;
        }
        self.dart.update(dt);
        if self.dart.active() {
            let q = self.dart.p;
            // Snap out, hold, snap back.
            let hold = if q < 0.15 {
                q / 0.15
            } else if q > 0.8 {
                (1.0 - q) / 0.2
            } else {
                1.0
            };
            look_x_add += self.dart_x * hold * (wd + ww);
            look_y_add += self.dart_y * hold * (wd + ww);
        }

        // Idle: a full turn now and then, with a jump.
        if self.state == BotAvatarState::Idle && t >= self.flip_at && !self.flip.active() {
            self.flip_poked = false;
            self.flip.duration = flip_duration(self.jump, false);
            self.flip_side = if self.rand.next() < 0.5 { -1.0 } else { 1.0 };
            self.flip.fire();
            self.flip_at = self.next_flip(t, 1.0);
        }
        self.flip.update(dt);
        if self.flip.active() {
            let j = self.jump;
            // The hop's own phase: below 0 the crouch, above 1 the recovery.
            let pre_s = flip_pre(j, self.flip_poked);
            let a = (self.flip.p * self.flip.duration - pre_s) / j.time;
            let q = a.clamp(0.0, 1.0);
            let arc = (PI * q).sin();
            // Whole turns, eased in and out, no overshoot to snap back from.
            spin += TAU * j.spin * ease_in_out(q);
            hop_y -= j.height * arc;
            // The crouch before take-off; then the landing squash from `land`
            // seconds round the moment of contact — before it, the body braces
            // for the ground; after it, it gives late.
            let tl = (a - 1.0) * j.time - j.land;
            let squash_time = if self.flip_poked {
                j.click_squash_time
            } else {
                j.squash_time
            };
            // The crouch: a click's builds over its squash time, an idle
            // jump's is the hop's short bump; then the hop, then the landing
            // squash.
            let poked = self.flip_poked;
            let crouch = |u: f32| {
                if poked {
                    u * u * (3.0 - 2.0 * u)
                } else {
                    hop_squash(u - 1.0)
                }
            };
            // The landing in three parts: the press down to the deepest squash
            // in the squash easing's own shape, the hold on the ground, then
            // the rise back to the body's shape — each with its own time and
            // easing, and all three meeting at the same depth, so nothing steps.
            let hold = j.ground_time.max(0.0);
            let peak_t = squash_peak(j.squash_ease) * squash_time;
            let rise = tl - peak_t - hold;
            let depth = if tl <= peak_t {
                squash_pulse(tl / squash_time, j.squash_ease)
            } else if rise <= 0.0 {
                ground_shape((tl - peak_t) / hold, j.ground_ease)
            } else {
                rise_pulse(rise / j.rise_time, j.rise_ease)
            };
            let land = (if a < 0.0 {
                crouch((1.0 + (a * j.time) / pre_s).max(0.0))
            } else if tl > 0.0 {
                depth
            } else if a < 0.2 {
                hop_squash(a)
            } else {
                0.0
            }) * j.squash;
            sx += 0.16 * land - 0.06 * arc * j.stretch;
            sy += -0.18 * land + 0.09 * arc * j.stretch;
            // Lean into it; eyes shut for a turn.
            roll_add += self.flip_side * j.lean * DEG * arc;
            if j.spin > 0.0 {
                laugh = laugh.max(arc);
                whirl = whirl.max(whirl_envelope(q));
                whirl_angle = whirl_ring_angle(q);
            }
        }

        // Working: hops all the time; every third one spins. Leaving the state,
        // the hop under way keeps the strength it had — it flies its whole arc,
        // finishes its turn and lands — and that last landing is not the
        // rhythm's quick bump but a jump's proper one: the hold on the ground
        // and the rise back, from the depth the hop reached.
        let j = self.jump;
        let exit_hold = j.ground_time.max(0.0);
        let exit_for = exit_hold + j.rise_time;
        if self.state == BotAvatarState::Working {
            self.hop_gain = ww;
        }
        if self.hop_gain > 0.02 && (self.state == BotAvatarState::Working || self.hop_phase > 0.0) {
            self.hop_phase += dt / HOP_T;
            if self.hop_phase >= 1.0 {
                if self.state == BotAvatarState::Working {
                    self.hop_phase -= 1.0;
                    self.hop_count += 1;
                } else if (self.hop_phase - 1.0) * HOP_T >= exit_for {
                    // The last hop has landed and settled.
                    self.hop_phase = 0.0;
                    self.hop_gain = 0.0;
                }
            }
            let g = self.hop_gain;
            // Past 1 the body is down and only the landing is still playing.
            let q = self.hop_phase.min(1.0);
            let arc = (PI * q).sin();
            let spinning = self.hop_count % 3 == 2;
            let h = if spinning { HOP_SPIN_H } else { 18.0 };
            hop_y -= h * arc * g;
            // Squash on landing, stretch at the top.
            let exit_t = if self.state != BotAvatarState::Working && self.hop_phase > 1.0 {
                (self.hop_phase - 1.0) * HOP_T
            } else {
                -1.0
            };
            let land = if exit_t < 0.0 {
                hop_squash(self.hop_phase)
            } else if exit_t < exit_hold {
                ground_shape(exit_t / exit_hold, j.ground_ease)
            } else {
                rise_pulse((exit_t - exit_hold) / j.rise_time, j.rise_ease)
            };
            sx += (0.16 * land - 0.06 * arc) * g;
            sy += (-0.18 * land + 0.09 * arc) * g;
            if spinning {
                spin += TAU * ease_in_out(q) * g;
                // Eyes shut for the spin.
                laugh = laugh.max(arc * g);
                let e = whirl_envelope(q) * g;
                if e > whirl {
                    whirl = e;
                    whirl_angle = whirl_ring_angle(q);
                }
            }
            // Lean into each hop, alternating sides.
            roll_add += (if self.hop_count.is_multiple_of(2) {
                1.0
            } else {
                -1.0
            }) * 6.0
                * DEG
                * arc
                * g;
        }

        // Working: now and then a laugh shuts the eyes into arcs, then they
        // open again.
        if self.state == BotAvatarState::Working && t >= self.laugh_at && !self.laugh_ev.active() {
            self.laugh_ev.fire();
            self.laugh_ev.duration = 0.6 + self.rand.next() * 0.5;
            self.laugh_at = t + 1.6 + self.rand.next() * 2.2;
        }
        self.laugh_ev.update(dt);
        if self.laugh_ev.active() {
            let q = self.laugh_ev.p;
            // Quick shut, hold, quick open.
            laugh = laugh.max(if q < 0.18 {
                q / 0.18
            } else if q > 0.78 {
                (1.0 - q) / 0.22
            } else {
                1.0
            });
        }

        // Sleeping: the head drops, then jerks back up.
        if self.state == BotAvatarState::Sleeping && t >= self.nod_at && !self.nod.active() {
            self.nod.fire();
            self.nod_at = t + 4.0 + self.rand.next() * 4.0;
        }
        self.nod.update(dt);
        if self.nod.active() {
            let q = self.nod.p;
            // Slow slide down, quick recovery.
            let dip = if q < 0.72 {
                ease_in_out(q / 0.72)
            } else {
                1.0 - ease_in_out((q - 0.72) / 0.28)
            };
            pitch_add -= 13.0 * DEG * dip * ws;
        }

        // Breathing, always, deeper and slower asleep. The phase is integrated
        // so the period can change with the state without the cycle jumping.
        self.breath_phase += dt / (3.6 + 1.2 * ws);
        let breath = (self.breath_phase * TAU).sin();
        pose.breath = breath;
        sx += breath * (0.008 + 0.014 * ws);
        sy += breath * (0.012 + 0.02 * ws);
        let bob = (t * TAU / 3.4).sin() * 2.0 * (1.0 - ws);

        // ── compose ──
        pose.yaw = self.base_yaw + spin;

        // The jelly: the faster the head turns, the more the body stretches
        // along the turn, on a spring that overshoots and settles. A flip's spin
        // is left out: that is a jump, and the landing spring handles it.
        let mut dyaw = self.base_yaw - self.prev_yaw;
        dyaw = ((dyaw + PI) % TAU + TAU) % TAU - PI;
        self.prev_yaw = self.base_yaw;
        let rate = if dt > 0.0 { dyaw.abs() / dt } else { 0.0 };
        // The eyes lead a head turn, the way a gaze shift goes: they run ahead
        // of it and come back as the head settles.
        let lead_target = if dt > 0.0 {
            ((dyaw / dt) * 2.4).clamp(-2.2, 2.2)
        } else {
            0.0
        };
        self.gaze_lead = approach(self.gaze_lead, lead_target, 9.0, dt);
        let jelly_target = (0.055 * rate).min(0.22);
        let omega = 16.0;
        let zeta = 0.45;
        self.jelly_v +=
            (omega * omega * (jelly_target - self.jelly) - 2.0 * zeta * omega * self.jelly_v) * dt;
        self.jelly += self.jelly_v * dt;
        let jelly = self.jelly.clamp(-0.08, 0.28) * 0.6;
        sx *= 1.0 + jelly;
        sy *= 1.0 - 0.55 * jelly;

        pose.pitch = base_pitch + pitch_add;
        pose.roll = base_roll + roll_add;
        pose.x = 0.0;
        pose.y = base_y + hop_y + bob;
        pose.sx = sx;
        pose.sy = sy;
        // The lids: the sleeping state closes them through its own weight in
        // the renderer, so here the eyes stay open apart from blinks.
        pose.eye_open = 1.0;
        pose.laugh = approach(pose.laugh, laugh, 30.0, dt);
        pose.blink_l = blink_close;
        pose.blink_r = blink_close;
        pose.look_x = base_look_x + look_x_add + self.gaze_lead;
        pose.look_y = base_look_y + look_y_add;
        pose.whirl = whirl;
        pose.whirl_angle = whirl_angle;
        pose.w = w;

        self.pose = pose;
    }

    /// The seed this simulation was created with.
    pub fn seed(&self) -> f32 {
        self.seed
    }
}

/// The still pose of a state, for reduced motion and the first paint.
pub fn rest_pose(state: BotAvatarState) -> Pose {
    let r = rest_for(state);
    let w = match state {
        BotAvatarState::Idle => [1.0, 0.0, 0.0],
        BotAvatarState::Working => [0.0, 1.0, 0.0],
        BotAvatarState::Sleeping => [0.0, 0.0, 1.0],
    };
    Pose {
        yaw: 0.0,
        pitch: r.pitch,
        roll: r.roll,
        x: 0.0,
        y: r.y,
        sx: 1.0,
        sy: 1.0,
        eye_open: 1.0,
        blink_l: 0.0,
        blink_r: 0.0,
        look_x: r.look_x,
        look_y: r.look_y,
        breath: 0.0,
        laugh: 0.0,
        whirl: 0.0,
        whirl_angle: 0.0,
        w,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-4;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= EPS,
            "expected {expected}, got {actual}"
        );
    }

    fn assert_pose_finite(pose: &Pose) {
        let fields = [
            ("yaw", pose.yaw),
            ("pitch", pose.pitch),
            ("roll", pose.roll),
            ("x", pose.x),
            ("y", pose.y),
            ("sx", pose.sx),
            ("sy", pose.sy),
            ("eye_open", pose.eye_open),
            ("blink_l", pose.blink_l),
            ("blink_r", pose.blink_r),
            ("look_x", pose.look_x),
            ("look_y", pose.look_y),
            ("breath", pose.breath),
            ("laugh", pose.laugh),
            ("whirl", pose.whirl),
            ("whirl_angle", pose.whirl_angle),
            ("w0", pose.w[0]),
            ("w1", pose.w[1]),
            ("w2", pose.w[2]),
        ];
        for (name, value) in fields {
            assert!(value.is_finite(), "{name} is not finite: {value}");
        }
    }

    fn run(sim: &mut BotAvatarSim, frames: usize) {
        for _ in 0..frames {
            sim.update(1.0 / 60.0);
        }
    }

    #[test]
    #[allow(clippy::excessive_precision)] // pinned reference values
    fn same_seed_replays_identical_sequence() {
        let mut a = BotAvatarSim::new(0.42, BotAvatarState::Idle);
        let mut b = BotAvatarSim::new(0.42, BotAvatarState::Idle);
        run(&mut a, 120);
        run(&mut b, 120);
        assert_eq!(a.pose(), b.pose());
        // Pinned from the reference run: 120 frames of 1/60 s at seed 0.42.
        let pose = a.pose();
        assert_close(pose.yaw, 0.547_167_1);
        assert_close(pose.pitch, -0.219_765_0);
        assert_close(pose.roll, 0.049_487_16);
        assert_close(pose.x, 0.0);
        assert_close(pose.y, -1.689_349_5);
        assert_close(pose.sx, 1.008_654_1);
        assert_close(pose.sy, 1.011_342_5);
        assert_close(pose.look_x, -2.488_191_4);
        assert_close(pose.look_y, -1.344_746_9);
        assert_close(pose.breath, 0.981_957_8);
        assert_eq!(pose.w, [1.0, 0.0, 0.0]);
    }

    #[test]
    fn working_hop_sequence_is_pinned() {
        let mut sim = BotAvatarSim::new(0.42, BotAvatarState::Working);
        run(&mut sim, 120);
        let pose = sim.pose();
        assert_close(pose.yaw, 6.413_044_5);
        assert_close(pose.pitch, 0.070_261_93);
        assert_close(pose.roll, 0.019_242_38);
        assert_close(pose.y, -6.466_880_3);
        assert_close(pose.sx, 1.127_124_7);
        assert_close(pose.sy, 0.888_653_7);
        assert_close(pose.look_x, -0.367_752_6);
        assert_close(pose.look_y, -0.875_662_8);
        assert_close(pose.breath, 0.981_957_8);
        assert_close(pose.laugh, 0.294_315_9);
        assert_eq!(pose.w, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = BotAvatarSim::new(0.1, BotAvatarState::Idle);
        let mut b = BotAvatarSim::new(0.9, BotAvatarState::Idle);
        run(&mut a, 120);
        run(&mut b, 120);
        assert!((a.pose().yaw - b.pose().yaw).abs() > EPS);
    }

    #[test]
    fn update_clamps_large_steps_to_fifty_milliseconds() {
        let mut a = BotAvatarSim::new(0.5, BotAvatarState::Idle);
        let mut b = BotAvatarSim::new(0.5, BotAvatarState::Idle);
        a.update(1.0);
        b.update(0.05);
        assert_eq!(a.pose(), b.pose());
        assert_eq!(a.t, b.t);
    }

    #[test]
    fn state_switch_weights_stay_normalised() {
        let mut sim = BotAvatarSim::new(0.3, BotAvatarState::Idle);
        run(&mut sim, 30);
        sim.set_state(BotAvatarState::Working);
        for _ in 0..120 {
            sim.update(1.0 / 60.0);
            let sum: f32 = sim.pose().w.iter().sum();
            assert!((sum - 1.0).abs() < 1e-5, "weights sum to {sum}");
            for value in sim.pose().w {
                assert!((0.0..=1.0).contains(&value), "weight out of range: {value}");
            }
        }
        assert_close(sim.pose().w[1], 1.0);
        assert_close(sim.pose().w[0], 0.0);
    }

    #[test]
    fn rest_pose_carries_the_state_targets() {
        let idle = rest_pose(BotAvatarState::Idle);
        assert_eq!(idle.w, [1.0, 0.0, 0.0]);
        assert_eq!(idle.pitch, 0.0);
        assert_eq!(idle.roll, 0.0);
        assert_eq!(idle.eye_open, 1.0);

        let working = rest_pose(BotAvatarState::Working);
        assert_eq!(working.w, [0.0, 1.0, 0.0]);
        assert_close(working.pitch, 5.0 * DEG);

        let sleeping = rest_pose(BotAvatarState::Sleeping);
        assert_eq!(sleeping.w, [0.0, 0.0, 1.0]);
        assert_close(sleeping.pitch, -16.0 * DEG);
        assert_close(sleeping.roll, 6.0 * DEG);
        assert_close(sleeping.y, 3.0);
        assert_close(sleeping.look_y, 1.0);
    }

    #[test]
    fn poke_starts_a_spinning_flip() {
        let mut sim = BotAvatarSim::new(0.2, BotAvatarState::Idle);
        run(&mut sim, 10);
        let yaw_before = sim.pose().yaw;
        sim.poke();
        let mut max_whirl: f32 = 0.0;
        let mut max_yaw_shift: f32 = 0.0;
        for _ in 0..40 {
            sim.update(1.0 / 60.0);
            max_whirl = max_whirl.max(sim.pose().whirl);
            max_yaw_shift = max_yaw_shift.max((sim.pose().yaw - yaw_before).abs());
        }
        assert!(max_whirl > 0.5, "whirl peaked at {max_whirl}");
        assert!(max_yaw_shift > 1.0, "yaw shifted by {max_yaw_shift}");
        assert!(sim.pose().laugh > 0.0);
    }

    #[test]
    fn six_hundred_frames_of_every_state_stay_finite() {
        for state in [
            BotAvatarState::Idle,
            BotAvatarState::Working,
            BotAvatarState::Sleeping,
        ] {
            let mut sim = BotAvatarSim::new(0.77, state);
            sim.set_pointer(0.6, -0.4, 0.8);
            sim.set_turn(1.3);
            sim.set_jump(JumpConfig {
                every: 2.0,
                ..JUMP_DEFAULTS
            });
            for _ in 0..600 {
                sim.update(1.0 / 60.0);
                assert_pose_finite(sim.pose());
            }
            assert_eq!(sim.state(), state);
        }
    }

    #[test]
    fn sleeping_settles_with_open_eyes_and_closed_lids() {
        let mut sim = BotAvatarSim::new(0.31, BotAvatarState::Idle);
        run(&mut sim, 30);
        sim.set_state(BotAvatarState::Sleeping);
        run(&mut sim, 200);
        assert_close(sim.pose().w[2], 1.0);
        assert_close(sim.pose().w[0], 0.0);
        assert_eq!(sim.pose().eye_open, 1.0);
        assert_eq!(sim.pose().blink_l, 0.0);
        assert_eq!(sim.pose().blink_r, 0.0);
        assert!(sim.pose().pitch < 0.0);
    }

    #[test]
    fn immediate_state_change_snaps_and_reports_seed() {
        let mut sim = BotAvatarSim::new(0.6, BotAvatarState::Idle);
        assert_eq!(sim.seed(), 0.6);
        sim.set_state_immediate(BotAvatarState::Sleeping);
        assert_eq!(sim.state(), BotAvatarState::Sleeping);
        assert_eq!(sim.pose().w, [0.0, 0.0, 1.0]);
        // Setting the state it is already in is a no-op.
        sim.set_state(BotAvatarState::Sleeping);
        assert_eq!(sim.pose().w, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn squash_curves_are_bounded_and_end_at_rest() {
        let eases = [
            BotAvatarSquashEase::Sharp,
            BotAvatarSquashEase::Pulse,
            BotAvatarSquashEase::Soft,
            BotAvatarSquashEase::Bouncy,
        ];
        for ease in eases {
            assert_eq!(squash_pulse(0.0, ease), 0.0);
            assert_eq!(squash_pulse(1.0, ease), 0.0);
            assert_eq!(rise_pulse(0.0, ease), 1.0);
            assert_eq!(rise_pulse(1.0, ease), 0.0);
            for i in 0..=100 {
                let u = i as f32 / 100.0;
                assert!(squash_pulse(u, ease).is_finite());
                assert!(rise_pulse(u, ease).is_finite());
                assert!(ground_shape(u, ease).is_finite());
            }
        }
    }
}
