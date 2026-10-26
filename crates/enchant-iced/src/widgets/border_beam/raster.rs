//! The software rasterizer shared by both iced backends.
//!
//! Layers are painted as premultiplied float planes at the requested pixel
//! density, masked with analytic coverage, blurred with the same gaussian
//! approximation browsers use, filtered with the CSS colour matrices and then
//! converted to RGBA8 images. `above` is drawn over the child; `behind` (only
//! `pulse-outside`) is drawn under it and carries its own logical offset.

use std::f32::consts::PI;

use super::blur::blur;
use super::data::{
    PALETTES, PULSE_INNER_BLOOM, PULSE_INNER_SIZES, PULSE_OUTER_BLOOM, PULSE_OUTER_CORE,
    PULSE_RING_MAP,
};
use super::frame::{BorderBeamFrame, LineFrame, PulseFrame};
use super::options::BorderBeamSpec;
use super::paint::{
    Fx, GradientStop, Plane, Px, coverage, over, paint_conic, paint_radial, sd_round_rect, stop,
};
use super::palette::{
    BorderBeamSize, GradientDef, LineBloomSpike, PaletteColors, PulseGradientDef,
    ResolvedBeamTheme, Rgba,
};

/// A raster image ready for `iced::advanced::image::Handle::from_rgba`.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::{
///     BeamHost, BeamRenderOptions, BorderBeamOptions, ResolvedBeamTheme,
/// };
/// use std::time::Duration;
///
/// let layers = BorderBeamOptions::new().render_at(
///     Duration::ZERO,
///     BeamHost::new(348.0, 122.0),
///     ResolvedBeamTheme::Dark,
///     &BeamRenderOptions::new(),
/// );
/// let bitmap = layers.above().expect("md always has an above layer");
/// assert_eq!(bitmap.rgba().len(), bitmap.width() as usize * bitmap.height() as usize * 4);
/// ```
#[must_use = "a bitmap does nothing unless drawn or inspected"]
#[derive(Clone, Debug, PartialEq)]
pub struct BeamBitmap {
    width: u32,
    height: u32,
    left: f32,
    top: f32,
    density: f32,
    rgba: Vec<u8>,
}

impl BeamBitmap {
    /// Image width in device pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Image height in device pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Logical x of the top-left corner relative to the wrapped child.
    pub fn left(&self) -> f32 {
        self.left
    }

    /// Logical y of the top-left corner relative to the wrapped child.
    pub fn top(&self) -> f32 {
        self.top
    }

    /// Device pixels per logical pixel.
    pub fn density(&self) -> f32 {
        self.density
    }

    /// Straight RGBA8 pixels, row-major.
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    /// Consumes the bitmap and returns its pixels.
    pub fn into_rgba(self) -> Vec<u8> {
        self.rgba
    }
}

/// The images produced for one frame.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::{
///     BeamHost, BeamRenderOptions, BorderBeamOptions, BorderBeamSize, ResolvedBeamTheme,
/// };
/// use std::time::Duration;
///
/// let layers = BorderBeamOptions::new()
///     .size(BorderBeamSize::PulseOutside)
///     .render_at(
///         Duration::from_secs_f32(0.8),
///         BeamHost::new(348.0, 122.0),
///         ResolvedBeamTheme::Dark,
///         &BeamRenderOptions::new(),
///     );
/// assert!(layers.behind().is_some());
/// ```
#[must_use = "layers do nothing unless drawn or inspected"]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BeamLayers {
    above: Option<BeamBitmap>,
    behind: Option<BeamBitmap>,
}

impl BeamLayers {
    /// The image drawn over the child (`md`, `sm`, `line`, `pulse-inner`, and
    /// the stroke of `pulse-outside`).
    pub fn above(&self) -> Option<&BeamBitmap> {
        self.above.as_ref()
    }

    /// The image drawn behind the child (only `pulse-outside`).
    pub fn behind(&self) -> Option<&BeamBitmap> {
        self.behind.as_ref()
    }

    /// Whether neither layer was allocated (a layer may still be fully
    /// transparent when the effect is clipped away).
    pub fn is_empty(&self) -> bool {
        self.above.is_none() && self.behind.is_none()
    }

    /// Consumes the layers, returning `(above, behind)` without copying the
    /// pixel buffers.
    pub fn into_parts(self) -> (Option<BeamBitmap>, Option<BeamBitmap>) {
        (self.above, self.behind)
    }
}

/// A reusable rasterizer. Keep one per widget: it retains the float buffers
/// between frames.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::{
///     BeamHost, BorderBeamOptions, BorderBeamRaster, ResolvedBeamTheme,
/// };
/// use std::time::Duration;
///
/// let frame = BorderBeamOptions::new().frame_at(
///     Duration::from_secs_f32(0.5),
///     BeamHost::new(348.0, 122.0),
///     ResolvedBeamTheme::Dark,
/// );
/// let mut raster = BorderBeamRaster::new();
/// let layers = raster.render(&frame, 1.0, 2.0);
/// assert!(layers.above().is_some());
/// ```
#[derive(Clone, Default)]
pub struct BorderBeamRaster {
    scratch: Scratch,
    above: Plane,
    behind: Plane,
}

#[derive(Clone, Debug, Default)]
struct Scratch {
    plane: Plane,
    tmp: Vec<Px>,
}

impl std::fmt::Debug for BorderBeamRaster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BorderBeamRaster")
            .field("above", &(self.above.w, self.above.h))
            .field("behind", &(self.behind.w, self.behind.h))
            .finish_non_exhaustive()
    }
}

