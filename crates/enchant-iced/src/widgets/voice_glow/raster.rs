//! The software renderer: a [`VoiceGlowFrame`] to pixels.
//!
//! It paints the same layers, in the same z-order, as the stylesheet of
//! `packages/voice-glow` (`styles.ts`):
//!
//! | z | layer | here |
//! |---|-------|------|
//! | 1 | `::before` inner light + its warp mirror | [`Raster::inner`] |
//! | 2 | `::after` 1px edge ring with the hot spot | [`Raster::stroke`] |
//! | 3 | bloom + its warp mirror | [`Raster::bloom`] |
//! | 4 | band canvas (halo + red/blue fringes) | [`Raster::band`] |
//! | 4 | epicentre wash | [`Raster::core`] |
//!
//! as one clip to the host's rounded rectangle. The soft layers are painted
//! at half size and scaled up bilinearly, exactly like upstream's
//! `data-voice-halfres` layers (`--vb-z` 0.5): the displacement noise and
//! amount therefore live in that half-size space too.

use super::blur::blur;
use super::config::{VoiceGlowConfig, VoiceGlowStyle};
use super::frame::{BandStroke, DistortionFrame, VoiceGlowFrame};
use super::noise::Turbulence;
use super::paint::{EdgeMask, Fx, Plane, Px, over, paint_lobes, sd_round_rect};
use super::palette::{ColorFilter, Rgb};

/// The raster factor of the soft layers (`--vb-z`).
const SOFT: f64 = 0.5;
/// Upstream caps the band canvas's backing store at 2x.
const BAND_DPR_MAX: f64 = 2.0;
/// The seed of the distortion's `feTurbulence`.
const NOISE_SEED: i64 = 7;
/// Where Chromium's `feTurbulence` samples the lattice relative to the soft
/// layer's raster pixel grid, in raster pixels (x, y): the noise is anchored a
/// pixel and a half past the layer's top-left corner, not at the pixel centre
/// the SVG reference code implies. Measured by minimising the error of the
/// warped bloom against Chromium 1234 captures (2x DPR; the optimum is flat
/// within +-0.25 px).
const NOISE_PHASE: (f64, f64) = (1.5, 1.75);

/// Straight-alpha RGBA8 pixels.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GlowBitmap {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width * height * 4` bytes, straight (non-premultiplied) alpha.
    pub rgba: Vec<u8>,
}

#[derive(Debug, Default)]
struct Scratch {
    stroke: Plane,
    inner: Plane,
    bloom: Plane,
    band: Plane,
    halo: Plane,
    warp: Plane,
    core: Vec<[f32; 1]>,
    core_w: usize,
    core_h: usize,
    tmp4: Vec<Px>,
    tmp1: Vec<[f32; 1]>,
    /// The displacement map (noise red channel) of the soft grid.
    disp: Vec<f32>,
    /// The band line's y at each soft pixel column.
    below_y: Vec<f32>,
}

/// A reusable software renderer: buffers are allocated once per size.
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use enchant_iced::widgets::voice_glow::{
///     HostSize, ResolvedTheme, VoiceGlow, VoiceGlowRaster,
/// };
///
/// let glow = VoiceGlow::new().level(0.8);
/// let host = HostSize::new(120.0, 40.0);
/// let config = glow.config(ResolvedTheme::Dark);
/// let frame = glow.frame_at(Duration::from_secs(1), host, ResolvedTheme::Dark);
/// let mut raster = VoiceGlowRaster::new();
/// let bitmap = raster.render(&frame, &config, 1.0);
/// assert_eq!((bitmap.width, bitmap.height), (120, 40));
/// assert_eq!(bitmap.rgba.len(), 120 * 40 * 4);
/// ```
#[derive(Debug)]
pub struct VoiceGlowRaster {
    scratch: Scratch,
    turbulence: Turbulence,
    out: GlowBitmap,
}

impl Default for VoiceGlowRaster {
    fn default() -> Self {
        Self::new()
    }
}

/// Geometry shared by the passes of one render.
struct Ctx<'a> {
    frame: &'a VoiceGlowFrame,
    style: &'a VoiceGlowStyle,
    fx: Fx,
    density: f64,
    host_w: f64,
    host_h: f64,
    soft_w: usize,
    soft_h: usize,
}

