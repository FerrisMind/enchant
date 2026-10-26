//! The [`MetalFx`] iced widget: clock, glow state and layered drawing.

use std::cell::RefCell;
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use iced::advanced::image::{self, Renderer as _};
use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer::{self, Renderer as _};
use iced::advanced::widget::{self, Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse, window};
use iced_wgpu::primitive::Renderer as _;

use super::bend::{MetalBendField, MetalBendState};
use super::bend_config::{MetalBendConfig, bend_config};
use super::cursor_light::{
    MetalCursorLight, MetalCursorSpill, bake_spill_rgba, cursor_light_config, fade_alpha,
    nearest_outline_point, proximity_weight, spill_frame,
};
use super::geometry::MetalFxVariant;
use super::glow::{GlowFrame, GlowSampler, GlowState, GlowTick, MaterialSampler};
use super::glow_config::{MetalGlowConfig, glow_config};
use super::mask::{MetalFxMask, MetalGlowMode};
use super::reflection_anchor::{MetalReflectionAnchor, register_reflection_anchor};
use super::material::{MetalMaterial, SheetMapping};
use super::options::{MetalFxLayer, MetalFxOptions, MetalFxSurface};
use super::preset::{MetalFxPreset, MetalFxResolvedTheme, MetalFxTheme, MetalPresetMode};
use super::primitive::MetalFxPrimitive;
use super::raster::{MetalFxHost, SceneParams};
use super::rim::MetalRimOptions;
use super::scene::{Layer, Scene};

/// Frame cadence the widget asks for while animating (~60 fps).
const FRAME: Duration = Duration::from_millis(16);
/// Like the reference's `IntersectionObserver` `rootMargin`: start animating
/// slightly before the element scrolls into view.
const VISIBILITY_MARGIN: f32 = 64.0;

/// Wraps a child in the Libraries.dev liquid-metal ring.
///
/// The widget measures its child (the child's bounds are the box), draws the
/// wrapper surface under it, a shader-driven metal band along its edge, the
/// wandering glow, the optional inner-shadow rim and the hairlines over it,
/// with the same layering as `styles.ts`. Every instance draws through one
/// shared GPU pipeline. Each layer is a shader primitive, so on a backend
/// without wgpu nothing but the child is drawn.
///
/// iced cannot read the child's corner radius like the reference reads its
/// computed style: the ring is fully rounded unless
/// [`border_radius`](Self::border_radius) says otherwise.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::{
///     MetalFx, MetalFxPreset, MetalFxTheme, MetalFxVariant,
/// };
/// use iced::widget::{container, text};
/// use iced::{Element, Length};
///
/// #[derive(Clone)]
/// enum Message {}
///
/// fn view<'a>() -> Element<'a, Message> {
///     MetalFx::new(
///         container(text("Upgrade"))
///             .padding([10, 24])
///             .width(Length::Shrink),
///     )
///     .variant(MetalFxVariant::Button)
///     .preset(MetalFxPreset::Silver)
///     .theme(MetalFxTheme::Dark)
///     .strength(0.9)
///     .inner_shadow(true)
///     .into()
/// }
/// # let _ = view;
/// ```
#[must_use = "widgets do nothing unless turned into an iced Element"]
pub struct MetalFx<'a, Message> {
    options: MetalFxOptions,
    child: Element<'a, Message>,
}

impl<Message> fmt::Debug for MetalFx<'_, Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MetalFx")
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

impl<'a, Message> MetalFx<'a, Message> {
    /// Wraps `child`.
    pub fn new(child: impl Into<Element<'a, Message>>) -> Self {
        Self {
            options: MetalFxOptions::new(),
            child: child.into(),
        }
    }

    /// Replaces every option at once.
    pub fn with_options(mut self, options: MetalFxOptions) -> Self {
        self.options = options;
        self
    }

    /// Sets the variant.
    pub fn variant(mut self, variant: MetalFxVariant) -> Self {
        self.options = self.options.variant(variant);
        self
    }

