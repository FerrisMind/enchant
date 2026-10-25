//! Gravity: the orb pulls the pointer in.
//!
//! As the pointer nears an orb, its body bends toward it and the side facing
//! the orb streaks toward the orb's centre, like something caught in a well.
//! The pointer's tip stays exactly where the OS has it; it is never rotated or
//! scaled as a whole.
//!
//! A toolkit cannot reshape the OS pointer, so the effect hides it (via
//! [`mouse::Interaction::Hidden`](iced::mouse::Interaction::Hidden)) and draws a
//! raster of the platform's real pointer in its place, warped by the well. That
//! raster has to match the real pointer or the swap shows, so the host supplies
//! it with [`set_sprite`]; without one the effect stays off.
//!
//! The state lives in one process-wide runtime, because the swap is a global
//! act: with several orbs on screen only the nearest one pulls, and the pull
//! hands over smoothly between them.
//!
//! # Fail-safes
//!
//! * Off per orb under [`MotionPreference::Reduced`](super::MotionPreference).
//! * The OS pointer is hidden only while the pointer is in a well, and given
//!   back the moment it leaves, the window loses it, or a key is pressed.
//! * A frame-time watchdog disables the effect if it ever becomes expensive.
//!   [`reset`] re-arms it.
//! * Widgets with a cursor of their own (a button's hand) keep it: iced picks
//!   the strongest interaction, and `Hidden` is the weakest. The sprite is
//!   still painted over them, so keep orbs and controls apart or lower `reach`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use iced::widget::image;
use iced::{Point, Rectangle, Size};

use super::raster::{
    Bend, Mask, Region, bend_sprite, blit, gaussian_blur, premultiply, shifted_masked, under,
    unpremultiply,
};
use super::sprite::CursorSprite;

/// Room around the sprite for the tail and the bend, logical px.
const PAD: f64 = 48.0;
/// Ghost passes per frame: more is smoother.
const PASSES: usize = 10;
/// After the pointer leaves every orb's reach the swap is kept this long, the
/// sprite drawn plain at the pointer, so crossing between orbs never gives the
/// OS pointer back and takes it again.
const LINGER_S: f64 = 0.5;
/// Frames slower than this many seconds count against the watchdog.
const SLOW_FRAME_S: f64 = 0.012;
/// Consecutive slow frames before the effect disables itself.
const SLOW_FRAME_LIMIT: u32 = 30;

/// Tuning for the pull. All values are clamped to their documented range.
///
/// # Examples
///
/// ```
/// use enchant_iced::Gravity;
///
/// let gravity = Gravity::new().reach(220.0).strength(14.0).blur(0.0);
/// assert_eq!(gravity.get_reach(), 220.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gravity {
    reach: f32,
    strength: f32,
    deform: f32,
    taper: f32,
    curve: f32,
    falloff: f32,
    smoothing: f32,
    handover: f32,
    squash: f32,
    blur: f32,
    fade_ms: f32,
}

impl Default for Gravity {
    fn default() -> Self {
        Self {
            reach: 160.0,
            strength: 11.0,
            deform: 19.0,
            taper: 1.95,
            curve: 2.8,
            falloff: 24.0,
            smoothing: 0.3,
            handover: 0.6,
            squash: 1.2,
            blur: 1.0,
            fade_ms: 180.0,
        }
    }
}

impl Gravity {
    /// The reference library's tuning.
    pub fn new() -> Self {
        Self::default()
    }

    /// How far outside the orb's edge the pull is felt, logical px (min 1).
    pub fn reach(mut self, reach: f32) -> Self {
        self.reach = finite_or(reach, self.reach).max(1.0);
        self
    }

    /// How far the pointer trails toward the orb on contact, logical px
    /// (0..=64). The pointer itself never moves; this is its tail's length.
    pub fn strength(mut self, strength: f32) -> Self {
        self.strength = finite_or(strength, self.strength).clamp(0.0, 64.0);
        self
    }