impl VoiceGlowRaster {
    /// A renderer with empty buffers.
    #[must_use]
    pub fn new() -> Self {
        Self {
            scratch: Scratch::default(),
            turbulence: Turbulence::new(NOISE_SEED),
            out: GlowBitmap::default(),
        }
    }

    /// Paints `frame` for a host of `frame.host` logical px at `density`
    /// pixels per logical px, returning the bitmap (`host * density` pixels;
    /// empty for an empty host).
    ///
    /// `config` is the resolved configuration the frame came from
    /// ([`VoiceGlow::config`](super::VoiceGlow::config) or
    /// [`VoiceGlowEngine::config`](super::VoiceGlowEngine::config)): it carries
    /// the layer recipes the frame does not repeat.
    pub fn render(
        &mut self,
        frame: &VoiceGlowFrame,
        config: &VoiceGlowConfig,
        density: f32,
    ) -> &GlowBitmap {
        let host = frame.host;
        if host.is_empty() {
            self.out = GlowBitmap::default();
            return &self.out;
        }
        let density = f64::from(density).clamp(0.25, 8.0);
        let ow = ((host.width * density).round() as usize).max(1);
        let oh = ((host.height * density).round() as usize).max(1);
        let soft_w = ((host.width * SOFT * density).round() as usize).max(1);
        let soft_h = ((host.height * SOFT * density).round() as usize).max(1);
        let style = &config.style;
        let ctx = Ctx {
            frame,
            style,
            fx: Fx::new(&ColorFilter::new(
                frame.hue_degrees,
                style.brightness,
                style.saturation,
            )),
            density,
            host_w: host.width,
            host_h: host.height,
            soft_w,
            soft_h,
        };
        let mut s = std::mem::take(&mut self.scratch);

        build_below_y(&mut s, &ctx);
        let warped = frame.distortion.filter(|d| d.amount > 1e-4);
        if warped.is_some() {
            build_displacement_map(&mut s, &ctx, &self.turbulence);
        }
        let opacity = &frame.layers;
        if opacity.inner > 0.0 {
            inner(&mut s, &ctx, warped);
        } else {
            s.inner.reset(0, 0, ctx.host_w, ctx.host_h);
        }
        if opacity.stroke > 0.0 {
            stroke(&mut s, &ctx, ow, oh);
        } else {
            s.stroke.reset(0, 0, ctx.host_w, ctx.host_h);
        }
        if opacity.bloom > 0.0 {
            bloom(&mut s, &ctx, warped);
        } else {
            s.bloom.reset(0, 0, ctx.host_w, ctx.host_h);
        }
        band(&mut s, &ctx);
        core(&mut s, &ctx);
        compose(&mut self.out, &s, &ctx, ow, oh);

        self.scratch = s;
        &self.out
    }
}

// ── shared helpers ──────────────────────────────────────────────────────────

/// The band line as a function of x, extended to the host's bottom corners
/// like the clip polygons are.
fn build_below_y(s: &mut Scratch, ctx: &Ctx<'_>) {
    let line = &ctx.frame.band_line;
    s.below_y.clear();
    if line.is_empty() {
        s.below_y.resize(ctx.soft_w, f32::INFINITY);
        return;
    }
    let mut pts: Vec<(f32, f32)> = Vec::with_capacity(line.len() + 2);
    let (first, last) = (line[0], line[line.len() - 1]);
    if first.0 > 0.0 {
        pts.push((0.0, ctx.host_h as f32));
    }
    pts.extend(line.iter().map(|&(x, y)| (x as f32, y as f32)));
    if last.0 < ctx.host_w {
        pts.push((ctx.host_w as f32, ctx.host_h as f32));
    }
    let kx = ctx.soft_w as f32 / ctx.host_w as f32;
    s.below_y
        .extend((0..ctx.soft_w).map(|i| interp(&pts, (i as f32 + 0.5) / kx).0));
}