    /// Sets the colour preset.
    pub fn preset(mut self, preset: MetalFxPreset) -> Self {
        self.options = self.options.preset(preset);
        self
    }

    /// Sets the theme mode.
    pub fn theme(mut self, theme: MetalFxTheme) -> Self {
        self.options = self.options.theme(theme);
        self
    }

    /// Effect strength in `0..=1`.
    pub fn strength(mut self, strength: f32) -> Self {
        self.options = self.options.strength(strength);
        self
    }

    /// Extra multiplier on the glow only.
    pub fn glow_gain(mut self, gain: f32) -> Self {
        self.options = self.options.glow_gain(gain);
        self
    }

    /// Pauses the animation; the ring keeps its last frame.
    pub fn paused(mut self, paused: bool) -> Self {
        self.options = self.options.paused(paused);
        self
    }

    /// Overrides the corner radius, logical px.
    pub fn border_radius(mut self, radius: f32) -> Self {
        self.options = self.options.border_radius(radius);
        self
    }

    /// Turns the wandering halo on or off.
    pub fn glow(mut self, glow: bool) -> Self {
        self.options = self.options.glow(glow);
        self
    }

    /// Light rim along the top inside edge of the ring.
    pub fn inner_shadow(mut self, enabled: bool) -> Self {
        self.options = self.options.inner_shadow(enabled);
        self
    }

    /// Enables the inner shadow with explicit tuning.
    pub fn inner_shadow_with(mut self, rim: MetalRimOptions) -> Self {
        self.options = self.options.inner_shadow_with(rim);
        self
    }

    /// Overrides the shader sampling scale.
    pub fn shader_scale(mut self, scale: f32) -> Self {
        self.options = self.options.shader_scale(scale);
        self
    }

    /// Custom alpha mask / sheet fill.
    pub fn mask(mut self, mask: MetalFxMask) -> Self {
        self.options = self.options.mask(mask);
        self
    }

    /// Glow placement when a fill mask is set.
    pub fn glow_mode(mut self, mode: MetalGlowMode) -> Self {
        self.options = self.options.glow_mode(mode);
        self
    }

    /// Enables the cursor liquid dent (`useMetalBend`).
    pub fn bend(mut self, bend: bool) -> Self {
        self.options = self.options.bend(bend);
        self
    }

    /// Publishes this instance for neighbour reflections.
    pub fn anchor_id(mut self, id: impl Into<String>) -> Self {
        self.options = self.options.anchor_id(id);
        self
    }

    /// Overrides the ring thickness, logical px.
    pub fn ring_px(mut self, px: f32) -> Self {
        self.options = self.options.ring_px(px);
        self
    }

    /// Master scale of every absolute-pixel constant.
    pub fn scale(mut self, scale: f32) -> Self {
        self.options = self.options.scale(scale);
        self
    }

    /// The wrapper's background under the child.
    pub fn surface(mut self, surface: MetalFxSurface) -> Self {
        self.options = self.options.surface(surface);
        self
    }

    /// Where the metal layers sit relative to the child.
    pub fn layer(mut self, layer: MetalFxLayer) -> Self {
        self.options = self.options.layer(layer);
        self
    }

    /// Replaces the preset with raw Paper `liquidMetal` parameters.
    pub fn preset_mode(mut self, mode: MetalPresetMode) -> Self {
        self.options = self.options.preset_mode(mode);
        self
    }

    /// The current options (useful for headless rendering of the same style).
    pub fn options(&self) -> &MetalFxOptions {
        &self.options
    }
}

impl<'a, Message: 'a> From<MetalFx<'a, Message>> for Element<'a, Message> {
    fn from(metal: MetalFx<'a, Message>) -> Self {
        Element::new(metal)
    }
}