impl BorderBeamRaster {
    /// Creates an empty rasterizer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Rasterizes `frame` at `density` device pixels per logical pixel.
    ///
    /// `opacity` is the fade multiplier in `0..=1`; the resolved
    /// [`strength`](BorderBeamSpec::strength) is applied on top of it, exactly
    /// like the reference's `--beam-strength`.
    ///
    /// Density is lowered automatically when the host would exceed 4
    /// megapixels or 8192 pixels on an axis, so very large cards render at a
    /// coarser resolution instead of exhausting memory or coming back empty.
    pub fn render(&mut self, frame: &BorderBeamFrame, opacity: f32, density: f32) -> BeamLayers {
        let spec = &frame.spec;
        let host = frame.host;
        let density = if density.is_finite() {
            density.clamp(0.5, 4.0)
        } else {
            1.0
        };
        let opacity = if opacity.is_finite() {
            opacity.clamp(0.0, 1.0) * spec.strength.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let (host_w, host_h) = (host.width(), host.height());
        let density = effective_density(host_w, host_h, density);
        let (w, h) = device_size(host_w, host_h, density);
        if w == 0 || h == 0 || opacity <= 0.0 {
            return BeamLayers::default();
        }

        let radius = spec.radius.min(host_w.min(host_h) * 0.5).max(0.0);
        let theme = spec.theme;
        let palette = PALETTES[spec.palette.index()];

        self.above.reset(w, h, host_w, host_h);
        let behind_margin = outside_margin(spec);
        if behind_margin > 0.0 {
            let pad = (behind_margin * density).ceil() as usize;
            self.behind
                .reset_padded((w, h), (host_w, host_h), (pad, pad));
        } else {
            self.behind = Plane::default();
        }

        match spec.size {
            BorderBeamSize::Sm => {
                draw_rotating(
                    &mut self.scratch,
                    &mut self.above,
                    frame,
                    density,
                    opacity,
                    true,
                    &palette,
                    theme,
                    radius,
                );
            }
            BorderBeamSize::Md => {
                draw_rotating(
                    &mut self.scratch,
                    &mut self.above,
                    frame,
                    density,
                    opacity,
                    false,
                    &palette,
                    theme,
                    radius,
                );
            }
            BorderBeamSize::Line => {
                draw_line(
                    &mut self.scratch,
                    &mut self.above,
                    frame,
                    density,
                    opacity,
                    &palette,
                    theme,
                    radius,
                );
            }
            BorderBeamSize::PulseInner => {
                draw_pulse_inner(
                    &mut self.scratch,
                    &mut self.above,
                    frame,
                    density,
                    opacity,
                    &palette,
                    theme,
                    radius,
                );
            }
            BorderBeamSize::PulseOutside => {
                draw_pulse_outside(
                    &mut self.scratch,
                    &mut self.above,
                    &mut self.behind,
                    frame,
                    density,
                    opacity,
                    &palette,
                    theme,
                    radius,
                );
            }
        }

        // The wrapper clips every above-child layer to its rounded border.
        apply_rounded_clip(&mut self.above, 0.0, 0.0, host_w, host_h, radius);

        let above = bitmap_from_plane(&self.above, 0.0, 0.0, density);
        let behind = (behind_margin > 0.0)
            .then(|| bitmap_from_plane(&self.behind, -behind_margin, -behind_margin, density));
        BeamLayers {
            above: Some(above),
            behind,
        }
    }
}

// ── sizes / bitmaps ────────────────────────────────────────────────────────

const MAX_DEVICE_SIDE: f32 = 8192.0;
const MAX_DEVICE_PIXELS: f32 = 4_000_000.0;

fn effective_density(host_w: f32, host_h: f32, density: f32) -> f32 {
    let pixels = host_w.max(1.0) * host_h.max(1.0);
    let by_pixels = (MAX_DEVICE_PIXELS / pixels).sqrt();
    let by_side = (MAX_DEVICE_SIDE / host_w.max(1.0)).min(MAX_DEVICE_SIDE / host_h.max(1.0));
    density.min(by_pixels).min(by_side).max(0.005)
}

fn device_size(host_w: f32, host_h: f32, density: f32) -> (usize, usize) {
    let w = (host_w * density).round().clamp(0.0, MAX_DEVICE_SIDE) as usize;
    let h = (host_h * density).round().clamp(0.0, MAX_DEVICE_SIDE) as usize;
    (w, h)
}

fn bitmap_from_plane(plane: &Plane, left: f32, top: f32, density: f32) -> BeamBitmap {
    let mut rgba = Vec::with_capacity(plane.w * plane.h * 4);
    for p in &plane.px {
        let a = p[3].clamp(0.0, 1.0);
        if a <= 1e-6 {
            rgba.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            let inv = 1.0 / a;
            rgba.push(((p[0] * inv).clamp(0.0, 1.0) * 255.0).round() as u8);
            rgba.push(((p[1] * inv).clamp(0.0, 1.0) * 255.0).round() as u8);
            rgba.push(((p[2] * inv).clamp(0.0, 1.0) * 255.0).round() as u8);
            rgba.push((a * 255.0).round() as u8);
        }
    }
    BeamBitmap {
        width: plane.w as u32,
        height: plane.h as u32,
        left,
        top,
        density,
        rgba,
    }
}

// ── layer plumbing ─────────────────────────────────────────────────────────

fn begin_layer(
    scratch: &mut Scratch,
    host_w: f32,
    host_h: f32,
    density: f32,
    pad_host: f32,
) -> usize {
    let (w, h) = device_size(host_w, host_h, density);
    let pad = (pad_host * density).ceil().max(0.0) as usize;
    scratch
        .plane
        .reset_padded((w, h), (host_w, host_h), (pad, pad));
    pad
}

fn blur_layer(scratch: &mut Scratch, sigma_x: f32, sigma_y: f32) {
    let (sx, sy) = (sigma_x * scratch.plane.kx, sigma_y * scratch.plane.ky);
    blur(
        &mut scratch.plane.px,
        scratch.plane.w,
        scratch.plane.h,
        sx,
        sy,
        &mut scratch.tmp,
    );
}

fn fx_layer(scratch: &mut Scratch, hue: f32, brightness: f32, saturation: f32) {
    Fx::new(hue, brightness, saturation).apply_plane(&mut scratch.plane.px);
}

fn composite_crop(out: &mut Plane, layer: &Plane, pad: usize, opacity: f32) {
    for j in 0..out.h {
        for i in 0..out.w {
            let src = layer.px[(j + pad) * layer.w + (i + pad)];
            over_scaled(&mut out.px[j * out.w + i], src, opacity);
        }
    }
}

fn composite_same(out: &mut Plane, layer: &Plane, opacity: f32) {
    let n = out.w.min(layer.w) * out.h.min(layer.h);
    for i in 0..n {
        over_scaled(&mut out.px[i], layer.px[i], opacity);
    }
}

#[inline]
fn over_scaled(dst: &mut Px, src: Px, opacity: f32) {
    over(
        dst,
        [
            src[0] * opacity,
            src[1] * opacity,
            src[2] * opacity,
            src[3] * opacity,
        ],
    );
}

// ── gradients ──────────────────────────────────────────────────────────────

fn paint_def(plane: &mut Plane, def: &GradientDef, host_w: f32, host_h: f32) {
    paint_def_alpha(plane, def, host_w, host_h, 1.0);
}

/// Paints one palette gradient with its authored alpha scaled by `alpha`.
fn paint_def_alpha(plane: &mut Plane, def: &GradientDef, host_w: f32, host_h: f32, alpha: f32) {
    let stops = [
        GradientStop {
            at: 0.0,
            color: def.color.with_alpha(def.color.a * alpha),
        },
        GradientStop {
            at: 1.0,
            color: Rgba::TRANSPARENT,
        },
    ];
    paint_radial(
        plane,
        def.cx * host_w,
        def.cy * host_h,
        def.w,
        def.h,
        &stops,
    );
}

fn paint_background(plane: &mut Plane, defs: &[GradientDef], host_w: f32, host_h: f32) {
    for def in defs.iter().rev() {
        paint_def(plane, def, host_w, host_h);
    }
}

// ── masks ──────────────────────────────────────────────────────────────────

fn apply_mask(plane: &mut Plane, mask: impl Fn(f32, f32) -> f32) {
    for j in 0..plane.h {
        let y = plane.y(j);
        for i in 0..plane.w {
            let m = mask(plane.x(i), y).clamp(0.0, 1.0);
            if m <= 0.0 {
                let p = &mut plane.px[j * plane.w + i];
                *p = [0.0; 4];
            } else if m < 1.0 {
                for c in &mut plane.px[j * plane.w + i] {
                    *c *= m;
                }
            }
        }
    }
}

fn apply_rounded_clip(plane: &mut Plane, x0: f32, y0: f32, x1: f32, y1: f32, radius: f32) {
    let k = (plane.kx + plane.ky) * 0.5;
    apply_mask(plane, |x, y| {
        coverage(sd_round_rect(x, y, x0, y0, x1, y1, radius), k)
    });
}

/// `outer_radius` ring minus a rounded inner box inset by `inset`.
fn ring_mask(
    plane: &mut Plane,
    host_w: f32,
    host_h: f32,
    outer_radius: f32,
    inset: f32,
    inner_radius: f32,
) {
    let k = (plane.kx + plane.ky) * 0.5;
    apply_mask(plane, |x, y| {
        let outer = coverage(
            sd_round_rect(x, y, 0.0, 0.0, host_w, host_h, outer_radius),
            k,
        );
        let inner = coverage(
            sd_round_rect(
                x,
                y,
                inset,
                inset,
                host_w - inset,
                host_h - inset,
                inner_radius,
            ),
            k,
        );
        outer * (1.0 - inner)
    });
}

/// The two 28px-wide edge bands, unioned.
fn apply_edge_bands(plane: &mut Plane, host_w: f32, host_h: f32, band: f32) {
    apply_mask(plane, |x, y| {
        let v = (1.0 - y / band)
            .clamp(0.0, 1.0)
            .max(((y - (host_h - band)) / band).clamp(0.0, 1.0));
        let h = (1.0 - x / band)
            .clamp(0.0, 1.0)
            .max(((x - (host_w - band)) / band).clamp(0.0, 1.0));
        v.max(h)
    });
}

/// A radial alpha mask with explicit `(position, alpha)` stops.
fn apply_ellipse_mask(plane: &mut Plane, cx: f32, cy: f32, rx: f32, ry: f32, stops: &[(f32, f32)]) {
    if rx <= 0.0 || ry <= 0.0 || stops.is_empty() {
        apply_mask(plane, |_, _| 0.0);
        return;
    }
    let (inv_rx, inv_ry) = (1.0 / rx, 1.0 / ry);
    apply_mask(plane, |x, y| {
        let nx = (x - cx) * inv_rx;
        let ny = (y - cy) * inv_ry;
        ellipse_stops(stops, (nx * nx + ny * ny).sqrt())
    });
}

fn ellipse_stops(stops: &[(f32, f32)], d: f32) -> f32 {
    if d <= stops[0].0 {
        return stops[0].1;
    }
    for pair in stops.windows(2) {
        let (a, va) = pair[0];
        let (b, vb) = pair[1];
        if d <= b {
            let t = if b > a { (d - a) / (b - a) } else { 1.0 };
            return va + (vb - va) * t;
        }
    }
    0.0
}

// ── md / sm ────────────────────────────────────────────────────────────────

const WHITE_CONIC_DARK: &[GradientStop] = &[
    stop(0.0, 255, 255, 255, 0.0),
    stop(0.54, 255, 255, 255, 0.0),
    stop(0.57, 255, 255, 255, 0.1),
    stop(0.60, 255, 255, 255, 0.3),
    stop(0.63, 255, 255, 255, 0.6),
    stop(0.66, 255, 255, 255, 0.75),
    stop(0.69, 255, 255, 255, 0.6),
    stop(0.72, 255, 255, 255, 0.3),
    stop(0.75, 255, 255, 255, 0.1),
    stop(0.78, 255, 255, 255, 0.0),
    stop(1.0, 255, 255, 255, 0.0),
];

const WHITE_CONIC_LIGHT: &[GradientStop] = &[
    stop(0.0, 0, 0, 0, 0.0),
    stop(0.54, 0, 0, 0, 0.0),
    stop(0.57, 0, 0, 0, 0.08),
    stop(0.60, 0, 0, 0, 0.2),
    stop(0.63, 0, 0, 0, 0.4),
    stop(0.66, 0, 0, 0, 0.55),
    stop(0.69, 0, 0, 0, 0.4),
    stop(0.72, 0, 0, 0, 0.2),
    stop(0.75, 0, 0, 0, 0.08),
    stop(0.78, 0, 0, 0, 0.0),
    stop(1.0, 0, 0, 0, 0.0),
];

const BLOOM_CONIC_DARK: &[GradientStop] = &[
    stop(0.0, 255, 255, 255, 0.0),
    stop(0.58, 255, 255, 255, 0.0),
    stop(0.62, 255, 255, 255, 0.03),
    stop(0.65, 255, 255, 255, 0.08),
    stop(0.67, 255, 255, 255, 0.2),
    stop(0.69, 255, 255, 255, 0.45),
    stop(0.70, 255, 255, 255, 0.85),
    stop(0.705, 255, 255, 255, 0.85),
    stop(0.715, 255, 255, 255, 0.45),
    stop(0.73, 255, 255, 255, 0.2),
    stop(0.75, 255, 255, 255, 0.08),
    stop(0.78, 255, 255, 255, 0.03),
    stop(0.82, 255, 255, 255, 0.0),
    stop(1.0, 255, 255, 255, 0.0),
];

const BLOOM_CONIC_LIGHT: &[GradientStop] = &[
    stop(0.0, 0, 0, 0, 0.0),
    stop(0.58, 0, 0, 0, 0.0),
    stop(0.62, 0, 0, 0, 0.02),
    stop(0.65, 0, 0, 0, 0.08),
    stop(0.67, 0, 0, 0, 0.2),
    stop(0.69, 0, 0, 0, 0.4),
    stop(0.70, 0, 0, 0, 0.6),
    stop(0.705, 0, 0, 0, 0.6),
    stop(0.715, 0, 0, 0, 0.4),
    stop(0.73, 0, 0, 0, 0.2),
    stop(0.75, 0, 0, 0, 0.08),
    stop(0.78, 0, 0, 0, 0.02),
    stop(0.82, 0, 0, 0, 0.0),
    stop(1.0, 0, 0, 0, 0.0),
];

const BEAM_MASK: &[GradientStop] = &[
    stop(0.0, 255, 255, 255, 0.0),
    stop(0.30, 255, 255, 255, 0.0),
    stop(0.36, 255, 255, 255, 0.1),
    stop(0.44, 255, 255, 255, 0.35),
    stop(0.52, 255, 255, 255, 1.0),
    stop(0.80, 255, 255, 255, 1.0),
    stop(0.86, 255, 255, 255, 0.35),
    stop(0.92, 255, 255, 255, 0.1),
    stop(0.95, 255, 255, 255, 0.0),
    stop(1.0, 255, 255, 255, 0.0),
];

const SMALL_MASK: &[GradientStop] = &[
    stop(0.0, 255, 255, 255, 0.0),
    stop(0.22, 255, 255, 255, 0.0),
    stop(0.28, 255, 255, 255, 0.12),
    stop(0.36, 255, 255, 255, 0.4),
    stop(0.46, 255, 255, 255, 1.0),
    stop(0.82, 255, 255, 255, 1.0),
    stop(0.88, 255, 255, 255, 0.4),
    stop(0.94, 255, 255, 255, 0.12),
    stop(0.97, 255, 255, 255, 0.0),
    stop(1.0, 255, 255, 255, 0.0),
];

#[allow(clippy::too_many_arguments)]
fn draw_rotating(
    scratch: &mut Scratch,
    out: &mut Plane,
    frame: &BorderBeamFrame,
    density: f32,
    opacity: f32,
    small: bool,
    palette: &PaletteColors,
    theme: ResolvedBeamTheme,
    radius: f32,
) {
    let spec = &frame.spec;
    let (w, h) = (frame.host.width(), frame.host.height());
    let bw = spec.border_width.min(w.min(h) * 0.5);
    let inner_radius = (radius - bw).max(0.0);
    let static_colors = spec.static_colors;
    let fx = if static_colors {
        None
    } else {
        Some(Fx::new(frame.hue_degrees, spec.brightness, spec.saturation))
    };

    // Stroke ring.
    let pad = begin_layer(scratch, w, h, density, 0.0);
    if small {
        paint_background(&mut scratch.plane, &palette.small_border, w, h);
    } else {
        paint_background(&mut scratch.plane, &palette.border, w, h);
    }
    let white = if theme.is_dark() {
        WHITE_CONIC_DARK
    } else {
        WHITE_CONIC_LIGHT
    };
    paint_conic(
        &mut scratch.plane,
        w * 0.5,
        h * 0.5,
        frame.angle_degrees,
        white,
    );
    if small {
        apply_mask(&mut scratch.plane, |x, y| {
            conic_mask(w, h, x, y, frame.angle_degrees, SMALL_MASK)
        });
    } else {
        apply_mask(&mut scratch.plane, |x, y| {
            conic_mask(w, h, x, y, frame.angle_degrees, BEAM_MASK)
        });
    }
    ring_mask(
        &mut scratch.plane,
        w,
        h,
        inner_radius,
        bw,
        (inner_radius - bw).max(0.0),
    );
    if let Some(fx) = fx {
        fx.apply_plane(&mut scratch.plane.px);
    }
    composite_crop(out, &scratch.plane, pad, opacity * spec.stroke_opacity);

    // Inner glow.
    let pad = begin_layer(scratch, w, h, density, 0.0);
    if small {
        paint_background(&mut scratch.plane, &palette.small_inner, w, h);
    } else {
        paint_inner_gradients(&mut scratch.plane, palette, spec.palette.is_mono(), w, h);
    }
    paint_inset_shadow(
        &mut scratch.plane,
        w,
        h,
        radius,
        if small { 5.0 } else { 9.0 },
        spec.inner_shadow,
    );
    if small {
        apply_mask(&mut scratch.plane, |x, y| {
            conic_mask(w, h, x, y, frame.angle_degrees, SMALL_MASK)
        });
    } else {
        apply_mask(&mut scratch.plane, |x, y| {
            conic_mask(w, h, x, y, frame.angle_degrees, BEAM_MASK)
        });
        apply_edge_bands(&mut scratch.plane, w, h, 28.0);
    }
    if let Some(fx) = fx {
        fx.apply_plane(&mut scratch.plane.px);
    }
    composite_crop(out, &scratch.plane, pad, opacity * spec.inner_opacity);

    // Bloom ring.
    let bloom_blur = scale_blur(8.0, spec.glow_size);
    let pad = begin_layer(scratch, w, h, density, 3.0 * bloom_blur + 1.0);
    let bloom = if theme.is_dark() {
        BLOOM_CONIC_DARK
    } else {
        BLOOM_CONIC_LIGHT
    };
    paint_conic(
        &mut scratch.plane,
        w * 0.5,
        h * 0.5,
        frame.angle_degrees,
        bloom,
    );
    ring_mask(
        &mut scratch.plane,
        w,
        h,
        inner_radius,
        bw,
        (inner_radius - bw).max(0.0),
    );
    blur_layer(scratch, bloom_blur, bloom_blur);
    fx_layer(scratch, 0.0, spec.brightness, spec.saturation);
    composite_crop(out, &scratch.plane, pad, opacity * spec.bloom_opacity);
}

fn conic_mask(w: f32, h: f32, x: f32, y: f32, angle: f32, stops: &[GradientStop]) -> f32 {
    let dx = x - w * 0.5;
    let dy = y - h * 0.5;
    let deg = dx.atan2(-dy).to_degrees().rem_euclid(360.0);
    let p = ((deg - angle).rem_euclid(360.0)) / 360.0;
    sample_alpha(stops, p)
}

fn sample_alpha(stops: &[GradientStop], p: f32) -> f32 {
    if p <= stops[0].at {
        return stops[0].color.a;
    }
    for pair in stops.windows(2) {
        let a = pair[0];
        let b = pair[1];
        if p <= b.at {
            let t = if b.at > a.at {
                (p - a.at) / (b.at - a.at)
            } else {
                1.0
            };
            return a.color.a + (b.color.a - a.color.a) * t;
        }
    }
    stops[stops.len() - 1].color.a
}

/// The 0.9-size, 45% (22.5% mono) inner gradients derived from the border
/// palette.
fn paint_inner_gradients(plane: &mut Plane, palette: &PaletteColors, mono: bool, w: f32, h: f32) {
    let alpha = if mono { 0.225 } else { 0.45 };
    for def in palette.border.iter().rev() {
        let stop = GradientStop {
            at: 0.0,
            color: def.color.with_alpha(alpha),
        };
        paint_radial(
            plane,
            def.cx * w,
            def.cy * h,
            (def.w * 0.9).round(),
            (def.h * 0.9).round(),
            &[
                stop,
                GradientStop {
                    at: 1.0,
                    color: Rgba::TRANSPARENT,
                },
            ],
        );
    }
}

fn paint_inset_shadow(plane: &mut Plane, w: f32, h: f32, radius: f32, blur: f32, color: Rgba) {
    if color.a <= 0.0 {
        return;
    }
    let sigma = blur * 0.5;
    let inset = 1.0_f32;
    let inner_radius = (radius - inset).max(0.0);
    for j in 0..plane.h {
        let y = plane.y(j);
        for i in 0..plane.w {
            let x = plane.x(i);
            let sd = sd_round_rect(x, y, inset, inset, w - inset, h - inset, inner_radius);
            if sd < -3.0 * sigma {
                continue;
            }
            let inside = 0.5 * (1.0 - libm::erff(sd / (sigma * std::f32::consts::SQRT_2)));
            let alpha = color.a * (1.0 - inside);
            if alpha > 0.0 {
                over(
                    &mut plane.px[j * plane.w + i],
                    [color.r * alpha, color.g * alpha, color.b * alpha, alpha],
                );
            }
        }
    }
}

// ── line ───────────────────────────────────────────────────────────────────

// Cohesive per-layer paint inputs; a parameter struct would only move the
// noise to the single call site.
#[allow(clippy::too_many_arguments)]
fn draw_line(
    scratch: &mut Scratch,
    out: &mut Plane,
    frame: &BorderBeamFrame,
    density: f32,
    opacity: f32,
    palette: &PaletteColors,
    theme: ResolvedBeamTheme,
    radius: f32,
) {
    let spec = &frame.spec;
    let line = frame.line.expect("line frame");
    let (w, h) = (frame.host.width(), frame.host.height());
    let bw = spec.border_width.min(w.min(h) * 0.5);
    let inner_radius = (radius - bw).max(0.0);
    let fx =
        (!spec.static_colors).then(|| Fx::new(frame.hue_degrees, spec.brightness, spec.saturation));
    let (x, wscale, hscale) = (line.x, line.w, line.h);

    // Stroke ring: colours + white highlight, masked by the beam ellipse.
    let pad = begin_layer(scratch, w, h, density, 0.0);
    let sprites = if theme.is_dark() {
        &palette.line_dark
    } else {
        &palette.line_light
    };
    for sprite in sprites.iter().rev() {
        paint_line_sprite(
            &mut scratch.plane,
            sprite.color,
            sprite.w * wscale,
            sprite.h * hscale,
            x * w + sprite.dx,
            h + sprite.dy,
        );
    }
    let (highlight_w, highlight_h, highlight) = if theme.is_dark() {
        (
            24.0,
            28.0,
            [
                GradientStop {
                    at: 0.0,
                    color: rgba255(255, 255, 255, 0.38),
                },
                GradientStop {
                    at: 0.30,
                    color: rgba255(255, 255, 255, 0.12),
                },
                GradientStop {
                    at: 0.65,
                    color: Rgba::TRANSPARENT,
                },
            ],
        )
    } else {
        (
            35.0,
            28.0,
            [
                GradientStop {
                    at: 0.0,
                    color: rgba255(0, 0, 0, 0.6),
                },
                GradientStop {
                    at: 0.35,
                    color: rgba255(0, 0, 0, 0.25),
                },
                GradientStop {
                    at: 0.70,
                    color: Rgba::TRANSPARENT,
                },
            ],
        )
    };
    paint_radial(
        &mut scratch.plane,
        x * w,
        h + 2.0,
        highlight_w * wscale,
        highlight_h * hscale,
        &highlight,
    );
    apply_ellipse_mask(
        &mut scratch.plane,
        x * w,
        h,
        78.0 * wscale,
        60.0 * hscale,
        &[(0.0, 1.0), (0.45, 0.5), (1.0, 0.0)],
    );
    ring_mask(
        &mut scratch.plane,
        w,
        h,
        inner_radius,
        bw,
        (inner_radius - bw).max(0.0),
    );
    if let Some(fx) = fx {
        fx.apply_plane(&mut scratch.plane.px);
    }
    composite_crop(
        out,
        &scratch.plane,
        pad,
        opacity * line.edge * spec.stroke_opacity,
    );

    // Inner glow.
    let pad = begin_layer(scratch, w, h, density, 0.0);
    for sprite in palette.line_inner.iter().rev() {
        paint_line_sprite(
            &mut scratch.plane,
            sprite.color,
            sprite.w * wscale,
            sprite.h * hscale,
            x * w + sprite.dx,
            h + sprite.dy,
        );
    }
    paint_inset_shadow(&mut scratch.plane, w, h, radius, 9.0, spec.inner_shadow);
    apply_ellipse_mask(
        &mut scratch.plane,
        x * w,
        h,
        78.0 * wscale,
        60.0 * hscale,
        &[(0.0, 1.0), (0.45, 0.5), (1.0, 0.0)],
    );
    apply_edge_bands(&mut scratch.plane, w, h, 28.0);
    if let Some(fx) = fx {
        fx.apply_plane(&mut scratch.plane.px);
    }
    composite_crop(
        out,
        &scratch.plane,
        pad,
        opacity * line.edge * spec.inner_opacity,
    );

    // Bloom.
    let mono = spec.palette.is_mono();
    let bloom_blur = if mono {
        6.0
    } else {
        scale_blur(8.0, spec.glow_size)
    };
    let pad = begin_layer(scratch, w, h, density, 3.0 * bloom_blur + 1.0);
    paint_line_bloom(
        &mut scratch.plane,
        palette,
        theme,
        line,
        x,
        wscale,
        mono,
        w,
        h,
    );
    apply_ellipse_mask(
        &mut scratch.plane,
        x * w,
        h,
        84.0 * wscale,
        110.0 * hscale,
        &[(0.0, 1.0), (0.35, 0.5), (1.0, 0.0)],
    );
    if mono {
        blur_layer(scratch, bloom_blur, bloom_blur);
    } else if !spec.static_colors {
        blur_layer(scratch, bloom_blur, bloom_blur);
        let range = spec.hue_range;
        let bloom_range = range + 10.0;
        let bloom_hue = -bloom_range
            + 2.0
                * bloom_range
                * ((1.0 - (2.0 * PI * frame.elapsed.as_secs_f32() / 8.0).cos()) * 0.5);
        fx_layer(scratch, bloom_hue, spec.brightness, spec.saturation);
    }
    composite_crop(
        out,
        &scratch.plane,
        pad,
        opacity * line.edge * spec.bloom_opacity,
    );
}

fn paint_line_sprite(plane: &mut Plane, color: Rgba, rx: f32, ry: f32, cx: f32, cy: f32) {
    paint_radial(
        plane,
        cx,
        cy,
        rx,
        ry,
        &[
            GradientStop { at: 0.0, color },
            GradientStop {
                at: 1.0,
                color: Rgba::TRANSPARENT,
            },
        ],
    );
}

const fn rgba255(r: u8, g: u8, b: u8, a: f32) -> Rgba {
    super::palette::rgba(r, g, b, a)
}

#[allow(clippy::too_many_arguments)]
fn paint_line_bloom(
    plane: &mut Plane,
    palette: &PaletteColors,
    theme: ResolvedBeamTheme,
    line: LineFrame,
    x: f32,
    wscale: f32,
    mono: bool,
    host_w: f32,
    host_h: f32,
) {
    let dark = theme.is_dark();
    let spike_colors = if dark {
        palette.spike_dark
    } else {
        palette.spike_light
    };
    let spikes: [LineBloomSpike; 5] = if dark {
        palette.bloom_dark
    } else {
        palette.bloom_light
    };
    let (sc1, sc1_mid, sc2, sc2_mid) = if mono {
        (
            attenuate(spike_colors[0], 0.14),
            attenuate(spike_colors[0], 0.09),
            attenuate(spike_colors[1], 0.12),
            with_alpha(spike_colors[1], 0.06),
        )
    } else if dark {
        (
            spike_colors[0],
            spike_colors[0],
            spike_colors[1],
            with_alpha(spike_colors[1], 0.49),
        )
    } else {
        (
            with_alpha(spike_colors[0], 0.85),
            with_alpha(spike_colors[0], 0.85),
            with_alpha(spike_colors[1], 0.7),
            with_alpha(spike_colors[1], 0.7),
        )
    };
    let attenuated: [LineBloomSpike; 5] = if mono {
        spikes.map(|s| LineBloomSpike {
            c1: attenuate(s.c1, 0.14),
            c2: attenuate(s.c2, 0.098),
        })
    } else {
        spikes
    };
    let thin_w1 = if mono { 12.0 } else { 0.8 };
    let thin_w2 = if mono { 14.0 } else { 2.0 };
    let thin_w3 = if mono { 12.0 } else { 1.2 };
    let thin_lw = if mono { 12.0 } else { 1.0 };
    let thin_h1 = if mono { 42.0 } else { 92.0 };
    let thin_h2 = if mono { 38.0 } else { 72.0 };
    let thin_h3 = if mono { 40.0 } else { 85.0 };
    let thin_h4 = if mono { 32.0 } else { 60.0 };
    let spike = line.spike;
    let spike2 = line.spike2;
    let bgh = line.h;

    let mut grads: Vec<BloomRadial> = Vec::with_capacity(9);
    grads.push(BloomRadial::two(
        0.08 * host_w,
        host_h - 2.0,
        thin_w1 * spike,
        thin_h1 * bgh,
        sc1,
        sc1_mid,
        0.30,
        0.88,
    ));
    grads.push(BloomRadial::two(
        0.22 * host_w,
        host_h - 4.0,
        10.0 * spike2,
        35.0 * bgh,
        sc2,
        sc2_mid,
        0.50,
        0.95,
    ));
    grads.push(BloomRadial::two(
        0.36 * host_w,
        host_h - 3.0,
        thin_w2 * (2.0 - spike),
        thin_h2 * bgh,
        attenuated[0].c1,
        attenuated[0].c2,
        0.40,
        0.90,
    ));
    grads.push(BloomRadial::two(
        0.50 * host_w,
        host_h - 2.0,
        14.0 * spike2,
        28.0 * bgh,
        attenuated[1].c1,
        attenuated[1].c2,
        0.55,
        0.96,
    ));
    grads.push(BloomRadial::two(
        0.64 * host_w,
        host_h - 4.0,
        thin_w3 * (2.0 - spike2),
        thin_h3 * bgh,
        attenuated[2].c1,
        attenuated[2].c2,
        0.35,
        0.89,
    ));
    grads.push(BloomRadial::two(
        0.78 * host_w,
        host_h - 2.0,
        7.0 * spike,
        45.0 * bgh,
        attenuated[3].c1,
        attenuated[3].c2,
        0.48,
        0.94,
    ));
    grads.push(BloomRadial::two(
        0.92 * host_w,
        host_h - 3.0,
        thin_lw * (2.0 - spike),
        thin_h4 * bgh,
        attenuated[4].c1,
        attenuated[4].c2,
        0.42,
        0.91,
    ));
    if dark {
        grads.push(BloomRadial {
            cx: x * host_w,
            cy: host_h + 1.0,
            rx: 21.0 * spike,
            ry: 15.0 * spike2,
            stops: [
                GradientStop {
                    at: 0.0,
                    color: if mono {
                        rgba255(255, 255, 255, 0.5)
                    } else {
                        rgba255(255, 255, 255, 1.0)
                    },
                },
                GradientStop {
                    at: 0.20,
                    color: if mono {
                        rgba255(255, 255, 255, 0.45)
                    } else {
                        rgba255(255, 255, 255, 0.9)
                    },
                },
                GradientStop {
                    at: 0.50,
                    color: if mono {
                        rgba255(255, 255, 255, 0.25)
                    } else {
                        rgba255(255, 255, 255, 0.5)
                    },
                },
                GradientStop {
                    at: 1.0,
                    color: Rgba::TRANSPARENT,
                },
                GradientStop {
                    at: 1.0,
                    color: Rgba::TRANSPARENT,
                },
            ],
            n: 4,
        });
        let (amb0, amb25, amb55) = if mono {
            (0.15, 0.06, 0.015)
        } else {
            (0.3, 0.12, 0.03)
        };
        grads.push(BloomRadial {
            cx: x * host_w,
            cy: host_h,
            rx: 42.0 * wscale,
            ry: 40.0 * bgh,
            stops: [
                GradientStop {
                    at: 0.0,
                    color: rgba255(255, 255, 255, amb0),
                },
                GradientStop {
                    at: 0.25,
                    color: rgba255(255, 255, 255, amb25),
                },
                GradientStop {
                    at: 0.55,
                    color: rgba255(255, 255, 255, amb55),
                },
                GradientStop {
                    at: 0.80,
                    color: Rgba::TRANSPARENT,
                },
                GradientStop {
                    at: 1.0,
                    color: Rgba::TRANSPARENT,
                },
            ],
            n: 4,
        });
    } else {
        grads.push(BloomRadial {
            cx: x * host_w,
            cy: host_h,
            rx: 50.0 * wscale,
            ry: 32.0 * bgh,
            stops: [
                GradientStop {
                    at: 0.0,
                    color: rgba255(0, 0, 0, 0.5),
                },
                GradientStop {
                    at: 0.30,
                    color: rgba255(0, 0, 0, 0.18),
                },
                GradientStop {
                    at: 0.60,
                    color: rgba255(0, 0, 0, 0.03),
                },
                GradientStop {
                    at: 0.85,
                    color: Rgba::TRANSPARENT,
                },
                GradientStop {
                    at: 1.0,
                    color: Rgba::TRANSPARENT,
                },
            ],
            n: 4,
        });
    }
    for g in grads.iter().rev() {
        paint_radial(plane, g.cx, g.cy, g.rx, g.ry, &g.stops[..g.n]);
    }
}

struct BloomRadial {
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    stops: [GradientStop; 5],
    n: usize,
}

impl BloomRadial {
    #[allow(clippy::too_many_arguments)]
    fn two(cx: f32, cy: f32, rx: f32, ry: f32, c1: Rgba, c2: Rgba, at2: f32, at3: f32) -> Self {
        Self {
            cx,
            cy,
            rx,
            ry,
            stops: [
                GradientStop { at: 0.0, color: c1 },
                GradientStop { at: at2, color: c2 },
                GradientStop {
                    at: at3,
                    color: Rgba::TRANSPARENT,
                },
                GradientStop {
                    at: 1.0,
                    color: Rgba::TRANSPARENT,
                },
                GradientStop {
                    at: 1.0,
                    color: Rgba::TRANSPARENT,
                },
            ],
            n: 3,
        }
    }
}

fn attenuate(color: Rgba, factor: f32) -> Rgba {
    color.with_alpha(color.a * factor)
}

fn with_alpha(color: Rgba, alpha: f32) -> Rgba {
    Rgba { a: alpha, ..color }
}

// ── pulse inner ────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn draw_pulse_inner(
    scratch: &mut Scratch,
    out: &mut Plane,
    frame: &BorderBeamFrame,
    density: f32,
    opacity: f32,
    palette: &PaletteColors,
    theme: ResolvedBeamTheme,
    radius: f32,
) {
    let spec = &frame.spec;
    let pulse = frame.pulse.expect("pulse frame");
    let (w, h) = (frame.host.width(), frame.host.height());
    let bw = spec.border_width.min(w.min(h) * 0.5);
    let dark = theme.is_dark();

    // Ring.
    let pad = begin_layer(scratch, w, h, density, 0.0);
    paint_pulse_ring(&mut scratch.plane, palette, &pulse, w, h);
    ring_mask(&mut scratch.plane, w, h, radius, bw, (radius - bw).max(0.0));
    pulse_fx(scratch, frame);
    composite_crop(out, &scratch.plane, pad, opacity * spec.stroke_opacity);

    // Inner perimeter glow + corner accents.
    let pad = begin_layer(scratch, w, h, density, 0.0);
    paint_pulse_inner_glow(&mut scratch.plane, palette, &pulse, w, h, dark);
    apply_edge_bands(&mut scratch.plane, w, h, 28.0);
    pulse_fx(scratch, frame);
    composite_crop(out, &scratch.plane, pad, opacity * spec.inner_opacity);

    // Bloom ring, frozen at the time-average alpha.
    let bloom_blur = scale_blur(8.0, spec.glow_size);
    let pad = begin_layer(scratch, w, h, density, 3.0 * bloom_blur + 1.0);
    paint_pulse_table_static(
        &mut scratch.plane,
        &PULSE_INNER_BLOOM,
        palette,
        &pulse,
        w,
        h,
        1.0 - pulse_alpha_amplitude(frame) * 0.5,
    );
    ring_mask(&mut scratch.plane, w, h, radius, bw, (radius - bw).max(0.0));
    blur_layer(scratch, bloom_blur, bloom_blur);
    pulse_fx(scratch, frame);
    composite_crop(out, &scratch.plane, pad, opacity * spec.bloom_opacity);
}