    /// How far the pointer's body bends toward the orb on contact, logical px
    /// (0..=32). The tip stays pinned; the rest leans in behind it.
    pub fn deform(mut self, deform: f32) -> Self {
        self.deform = finite_or(deform, self.deform).clamp(0.0, 32.0);
        self
    }

    /// How the bend spreads along the body (1..=4): 1 bends evenly from the
    /// tip outward, higher keeps the tip firm and bends the far end hardest.
    pub fn taper(mut self, taper: f32) -> Self {
        self.taper = finite_or(taper, self.taper).clamp(1.0, 4.0);
        self
    }

    /// How late the pull builds (1..=4): proximity is raised to this power.
    pub fn curve(mut self, curve: f32) -> Self {
        self.curve = finite_or(curve, self.curve).clamp(1.0, 4.0);
        self
    }

    /// Width of the band across the pointer, logical px (2..=200), over which
    /// the tail fades from the side facing the orb to the side away from it.
    pub fn falloff(mut self, falloff: f32) -> Self {
        self.falloff = finite_or(falloff, self.falloff).clamp(2.0, 200.0);
        self
    }

    /// Inertia (0..=1): how much the deformation lags the pointer.
    pub fn smoothing(mut self, smoothing: f32) -> Self {
        self.smoothing = finite_or(smoothing, self.smoothing).clamp(0.0, 1.0);
        self
    }

    /// Handover (0..=1): how slowly the pull swings from one orb to the next.
    pub fn handover(mut self, handover: f32) -> Self {
        self.handover = finite_or(handover, self.handover).clamp(0.0, 1.0);
        self
    }

    /// Squash (0..=3): extra bend and tail when the orb lies off the tip's
    /// side, where the pull shortens the body instead of stretching it.
    pub fn squash(mut self, squash: f32) -> Self {
        self.squash = finite_or(squash, self.squash).clamp(0.0, 3.0);
        self
    }

    /// Progressive blur on the tail, logical px (0..=24): none at the pointer,
    /// this much at the tail's far end.
    pub fn blur(mut self, blur: f32) -> Self {
        self.blur = finite_or(blur, self.blur).clamp(0.0, 24.0);
        self
    }

    /// Envelope on enter/leave, milliseconds (~95% settled; min 1).
    pub fn fade_ms(mut self, fade_ms: f32) -> Self {
        self.fade_ms = finite_or(fade_ms, self.fade_ms).max(1.0);
        self
    }

    /// See [`Self::reach`].
    pub fn get_reach(&self) -> f32 {
        self.reach
    }
    /// See [`Self::strength`].
    pub fn get_strength(&self) -> f32 {
        self.strength
    }
    /// See [`Self::deform`].
    pub fn get_deform(&self) -> f32 {
        self.deform
    }
    /// See [`Self::taper`].
    pub fn get_taper(&self) -> f32 {
        self.taper
    }
    /// See [`Self::curve`].
    pub fn get_curve(&self) -> f32 {
        self.curve
    }
    /// See [`Self::falloff`].
    pub fn get_falloff(&self) -> f32 {
        self.falloff
    }
    /// See [`Self::smoothing`].
    pub fn get_smoothing(&self) -> f32 {
        self.smoothing
    }
    /// See [`Self::handover`].
    pub fn get_handover(&self) -> f32 {
        self.handover
    }
    /// See [`Self::squash`].
    pub fn get_squash(&self) -> f32 {
        self.squash
    }
    /// See [`Self::blur`].
    pub fn get_blur(&self) -> f32 {
        self.blur
    }
    /// See [`Self::fade_ms`].
    pub fn get_fade_ms(&self) -> f32 {
        self.fade_ms
    }

    fn with_tuning(mut self, t: &GravityTuning) -> Self {
        macro_rules! lay {
            ($($f:ident),*) => {$(
                if let Some(v) = t.$f { self = self.$f(v); }
            )*};
        }
        lay!(
            reach, strength, deform, taper, curve, falloff, smoothing, handover, squash, blur,
            fade_ms
        );
        self
    }
}

fn finite_or(v: f32, fallback: f32) -> f32 {
    if v.is_finite() { v } else { fallback }
}