/// Per-widget state that outlives `view` calls.
struct Inner {
    /// Animated seconds; frozen while paused.
    clock: f64,
    last: Option<Instant>,
    glow: GlowState,
    glow_frame: Option<GlowFrame>,
    bend: MetalBendState,
    bend_field: Option<MetalBendField>,
    /// Pointer in box-local CSS px, if inside the reach of this instance.
    pointer: Option<[f32; 2]>,
    pointer_vel: [f32; 2],
    last_pointer: Option<([f32; 2], Instant)>,
    pressed: bool,
    /// Smoothed proximity for catch-light / spill (`uS` in light.ts).
    cursor_env: f32,
    cursor_light: Option<MetalCursorLight>,
    spill: Option<MetalCursorSpill>,
}

impl Inner {
    fn new(seed: u64) -> Self {
        Self {
            clock: 0.0,
            last: None,
            glow: GlowState::new(seed),
            glow_frame: None,
            bend: MetalBendState::new(),
            bend_field: None,
            pointer: None,
            pointer_vel: [0.0; 2],
            last_pointer: None,
            pressed: false,
            cursor_env: 0.0,
            cursor_light: None,
            spill: None,
        }
    }

    fn tick_cursor_light(
        &mut self,
        host_w: f32,
        host_h: f32,
        radius: f32,
        kind: MetalFxVariant,
        dt_secs: f32,
        sampler: &MaterialSampler,
    ) {
        let cfg = cursor_light_config();
        let a = fade_alpha(dt_secs.max(0.001), cfg.fade_ms);
        let (target_u, outline, signed) = if let Some(p) = self.pointer {
            let (d, pt) = nearest_outline_point(p, host_w, host_h, radius, kind);
            (proximity_weight(d.abs(), cfg.reach), pt, d)
        } else {
            (0.0, [host_w * 0.5, 0.0], 0.0)
        };
        self.cursor_env += (target_u - self.cursor_env) * a;
        if !cfg.enabled || self.cursor_env < 0.002 {
            self.cursor_light = None;
            self.spill = None;
            if self.cursor_env < 0.002 {
                self.cursor_env = 0.0;
            }
            return;
        }
        if cfg.catch_light {
            self.cursor_light = Some(MetalCursorLight {
                x: outline[0],
                y: outline[1],
                weight: self.cursor_env,
            });
        } else {
            self.cursor_light = None;
        }
        if cfg.spill {
            let rgb = sampler.rgb(outline);
            let lum = sampler.luminance(outline);
            let inside = signed < 0.0;
            self.spill = spill_frame(
                &cfg,
                self.pointer.unwrap_or(outline),
                outline,
                self.cursor_env,
                rgb,
                lum,
                inside,
            );
        } else {
            self.spill = None;
        }
    }

    fn tick_bend(&mut self, host_w: f32, host_h: f32, radius: f32, cfg: &MetalBendConfig) {
        let now_ms = self.clock * 1000.0;
        let ecx = host_w * 0.5;
        let ecy = host_h * 0.5;
        let (contact, target_disp, target_liquid, active) = if let Some(p) = self.pointer {
            let ddx = p[0] - ecx;
            let ddy = p[1] - ecy;
            let d = ddx.hypot(ddy).max(1e-3);
            let ux = ddx / d;
            let uy = ddy / d;
            // Signed distance to the rounded-rect edge (circle-ish for pills).
            let r_edge = radius.min(host_w.min(host_h) * 0.5);
            let signed = d - r_edge;
            let speed = self.pointer_vel[0].hypot(self.pointer_vel[1]);
            let (mut dir, liquid) = super::bend::pointer_targets(
                signed,
                [-ux, -uy],
                [ux, uy],
                speed,
                cfg,
            );
            if self.pressed {
                dir[0] -= ux * cfg.press;
                dir[1] -= uy * cfg.press;
            }
            (p, dir, liquid, true)
        } else {
            ([ecx, ecy], [0.0, 0.0], 0.0, false)
        };
        self.bend_field = self.bend.tick(
            now_ms,
            contact,
            target_disp,
            target_liquid,
            active && cfg.enabled,
            cfg,
        );
    }
}