/// `y` and slope of the polyline `pts` (sorted by x) at `x`, clamped to its
/// ends.
fn interp(pts: &[(f32, f32)], x: f32) -> (f32, f32) {
    let n = pts.len();
    if n == 1 || x <= pts[0].0 {
        return (pts[0].1, 0.0);
    }
    if x >= pts[n - 1].0 {
        return (pts[n - 1].1, 0.0);
    }
    let i = pts.partition_point(|p| p.0 <= x).clamp(1, n - 1);
    let (a, b) = (pts[i - 1], pts[i]);
    let dx = (b.0 - a.0).max(1e-6);
    let slope = (b.1 - a.1) / dx;
    (a.1 + slope * (x - a.0), slope)
}

/// Fraction of the pixel row `j` of the soft grid that lies below the band
/// line in column `i`.
#[inline]
fn below(below_y: &[f32], plane: &Plane, i: usize, j: usize) -> f32 {
    ((plane.y(j) - below_y[i]) * plane.ky + 0.5).clamp(0.0, 1.0)
}

/// Fills `disp` with the displacement map: the red channel of the filter's
/// fractal noise, drifting by the `feOffset`, in the soft layer's own px.
fn build_displacement_map(s: &mut Scratch, ctx: &Ctx<'_>, noise: &Turbulence) {
    let Some(d) = ctx.frame.distortion else {
        return;
    };
    let (w, h) = (ctx.soft_w, ctx.soft_h);
    let freq = ctx.style.noise_frequency;
    s.disp.clear();
    s.disp.reserve(w * h);
    for j in 0..h {
        let v = (j as f64 + NOISE_PHASE.1) / ctx.density - d.noise_offset.1;
        for i in 0..w {
            let u = (i as f64 + NOISE_PHASE.0) / ctx.density - d.noise_offset.0;
            s.disp.push(noise.fractal_red(u, v, freq, 2));
        }
    }
}

/// `dst(x, y) = src(x + shift * (map - 0.5), y)`: `feDisplacementMap` with
/// the red channel on x and the green channel pinned at 0.5.
fn displace(src: &Plane, dst: &mut Plane, disp: &[f32], amount_px: f32) {
    dst.reset(
        src.w,
        src.h,
        src.w as f64 / f64::from(src.kx),
        src.h as f64 / f64::from(src.ky),
    );
    for j in 0..src.h {
        let row = j * src.w;
        for i in 0..src.w {
            let shift = amount_px * (disp[row + i] - 0.5);
            let fx = i as f32 + shift;
            let x0 = fx.floor();
            let t = fx - x0;
            let x0 = x0 as isize;
            let at = |x: isize| -> Px {
                if x < 0 || x >= src.w as isize {
                    [0.0; 4]
                } else {
                    src.px[row + x as usize]
                }
            };
            let (a, b) = (at(x0), at(x0 + 1));
            let mut o = [0.0; 4];
            for k in 0..4 {
                o[k] = a[k] + (b[k] - a[k]) * t;
            }
            dst.px[row + i] = o;
        }
    }
}

/// Mixes the base layer (clipped above the band line) with its displaced
/// mirror (clipped below it), then masks and fades it.
struct SoftPass<'a> {
    warp: &'a mut Plane,
    disp: &'a [f32],
    below_y: &'a [f32],
    warped: Option<DistortionFrame>,
    density: f64,
}

impl SoftPass<'_> {
    fn finish(self, plane: &mut Plane, mask: impl Fn(f32, f32) -> f32, opacity: f64) {
        if let Some(d) = self.warped {
            displace(
                plane,
                self.warp,
                self.disp,
                (d.amount * self.density) as f32,
            );
            for j in 0..plane.h {
                for i in 0..plane.w {
                    let cb = below(self.below_y, plane, i, j);
                    let idx = j * plane.w + i;
                    for k in 0..4 {
                        plane.px[idx][k] =
                            plane.px[idx][k] * (1.0 - cb) + self.warp.px[idx][k] * cb;
                    }
                }
            }
        }
        let opacity = opacity as f32;
        for j in 0..plane.h {
            let y = plane.y(j);
            for i in 0..plane.w {
                let m = mask(plane.x(i), y) * opacity;
                for c in &mut plane.px[j * plane.w + i] {
                    *c *= m;
                }
            }
        }
    }
}

fn lerp(t: f32, a: f32, b: f32) -> f32 {
    a + (b - a) * t
}