/// A live override for tuning: while installed with [`set_tuning`], every orb
/// uses these values in place of its own. Dev panels write it; ship code
/// leaves it unset.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GravityTuning {
    reach: Option<f32>,
    strength: Option<f32>,
    deform: Option<f32>,
    taper: Option<f32>,
    curve: Option<f32>,
    falloff: Option<f32>,
    smoothing: Option<f32>,
    handover: Option<f32>,
    squash: Option<f32>,
    blur: Option<f32>,
    fade_ms: Option<f32>,
}

macro_rules! tuning_setters {
    ($($f:ident),*) => {
        impl GravityTuning {
            $(
                #[doc = concat!("Overrides `", stringify!($f), "` (clamped like [`Gravity::", stringify!($f), "`]).")]
                pub fn $f(mut self, value: f32) -> Self {
                    self.$f = Some(value);
                    self
                }
            )*

            fn merged(mut self, patch: &GravityTuning) -> Self {
                $( if patch.$f.is_some() { self.$f = patch.$f; } )*
                self
            }
        }
    };
}
tuning_setters!(
    reach, strength, deform, taper, curve, falloff, smoothing, handover, squash, blur, fade_ms
);

impl GravityTuning {
    /// An empty override.
    pub fn new() -> Self {
        Self::default()
    }
}

/// What the runtime is doing, for dev tooling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GravityStatus {
    /// Orbs currently registered with gravity on.
    pub orbs: usize,
    /// Whether the sprite is on screen right now.
    pub active: bool,
    /// Why the effect cannot run, or `None` when it can.
    pub blocked_by: Option<String>,
}

// ─── Public runtime controls ───────────────────────────────────────────

/// Supplies the pointer raster, or `None` to turn the effect off. It must
/// match the OS pointer pixel for pixel, or the swap is visible. Re-arms the
/// fail-safes.
pub fn set_sprite(sprite: Option<CursorSprite>) {
    let mut rt = runtime();
    if let (Some(a), Some(b)) = (&rt.sprite, &sprite)
        && a.same_as(b)
    {
        return;
    }
    rt.sprite = sprite;
    rt.disabled = None;
    rt.slow_frames = 0;
    rt.hide_cursor();
}

/// Clears the fail-safe off-switch (a dev panel's "re-arm").
pub fn reset() {
    let mut rt = runtime();
    rt.disabled = None;
    rt.slow_frames = 0;
}

/// Installs (or, with `None`, removes) a live tuning override.
pub fn set_tuning(patch: Option<GravityTuning>) {
    let mut rt = runtime();
    rt.tuning = patch.map(|p| rt.tuning.unwrap_or_default().merged(&p));
}

/// The tuning every orb currently falls back to: the defaults with any live
/// override laid over them.
pub fn tuning() -> Gravity {
    let rt = runtime();
    match &rt.tuning {
        Some(t) => Gravity::default().with_tuning(t),
        None => Gravity::default(),
    }
}

/// Whether the effect can run, and why not if it cannot.
pub fn status() -> GravityStatus {
    let rt = runtime();
    GravityStatus {
        orbs: rt.orbs.len(),
        active: rt.shown,
        blocked_by: if rt.orbs.is_empty() {
            Some("no orb has gravity on".to_owned())
        } else {
            rt.blocked_by()
        },
    }
}

// ─── Runtime ───────────────────────────────────────────────────────────

/// A registered orb: where it is and how it pulls.
struct Orb {
    id: u64,
    rect: Rectangle,
    opts: Gravity,
}

/// One finished frame of the bent pointer, ready to draw.
#[derive(Clone)]
pub(crate) struct SpriteFrame {
    pub(crate) handle: image::Handle,
    pub(crate) bounds: Rectangle,
}

struct Runtime {
    sprite: Option<CursorSprite>,
    /// Sticky off-switch flipped by the fail-safes, with why.
    disabled: Option<String>,
    slow_frames: u32,
    tuning: Option<GravityTuning>,
    orbs: Vec<Orb>,

    pointer: Option<Point>,
    last_pointer: Point,
    typing: bool,