struct MetalState {
    inner: RefCell<Inner>,
}

fn ambient_is_dark(theme: &Theme) -> bool {
    theme.extended_palette().is_dark
}

fn within_margin(bounds: Rectangle, viewport: &Rectangle) -> bool {
    bounds.intersects(&Rectangle {
        x: viewport.x - VISIBILITY_MARGIN,
        y: viewport.y - VISIBILITY_MARGIN,
        width: viewport.width + 2.0 * VISIBILITY_MARGIN,
        height: viewport.height + 2.0 * VISIBILITY_MARGIN,
    })
}

impl<Message> MetalFx<'_, Message> {
    fn child_layout(layout: Layout<'_>) -> Layout<'_> {
        layout.children().next().expect("metal fx child layout")
    }

    /// Advances the glow to the widget clock and builds the frame's scene.
    fn scene_for(
        &self,
        inner: &mut Inner,
        bounds: Rectangle,
        theme: MetalFxResolvedTheme,
    ) -> Scene {
        let host = MetalFxHost::new(bounds.width, bounds.height);
        let config: MetalGlowConfig = glow_config();
        let options = &self.options;
        let mode = options.resolved_mode(theme);
        let material = MetalMaterial::new(mode);
        let u_seconds = inner.clock as f32;
        let mut frame = inner.glow_frame;

        let radius = options.resolved_radius(host.width(), host.height());
        let mapping =
            SheetMapping::new(host.width(), host.height(), options.resolved_shader_scale());
        let sampler = MaterialSampler::new(material, mapping, material.u_time(u_seconds));

        // Cursor-light proximity (catch + spill) — runs even when glow is off
        // so spill/sprite APIs stay live; catch needs glow below.
        if !options.is_paused() {
            let dt = 1.0 / 60.0;
            inner.tick_cursor_light(
                host.width(),
                host.height(),
                radius,
                options.variant,
                dt,
                &sampler,
            );
        }

        if options.has_glow() {
            inner.glow.configure(
                host.width(),
                host.height(),
                radius,
                options.variant,
                options.scale,
                &config,
            );
            // A paused instance keeps its halo frozen (but still shows the
            // frame it had); the very first frame is ticked once regardless.
            if !options.is_paused() || frame.is_none() {
                frame = inner.glow.tick(&GlowTick {
                    now_ms: inner.clock * 1000.0,
                    strength: (options.strength * options.glow_gain).clamp(0.0, 1.0),
                    theme,
                    sampler: &sampler,
                    config: &config,
                    cursor_light: inner.cursor_light,
                });
                inner.glow_frame = frame;
            }
        }

        if options.has_bend() {
            let radius = options.resolved_radius(host.width(), host.height());
            let bend_cfg = bend_config();
            if !options.is_paused() || inner.bend_field.is_none() {
                inner.tick_bend(host.width(), host.height(), radius, &bend_cfg);
            }
        } else {
            inner.bend_field = None;
        }

        options.scene(&SceneParams {
            host,
            theme,
            density: 1.0,
            elapsed_seconds: u_seconds,
            glow: frame,
            config: &config,
            target_srgb: false,
            bend: inner.bend_field,
        })
    }

    fn publish_anchor(&self, inner: &Inner, bounds: Rectangle, theme: MetalFxResolvedTheme) {
        let Some(id) = self.options.anchor_id_value() else {
            return;
        };
        let host = MetalFxHost::new(bounds.width, bounds.height);
        let radius = self.options.resolved_radius(host.width(), host.height());
        let is_sheet = !matches!(self.options.mask, MetalFxMask::Ring);
        register_reflection_anchor(
            id,
            MetalReflectionAnchor {
                origin: [bounds.x, bounds.y],
                width: host.width(),
                height: host.height(),
                radius,
                variant: self.options.variant,
                mode: self.options.resolved_mode(theme),
                theme,
                clock: inner.clock as f32,
                strength: self.options.strength,
                ring_px: self.options.resolved_ring_px(),
                shader_scale: self.options.resolved_shader_scale(),
                is_sheet,
            },
        );
    }
}