// ── z 1: the inner light ────────────────────────────────────────────────────

fn inner(s: &mut Scratch, ctx: &Ctx<'_>, warped: Option<DistortionFrame>) {
    let (w, h) = (ctx.soft_w, ctx.soft_h);
    let style = ctx.style;
    let frame = ctx.frame;
    s.inner.reset(w, h, ctx.host_w, ctx.host_h);
    paint_lobes(&mut s.inner, frame, &style.inner);

    // box-shadow: inset 0 0 <blur> 1px <colour>
    let (sr, sg, sb, sa) = style.inner_shadow;
    let sigma = (style.inset_blur / 2.0) as f32;
    let radius = style.border_radius as f32;
    let (hw, hh) = (ctx.host_w as f32, ctx.host_h as f32);
    if sa > 0.0 && sigma > 0.0 {
        let color = [
            f32::from(sr) / 255.0,
            f32::from(sg) / 255.0,
            f32::from(sb) / 255.0,
        ];
        let sa = sa as f32;
        for j in 0..h {
            let y = s.inner.y(j);
            for i in 0..w {
                let x = s.inner.x(i);
                let sd = sd_round_rect(x, y, 1.0, 1.0, hw - 1.0, hh - 1.0, (radius - 1.0).max(0.0));
                if sd < -3.0 * sigma {
                    continue;
                }
                // The shadow is the blurred complement of the shrunk box.
                let inside = 0.5 * (1.0 - libm::erff(sd / (sigma * std::f32::consts::SQRT_2)));
                let a = sa * (1.0 - inside);
                over(
                    &mut s.inner.px[j * w + i],
                    [color[0] * a, color[1] * a, color[2] * a, a],
                );
            }
        }
    }
    ctx.fx.apply_plane(&mut s.inner.px);

    let edge = EdgeMask::new(frame, 170.0, 64.0, 0.45, Some(0.3));
    let cf = (style.corner_fade as f32).max(1e-3);
    // Two linear gradients added together: white at the borders, clear
    // beyond `cornerFade` px from them.
    let rim = move |d: f32, len: f32| -> f32 {
        let d = d.min(len - d).max(0.0);
        (1.0 - d / cf).max(0.0)
    };
    let mask = move |x: f32, y: f32| -> f32 {
        let (v, hz) = (rim(y, hh), rim(x, hw));
        edge.at(x, y) * (v + hz - v * hz)
    };
    SoftPass {
        warp: &mut s.warp,
        disp: &s.disp,
        below_y: &s.below_y,
        warped,
        density: ctx.density,
    }
    .finish(&mut s.inner, mask, frame.layers.inner.clamp(0.0, 1.0));
}

// ── z 3: the bloom ──────────────────────────────────────────────────────────

fn bloom(s: &mut Scratch, ctx: &Ctx<'_>, warped: Option<DistortionFrame>) {
    let (w, h) = (ctx.soft_w, ctx.soft_h);
    let style = ctx.style;
    s.bloom.reset(w, h, ctx.host_w, ctx.host_h);
    paint_lobes(&mut s.bloom, ctx.frame, &style.bloom);
    // filter: blur(r) hue-rotate() brightness() saturate() [url(warp)]
    let sigma = style.bloom_blur as f32;
    blur(
        &mut s.bloom.px,
        w,
        h,
        sigma * s.bloom.kx,
        sigma * s.bloom.ky,
        &mut s.tmp4,
    );
    ctx.fx.apply_plane(&mut s.bloom.px);
    let edge = EdgeMask::new(ctx.frame, 200.0, 130.0, 0.35, None);
    SoftPass {
        warp: &mut s.warp,
        disp: &s.disp,
        below_y: &s.below_y,
        warped,
        density: ctx.density,
    }
    .finish(
        &mut s.bloom,
        move |x, y| edge.at(x, y),
        ctx.frame.layers.bloom.clamp(0.0, 1.0),
    );
}

// ── z 2: the edge stroke ────────────────────────────────────────────────────