    last_now: f64,
    u_s: f64,
    near: Option<u64>,
    last_reach: Option<f64>,
    hiding: bool,
    shown: bool,
    frame: Option<SpriteFrame>,
    centre: Point,
    amp: f64,
    bend: f64,
    well: Option<Point>,
    wants_frames: bool,
}

static RUNTIME: Mutex<Runtime> = Mutex::new(Runtime::new());
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn runtime() -> MutexGuard<'static, Runtime> {
    RUNTIME.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A fresh orb id for registration.
pub(crate) fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// Registers (or updates) an orb's rectangle and options.
pub(crate) fn register(id: u64, rect: Rectangle, opts: Gravity) {
    let mut rt = runtime();
    match rt.orbs.iter_mut().find(|o| o.id == id) {
        Some(orb) => {
            orb.rect = rect;
            orb.opts = opts;
        }
        None => rt.orbs.push(Orb { id, rect, opts }),
    }
}

/// Removes an orb; tears everything down once the last one is gone.
pub(crate) fn unregister(id: u64) {
    let mut rt = runtime();
    rt.orbs.retain(|o| o.id != id);
    if rt.near == Some(id) {
        rt.near = None;
    }
    if rt.orbs.is_empty() {
        rt.u_s = 0.0;
        rt.hide_cursor();
    }
}

/// A key press: keeps the sprite off while someone types.
pub(crate) fn key_pressed() {
    let mut rt = runtime();
    rt.typing = true;
    rt.hide_cursor();
}

/// A pointer move: ends the typing state.
pub(crate) fn pointer_moved() {
    runtime().typing = false;
}

/// Advances the runtime to `now` (seconds) once per distinct frame.
pub(crate) fn tick(now: f64, pointer: Option<Point>) {
    let mut rt = runtime();
    if let Some(p) = pointer {
        rt.last_pointer = p;
    }
    rt.pointer = pointer;
    if now <= rt.last_now {
        return;
    }
    rt.step(now, std::time::Instant::now);
}

/// Whether the OS pointer should be hidden right now.
pub(crate) fn hides_pointer() -> bool {
    runtime().hiding
}

/// Whether the runtime needs frames even if no orb is animating.
pub(crate) fn wants_frames() -> bool {
    runtime().wants_frames
}

/// The sprite frame to draw if orb `id` owns the pull.
pub(crate) fn frame_for(id: u64) -> Option<SpriteFrame> {
    let rt = runtime();
    if rt.near == Some(id) && rt.shown {
        rt.frame.clone()
    } else {
        None
    }
}

impl Runtime {
    const fn new() -> Self {
        Self {
            sprite: None,
            disabled: None,
            slow_frames: 0,
            tuning: None,
            orbs: Vec::new(),
            pointer: None,
            last_pointer: Point::ORIGIN,
            typing: false,
            last_now: f64::NEG_INFINITY,
            u_s: 0.0,
            near: None,
            last_reach: None,
            hiding: false,
            shown: false,
            frame: None,
            centre: Point::ORIGIN,
            amp: 0.0,
            bend: 0.0,
            well: None,
            wants_frames: false,
        }
    }

    fn blocked_by(&self) -> Option<String> {
        if let Some(reason) = &self.disabled {
            return Some(reason.clone());
        }
        if self.sprite.is_none() {
            return Some("no pointer sprite set".to_owned());
        }
        None
    }

    fn effective(&self, opts: Gravity) -> Gravity {
        match &self.tuning {
            Some(t) => opts.with_tuning(t),
            None => opts,
        }
    }

    fn hide_sprite(&mut self) {
        self.shown = false;
        self.frame = None;
    }

    /// Gives the OS pointer back and drops the sprite and its smoothing.
    fn hide_cursor(&mut self) {
        self.hiding = false;
        self.hide_sprite();
        self.amp = 0.0;
        self.bend = 0.0;
        self.well = None;
    }