fn place(renderer: &mut Renderer, scene: &Arc<Scene>, bounds: Rectangle, layer: Layer) {
    if !scene.has_layer(layer) {
        return;
    }
    let (origin, size) = scene.quad(layer);
    renderer.draw_primitive(
        Rectangle {
            x: bounds.x + origin[0],
            y: bounds.y + origin[1],
            width: size[0],
            height: size[1],
        },
        MetalFxPrimitive::new(Arc::clone(scene), layer),
    );
}

/// Soft radial glint under the pointer (`spill` effect).
///
/// Matches the web's `radial-gradient(closest-side, … 0% / 0.35@45% / 0@100%)`
/// — a single soft disc, not hard concentric quads.
fn draw_spill(renderer: &mut Renderer, bounds: Rectangle, spill: MetalCursorSpill) {
    if spill.opacity < 0.01 || spill.radius < 0.5 {
        return;
    }
    // Bake at 2× so the soft falloff stays smooth when drawn into CSS px.
    let px = (spill.radius * 2.0).ceil().max(2.0) as u32;
    let (w, h, rgba) = bake_spill_rgba(px, spill.tint, spill.opacity);
    let handle = image::Handle::from_rgba(w, h, rgba);
    let r = spill.radius;
    let rect = Rectangle {
        x: bounds.x + spill.position[0] - r,
        y: bounds.y + spill.position[1] - r,
        width: r * 2.0,
        height: r * 2.0,
    };
    renderer.draw_image(
        image::Image::new(handle).filter_method(image::FilterMethod::Linear),
        rect,
        rect,
    );
}

