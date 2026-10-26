//! Per-item simulation: mirrored transitions, the `morph`/`move`/`bend`
//! physics, and the dissolve/melt proximity state.
//!
//! This is a direct port of upstream `observer.ts`. The engine is pure: given
//! the layout target of every item and a wall-clock delta it yields the boxes
//! and paint parameters the rasterizer consumes. The widget owns one
//! [`ItemState`] per item; headless tests can step the same states by hand.


use iced::{Point, Size, Vector};

use super::geometry::CornerRadii;
use super::options::{
    DissolveOptions, ImageMeltOptions, ItemBox, ItemOptions, LiquidLayout, Morph, Move,
    MoveOptions, zeta,
};
use super::raster::{DissolvePaint, HoleMask, MeltPaint, RasterItem, TailCircles};
use super::spring::{self, Easing, SpringConfig, Transition};

/// One item's layout target and configuration for a frame.
#[derive(Debug, Clone)]
pub(crate) struct ItemSpec {
    pub base: ItemBox,
    pub radii: CornerRadii,
    pub options: ItemOptions,
}

impl ItemSpec {
    /// The target box after offset, drag and scale.
    pub(crate) fn target(&self, drag: Vector) -> ItemBox {
        let offset = self.options.offset + drag;
        self.base
            .translated(offset)
            .scaled(self.options.scale)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Sim {
    cx: f32,
    cy: f32,
    w: f32,
    h: f32,
    r: f32,
    vcx: f32,
    vcy: f32,
    vw: f32,
    vh: f32,
    vr: f32,
}

impl Sim {
    fn new(target: ItemBox, radius: f32) -> Self {
        Self {
            cx: target.center().x,
            cy: target.center().y,
            w: target.size.width,
            h: target.size.height,
            r: radius,
            vcx: 0.0,
            vcy: 0.0,
            vw: 0.0,
            vh: 0.0,
            vr: 0.0,
        }
    }

    fn boxed(&self, inset: f32) -> ItemBox {
        let w = (self.w - inset * 2.0).max(0.0);
        let h = (self.h - inset * 2.0).max(0.0);
        ItemBox::new(self.cx - w / 2.0, self.cy - h / 2.0, w, h)
    }

    fn moving(&self) -> bool {
        self.vcx.abs() + self.vcy.abs() + self.vw.abs() + self.vh.abs() + self.vr.abs() > 0.5
    }

    fn settled(&self, target: ItemBox, target_radius: f32, speed: f32) -> bool {
        (self.cx - target.center().x).abs() < 0.05
            && (self.cy - target.center().y).abs() < 0.05
            && (self.w - target.size.width).abs() < 0.05
            && (self.h - target.size.height).abs() < 0.05
            && (self.r - target_radius).abs() < 0.05
            && speed < 1.0
            && self.vw.abs() + self.vh.abs() + self.vr.abs() < 1.0
    }
}

#[derive(Debug, Clone)]
struct MorphState {
    sim: Option<Sim>,
    t_prev: Option<Point>,
    tvx: f32,
    tvy: f32,
    lead01: f32,
    corner_t0: f64,
    last_move_t: f64,
    last_size: Option<Size>,
    active: bool,
    round01: f32,
    motion_env: f32,
}

impl MorphState {
    fn new() -> Self {
        Self {
            sim: None,
            t_prev: None,
            tvx: 0.0,
            tvy: 0.0,
            lead01: 0.0,
            corner_t0: 0.0,
            last_move_t: 0.0,
            last_size: None,
            active: false,
            round01: 0.0,
            motion_env: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TailState {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    r: f32,
    phase: f32,
    initialized: bool,
}

impl TailState {
    fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            r: 0.0,
            phase: 0.0,
            initialized: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TimedTween {
    from: ItemBox,
    start: f64,
    duration: f64,
    delay: f64,
    easing: Easing,
}

#[derive(Debug, Clone)]
struct MirrorState {
    current: Option<ItemBox>,
    vel: [f32; 4],
    target: Option<ItemBox>,
    timed: Option<TimedTween>,
    delay_left: f64,
}

impl MirrorState {
    fn new() -> Self {
        Self {
            current: None,
            vel: [0.0; 4],
            target: None,
            timed: None,
            delay_left: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
struct DissolveState {
    fade: f32,
    release: Option<(f32, f32)>,
    opacity: f32,
    phase: f32,
    prev: Option<Point>,
    last_other: Option<ItemBox>,
    axis_x: Option<bool>,
}

impl DissolveState {
    fn new() -> Self {
        Self {
            fade: 0.0,
            release: None,
            opacity: 0.0,
            phase: 0.0,
            prev: None,
            last_other: None,
            axis_x: None,
        }
    }
}

#[derive(Debug, Clone)]
struct MeltState {
    strength: f32,
    phase: f32,
    prev: Option<Point>,
}

impl MeltState {
    fn new() -> Self {
        Self {
            strength: 0.0,
            phase: 0.0,
            prev: None,
        }
    }
}

/// Mutable state of one item, kept in the widget tree.
#[derive(Debug, Clone)]
pub(crate) struct ItemState {
    /// Drag translation applied by the widget.
    pub drag: Vector,
    /// Whether the pointer is dragging this item.
    pub dragging: bool,
    mirror: MirrorState,
    morph: MorphState,
    tail: TailState,
    bend_cur: f32,
    bend_cur_x: f32,
    dissolve: DissolveState,
    melt: MeltState,
    inset_smooth: Option<f32>,
    last_radius: Option<f32>,
    initialized: bool,
    /// The last rendered box, exposed for hit testing.
    pub rendered: Option<ItemBox>,
    /// Clock of the last step, for drag release animation.
    pub last_step: f64,
}

impl Default for ItemState {
    fn default() -> Self {
        Self {
            drag: Vector::ZERO,
            dragging: false,
            mirror: MirrorState::new(),
            morph: MorphState::new(),
            tail: TailState::new(),
            bend_cur: 0.0,
            bend_cur_x: 0.0,
            dissolve: DissolveState::new(),
            melt: MeltState::new(),
            inset_smooth: None,
            last_radius: None,
            initialized: false,
            rendered: None,
            last_step: 0.0,
        }
    }
}

impl ItemState {
    /// Whether another frame is needed to finish the current animation.
    ///
    /// `previous` is the body drawn on the previous frame; comparing it with
    /// the freshly stepped body catches springs that are still integrating.
    pub(crate) fn is_active(&self, previous: Option<ItemBox>) -> bool {
        let moved = match (previous, self.rendered) {
            (Some(before), Some(after)) => before != after,
            (None, Some(_)) => true,
            _ => false,
        };
        moved
            || self.dragging
            || self.tail.r > 0.05
            || self.tail.vx.abs() + self.tail.vy.abs() > 0.5
            || self.melt.strength > 0.001
            || self.dissolve.fade > 0.001
            || self.dissolve.opacity > 0.001
            || self.dissolve.release.is_some()
            || self.mirror.timed.is_some()
            || self.mirror.delay_left > 0.0
            || self.morph.active
            || self.morph.motion_env > 0.001
            || self.bend_cur.abs() > 0.05
            || self.bend_cur_x.abs() > 0.05
            || self.morph.sim.as_ref().is_some_and(Sim::moving)
            || self.mirror.vel.iter().any(|v| v.abs() > 0.5)
    }
}

/// Steps every item and returns the paint inputs for the rasterizer.
pub(crate) fn step(
    states: &mut [ItemState],
    specs: &[ItemSpec],
    now: f64,
    dt: f64,
    goo_blur: f32,
    reduced_motion: bool,
) -> Vec<RasterItem> {
    let dt = if dt.is_finite() { dt.clamp(0.0, 0.25) } else { 0.0 };
    let dt_f32 = dt as f32;
    let targets: Vec<ItemBox> = specs
        .iter()
        .zip(states.iter())
        .map(|(spec, state)| spec.target(state.drag))
        .collect();
    let radii: Vec<f32> = specs
        .iter()
        .zip(targets.iter())
        .map(|(spec, target)| {
            spec.options
                .radius
                .map(|r| r.top_left)
                .unwrap_or_else(|| target.size.height.min(target.size.width) / 2.0)
        })
        .collect();

    let mut out = Vec::with_capacity(specs.len());
    for (index, ((spec, state), target)) in specs
        .iter()
        .zip(states.iter_mut())
        .zip(targets.iter())
        .enumerate()
    {
        let target = *target;
        let radius = radii[index];
        let evolve = spec.options.morph.is_some_and(|m| m.shape);
        let moving = spec.options.move_effect.is_some();
        let bending = spec.options.bend.is_some();
        let dynamics = evolve || moving || bending;

        let (body, render_radius, speed, content_blur) = if reduced_motion {
            // Reduced motion: still render the liquid, but every body snaps to
            // its target and no tail/bend is produced.
            state.morph.sim = Some(Sim::new(target, radius));
            state.mirror.current = Some(target);
            state.tail = TailState::new();
            state.bend_cur = 0.0;
            state.bend_cur_x = 0.0;
            (target, radius, 0.0, 0.0)
        } else if evolve {
            step_evolve(state, spec, target, radius, now, dt, goo_blur)
        } else if moving || bending {
            step_move(state, spec, target, radius, dt_f32)
        } else {
            step_mirror(state, spec, target, radius, now, dt)
        };

        // Effective blob inset (bridgeGrow).
        let base_inset = spec.options.morph.map_or(0.0, |m| m.blob_inset);
        let grow = spec.options.morph.map_or(0.0, |m| m.bridge_grow);
        let mut inset = base_inset;
        if grow > 0.0 {
            let range = (goo_blur * 3.0).max(14.0);
            let mut best = f32::INFINITY;
            for (other, other_target) in specs.iter().zip(targets.iter()) {
                if std::ptr::eq(other, spec) {
                    continue;
                }
                let gap = box_gap(target, *other_target);
                if gap < best {
                    best = gap;
                }
            }
            if best < range {
                inset -= grow * smoothstep(1.0 - best / range);
            }
            let smooth = state.inset_smooth.get_or_insert(inset);
            *smooth += (inset - *smooth) * (dt_f32 * 18.0).min(1.0);
            inset = *smooth;
        } else {
            state.inset_smooth = Some(inset);
        }
        let body = body.inset(inset);
        let render_radius = (render_radius - inset).max(0.0);
        state.rendered = Some(body);
        if radius != state.last_radius.unwrap_or(f32::NAN) {
            state.last_radius = Some(radius);
        }
        state.last_step = now;

        out.push(RasterItem {
            bounds: body,
            radii: CornerRadii::uniform(render_radius),
            image: None,
            content_blur,
            bend: if bending && (state.bend_cur.abs() > 0.35 || state.bend_cur_x.abs() > 0.35) {
                Some((state.bend_cur, state.bend_cur_x))
            } else {
                None
            },
            tail: tail_circles(state),
            dissolve: None,
            melt: None,
            silhouette: true,
        });
        let _ = speed;
        let _ = dynamics;
    }

    // Proximity pass: dissolve and melt need every item's measured rect.
    for (index, state) in states.iter_mut().enumerate() {
        let rect = out[index].bounds;
        let spec = &specs[index];
        let item = &mut out[index];
        if let Some(options) = spec.options.dissolve {
            item.dissolve = dissolve_paint(
                index,
                rect,
                &targets,
                &mut state.dissolve,
                options,
                dt,
                goo_blur,
            );
        }
        if let Some(options) = spec.options.melt {
            item.melt =
                melt_paint(index, rect, &targets, &mut state.melt, options, dt);
        }
    }

    out
}

fn step_mirror(
    state: &mut ItemState,
    spec: &ItemSpec,
    target: ItemBox,
    radius: f32,
    now: f64,
    dt: f64,
) -> (ItemBox, f32, f32, f32) {
    let mirror = &mut state.mirror;
    if !state.initialized {
        mirror.current = Some(target);
        mirror.target = Some(target);
        state.initialized = true;
        return (target, radius, 0.0, 0.0);
    }
    let changed = mirror.target.is_none_or(|t| !boxes_close(t, target));
    if changed {
        mirror.target = Some(target);
        let delay = spec.options.delay.map_or(0.0, |d| d.as_secs_f64());
        match spec.options.transition {
            Transition::Timed { duration, easing } => {
                mirror.timed = Some(TimedTween {
                    from: mirror.current.unwrap_or(target),
                    start: now,
                    duration: duration.as_secs_f64(),
                    delay,
                    easing,
                });
            }
            Transition::Preset(preset) => {
                mirror.delay_left = delay;
                let cfg = preset.config();
                mirror.vel = [0.0; 4];
                let _ = cfg;
            }
            Transition::Spring(cfg) => {
                mirror.delay_left = delay;
                mirror.vel = [0.0; 4];
                let _ = cfg;
            }
            Transition::Smooth => {
                mirror.delay_left = delay;
                mirror.vel = [0.0; 4];
            }
        }
    }
    let target = mirror.target.unwrap_or(target);
    if let Some(tween) = mirror.timed {
        let elapsed = now - tween.start - tween.delay;
        if elapsed <= 0.0 {
            let current = mirror.current.unwrap_or(target);
            return (current, scaled_radius(radius, current, target), 0.0, 0.0);
        }
        let t = if tween.duration <= 0.0 {
            1.0
        } else {
            (elapsed / tween.duration).clamp(0.0, 1.0)
        };
        let e = tween.easing.evaluate(t as f32);
        let current = lerp_box(tween.from, target, e);
        mirror.current = Some(current);
        if t >= 1.0 {
            mirror.timed = None;
        }
        return (current, scaled_radius(radius, current, target), 0.0, 0.0);
    }
    if mirror.delay_left > 0.0 {
        mirror.delay_left = (mirror.delay_left - dt).max(0.0);
        let current = mirror.current.unwrap_or(target);
        return (current, scaled_radius(radius, current, target), 0.0, 0.0);
    }
    let cfg = match spec.options.transition {
        Transition::Preset(preset) => preset.config(),
        Transition::Spring(cfg) => cfg,
        Transition::Timed { .. } => SpringConfig::default(),
        Transition::Smooth => Transition::Smooth.preset_config(),
    };
    let cur = mirror.current.unwrap_or(target);
    let k = cfg.stiffness / cfg.mass.max(1e-3);
    let c = cfg.damping / cfg.mass.max(1e-3);
    let (x, vx) = spring::advance(cur.origin.x, mirror.vel[0], target.origin.x, k, c, dt as f32);
    let (y, vy) = spring::advance(cur.origin.y, mirror.vel[1], target.origin.y, k, c, dt as f32);
    let (w, vw) = spring::advance(
        cur.size.width,
        mirror.vel[2],
        target.size.width,
        k,
        c,
        dt as f32,
    );
    let (h, vh) = spring::advance(
        cur.size.height,
        mirror.vel[3],
        target.size.height,
        k,
        c,
        dt as f32,
    );
    mirror.vel = [vx, vy, vw, vh];
    let current = ItemBox::new(x, y, w.max(0.0), h.max(0.0));
    mirror.current = Some(current);
    (current, scaled_radius(radius, current, target), 0.0, 0.0)
}

fn scaled_radius(radius: f32, current: ItemBox, target: ItemBox) -> f32 {
    if target.size.width > 0.0 {
        radius * (current.size.width / target.size.width).clamp(0.0, 4.0)
    } else {
        radius
    }
}

fn step_evolve(
    state: &mut ItemState,
    spec: &ItemSpec,
    target: ItemBox,
    radius: f32,
    now: f64,
    dt: f64,
    _goo_blur: f32,
) -> (ItemBox, f32, f32, f32) {
    let morph = spec.options.morph.unwrap_or_else(Morph::new);
    let eo = morph.resolve();
    let tcx = target.center().x;
    let tcy = target.center().y;
    let mut sim = state.morph.sim.unwrap_or_else(|| Sim::new(target, radius));
    if !state.initialized {
        state.morph.sim = Some(sim);
        state.morph.last_size = Some(target.size);
        state.initialized = true;
        return (target, radius, 0.0, 0.0);
    }

    // Velocity lead.
    let (raw_vx, raw_vy) = state.morph.t_prev.map_or((0.0, 0.0), |prev| {
        if dt > 0.0 {
            ((tcx - prev.x) / dt as f32, (tcy - prev.y) / dt as f32)
        } else {
            (0.0, 0.0)
        }
    });
    state.morph.tvx = state.morph.tvx * 0.7 + raw_vx * 0.3;
    state.morph.tvy = state.morph.tvy * 0.7 + raw_vy * 0.3;
    state.morph.t_prev = Some(Point::new(tcx, tcy));
    let rem_x = tcx - sim.cx;
    let rem_y = tcy - sim.cy;
    let rem = (rem_x * rem_x + rem_y * rem_y).sqrt();
    let v_mag = (state.morph.tvx * state.morph.tvx + state.morph.tvy * state.morph.tvy).sqrt();
    let (mut dx, mut dy) = (0.0, 0.0);
    if v_mag > 1e-3 {
        dx = state.morph.tvx / v_mag;
        dy = state.morph.tvy / v_mag;
    } else if rem > 1e-3 {
        dx = rem_x / rem;
        dy = rem_y / rem;
    }
    let tau = (eo.anticipation.max(0.0) / 1000.0) as f64;
    let k = if tau > 0.0 {
        1.0 - (-dt / tau).exp()
    } else {
        1.0
    };
    state.morph.lead01 += ((if rem > 0.5 { 1.0 } else { 0.0 }) - state.morph.lead01) * k as f32;
    let reach = (eo.travel.max(0.0) * state.morph.lead01).min(rem);
    let ox = dx * reach;
    let oy = dy * reach;
    let (cx, vcx) = spring::advance(
        sim.cx,
        sim.vcx,
        tcx + ox,
        eo.mass_stiffness,
        eo.mass_damping,
        dt as f32,
    );
    let (cy, vcy) = spring::advance(
        sim.cy,
        sim.vcy,
        tcy + oy,
        eo.mass_stiffness,
        eo.mass_damping,
        dt as f32,
    );
    let (w, vw) = spring::advance(
        sim.w,
        sim.vw,
        target.size.width,
        eo.size_stiffness,
        eo.size_damping,
        dt as f32,
    );
    let (h, vh) = spring::advance(
        sim.h,
        sim.vh,
        target.size.height,
        eo.size_stiffness,
        eo.size_damping,
        dt as f32,
    );
    let (r, vr) = spring::advance(
        sim.r,
        sim.vr,
        radius,
        eo.radius_stiffness,
        eo.radius_damping,
        dt as f32,
    );
    sim = Sim {
        cx,
        cy,
        w: w.max(0.0),
        h: h.max(0.0),
        r: r.max(0.0),
        vcx,
        vcy,
        vw,
        vh,
        vr,
    };
    state.morph.sim = Some(sim);

    // Corner timeline.
    let now_ms = now * 1000.0;
    let corner_total = (eo.corner_delay.max(0.0) + eo.corner_duration.max(1.0)) as f64;
    let prev = state.morph.last_size;
    let size_delta = prev.map_or(0.0, |p| {
        (target.size.width - p.width).abs() + (target.size.height - p.height).abs()
    });
    if size_delta > 0.5 {
        if !state.morph.active {
            state.morph.corner_t0 = now_ms;
            state.morph.active = true;
        }
        state.morph.last_move_t = now_ms;
    } else if state.morph.active
        && now_ms - state.morph.last_move_t > 150.0
        && now_ms - state.morph.corner_t0 > corner_total
    {
        state.morph.active = false;
    }
    state.morph.last_size = Some(target.size);
    let mut target01 = 0.0;
    if state.morph.corner_t0 > 0.0
        && eo.roundness > 0.0
        && now_ms - state.morph.corner_t0 < corner_total
    {
        let p = ((now_ms - state.morph.corner_t0 - eo.corner_delay.max(0.0) as f64)
            / eo.corner_duration.max(1.0) as f64)
            .clamp(0.0, 1.0);
        let eased = eo.corner_ease.evaluate(p as f32);
        target01 = ((1.0 - eased) * eo.roundness).clamp(0.0, 1.0);
    }
    let max_step = (dt as f32) * 8.0;
    state.morph.round01 += (target01 - state.morph.round01).clamp(-max_step, max_step);
    let mut render_radius = sim.r;
    if state.morph.round01 > 0.001 {
        let round_target = (sim.w.min(sim.h) / 2.0).max(render_radius);
        render_radius += (round_target - render_radius) * state.morph.round01;
        render_radius = render_radius.max(radius);
    }

    // Content blur envelope.
    let motion_raw = ((sim.vcx.powi(2) + sim.vcy.powi(2)).sqrt()
        + sim.vw.abs()
        + sim.vh.abs())
    / 420.0;
    state.morph.motion_env =
        motion_raw.min(1.0).max(state.morph.motion_env - (dt as f32) * 1.9);
    let motion = state.morph.motion_env;
    let content_blur = motion * motion * eo.content_blur.max(0.0);

    let boxed = sim.boxed(0.0);
    (
        boxed,
        render_radius,
        (sim.vcx.powi(2) + sim.vcy.powi(2)).sqrt(),
        content_blur,
    )
}

fn step_move(
    state: &mut ItemState,
    spec: &ItemSpec,
    target: ItemBox,
    radius: f32,
    dt: f32,
) -> (ItemBox, f32, f32, f32) {
    let opts = if let Some(bend) = spec.options.bend {
        bend.resolve()
    } else {
        spec.options
            .move_effect
            .map_or_else(Move::new, |m| m)
            .resolve()
    };
    let tcx = target.center().x;
    let tcy = target.center().y;
    let mut sim = state
        .morph
        .sim
        .unwrap_or_else(|| Sim::new(target, radius));
    if !state.initialized {
        state.morph.sim = Some(sim);
        state.initialized = true;
        state.tail = TailState::new();
        return (target, radius, 0.0, 0.0);
    }
    let (cx, vcx) = spring::advance(
        sim.cx,
        sim.vcx,
        tcx,
        opts.stiffness,
        opts.damping,
        dt,
    );
    let (cy, vcy) = spring::advance(
        sim.cy,
        sim.vcy,
        tcy,
        opts.stiffness,
        opts.damping,
        dt,
    );
    sim = Sim {
        cx,
        cy,
        w: target.size.width,
        h: target.size.height,
        r: radius,
        vcx,
        vcy,
        vw: 0.0,
        vh: 0.0,
        vr: 0.0,
    };
    state.morph.sim = Some(sim);
    let speed = (vcx * vcx + vcy * vcy).sqrt();

    // Trailing droplets.
    let tail = &mut state.tail;
    if !tail.initialized {
        tail.x = cx;
        tail.y = cy;
        tail.initialized = true;
    }
    let (tx, tvx) = spring::advance(tail.x, tail.vx, cx, 170.0, 22.0, dt);
    let (ty, tvy) = spring::advance(tail.y, tail.vy, cy, 170.0, 22.0, dt);
    tail.x = tx;
    tail.y = ty;
    tail.vx = tvx;
    tail.vy = tvy;
    let bi = spec.options.morph.map_or(0.0, |m| m.blob_inset);
    let (ux, uy) = if speed > 0.001 {
        (vcx / speed, vcy / speed)
    } else {
        (1.0, 0.0)
    };
    let perp = (ux.abs() * sim.h + uy.abs() * sim.w - bi * 2.0).max(4.0);
    let half_along = (ux.abs() * sim.w + uy.abs() * sim.h) / 2.0;
    let base = perp / 2.0;
    let lag_x = tail.x - cx;
    let lag_y = tail.y - cy;
    let lag = (lag_x * lag_x + lag_y * lag_y).sqrt();
    let max_lag = half_along + base * (0.2 + opts.force * 1.6);
    if lag > max_lag && lag > 1e-4 {
        tail.x = cx + lag_x / lag * max_lag;
        tail.y = cy + lag_y / lag * max_lag;
    }
    let onset = ((speed - 20.0) / 120.0).clamp(0.0, 1.0);
    let target_r = base * opts.tail * onset;
    tail.r += (target_r - tail.r) * (dt * 10.0).min(1.0);
    tail.phase += speed * dt * 0.045;
    let _ = tail.phase;

    // Bends.
    let cap = (sim.w.min(sim.h) * 0.5).max(0.0);
    let bty = (sim.vcy * 0.05).clamp(-cap, cap) * opts.bend;
    let cap_x = sim.w.min(sim.h) * 0.9;
    let btx = (sim.vcx * 0.09).clamp(-cap_x, cap_x) * opts.bend_x;
    state.bend_cur += (bty - state.bend_cur) * (dt * 9.0).min(1.0);
    state.bend_cur_x += (btx - state.bend_cur_x) * (dt * 9.0).min(1.0);

    (sim.boxed(0.0), radius, speed, 0.0)
}

fn tail_circles(state: &ItemState) -> Option<TailCircles> {
    let tail = state.tail;
    if tail.r < 0.3 || !tail.initialized {
        return None;
    }
    let sim = state.morph.sim?;
    let speed = (sim.vcx.powi(2) + sim.vcy.powi(2)).sqrt();
    if speed <= 0.001 {
        return None;
    }
    let ux = sim.vcx / speed;
    let uy = sim.vcy / speed;
    let lag_x = tail.x - sim.cx;
    let lag_y = tail.y - sim.cy;
    let wob = tail.r * 0.16;
    let px = -uy;
    let py = ux;
    let w1 = tail.phase.sin() * wob;
    let w2 = (tail.phase + 2.4).sin() * -wob;
    Some(TailCircles {
        tail: (Point::new(tail.x, tail.y), tail.r),
        mid_a: (
            Point::new(sim.cx + lag_x * 0.45 + px * w1, sim.cy + lag_y * 0.45 + py * w1),
            tail.r * 0.62,
        ),
        mid_b: (
            Point::new(sim.cx + lag_x * 0.75 + px * w2, sim.cy + lag_y * 0.75 + py * w2),
            tail.r * 0.4,
        ),
    })
}

#[allow(clippy::too_many_arguments)]
fn dissolve_paint(
    index: usize,
    rect: ItemBox,
    targets: &[ItemBox],
    state: &mut DissolveState,
    options: DissolveOptions,
    dt: f64,
    goo_blur: f32,
) -> Option<DissolvePaint> {
    let range = options
        .range
        .unwrap_or_else(|| (goo_blur * 2.5).max(10.0));
    let mut best_gap = f32::INFINITY;
    let mut best_other: Option<ItemBox> = None;
    for (other_index, other) in targets.iter().enumerate() {
        if other_index == index {
            continue;
        }
        let gap = box_gap(rect, *other);
        if gap < best_gap {
            best_gap = gap;
            best_other = Some(*other);
        }
    }
    let mut embed = 0.0;
    let mut contact_span = 0.0;
    if let Some(other) = best_other {
        if best_gap <= 0.0 {
            let ox = (rect.right()).min(other.right()) - rect.origin.x.max(other.origin.x);
            let oy = (rect.bottom()).min(other.bottom()) - rect.origin.y.max(other.origin.y);
            let span = rect
                .size
                .width
                .min(rect.size.height)
                .min(other.size.width)
                .min(other.size.height)
                .max(1.0);
            embed = ox.min(oy).max(0.0) / span;
            contact_span = ox.max(oy).max(0.0);
        }
    }
    let mut target = 0.0;
    if best_other.is_some() && best_gap < range && options.active {
        let raw = smoothstep(((1.0 - best_gap / range) / 0.65).min(1.0));
        let strength = options.strength.clamp(0.0, 1.0);
        let sink = options.sink.max(0.01);
        let sunk = smoothstep(((embed - sink * 0.2) / (sink * 0.8).max(0.01)).clamp(0.0, 1.0));
        target = raw.powf(1.25) * strength * (1.0 - sunk);
    }
    if target >= state.fade {
        state.fade += (target - state.fade) * (dt as f32 * 16.0).min(1.0);
        state.release = None;
    } else if target > 0.02 {
        state.fade += (target - state.fade) * (dt as f32 * 6.0).min(1.0);
        state.release = None;
    } else {
        let rel_ms = (options.release_ms.max(options.fade_ms.unwrap_or(options.release_ms)))
            .max(40.0);
        let (from, elapsed) = state.release.get_or_insert((state.fade, 0.0));
        *elapsed += dt as f32 * 1000.0;
        let k = (*elapsed / rel_ms).min(1.0);
        state.fade = target + (*from - target) * (1.0 - k) * (1.0 - k);
        if k >= 1.0 {
            state.fade = target;
        }
    }
    if target == 0.0 && state.fade < 0.001 {
        state.fade = 0.0;
    }
    let s = state.fade;
    if s <= 0.001 {
        state.opacity = 0.0;
        return None;
    }
    let other = if best_gap < range {
        let other = best_other?;
        state.last_other = Some(other);
        other
    } else {
        state.last_other?
    };
    let seam = seam_point(rect, other);
    let zone = options.zone.unwrap_or(goo_blur * 2.2 + 4.0);
    let d = (rect.size.width.min(rect.size.height) * 0.9)
        .min(zone * (0.7 + 0.6 * s));
    // Opacity fade clock.
    if let Some((_, elapsed)) = state.release {
        let fade_ms = options
            .fade_ms
            .unwrap_or(options.release_ms)
            .max(40.0);
        let k = (elapsed / fade_ms).min(1.0);
        let rel_fade = (1.0 - k) * (1.0 - k);
        state.opacity = if rel_fade < state.opacity {
            rel_fade
        } else {
            state.opacity + (rel_fade - state.opacity) * (dt as f32 * 16.0).min(1.0)
        };
    } else {
        state.opacity = s;
    }
    let s_struct = state.fade;
    // Churn only while the item moves.
    let pos = rect.center();
    let move_speed = state.prev.map_or(0.0, |prev| {
        if dt > 0.0 {
            ((pos.x - prev.x).powi(2) + (pos.y - prev.y).powi(2)).sqrt() / dt as f32
        } else {
            0.0
        }
    });
    state.prev = Some(pos);
    let phase_adv = (dt.min(1.0 / 24.0) as f32)
        * options.flow_speed.max(0.0)
        * 0.12
        * (move_speed / 40.0).min(1.0);
    state.phase += phase_adv;

    let neighbor_center = other.center();
    let gx = neighbor_center.x - seam.x;
    let gy = neighbor_center.y - seam.y;
    let gl = (gx * gx + gy * gy).sqrt();
    let (gux, guy) = if gl > 1e-4 { (gx / gl, gy / gl) } else { (1.0, 0.0) };
    let axis_x = match state.axis_x {
        Some(true) => {
            if guy.abs() > gux.abs() * 1.25 {
                false
            } else {
                true
            }
        }
        Some(false) => {
            if gux.abs() > guy.abs() * 1.25 {
                true
            } else {
                false
            }
        }
        None => gux.abs() >= guy.abs(),
    };
    state.axis_x = Some(axis_x);

    // Edge hole on the original image.
    let local_seam = Point::new(seam.x - rect.origin.x, seam.y - rect.origin.y);
    let gap_to_image = distance_box(seam, rect);
    let hole = if gap_to_image > d {
        None
    } else {
        let rim = rect.size.width.min(rect.size.height) / 2.0;
        let mut lx = local_seam.x;
        let mut ly = local_seam.y;
        let mut vx = lx - rect.size.width / 2.0;
        let mut vy = ly - rect.size.height / 2.0;
        let vlen = (vx * vx + vy * vy).sqrt();
        if vlen < rim {
            if vlen < 1e-3 {
                vx = gux;
                vy = guy;
            } else {
                vx /= vlen;
                vy /= vlen;
            }
            lx = rect.size.width / 2.0 + vx * rim;
            ly = rect.size.height / 2.0 + vy * rim;
        }
        let bridge_range = (goo_blur * 2.5).max(10.0);
        let s_bridge = if best_other.is_some() {
            if best_gap < bridge_range {
                smoothstep(1.0 - best_gap / bridge_range)
            } else {
                0.0
            }
        } else {
            s
        };
        let alpha = (1.0 - s.min(s_bridge) * 2.2).clamp(0.0, 1.0);
        let hd = (d.max(contact_span * 0.75)).min(rim);
        if best_gap <= 0.0 && contact_span > 4.0 && best_other.is_some() {
            let ncx = neighbor_center.x - rect.origin.x;
            let ncy = neighbor_center.y - rect.origin.y;
            let ox0 = rect.origin.x.max(other.origin.x) - rect.origin.x;
            let oy0 = rect.origin.y.max(other.origin.y) - rect.origin.y;
            let ox1 = rect.right().min(other.right()) - rect.origin.x;
            let oy1 = rect.bottom().min(other.bottom()) - rect.origin.y;
            let r_end = [
                distance(Point::new(ox0, oy0), Point::new(ncx, ncy)),
                distance(Point::new(ox1, oy0), Point::new(ncx, ncy)),
                distance(Point::new(ox0, oy1), Point::new(ncx, ncy)),
                distance(Point::new(ox1, oy1), Point::new(ncx, ncy)),
            ]
            .into_iter()
            .fold(0.0_f32, f32::max)
                + 3.0;
            let seam_blur = options.seam_blur.unwrap_or(options.blur * 1.6);
            let band = (rim * 2.6).min(hd.max(seam_blur * 3.2));
            Some(HoleMask::Linear {
                center: Point::new(ncx, ncy),
                alpha,
                core: (r_end - band).max(0.0),
                outer: r_end,
            })
        } else {
            Some(HoleMask::Radial {
                center: Point::new(lx, ly),
                alpha,
                core: hd * 0.2,
                mid: hd * 0.45,
                outer: hd * 0.78,
            })
        }
    };

    Some(DissolvePaint {
        options,
        strength: s_struct,
        opacity: state.opacity.min(1.0),
        seam,
        neighbor_center,
        gap: best_gap.max(0.0),
        embed,
        d,
        phase: state.phase,
        axis_x,
        hole,
    })
}

fn melt_paint(
    index: usize,
    rect: ItemBox,
    targets: &[ItemBox],
    state: &mut MeltState,
    options: ImageMeltOptions,
    dt: f64,
) -> Option<MeltPaint> {
    let mut best_gap = f32::INFINITY;
    let mut best_other: Option<ItemBox> = None;
    for (other_index, other) in targets.iter().enumerate() {
        if other_index == index {
            continue;
        }
        let gap = box_gap(rect, *other);
        if gap < best_gap {
            best_gap = gap;
            best_other = Some(*other);
        }
    }
    let other = best_other?;
    let range = (options.blur * 2.6).max(8.0);
    let near = (1.0 - best_gap / range).clamp(0.0, 1.0);
    let target = near * near * (3.0 - 2.0 * near);
    state.strength += (target - state.strength) * (dt as f32 * 12.0).min(1.0);
    if state.strength <= 0.001 {
        return None;
    }
    let seam = seam_point(rect, other);
    let pos = rect.center();
    let move_speed = state.prev.map_or(0.0, |prev| {
        if dt > 0.0 {
            ((pos.x - prev.x).powi(2) + (pos.y - prev.y).powi(2)).sqrt() / dt as f32
        } else {
            0.0
        }
    });
    state.prev = Some(pos);
    state.phase += (dt.min(1.0 / 24.0) as f32) * 0.12 * (move_speed / 40.0).min(1.0);
    let diag = (rect.size.width.powi(2) + rect.size.height.powi(2)).sqrt() / 2.0;
    let d = diag * options.reach.clamp(0.0, 2.0) * state.strength;
    Some(MeltPaint {
        options,
        strength: state.strength,
        seam,
        neighbor_center: other.center(),
        d,
        phase: state.phase,
        overlap: best_gap <= 0.0,
    })
}

fn seam_point(a: ItemBox, b: ItemBox) -> Point {
    let cx = if a.right() < b.origin.x {
        (a.right() + b.origin.x) / 2.0
    } else if b.right() < a.origin.x {
        (b.right() + a.origin.x) / 2.0
    } else {
        (a.origin.x.max(b.origin.x) + a.right().min(b.right())) / 2.0
    };
    let cy = if a.bottom() < b.origin.y {
        (a.bottom() + b.origin.y) / 2.0
    } else if b.bottom() < a.origin.y {
        (b.bottom() + a.origin.y) / 2.0
    } else {
        (a.origin.y.max(b.origin.y) + a.bottom().min(b.bottom())) / 2.0
    };
    Point::new(cx, cy)
}

fn box_gap(a: ItemBox, b: ItemBox) -> f32 {
    let dx = (a.origin.x - b.right()).max(b.origin.x - a.right()).max(0.0);
    let dy = (a.origin.y - b.bottom()).max(b.origin.y - a.bottom()).max(0.0);
    (dx * dx + dy * dy).sqrt()
}

fn distance(p: Point, q: Point) -> f32 {
    ((p.x - q.x).powi(2) + (p.y - q.y).powi(2)).sqrt()
}

fn distance_box(p: Point, rect: ItemBox) -> f32 {
    let dx = (rect.origin.x - p.x).max(p.x - rect.right()).max(0.0);
    let dy = (rect.origin.y - p.y).max(p.y - rect.bottom()).max(0.0);
    (dx * dx + dy * dy).sqrt()
}

fn boxes_close(a: ItemBox, b: ItemBox) -> bool {
    (a.origin.x - b.origin.x).abs() < 0.05
        && (a.origin.y - b.origin.y).abs() < 0.05
        && (a.size.width - b.size.width).abs() < 0.05
        && (a.size.height - b.size.height).abs() < 0.05
}

fn lerp_box(a: ItemBox, b: ItemBox, t: f32) -> ItemBox {
    let t = t.clamp(0.0, 1.0);
    ItemBox {
        origin: Point::new(
            a.origin.x + (b.origin.x - a.origin.x) * t,
            a.origin.y + (b.origin.y - a.origin.y) * t,
        ),
        size: Size::new(
            a.size.width + (b.size.width - a.size.width) * t,
            a.size.height + (b.size.height - a.size.height) * t,
        ),
    }
}

fn smoothstep(t: f32) -> f32 {
    let c = t.clamp(0.0, 1.0);
    c * c * (3.0 - 2.0 * c)
}

/// Computes the layout boxes for a group without stepping physics.
pub(crate) fn layout_targets(
    layout: LiquidLayout,
    padding: f32,
    sizes: &[Size],
    positions: &[Option<Point>],
    offsets: &[Vector],
    scales: &[f32],
) -> Vec<ItemBox> {
    let mut out = Vec::with_capacity(sizes.len());
    let mut cursor = padding;
    let mut max_cross = 0.0_f32;
    for (index, size) in sizes.iter().enumerate() {
        let position = positions[index];
        let offset = offsets[index];
        let scale = scales[index].max(0.0);
        let boxed = match layout {
            LiquidLayout::Row { spacing } => {
                let x = padding + cursor;
                let y = padding;
                cursor += size.width + spacing;
                max_cross = max_cross.max(size.height);
                ItemBox::new(x, y, size.width, size.height)
            }
            LiquidLayout::Column { spacing } => {
                let x = padding;
                let y = padding + cursor;
                cursor += size.height + spacing;
                max_cross = max_cross.max(size.width);
                ItemBox::new(x, y, size.width, size.height)
            }
            LiquidLayout::Free => {
                let p = position.unwrap_or_default();
                ItemBox::new(padding + p.x, padding + p.y, size.width, size.height)
            }
        };
        out.push(boxed.translated(offset).scaled(scale));
    }
    let _ = max_cross;
    out
}

/// Effective zeta used by the public knob mapping (kept here for tests).
#[allow(dead_code)]
pub(crate) fn damping_ratio(bounce: f32) -> f32 {
    zeta(bounce)
}

/// The transition config a preset expands to.
pub(crate) trait TransitionExt {
    fn preset_config(self) -> SpringConfig;
}

impl TransitionExt for Transition {
    fn preset_config(self) -> SpringConfig {
        match self {
            Transition::Preset(preset) => preset.config(),
            Transition::Spring(cfg) => cfg,
            Transition::Timed { .. } | Transition::Smooth => {
                super::spring::TransitionPreset::Smooth.config()
            }
        }
    }
}

/// Converts the public bend/move knobs into the raw options (for tests).
#[allow(dead_code)]
pub(crate) fn resolved_move(spec: &ItemSpec) -> MoveOptions {
    spec.options
        .move_effect
        .map_or_else(Move::new, |m| m)
        .resolve()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(x: f32, w: f32) -> ItemSpec {
        let mut options = ItemOptions::default();
        options.observe = true;
        ItemSpec {
            base: ItemBox::new(x, 0.0, w, 40.0),
            radii: CornerRadii::uniform(20.0),
            options,
        }
    }

    #[test]
    fn layout_row_and_free() {
        let sizes = [Size::new(40.0, 40.0), Size::new(60.0, 40.0)];
        let targets = layout_targets(
            LiquidLayout::Row { spacing: 8.0 },
            0.0,
            &sizes,
            &[None, None],
            &[Vector::ZERO; 2],
            &[1.0; 2],
        );
        assert_eq!(targets[0].origin.x, 0.0);
        assert_eq!(targets[1].origin.x, 48.0);
        let targets = layout_targets(
            LiquidLayout::Free,
            10.0,
            &sizes,
            &[Some(Point::new(5.0, 6.0)), Some(Point::new(50.0, 0.0))],
            &[Vector::ZERO; 2],
            &[1.0; 2],
        );
        assert_eq!(targets[0].origin, Point::new(15.0, 16.0));
    }

    #[test]
    fn mirror_springs_toward_a_new_target() {
        let mut a = spec(0.0, 40.0);
        let mut states = vec![ItemState::default()];
        let _ = step(&mut states, &[a.clone()], 0.0, 0.016, 6.0, false);
        a.base = ItemBox::new(100.0, 0.0, 40.0, 40.0);
        for i in 0..240 {
            step(&mut states, &[a.clone()], 0.016 * (i + 1) as f64, 0.016, 6.0, false);
        }
        let rendered = states[0].rendered.unwrap();
        assert!((rendered.origin.x - 100.0).abs() < 0.5, "{rendered:?}");
    }

    #[test]
    fn dissolve_requires_proximity() {
        let mut options = ItemOptions::default();
        options.dissolve = Some(DissolveOptions::new());
        let a = ItemSpec {
            base: ItemBox::new(0.0, 0.0, 40.0, 40.0),
            radii: CornerRadii::uniform(20.0),
            options,
        };
        let b = spec(200.0, 40.0);
        let mut states = vec![ItemState::default(), ItemState::default()];
        let items = step(
            &mut states,
            &[a, b],
            0.0,
            0.016,
            6.0,
            false,
        );
        assert!(items[0].dissolve.is_none());
    }
}