    fn step(&mut self, now: f64, clock: impl Fn() -> std::time::Instant) {
        let t0 = clock();
        let dt = (now - self.last_now).clamp(0.001, 0.05);
        let first = !self.last_now.is_finite();
        self.last_now = now;
        self.step_inner(now, if first { 0.016 } else { dt });
        let took = clock().duration_since(t0).as_secs_f64();
        if took > SLOW_FRAME_S {
            self.slow_frames += 1;
            if self.slow_frames >= SLOW_FRAME_LIMIT && self.disabled.is_none() {
                self.disabled = Some(format!(
                    "disabled after slow frames (~{}ms each)",
                    (took * 1000.0).round()
                ));
                self.hide_cursor();
            }
        } else {
            self.slow_frames = 0;
        }
    }

    fn step_inner(&mut self, now: f64, dt: f64) {
        self.wants_frames = false;

        // Nearest orb within reach. The orb is the disc inscribed in its rect.
        let mut best: Option<u64> = None;
        let mut u = 0.0;
        if let Some(p) = self.pointer
            && self.blocked_by().is_none()
        {
            let (px, py) = (f64::from(p.x), f64::from(p.y));
            let mut best_edge = f64::INFINITY;
            let mut best_reach = 1.0;
            for orb in &self.orbs {
                let r = orb.rect;
                if r.width <= 0.0 {
                    continue;
                }
                let cx = f64::from(r.x + r.width / 2.0);
                let cy = f64::from(r.y + r.height / 2.0);
                let orb_r = f64::from(r.width.min(r.height)) / 2.0;
                let reach = f64::from(self.effective(orb.opts).reach);
                if px < cx - orb_r - reach
                    || px > cx + orb_r + reach
                    || py < cy - orb_r - reach
                    || py > cy + orb_r + reach
                {
                    continue;
                }
                let edge = (px - cx).hypot(py - cy) - orb_r;
                if edge <= reach && edge < best_edge {
                    best_edge = edge;
                    best = Some(orb.id);
                    best_reach = reach;
                    self.centre = Point::new(cx as f32, cy as f32);
                }
            }
            if best.is_some() {
                u = proximity(best_edge, best_reach);
                self.last_reach = Some(now);
            }
        }

        // Envelope on the proximity weight: no pops on enter/leave/jump.
        let fade = self
            .near
            .or(best)
            .and_then(|id| self.orbs.iter().find(|o| o.id == id))
            .map_or(Gravity::default().fade_ms, |o| {
                self.effective(o.opts).fade_ms
            });
        let a = 1.0 - (-(dt * 1000.0) / (f64::from(fade) / 3.0)).exp();
        self.u_s += (u - self.u_s) * a;

        if best.is_some() && best != self.near {
            self.near = best;
        }
        if best.is_none() && self.u_s < 0.002 {
            self.u_s = 0.0;
            let lingering = self.last_reach.is_some_and(|reach| now - reach < LINGER_S);
            if self.near.is_some()
                && self.hiding
                && !self.typing
                && self.pointer.is_some()
                && lingering
            {
                // Plain sprite at the pointer, swap kept, so a nearby orb can
                // take over without the pointer changing hands.
                self.draw_cursor(0.0, dt);
                self.wants_frames = true;
                return;
            }
            self.near = None;
            self.hide_cursor();
            return;
        }
        if self.near.is_none() {
            return;
        }

        if self.u_s > 0.002 && !self.typing && self.pointer.is_some() {
            self.hiding = true;
            self.draw_cursor(self.u_s, dt);
        } else {
            self.hide_cursor();
        }
        self.wants_frames = true;
    }