fn pulse_fx(scratch: &mut Scratch, frame: &BorderBeamFrame) {
    let spec = &frame.spec;
    fx_layer(
        scratch,
        if spec.static_colors {
            0.0
        } else {
            frame.hue_degrees
        },
        spec.brightness,
        spec.saturation,
    );
}

fn pulse_alpha_amplitude(frame: &BorderBeamFrame) -> f32 {
    let spec = &frame.spec;
    let dark = spec.theme.is_dark();
    if spec.size == BorderBeamSize::PulseInner {
        if dark { 0.48 } else { 0.45 }
    } else if dark {
        0.46
    } else {
        0.0
    }
}

fn paint_pulse_ring(
    plane: &mut Plane,
    palette: &PaletteColors,
    pulse: &PulseFrame,
    w: f32,
    h: f32,
) {
    for (i, def) in palette.border.iter().enumerate().rev() {
        let (region, quad) = PULSE_RING_MAP[i];
        let r = usize::from(region).saturating_sub(1);
        let alpha = pulse.vars.bop[quad.index()];
        let stops = [
            GradientStop {
                at: 0.0,
                color: def.color.with_alpha(alpha),
            },
            GradientStop {
                at: 1.0,
                color: Rgba::TRANSPARENT,
            },
        ];
        paint_radial(
            plane,
            def.cx * w + pulse.vars.bx[r],
            def.cy * h + pulse.vars.by[r],
            def.w * pulse.vars.bw[r],
            def.h * pulse.vars.bh[r] * pulse.vars.bgh,
            &stops,
        );
    }
}