impl<Message> Widget<Message, Theme, Renderer> for MetalFx<'_, Message> {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<MetalState>()
    }

    fn state(&self) -> widget::tree::State {
        // A distinct seed per instance keeps neighbouring halos from wandering
        // in lock-step.
        static NEXT_SEED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let seed = NEXT_SEED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        widget::tree::State::new(MetalState {
            inner: RefCell::new(Inner::new(seed)),
        })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.child)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.child));
    }

    fn size(&self) -> Size<Length> {
        self.child.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let node = self
            .child
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        let size = node.size();
        layout::Node::with_children(size, vec![node])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        self.child.as_widget_mut().operate(
            &mut tree.children[0],
            Self::child_layout(layout),
            renderer,
            operation,
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.child.as_widget_mut().update(
            &mut tree.children[0],
            event,
            Self::child_layout(layout),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let bounds = layout.bounds();
        let state = tree.state.downcast_ref::<MetalState>();
        {
            let mut inner = state.inner.borrow_mut();
            match event {
                Event::Mouse(mouse::Event::CursorMoved { position }) => {
                    let local = [position.x - bounds.x, position.y - bounds.y];
                    let now = Instant::now();
                    if let Some((prev, t0)) = inner.last_pointer {
                        let dt = now.saturating_duration_since(t0).as_secs_f32().max(0.004);
                        let a = 1.0 - (-dt / 0.04).exp();
                        let vx = (local[0] - prev[0]) / dt;
                        let vy = (local[1] - prev[1]) / dt;
                        inner.pointer_vel[0] += (vx - inner.pointer_vel[0]) * a;
                        inner.pointer_vel[1] += (vy - inner.pointer_vel[1]) * a;
                    }
                    inner.last_pointer = Some((local, now));
                    let cl = cursor_light_config();
                    let reach = bend_config().reach.max(cl.reach).max(cl.cursor_distance)
                        + bounds.width.max(bounds.height) * 0.5;
                    let cx = bounds.width * 0.5;
                    let cy = bounds.height * 0.5;
                    let d = (local[0] - cx).hypot(local[1] - cy);
                    inner.pointer = (d < reach).then_some(local);
                    if self.options.has_bend() || cl.enabled {
                        shell.request_redraw();
                    }
                }
                Event::Mouse(mouse::Event::CursorLeft) => {
                    inner.pointer = None;
                    inner.last_pointer = None;
                    if self.options.has_bend() || cursor_light_config().enabled {
                        shell.request_redraw();
                    }
                }
                Event::Mouse(mouse::Event::ButtonPressed(_)) => {
                    inner.pressed = true;
                    if self.options.has_bend() {
                        shell.request_redraw();
                    }
                }
                Event::Mouse(mouse::Event::ButtonReleased(_)) => {
                    inner.pressed = false;
                    if self.options.has_bend() {
                        shell.request_redraw();
                    }
                }
                _ => {}
            }
        }

        let Event::Window(window::Event::RedrawRequested(now)) = event else {
            return;
        };
        let mut inner = state.inner.borrow_mut();
        if !within_margin(bounds, viewport) {
            // Off-screen instances cost nothing and don't keep the loop alive.
            inner.last = None;
            return;
        }
        let dt = inner.last.map_or(0.0, |last| {
            now.saturating_duration_since(last).as_secs_f64()
        });
        inner.last = Some(*now);
        // Velocity decay (matches useMetalBend's exp(-dt*12)).
        let decay = (-(dt as f32) * 12.0).exp();
        inner.pointer_vel[0] *= decay;
        inner.pointer_vel[1] *= decay;
        if self.options.is_paused() {
            return;
        }
        inner.clock += dt.min(0.25);
        drop(inner);
        // Continuous clock + bend springs need a steady redraw cadence.
        shell.request_redraw_at(*now + FRAME);
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let draw_child = |renderer: &mut Renderer| {
            self.child.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                Self::child_layout(layout),
                cursor,
                viewport,
            );
        };
        if bounds.width < 1.0 || bounds.height < 1.0 || !bounds.intersects(viewport) {
            draw_child(renderer);
            return;
        }

        let resolved = self.options.resolved_theme(ambient_is_dark(theme));
        let (scene, spill) = {
            let state = tree.state.downcast_ref::<MetalState>();
            let mut inner = state.inner.borrow_mut();
            let scene = Arc::new(self.scene_for(&mut inner, bounds, resolved));
            self.publish_anchor(&inner, bounds, resolved);
            (scene, inner.spill)
        };

        match self.options.layer {
            MetalFxLayer::BehindChild => {
                for layer in [Layer::Surface, Layer::Ring, Layer::Glow, Layer::Rim] {
                    place(renderer, &scene, bounds, layer);
                }
                renderer.with_layer(*viewport, draw_child);
            }
            MetalFxLayer::OverChild => {
                place(renderer, &scene, bounds, Layer::Surface);
                renderer.with_layer(*viewport, draw_child);
                let reach = [Layer::Ring, Layer::Glow, Layer::Rim]
                    .into_iter()
                    .filter(|l| scene.has_layer(*l))
                    .map(|l| -scene.quad(l).0[0])
                    .fold(0.0_f32, f32::max);
                let expanded = Rectangle {
                    x: bounds.x - reach,
                    y: bounds.y - reach,
                    width: bounds.width + 2.0 * reach,
                    height: bounds.height + 2.0 * reach,
                };
                if let Some(clip) = expanded.intersection(viewport) {
                    renderer.with_layer(clip, |renderer| {
                        for layer in [Layer::Ring, Layer::Glow, Layer::Rim] {
                            place(renderer, &scene, bounds, layer);
                        }
                    });
                }
            }
        }
        if let Some(spill) = spill {
            draw_spill(renderer, bounds, spill);
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.child.as_widget().mouse_interaction(
            &tree.children[0],
            Self::child_layout(layout),
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.child.as_widget_mut().overlay(
            &mut tree.children[0],
            Self::child_layout(layout),
            renderer,
            viewport,
            translation,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::container;

    fn metal() -> MetalFx<'static, ()> {
        MetalFx::new(container("child"))
    }

    #[test]
    fn builder_forwards_every_option() {
        let m = metal()
            .variant(MetalFxVariant::Circle)
            .preset(MetalFxPreset::Gold)
            .theme(MetalFxTheme::Light)
            .strength(0.5)
            .glow_gain(2.0)
            .paused(true)
            .border_radius(6.0)
            .glow(false)
            .inner_shadow(true)
            .shader_scale(2.0)
            .ring_px(3.0)
            .scale(1.5)
            .surface(MetalFxSurface::Transparent)
            .layer(MetalFxLayer::BehindChild);
        let o = m.options();
        assert_eq!(o.variant_value(), MetalFxVariant::Circle);
        assert_eq!(o.preset_value(), MetalFxPreset::Gold);
        assert_eq!(o.theme_value(), MetalFxTheme::Light);
        assert_eq!(o.strength_value(), 0.5);
        assert!(o.is_paused() && !o.has_glow());
        assert!(o.inner_shadow_options().is_some());
        assert_eq!(o.resolved_ring_px(), 3.0);
        assert_eq!(o.resolved_shader_scale(), 2.0);
        assert_eq!(o.resolved_radius(40.0, 20.0), 10.0);
    }

    #[test]
    fn with_options_replaces_everything() {
        let o = MetalFxOptions::new().preset(MetalFxPreset::Silver);
        let m = metal().strength(0.1).with_options(o.clone());
        assert_eq!(m.options(), &o);
    }

    #[test]
    fn scene_ticks_the_glow_and_freezes_when_paused() {
        let m = metal().theme(MetalFxTheme::Dark);
        let mut inner = Inner::new(1);
        let bounds = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 134.0,
            height: 40.0,
        };
        inner.clock = 0.5;
        let a = m.scene_for(&mut inner, bounds, MetalFxResolvedTheme::Dark);
        assert!(a.glow.is_some());
        let first = inner.glow_frame;
        assert!(first.is_some());

        let paused = metal().theme(MetalFxTheme::Dark).paused(true);
        inner.clock = 3.0;
        let b = paused.scene_for(&mut inner, bounds, MetalFxResolvedTheme::Dark);
        assert_eq!(
            inner.glow_frame, first,
            "a paused glow keeps its last frame"
        );
        assert!(b.glow.is_some());

        let no_glow = metal().glow(false);
        let c = no_glow.scene_for(&mut inner, bounds, MetalFxResolvedTheme::Dark);
        assert!(c.glow.is_none());
    }

    #[test]
    fn scene_carries_the_clock_and_strength() {
        let m = metal().theme(MetalFxTheme::Dark).strength(0.5).glow(false);
        let mut inner = Inner::new(1);
        inner.clock = 2.0;
        let bounds = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 30.0,
        };
        let s = m.scene_for(&mut inner, bounds, MetalFxResolvedTheme::Dark);
        assert_eq!(s.u_time, 2.0);
        assert!(
            (s.opacity_mul
                - 0.5
                    * MetalFxPreset::Chromatic
                        .mode(MetalFxResolvedTheme::Dark)
                        .shader_opacity)
                .abs()
                < 1e-6
        );
        assert_eq!(s.radius, 15.0);
    }

    #[test]
    fn visibility_margin_matches_the_reference() {
        let viewport = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 300.0,
        };
        let near = Rectangle {
            x: 0.0,
            y: 340.0,
            width: 50.0,
            height: 20.0,
        };
        let far = Rectangle {
            x: 0.0,
            y: 420.0,
            width: 50.0,
            height: 20.0,
        };
        assert!(within_margin(near, &viewport));
        assert!(!within_margin(far, &viewport));
    }
}