    /// Draws the pointer being pulled toward the orb: the bend (the body leans
    /// toward the orb behind its pinned tip) and the tail (fading copies of the
    /// bent pointer, stamped along the line to the orb's centre and masked to
    /// the side facing the orb). `w` is the smoothed proximity weight.
    fn draw_cursor(&mut self, w: f64, dt: f64) {
        let Some(sprite) = self.sprite.clone() else {
            return;
        };
        let Some(near) = self
            .near
            .and_then(|id| self.orbs.iter().find(|o| o.id == id))
        else {
            return;
        };
        let o = self.effective(near.opts);
        let ppu = sprite.pixels_per_unit();
        let (sw, sh) = sprite.raster_size();
        let (spw, sph) = (f64::from(sprite.width()), f64::from(sprite.height()));
        let (hot_x, hot_y) = (f64::from(sprite.hotspot().0), f64::from(sprite.hotspot().1));
        let ow = ((spw + 2.0 * PAD) * ppu).ceil() as usize;
        let oh = ((sph + 2.0 * PAD) * ppu).ceil() as usize;

        // Pull and bend: strength * w^curve, each eased by `smoothing`.
        let k = w.powf(f64::from(o.curve));
        let ease = 1.0 - (-dt / (0.012 + f64::from(o.smoothing) * 0.14)).exp();
        self.amp += (f64::from(o.strength) * k - self.amp) * ease;
        self.bend += (f64::from(o.deform) * k - self.bend) * ease;
        let mut a_px = self.amp * ppu;
        let mut b_px = self.bend * ppu;

        // The well's centre eases toward the nearest orb instead of snapping.
        let target = self.centre;
        let well = match self.well {
            None => target,
            Some(wc) => {
                let swing = 1.0 - (-dt / (0.05 + f64::from(o.handover) * 0.6)).exp();
                Point::new(
                    wc.x + (target.x - wc.x) * swing as f32,
                    wc.y + (target.y - wc.y) * swing as f32,
                )
            }
        };
        self.well = Some(well);

        // Geometry in raster px: hotspot at (P + hot), orb centre placed
        // relative to it exactly as on screen.
        let lp = self.last_pointer;
        let p = PAD * ppu;
        let hx = p + hot_x * ppu;
        let hy = p + hot_y * ppu;
        let cxo = hx + f64::from(well.x - lp.x) * ppu;
        let cyo = hy + f64::from(well.y - lp.y) * ppu;
        let d_tip = (cxo - hx).hypot(cyo - hy);
        let d_tip = if d_tip == 0.0 { 1.0 } else { d_tip };
        let (ux, uy) = ((cxo - hx) / d_tip, (cyo - hy) / d_tip);
        let falloff_px = f64::from(o.falloff) * ppu;

        // A pull toward the tip's side shortens the body, and a shortening
        // reads weaker than a stretch of the same size: `squash` adds to both
        // in proportion to how far the pull runs against the body's axis.
        if o.squash > 0.0 {
            let ax_len = (spw * 0.5 - hot_x).hypot(sph - hot_y);
            let ax_len = if ax_len == 0.0 { 1.0 } else { ax_len };
            let axx = (spw * 0.5 - hot_x) / ax_len;
            let axy = (sph - hot_y) / ax_len;
            let against = (-(ux * axx + uy * axy)).max(0.0);
            let gain = 1.0 + f64::from(o.squash) * against;
            a_px *= gain;
            b_px *= gain;
        }

        // The only region any pixel can land in: the sprite's box, grown by
        // the bend, swept along the tail.
        let grow = b_px.max(1.0).ceil() + 4.0;
        let bx1 = (p.min(p + ux * a_px) - grow).floor();
        let by1 = (p.min(p + uy * a_px) - grow).floor();
        let bx2 = (p.max(p + ux * a_px) + spw * ppu + grow).ceil();
        let by2 = (p.max(p + uy * a_px) + sph * ppu + grow).ceil();
        let rx = bx1.max(0.0) as usize;
        let ry = by1.max(0.0) as usize;
        let region = Region {
            x: rx,
            y: ry,
            w: (bx2.min(ow as f64) as usize).saturating_sub(rx),
            h: (by2.min(oh as f64) as usize).saturating_sub(ry),
        };

        // The bent pointer, straight-alpha RGBA8.
        let mut bent = vec![0u8; ow * oh * 4];
        if b_px > 0.01 {
            bend_sprite(
                &Bend {
                    src: sprite.rgba(),
                    sw: sw as usize,
                    sh: sh as usize,
                    ow,
                    oh,
                    b: b_px,
                    taper: f64::from(o.taper),
                    p,
                    hx,
                    hy,
                    cxo,
                    cyo,
                },
                &mut bent,
            );
        } else {
            blit(
                sprite.rgba(),
                sw as usize,
                sh as usize,
                &mut bent,
                ow,
                oh,
                p.round() as i64,
            );
        }

        let mut acc = premultiply(&bent);
        if a_px > 0.5 && region.w > 0 && region.h > 0 {
            let bent_p = acc.clone();
            let bcx = p + spw * ppu * 0.42;
            let bcy = p + sph * ppu * 0.5;
            let half = falloff_px / 2.0;
            let mask = Mask {
                x0: bcx - ux * half,
                y0: bcy - uy * half,
                dx: ux,
                dy: uy,
                len: falloff_px,
            };
            // Nearest stamp last, painted *under* everything before it.
            for i in (1..=PASSES).rev() {
                let t = i as f64 / PASSES as f64;
                let mut stamp =
                    shifted_masked(&bent_p, ow, &region, ux * a_px * t, uy * a_px * t, &mask);
                let sigma = f64::from(o.blur) * t.sqrt() * ppu;
                if o.blur > 0.0 && sigma >= 0.05 {
                    gaussian_blur(&mut stamp, region.w, region.h, sigma);
                }
                let alpha = ((1.0 - t).powf(1.6) * 0.9) as f32;
                under(&mut acc, ow, &region, &stamp, alpha);
            }
        }

        let rgba = unpremultiply(&acc);
        let origin = Point::new(
            (f64::from(lp.x) - hot_x - PAD) as f32,
            (f64::from(lp.y) - hot_y - PAD) as f32,
        );
        self.frame = Some(SpriteFrame {
            handle: image::Handle::from_rgba(ow as u32, oh as u32, rgba),
            bounds: Rectangle::new(
                origin,
                Size::new((spw + 2.0 * PAD) as f32, (sph + 2.0 * PAD) as f32),
            ),
        });
        self.shown = true;
    }
}