fn paint_pulse_inner_glow(
    plane: &mut Plane,
    palette: &PaletteColors,
    pulse: &PulseFrame,
    w: f32,
    h: f32,
    dark: bool,
) {
    // The corner accents are the lowest background layer, so paint them first.
    let corner_alpha = if dark { 0.18 } else { 0.08 };
    let corner_rgb = if dark { (255, 255, 255) } else { (0, 0, 0) };
    for (x, y, quad) in [(0.0, 0.0, 0), (1.0, 0.0, 1), (0.0, 1.0, 2), (1.0, 1.0, 3)] {
        let alpha = corner_alpha * pulse.vars.bop[quad];
        paint_radial(
            plane,
            x * w,
            y * h,
            60.0,
            60.0,
            &[
                GradientStop {
                    at: 0.0,
                    color: rgba255(corner_rgb.0, corner_rgb.1, corner_rgb.2, alpha),
                },
                GradientStop {
                    at: 0.7,
                    color: Rgba::TRANSPARENT,
                },
                GradientStop {
                    at: 1.0,
                    color: Rgba::TRANSPARENT,
                },
            ],
        );
    }
    for (i, def) in palette.border.iter().enumerate().rev() {
        let (region, quad) = PULSE_RING_MAP[i];
        let r = usize::from(region).saturating_sub(1);
        let (sized_w, sized_h) = PULSE_INNER_SIZES[i];
        let alpha = pulse.vars.bop[quad.index()];
        let stops = [
            GradientStop {
                at: 0.0,
                color: def.color.with_alpha(alpha),
            },
            GradientStop {
                at: 1.0,
                color: Rgba::TRANSPARENT,
            },
        ];
        paint_radial(
            plane,
            def.cx * w + pulse.vars.bx[r],
            def.cy * h + pulse.vars.by[r],
            sized_w * pulse.vars.bw[r],
            sized_h * pulse.vars.bh[r] * pulse.vars.bgh,
            &stops,
        );
    }
}