/// Coverage of the pixel centred at `c` (1/k wide) by `[lo, hi]`.
#[inline]
fn span_cover(c: f32, k: f32, lo: f32, hi: f32) -> f32 {
    let half = 0.5 / k;
    (((c + half).min(hi) - (c - half).max(lo)) * k).clamp(0.0, 1.0)
}

fn stroke(s: &mut Scratch, ctx: &Ctx<'_>, ow: usize, oh: usize) {
    let style = ctx.style;
    let frame = ctx.frame;
    s.stroke.reset(ow, oh, ctx.host_w, ctx.host_h);
    let (hw, hh) = (ctx.host_w as f32, ctx.host_h as f32);
    let (kx, ky) = (s.stroke.kx, s.stroke.ky);
    let radius = style.border_radius as f32;
    let bw = style.border_width as f32;
    let edge = EdgeMask::new(frame, 170.0, 64.0, 0.45, None);
    let opacity = frame.layers.stroke.clamp(0.0, 1.0) as f32;

    // The ring is the rounded host minus its plain (square-cornered) content
    // box: `mask-composite: exclude` of the border box and the content box.
    // It only exists within `bw` px of an edge.
    let band_x = ((bw * kx).ceil() as usize + 1).min(ow);
    let band_y = ((bw * ky).ceil() as usize + 1).min(oh);

    // The hot spot sits at the centre of the edge.
    let hot = &style.hot_spot;
    let hot_c = (edge.cx, hh + 2.0 + frame.corner_lift as f32);
    let hot_r = (
        (hot.rx * frame.width_scale) as f32,
        (hot.ry * frame.height_scale) as f32,
    );
    let hot_rgb = if hot.black { 0.0 } else { 1.0 };
    let hot_a = |d: f32| -> f32 {
        let [(p1, a1), (p2, a2)] = hot.stops.map(|(p, a)| (p as f32, a as f32));
        let c = hot.centre_alpha as f32;
        if d >= p2 {
            0.0
        } else if d >= p1 {
            lerp((d - p1) / (p2 - p1), a1, a2)
        } else {
            lerp(d / p1, c, a1)
        }
    };

    let mut cover = vec![0.0_f32; ow];
    for j in 0..oh {
        let (full_row, edge_cols) = (j < band_y || j + band_y >= oh, band_x);
        let y = s.stroke.y(j);
        for (i, slot) in cover.iter_mut().enumerate() {
            if !full_row && i >= edge_cols && i + edge_cols < ow {
                *slot = 0.0;
                continue;
            }
            let x = s.stroke.x(i);
            let sd = sd_round_rect(x, y, 0.0, 0.0, hw, hh, radius);
            let outer = (0.5 - sd * kx.min(ky)).clamp(0.0, 1.0);
            let content = span_cover(x, kx, bw, hw - bw) * span_cover(y, ky, bw, hh - bw);
            *slot = outer * (1.0 - content) * edge.at(x, y);
        }
        // The gradients are evaluated only where the ring is.
        if !cover.iter().any(|&c| c > 0.0) {
            continue;
        }
        for (i, &m) in cover.iter().enumerate() {
            if m <= 0.0 {
                continue;
            }
            let x = s.stroke.x(i);
            let mut px = lobes_at(frame, &style.stroke, x, y, hh);
            // The highlight is the topmost background layer.
            let nx = (x - hot_c.0) / hot_r.0.max(1e-6);
            let ny = (y - hot_c.1) / hot_r.1.max(1e-6);
            let d = (nx * nx + ny * ny).sqrt();
            let a = hot_a(d);
            if a > 0.0 && hot_r.0 > 0.0 && hot_r.1 > 0.0 {
                over(&mut px, [hot_rgb * a, hot_rgb * a, hot_rgb * a, a]);
            }
            s.stroke.px[j * ow + i] = px.map(|c| c * m * opacity);
        }
    }
    // The colour filter acts on the unmasked colour; masks only scale the
    // premultiplied pixel, so filtering afterwards is equivalent.
    ctx.fx.apply_plane(&mut s.stroke.px);
}