/// Smoothstep of how deep into the reach the pointer is (0 at the edge of the
/// reach, 1 at the orb's rim).
pub(crate) fn proximity(edge: f64, reach: f64) -> f64 {
    let t = 1.0 - edge.max(0.0) / reach;
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setters_clamp_and_ignore_non_finite() {
        let g = Gravity::new()
            .reach(-5.0)
            .strength(500.0)
            .deform(-1.0)
            .taper(0.0)
            .curve(9.0)
            .falloff(0.0)
            .smoothing(2.0)
            .handover(-1.0)
            .squash(9.0)
            .blur(99.0)
            .fade_ms(0.0);
        assert_eq!(g.get_reach(), 1.0);
        assert_eq!(g.get_strength(), 64.0);
        assert_eq!(g.get_deform(), 0.0);
        assert_eq!(g.get_taper(), 1.0);
        assert_eq!(g.get_curve(), 4.0);
        assert_eq!(g.get_falloff(), 2.0);
        assert_eq!(g.get_smoothing(), 1.0);
        assert_eq!(g.get_handover(), 0.0);
        assert_eq!(g.get_squash(), 3.0);
        assert_eq!(g.get_blur(), 24.0);
        assert_eq!(g.get_fade_ms(), 1.0);
        assert_eq!(
            Gravity::new().reach(f32::NAN).get_reach(),
            Gravity::default().get_reach()
        );
    }

    #[test]
    fn proximity_is_a_smoothstep_over_the_reach() {
        assert_eq!(proximity(160.0, 160.0), 0.0);
        assert_eq!(proximity(0.0, 160.0), 1.0);
        assert_eq!(proximity(-20.0, 160.0), 1.0);
        assert!((proximity(80.0, 160.0) - 0.5).abs() < 1e-12);
    }

    #[test]
    fn tuning_patch_layers_over_defaults() {
        let t = GravityTuning::new().reach(300.0).blur(0.0);
        let g = Gravity::default().with_tuning(&t);
        assert_eq!(g.get_reach(), 300.0);
        assert_eq!(g.get_blur(), 0.0);
        assert_eq!(g.get_strength(), Gravity::default().get_strength());
        let merged = t.merged(&GravityTuning::new().reach(10.0));
        assert_eq!(merged.reach, Some(10.0));
        assert_eq!(merged.blur, Some(0.0));
    }

    fn opaque_sprite() -> CursorSprite {
        CursorSprite::new(vec![255u8; 8 * 12 * 4], 8, 12, 8.0, 12.0, 0.0, 0.0).unwrap()
    }

    fn runtime_with_orb() -> Runtime {
        let mut rt = Runtime::new();
        rt.sprite = Some(opaque_sprite());
        rt.orbs.push(Orb {
            id: 1,
            rect: Rectangle::new(Point::new(100.0, 100.0), Size::new(64.0, 64.0)),
            opts: Gravity::default(),
        });
        rt
    }

    fn run(rt: &mut Runtime, now: f64, pointer: Option<Point>) {
        if let Some(p) = pointer {
            rt.last_pointer = p;
        }
        rt.pointer = pointer;
        rt.step(now, std::time::Instant::now);
    }

    #[test]
    fn without_a_sprite_the_effect_stays_off() {
        let mut rt = runtime_with_orb();
        rt.sprite = None;
        run(&mut rt, 1.0, Some(Point::new(132.0, 132.0)));
        assert!(!rt.hiding && !rt.shown);
        assert_eq!(rt.blocked_by().as_deref(), Some("no pointer sprite set"));
    }

    #[test]
    fn pointer_in_reach_claims_and_pointer_far_away_does_not() {
        let mut rt = runtime_with_orb();
        run(&mut rt, 1.0, Some(Point::new(900.0, 900.0)));
        assert!(!rt.hiding);
        for i in 0..40 {
            run(
                &mut rt,
                1.0 + f64::from(i) * 0.016,
                Some(Point::new(170.0, 132.0)),
            );
        }
        assert!(rt.hiding && rt.shown);
        assert_eq!(rt.near, Some(1));
        assert!(rt.frame.is_some());
        assert!(rt.u_s > 0.5);
    }

    #[test]
    fn leaving_lingers_then_gives_the_pointer_back() {
        let mut rt = runtime_with_orb();
        let near = Point::new(170.0, 132.0);
        for i in 0..40 {
            run(&mut rt, f64::from(i) * 0.016, Some(near));
        }
        assert!(rt.hiding);
        let far = Point::new(900.0, 900.0);
        let mut now = 0.64;
        // right after leaving, the swap is held
        for _ in 0..20 {
            now += 0.016;
            run(&mut rt, now, Some(far));
        }
        assert!(rt.hiding, "swap should linger");
        // long after, it is released
        for _ in 0..60 {
            now += 0.016;
            run(&mut rt, now, Some(far));
        }
        assert!(!rt.hiding && !rt.shown);
        assert_eq!(rt.near, None);
    }

    #[test]
    fn typing_and_losing_the_pointer_release_it() {
        let mut rt = runtime_with_orb();
        let near = Point::new(170.0, 132.0);
        for i in 0..40 {
            run(&mut rt, f64::from(i) * 0.016, Some(near));
        }
        assert!(rt.hiding);
        run(&mut rt, 1.0, None);
        assert!(!rt.hiding, "no pointer, no swap");

        let mut rt = runtime_with_orb();
        for i in 0..40 {
            run(&mut rt, f64::from(i) * 0.016, Some(near));
        }
        rt.typing = true;
        rt.hide_cursor();
        run(&mut rt, 1.0, Some(near));
        assert!(!rt.hiding);
    }

    #[test]
    fn the_nearest_orb_wins() {
        let mut rt = runtime_with_orb();
        rt.orbs.push(Orb {
            id: 2,
            rect: Rectangle::new(Point::new(300.0, 100.0), Size::new(64.0, 64.0)),
            opts: Gravity::default(),
        });
        for i in 0..40 {
            run(
                &mut rt,
                f64::from(i) * 0.016,
                Some(Point::new(290.0, 132.0)),
            );
        }
        assert_eq!(rt.near, Some(2));
    }
}