fn paint_pulse_table_static(
    plane: &mut Plane,
    table: &[PulseGradientDef],
    palette: &PaletteColors,
    pulse: &PulseFrame,
    host_w: f32,
    host_h: f32,
    frozen_alpha: f32,
) {
    for def in table.iter().rev() {
        let base = palette.border[usize::from(def.palette_index)];
        let (sx, sy) = pulse.glow_scale;
        let cx = def.x.unwrap_or(base.cx) * host_w;
        let cy = def.y.unwrap_or(base.cy) * host_h;
        paint_radial(
            plane,
            cx,
            cy,
            def.w * sx,
            def.h * sy,
            &[
                GradientStop {
                    at: 0.0,
                    color: base.color.with_alpha(frozen_alpha),
                },
                GradientStop {
                    at: 1.0,
                    color: Rgba::TRANSPARENT,
                },
            ],
        );
    }
}

// ── pulse outside ──────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn draw_pulse_outside(
    scratch: &mut Scratch,
    out: &mut Plane,
    behind: &mut Plane,
    frame: &BorderBeamFrame,
    density: f32,
    opacity: f32,
    palette: &PaletteColors,
    theme: ResolvedBeamTheme,
    radius: f32,
) {
    let spec = &frame.spec;
    let pulse = frame.pulse.expect("pulse frame");
    let (w, h) = (frame.host.width(), frame.host.height());
    let dark = theme.is_dark();
    let (sx, sy) = pulse.glow_scale;
    let behind_pad = ((behind.w as f32 - out.w as f32) * 0.5 / density).max(0.0);

    // Outward core (inset -10, scale .95/.9), behind the child.
    let core_pad = behind_pad;
    begin_layer(scratch, w, h, density, core_pad);
    let core_box = (-10.0, -10.0, w + 20.0, h + 20.0);
    let core_scale = (0.95, 0.9);
    let core_radius = (radius + 10.0) * core_scale.1;
    for def in PULSE_OUTER_CORE.iter().rev() {
        let base = palette.border[usize::from(def.palette_index)];
        let (cx, cy) = pulse_scaled_center(def, &base, core_box, &pulse, core_scale);
        let alpha = pulse.vars.bop[def.quad.index()];
        paint_radial(
            &mut scratch.plane,
            cx,
            cy,
            def.w * pulse.vars.bw[usize::from(def.region) - 1] * sx * core_scale.0,
            def.h * pulse.vars.bh[usize::from(def.region) - 1] * pulse.vars.bgh * sy * core_scale.1,
            &[
                GradientStop {
                    at: 0.0,
                    color: base.color.with_alpha(alpha),
                },
                GradientStop {
                    at: 1.0,
                    color: Rgba::TRANSPARENT,
                },
            ],
        );
    }
    let (bx0, by0, bx1, by1) = scaled_box(core_box, (w * 0.5, h * 0.5), core_scale);
    apply_rounded_clip(&mut scratch.plane, bx0, by0, bx1, by1, core_radius);
    let glow_blur = scale_blur(if dark { 3.0 } else { 6.0 }, spec.glow_size);
    blur_layer(scratch, glow_blur * core_scale.0, glow_blur * core_scale.1);
    pulse_fx(scratch, frame);
    composite_same(behind, &scratch.plane, opacity * spec.inner_opacity);

    // Wide bloom (inset -30, scale .95/.9), behind the core's child.
    let bloom_blur = bloom_blur(spec);
    begin_layer(scratch, w, h, density, behind_pad);
    let bloom_box = (-30.0, -30.0, w + 60.0, h + 60.0);
    let bloom_scale = (0.95, 0.9);
    let frozen = 1.0 - pulse_alpha_amplitude(frame) * 0.5;
    for def in PULSE_OUTER_BLOOM.iter().rev() {
        let base = palette.border[usize::from(def.palette_index)];
        let (cx, cy) = pulse_scaled_center(def, &base, bloom_box, &pulse, bloom_scale);
        paint_radial(
            &mut scratch.plane,
            cx,
            cy,
            def.w * sx * bloom_scale.0,
            def.h * sy * bloom_scale.1,
            &[
                GradientStop {
                    at: 0.0,
                    color: base.color.with_alpha(frozen),
                },
                GradientStop {
                    at: 1.0,
                    color: Rgba::TRANSPARENT,
                },
            ],
        );
    }
    let (bx0, by0, bx1, by1) = scaled_box(bloom_box, (w * 0.5, h * 0.5), bloom_scale);
    apply_rounded_clip(
        &mut scratch.plane,
        bx0,
        by0,
        bx1,
        by1,
        (radius + 30.0) * bloom_scale.1,
    );
    blur_layer(
        scratch,
        bloom_blur * bloom_scale.0,
        bloom_blur * bloom_scale.1,
    );
    pulse_fx(scratch, frame);
    composite_same(behind, &scratch.plane, opacity * spec.bloom_opacity);

    // 1px stroke on top.
    let pad = begin_layer(scratch, w, h, density, 0.0);
    if spec.hairline_opacity > 0.0 {
        let hair = if dark {
            rgba255(70, 70, 70, spec.hairline_opacity)
        } else {
            rgba255(0, 0, 0, spec.hairline_opacity)
        };
        paint_ring_solid(&mut scratch.plane, w, h, radius, 1.0, hair);
    }
    for def in PULSE_OUTER_CORE.iter().rev() {
        let base = palette.border[usize::from(def.palette_index)];
        let cx = def.x.unwrap_or(base.cx) * w + pulse.vars.bx[usize::from(def.region) - 1];
        let cy = def.y.unwrap_or(base.cy) * h + pulse.vars.by[usize::from(def.region) - 1];
        let alpha = pulse.vars.bop[def.quad.index()];
        paint_radial(
            &mut scratch.plane,
            cx,
            cy,
            def.w * pulse.vars.bw[usize::from(def.region) - 1] * sx,
            def.h * pulse.vars.bh[usize::from(def.region) - 1] * pulse.vars.bgh * sy,
            &[
                GradientStop {
                    at: 0.0,
                    color: base.color.with_alpha(alpha),
                },
                GradientStop {
                    at: 1.0,
                    color: Rgba::TRANSPARENT,
                },
            ],
        );
    }
    ring_mask(
        &mut scratch.plane,
        w,
        h,
        radius,
        1.0,
        (radius - 1.0).max(0.0),
    );
    pulse_fx(scratch, frame);
    composite_crop(out, &scratch.plane, pad, opacity * spec.stroke_opacity);
}