/// The lobe gradients of one layer evaluated at one host point.
fn lobes_at(
    frame: &VoiceGlowFrame,
    layer: &super::config::LobeLayer,
    x: f32,
    y: f32,
    host_h: f32,
) -> Px {
    let fade = (layer.fade_percent / 100.0) as f32;
    let mut out = [0.0_f32; 4];
    for lobe in frame.lobes.iter().rev() {
        let (rx, ry) = lobe.radii(layer, frame);
        let (rx, ry) = (rx as f32, ry as f32);
        if rx <= 0.0 || ry <= 0.0 {
            continue;
        }
        let nx = (x - lobe.center_x as f32) / rx;
        let ny = (y - (host_h + layer.y as f32 + lobe.lift as f32)) / ry;
        let u = (nx * nx + ny * ny).sqrt() / fade;
        if u >= 1.0 {
            continue;
        }
        let a = layer.alpha as f32 * (1.0 - u);
        let [r, g, b] = lobe.color.to_unit().map(|v| v as f32);
        over(&mut out, [r * a, g * a, b * a, a]);
    }
    out
}

// ── z 4: the band ───────────────────────────────────────────────────────────

/// One stroke of the band: the line offset by `offset`, `width` px thick, at
/// `alpha`, faded at both ends. The distance to the line is measured along
/// the normal (vertical distance over `sqrt(1 + slope^2)`).
#[allow(clippy::too_many_arguments)]
fn stroke_line(
    plane: &mut Plane,
    line: &[(f32, f32)],
    columns: &[(f32, f32)],
    stroke: &BandStroke,
    color: Rgb,
    end_fade: f32,
    x_range: (f32, f32),
) {
    let [r, g, b] = color.to_unit().map(|v| v as f32);
    let half = stroke.width as f32 / 2.0;
    let _ = line;
    let ky = plane.ky;
    for (i, &(yl, norm)) in columns.iter().enumerate() {
        let x = plane.x(i);
        let t = (x - x_range.0) / (x_range.1 - x_range.0).max(1e-6);
        if t <= 0.0 || t >= 1.0 {
            continue;
        }
        let g_alpha = (t / end_fade).min((1.0 - t) / end_fade).min(1.0);
        let a_col = stroke.alpha as f32 * g_alpha;
        if a_col <= 0.0 {
            continue;
        }
        let reach = (half + 1.0) / norm.max(1e-3);
        let j0 = (((yl - reach - plane.oy) * ky).floor().max(0.0)) as usize;
        let j1 = (((yl + reach - plane.oy) * ky).ceil().max(0.0) as usize).min(plane.h);
        for j in j0..j1 {
            let dist = (plane.y(j) - yl).abs() * norm;
            let cov = ((half - dist) * ky + 0.5).clamp(0.0, 1.0);
            if cov <= 0.0 {
                continue;
            }
            let a = a_col * cov;
            over(&mut plane.px[j * plane.w + i], [r * a, g * a, b * a, a]);
        }
    }
}

/// The per-column `(y, 1/sqrt(1 + slope^2))` of the line shifted by `offset`.
fn columns(plane: &Plane, line: &[(f32, f32)], offset: (f32, f32)) -> Vec<(f32, f32)> {
    (0..plane.w)
        .map(|i| {
            let (y, slope) = interp(line, plane.x(i) - offset.0);
            (y + offset.1, 1.0 / (1.0 + slope * slope).sqrt())
        })
        .collect()
}

fn band(s: &mut Scratch, ctx: &Ctx<'_>) {
    let frame = ctx.frame;
    let Some(paint) = frame.band.as_ref().filter(|_| frame.layers.band > 0.0) else {
        s.band.reset(0, 0, ctx.host_w, ctx.host_h);
        return;
    };
    if frame.band_line.len() < 2 {
        s.band.reset(0, 0, ctx.host_w, ctx.host_h);
        return;
    }
    let density = ctx.density.min(BAND_DPR_MAX);
    let w = ((ctx.host_w * density).round() as usize).max(1);
    let h = ((ctx.host_h * density).round() as usize).max(1);
    s.halo.reset(w, h, ctx.host_w, ctx.host_h);
    s.band.reset(w, h, ctx.host_w, ctx.host_h);
    let line: Vec<(f32, f32)> = frame
        .band_line
        .iter()
        .map(|&(x, y)| (x as f32, y as f32))
        .collect();
    let x_range = (line[0].0, line[line.len() - 1].0);
    let end_fade = (paint.end_fade as f32).max(1e-4);

    // A canvas 2D `filter: blur()` blurs the whole shape, including what
    // hangs past the canvas, so the strokes are drawn on planes with a
    // margin of what can still reach the visible area.
    let (kx, ky) = (s.halo.kx, s.halo.ky);
    let reach = paint
        .ridges
        .iter()
        .flat_map(|r| r.strokes.iter().map(move |st| (st.width, r.offset)))
        .chain(std::iter::once((paint.halo.width, (0.0, 0.0))))
        .fold(0.0_f64, |m, (width, off)| {
            m.max(width / 2.0 + off.0.abs().max(off.1.abs()))
        });
    let margin = (3.0 * paint.halo_blur.max(paint.blur) + reach + 2.0).ceil() as f32;
    let pad = ((margin * kx).ceil() as usize, (margin * ky).ceil() as usize);
    let host = (ctx.host_w, ctx.host_h);
    s.halo.reset_padded((w, h), host, pad);
    s.band.reset_padded((w, h), host, pad);
    let (pw, ph) = (s.band.w, s.band.h);

    let base = columns(&s.halo, &line, (0.0, 0.0));
    stroke_line(
        &mut s.halo,
        &line,
        &base,
        &paint.halo,
        paint.halo_color,
        end_fade,
        x_range,
    );
    // Blur radii are CSS px, as upstream makes them canonical.
    let halo_sigma = paint.halo_blur as f32;
    blur(
        &mut s.halo.px,
        pw,
        ph,
        halo_sigma * kx,
        halo_sigma * ky,
        &mut s.tmp4,
    );

    for ridge in &paint.ridges {
        let off = (ridge.offset.0 as f32, ridge.offset.1 as f32);
        let cols = columns(&s.band, &line, off);
        for st in &ridge.strokes {
            stroke_line(
                &mut s.band,
                &line,
                &cols,
                st,
                ridge.color,
                end_fade,
                x_range,
            );
        }
    }
    let sigma = paint.blur as f32;
    blur(&mut s.band.px, pw, ph, sigma * kx, sigma * ky, &mut s.tmp4);
    let opacity = frame.layers.band.clamp(0.0, 1.0) as f32;
    // Halo under the ridges, cropped back to the host.
    let mut out = Vec::with_capacity(w * h);
    for j in 0..h {
        let row = (j + pad.1) * pw + pad.0;
        for i in 0..w {
            let mut p = s.halo.px[row + i];
            over(&mut p, s.band.px[row + i]);
            out.push(p);
        }
    }
    s.band.px = out;
    s.band.w = w;
    s.band.h = h;
    s.band.ox = 0.0;
    s.band.oy = 0.0;
    ctx.fx.apply_plane(&mut s.band.px);
    for p in &mut s.band.px {
        for c in p.iter_mut() {
            *c *= opacity;
        }
    }
}
// ── z 4: the epicentre wash ─────────────────────────────────────────────────

fn core(s: &mut Scratch, ctx: &Ctx<'_>) {
    let frame = ctx.frame;
    let style = ctx.style;
    let Some(core) = style
        .core_light
        .as_ref()
        .filter(|_| frame.layers.core_light > 0.0)
    else {
        s.core_w = 0;
        s.core_h = 0;
        return;
    };
    let (w, h) = (ctx.soft_w, ctx.soft_h);
    s.core_w = w;
    s.core_h = h;
    s.core.clear();
    s.core.resize(w * h, [0.0]);
    let (hw, hh) = (ctx.host_w as f32, ctx.host_h as f32);
    let (kx, ky) = (w as f32 / hw, h as f32 / hh);
    let cx = (hw as f64 / 2.0 + frame.center_offset * frame.width_scale) as f32;
    let cy = hh + frame.corner_lift as f32;
    let rx = (core.rx * frame.width_scale) as f32;
    let ry = (core.ry * frame.height_scale + frame.bend_height) as f32;
    let solid = (core.solid_percent / 100.0) as f32;
    let mid = (core.mid_percent / 100.0) as f32;
    let end = (core.end_percent / 100.0) as f32;
    let mid_a = core.mid_alpha as f32;
    for j in 0..h {
        let y = (j as f32 + 0.5) / ky;
        let ny = (y - cy) / ry.max(1e-6);
        for i in 0..w {
            let x = (i as f32 + 0.5) / kx;
            let nx = (x - cx) / rx.max(1e-6);
            let d = (nx * nx + ny * ny).sqrt();
            let a = if d <= solid {
                1.0
            } else if d <= mid {
                lerp((d - solid) / (mid - solid).max(1e-6), 1.0, mid_a)
            } else if d <= end {
                lerp((d - mid) / (end - mid).max(1e-6), mid_a, 0.0)
            } else {
                0.0
            };
            if a <= 0.0 {
                continue;
            }
            // Clipped to below the band line before it is blurred.
            let below = ((y - s.below_y[i]) * ky + 0.5).clamp(0.0, 1.0);
            s.core[j * w + i] = [a * below];
        }
    }
    let sigma = core.blur as f32;
    blur(&mut s.core, w, h, sigma * kx, sigma * ky, &mut s.tmp1);
    let opacity = frame.layers.core_light.clamp(0.0, 1.0) as f32;
    for p in &mut s.core {
        p[0] *= opacity;
    }
}