/// `scaleBlur()` from the reference: three decimals, floored at 0.5 px.
fn scale_blur(px: f32, glow_size: f32) -> f32 {
    ((px * glow_size * 100.0).round() / 100.0).max(0.5)
}

pub(crate) fn bloom_blur(spec: &BorderBeamSpec) -> f32 {
    let dark = spec.theme.is_dark();
    scale_blur(if dark { 22.5 } else { 15.0 }, spec.glow_size)
}

/// Logical pixels the `pulse-outside` bloom can paint past the host; zero for
/// the other sizes. Shared by the raster and the canvas backend.
pub(crate) fn outside_margin(spec: &BorderBeamSpec) -> f32 {
    if spec.size == BorderBeamSize::PulseOutside {
        (30.0 + 3.0 * bloom_blur(spec)).ceil()
    } else {
        0.0
    }
}

fn pulse_scaled_center(
    def: &PulseGradientDef,
    base: &GradientDef,
    (box_x, box_y, box_w, box_h): (f32, f32, f32, f32),
    pulse: &PulseFrame,
    scale: (f32, f32),
) -> (f32, f32) {
    let region = usize::from(def.region) - 1;
    let x = box_x + def.x.unwrap_or(base.cx) * box_w + pulse.vars.bx[region];
    let y = box_y + def.y.unwrap_or(base.cy) * box_h + pulse.vars.by[region];
    let pivot_x = box_x + box_w * 0.5;
    let pivot_y = box_y + box_h * 0.5;
    (
        pivot_x + (x - pivot_x) * scale.0,
        pivot_y + (y - pivot_y) * scale.1,
    )
}

fn scaled_box(
    (x, y, w, h): (f32, f32, f32, f32),
    (cx, cy): (f32, f32),
    (sx, sy): (f32, f32),
) -> (f32, f32, f32, f32) {
    let bx0 = cx + (x - cx) * sx;
    let bx1 = cx + (x + w - cx) * sx;
    let by0 = cy + (y - cy) * sy;
    let by1 = cy + (y + h - cy) * sy;
    (bx0, by0, bx1, by1)
}

fn paint_ring_solid(plane: &mut Plane, w: f32, h: f32, radius: f32, inset: f32, color: Rgba) {
    let k = (plane.kx + plane.ky) * 0.5;
    for j in 0..plane.h {
        let y = plane.y(j);
        for i in 0..plane.w {
            let x = plane.x(i);
            let outer = coverage(sd_round_rect(x, y, 0.0, 0.0, w, h, radius), k);
            let inner = coverage(
                sd_round_rect(
                    x,
                    y,
                    inset,
                    inset,
                    w - inset,
                    h - inset,
                    (radius - inset).max(0.0),
                ),
                k,
            );
            let m = outer * (1.0 - inner);
            if m > 0.0 {
                over(
                    &mut plane.px[j * plane.w + i],
                    [
                        color.r * color.a * m,
                        color.g * color.a * m,
                        color.b * color.a * m,
                        color.a * m,
                    ],
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::border_beam::options::BorderBeamOptions;
    use crate::widgets::border_beam::palette::BeamHost;
    use crate::widgets::border_beam::palette::BorderBeamPalette;
    use std::time::Duration;

    fn frame(size: BorderBeamSize, t: f32) -> BorderBeamFrame {
        let host = BeamHost::new(348.0, 122.0);
        BorderBeamOptions::new()
            .size(size)
            .palette(BorderBeamPalette::Colorful)
            .frame_at(Duration::from_secs_f32(t), host, ResolvedBeamTheme::Dark)
    }

    #[test]
    fn every_size_produces_a_non_empty_above_layer() {
        for size in BorderBeamSize::ALL {
            let layers = BorderBeamRaster::new().render(&frame(size, 0.8), 1.0, 1.0);
            let above = layers.above().expect("above");
            assert!(above.width() > 0 && above.height() > 0, "{size}");
            assert!(above.rgba().iter().any(|&v| v != 0), "{size} is blank");
        }
    }

    #[test]
    fn pulse_outside_has_a_behind_layer_with_a_negative_offset() {
        let layers =
            BorderBeamRaster::new().render(&frame(BorderBeamSize::PulseOutside, 0.8), 1.0, 1.0);
        let behind = layers.behind().expect("behind");
        assert!(behind.left() <= -30.0);
        assert!(behind.top() <= -30.0);
        assert!(behind.width() > 348);
    }

    #[test]
    fn zero_opacity_draws_nothing() {
        let layers = BorderBeamRaster::new().render(&frame(BorderBeamSize::Md, 0.5), 0.0, 1.0);
        assert!(layers.is_empty());
    }

    #[test]
    fn strength_scales_every_layer_alpha() {
        let host = BeamHost::new(348.0, 122.0);
        let strong = BorderBeamOptions::new().frame_at(
            Duration::from_secs_f32(0.8),
            host,
            ResolvedBeamTheme::Dark,
        );
        let weak = BorderBeamOptions::new().strength(0.5).frame_at(
            Duration::from_secs_f32(0.8),
            host,
            ResolvedBeamTheme::Dark,
        );
        let alpha_sum = |layers: &BeamLayers| {
            layers
                .above()
                .expect("above")
                .rgba()
                .chunks_exact(4)
                .map(|px| u64::from(px[3]))
                .sum::<u64>()
        };
        let full = alpha_sum(&BorderBeamRaster::new().render(&strong, 1.0, 1.0));
        let half = alpha_sum(&BorderBeamRaster::new().render(&weak, 1.0, 1.0));
        let ratio = half as f64 / full as f64;
        assert!((ratio - 0.5).abs() < 0.01, "alpha ratio {ratio}");
    }

    #[test]
    fn density_is_reduced_before_a_host_exceeds_the_budget() {
        let density = effective_density(6000.0, 4000.0, 4.0);
        assert!(density < 1.0, "density {density}");
        let (w, h) = device_size(6000.0, 4000.0, density);
        assert!(w <= 8192 && h <= 8192);
        assert!(w * h <= 4_100_000, "{w}x{h}");
    }
}