// ── composition ─────────────────────────────────────────────────────────────

fn compose(out: &mut GlowBitmap, s: &Scratch, ctx: &Ctx<'_>, ow: usize, oh: usize) {
    out.width = ow as u32;
    out.height = oh as u32;
    out.rgba.clear();
    out.rgba.reserve(ow * oh * 4);
    let (hw, hh) = (ctx.host_w as f32, ctx.host_h as f32);
    let radius = ctx.style.border_radius as f32;
    let k = (ow as f32 / hw).min(oh as f32 / hh);
    let has = |p: &Plane| p.w > 0 && p.h > 0;
    let same = |p: &Plane| p.w == ow && p.h == oh;
    for j in 0..oh {
        let v = (j as f32 + 0.5) / oh as f32;
        let y = v * hh;
        for i in 0..ow {
            let u = (i as f32 + 0.5) / ow as f32;
            let x = u * hw;
            let clip = (0.5 - sd_round_rect(x, y, 0.0, 0.0, hw, hh, radius) * k).clamp(0.0, 1.0);
            let mut acc = [0.0_f32; 4];
            if clip > 0.0 {
                let fetch = |p: &Plane| -> Px {
                    if same(p) {
                        p.px[j * ow + i]
                    } else {
                        p.sample(u, v)
                    }
                };
                if has(&s.inner) {
                    over(&mut acc, fetch(&s.inner));
                }
                if has(&s.stroke) {
                    over(&mut acc, fetch(&s.stroke));
                }
                if has(&s.bloom) {
                    over(&mut acc, fetch(&s.bloom));
                }
                if has(&s.band) {
                    over(&mut acc, fetch(&s.band));
                }
                if s.core_w > 0 {
                    let fx = u * s.core_w as f32 - 0.5;
                    let fy = v * s.core_h as f32 - 0.5;
                    let (x0, y0) = (fx.floor(), fy.floor());
                    let (tx, ty) = (fx - x0, fy - y0);
                    let at = |xx: f32, yy: f32| -> f32 {
                        let xx = (xx as isize).clamp(0, s.core_w as isize - 1) as usize;
                        let yy = (yy as isize).clamp(0, s.core_h as isize - 1) as usize;
                        s.core[yy * s.core_w + xx][0]
                    };
                    let top = lerp(tx, at(x0, y0), at(x0 + 1.0, y0));
                    let bottom = lerp(tx, at(x0, y0 + 1.0), at(x0 + 1.0, y0 + 1.0));
                    let a = lerp(ty, top, bottom);
                    over(&mut acc, [a, a, a, a]);
                }
            }
            for c in &mut acc {
                *c *= clip;
            }
            let a = acc[3].clamp(0.0, 1.0);
            if a <= 0.0 {
                out.rgba.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                for c in &acc[..3] {
                    out.rgba
                        .push(((c / a) * 255.0).round().clamp(0.0, 255.0) as u8);
                }
                out.rgba.push((a * 255.0).round() as u8);
            }
        }
    }
}
