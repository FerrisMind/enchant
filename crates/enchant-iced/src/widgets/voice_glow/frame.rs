//! The headless output of one rendered instant: everything a renderer needs
//! to paint the glow, with no CSS in between.
//!
//! Upstream writes these numbers as CSS custom properties (`--vb-level`,
//! `--vb-h`, `--vb-w`, `--vb-cx`, `--vb-xN`, `--vb-lN`, `--vb-yN`, ...) and
//! lets the stylesheet multiply them into gradients. [`VoiceGlowFrame`] is
//! those properties, plus the band line and its stroke recipe that
//! `drawBand` paints on a canvas, and the final per-layer opacities.

use super::config::{LobeLayer, VoiceGlowConfig};
use super::geometry::{
    BAND_SAMPLES, CEILING_HALF_WIDTH, CEILING_HEIGHT, bell, corner_lift, painted_radius, tail_lift,
};
use super::palette::Rgb;

/// The host element's size in logical px.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::HostSize;
///
/// assert_eq!(HostSize::new(350.0, 56.0).width, 350.0);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HostSize {
    /// Width, px.
    pub width: f64,
    /// Height, px.
    pub height: f64,
}

impl HostSize {
    /// A host size; negative or non-finite dimensions become 0.
    #[must_use]
    pub fn new(width: f64, height: f64) -> Self {
        let clean = |v: f64| if v.is_finite() { v.max(0.0) } else { 0.0 };
        Self {
            width: clean(width),
            height: clean(height),
        }
    }

    /// `true` when either side is empty (the band is skipped).
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }
}

/// One lobe at one instant.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LobeFrame {
    /// Offset from the beam centre along the flow, after gathering, px
    /// (`--vb-xN`).
    pub x: f64,
    /// Amplitude: the band's lift times the edge envelope (`--vb-lN`).
    pub amplitude: f64,
    /// Vertical lift along the host's corner arc, px; zero or negative, up is
    /// negative (`--vb-yN`).
    pub lift: f64,
    /// The lobe centre's x from the host's left edge, px.
    pub center_x: f64,
    /// The lobe's palette colour, before the colour filter.
    pub color: Rgb,
    /// The band that drives it: 0 low, 1 mid, 2 high.
    pub band: usize,
    /// Resting width, px.
    pub base_width: i32,
    /// Resting height, px.
    pub base_height: i32,
}

impl LobeFrame {
    /// The lobe's ellipse radii `(rx, ry)` in a layer, px.
    ///
    /// `rx = round(base_width * layer.width_scale) * frame.width_scale` and
    /// `ry = round(base_height * layer.height_scale) * frame.height_scale *
    /// amplitude`, exactly the `radial-gradient(ellipse ...)` sizes of the
    /// stylesheet.
    #[must_use]
    pub fn radii(&self, layer: &LobeLayer, frame: &VoiceGlowFrame) -> (f64, f64) {
        let round = |v: f64| (v + 0.5).floor();
        (
            round(f64::from(self.base_width) * layer.width_scale) * frame.width_scale,
            round(f64::from(self.base_height) * layer.height_scale)
                * frame.height_scale
                * self.amplitude,
        )
    }
}

/// The distortion filter's drive for one frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DistortionFrame {
    /// Displacement scale, px (`feDisplacementMap.scale`).
    pub amount: f64,
    /// Noise offset `(dx, dy)`, px (`feOffset`).
    pub noise_offset: (f64, f64),
}

/// The final opacity of each layer, fade and strength included. The products
/// are not clamped here (they mirror the stylesheet's `calc()`); like CSS
/// `opacity`, the renderer clamps each to `0..=1` when it paints.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerOpacity {
    /// The crisp edge stroke.
    pub stroke: f64,
    /// The soft inner light.
    pub inner: f64,
    /// The blurred bloom.
    pub bloom: f64,
    /// The band canvases.
    pub band: f64,
    /// The epicentre wash (zero when `core_light` is off).
    pub core_light: f64,
}

/// One pass of the band: a stroke of `width` px at `alpha`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BandStroke {
    /// Line width, px.
    pub width: f64,
    /// Stroke alpha, before the layer opacity.
    pub alpha: f64,
}

/// One fringe of the band: a colour, its offset from the line and the four
/// stacked strokes that give it a soft ramp across its thickness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BandRidge {
    /// The ridge's colour.
    pub color: Rgb,
    /// Offset of the whole line `(dx, dy)`, px (negative `dy` is up).
    pub offset: (f64, f64),
    /// The stacked strokes, widest first.
    pub strokes: [BandStroke; 4],
}

/// How to paint the band: a wide soft halo, then the four ridges in order
/// (above, mid, below, core), every stroke blurred and faded at both ends.
#[derive(Clone, Debug, PartialEq)]
pub struct BandPaint {
    /// Overall alpha, `min(1, 0.6 * bandStrength * bend strength)`.
    pub alpha: f64,
    /// Ridge thickness, px.
    pub thickness: f64,
    /// The wide halo stroke under everything (core colour).
    pub halo: BandStroke,
    /// The halo's colour.
    pub halo_color: Rgb,
    /// The ridges in paint order: above, mid, below, core.
    pub ridges: [BandRidge; 4],
    /// Fraction of the line's length over which each end fades to transparent.
    pub end_fade: f64,
    /// Blur radius of the ridges, CSS px.
    pub blur: f64,
    /// Blur radius of the halo, CSS px.
    pub halo_blur: f64,
}

/// A complete set of draw numbers for one instant.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct VoiceGlowFrame {
    /// The host size this frame was computed for.
    pub host: HostSize,
    /// The fade in / out progress (`--vb-opacity`), 0-1.
    pub opacity: f64,
    /// The smoothed level (`--vb-level`), 0-1.
    pub level: f64,
    /// The effective drive: voice with idle breathing, or the held level.
    pub drive: f64,
    /// Presence (`--vb-glow`), 0.15-1.
    pub glow: f64,
    /// Height multiplier on every lobe (`--vb-h`).
    pub height_scale: f64,
    /// Width multiplier on every lobe (`--vb-w`).
    pub width_scale: f64,
    /// Where the whole beam sits, px from the centre (`--vb-cx`).
    pub center_offset: f64,
    /// Mask-width multiplier: the range narrowed while gathered (`--vb-mw`).
    pub mask_width: f64,
    /// Extra height of the ceiling ellipse at the centre, px (`--vb-bh`).
    pub bend_height: f64,
    /// The bend's strength, 0-1 (`--vb-bendA`).
    pub bend_strength: f64,
    /// Lift of the beam centre along the corner arc, px, up is negative
    /// (`--vb-cy`).
    pub corner_lift: f64,
    /// The gather, 0-1: how far the lobes are pulled into one beam.
    pub gather: f64,
    /// The hue drift alone, degrees (`--vb-hue`).
    pub hue_drift: f64,
    /// The full hue rotation of the colour filter: the theme's base plus the
    /// drift, degrees.
    pub hue_degrees: f64,
    /// The distortion's share, 1 for the voice glow, settling to 0 while
    /// gathered.
    pub warp: f64,
    /// The distortion drive, `None` when the filter is off or has settled out.
    pub distortion: Option<DistortionFrame>,
    /// The seven lobes.
    pub lobes: [LobeFrame; 7],
    /// The band line in host px, left to right; empty for an empty host. The
    /// glow under the line is what the distortion warps.
    pub band_line: Vec<(f64, f64)>,
    /// How to paint the band, `None` when it is invisible.
    pub band: Option<BandPaint>,
    /// The final opacity of each layer.
    pub layers: LayerOpacity,
    /// The mask ellipse's range multipliers `(width, height)`.
    pub range: (f64, f64),
}

impl VoiceGlowFrame {
    /// The ellipse every layer is masked to, for a layer authored at
    /// `(half_width, height)` px at resting size (stroke and inner light use
    /// `(170, 64)`, the bloom `(200, 130)`).
    ///
    /// Returns `(center_x, center_y_from_bottom, rx, ry)` in px, y measured
    /// up from the host's bottom edge (negative is below it). The ellipse
    /// grows with the level and `bend_height` humps its top.
    #[must_use]
    pub fn mask_ellipse(&self, half_width: f64, height: f64) -> (f64, f64, f64, f64) {
        (
            self.host.width / 2.0 + self.center_offset * self.width_scale,
            -self.corner_lift,
            half_width * self.range.0 * self.width_scale * self.mask_width,
            height * self.range.1 * self.height_scale + self.bend_height,
        )
    }
}

/// The per-frame inputs of the band line.
pub(crate) struct BandInput {
    pub cx: f64,
    pub w: f64,
    pub h: f64,
    pub mw: f64,
    pub lift: f64,
    pub corner: f64,
}

/// The band line's points in host px, left to right (`bandPoints`).
pub(crate) fn band_points(
    config: &VoiceGlowConfig,
    f: &BandInput,
    host: HostSize,
) -> Vec<(f64, f64)> {
    let (cw, ch) = (host.width, host.height);
    let centre = cw / 2.0 + f.cx * f.w;
    let half = CEILING_HALF_WIDTH * config.range_width * f.w * f.mw;
    // Sit on the ceiling, but never leave the host: below scale 1 the clamp
    // shrinks with the scale, so a scaled-down beam keeps its proportions.
    let apex_cap = ch * 0.82 * config.scale.min(1.0);
    let apex =
        apex_cap.min((CEILING_HEIGHT * config.range_height * f.h + f.lift) * config.band_position);
    let base = ch - config.band_offset;
    // The tail hook is the resting look; the travelling beam has none. The
    // blend is finite and eased, gone by the time the gather reaches 25%.
    let tail_t = (f.corner * 4.0).min(1.0);
    let tail = config.band_tail * (1.0 - tail_t * tail_t * (3.0 - 2.0 * tail_t));
    let with_tail = tail > 0.001;
    let over = if with_tail {
        config.band_tail_overflow
    } else {
        0.0
    };
    let (x0, x1) = if with_tail {
        (-over, cw + over)
    } else {
        (centre - half, centre + half)
    };
    let arc_radius = painted_radius(config.radius, cw, ch);
    (0..=BAND_SAMPLES)
        .map(|i| {
            let x = x0 + (x1 - x0) * i as f64 / BAND_SAMPLES as f64;
            let t = ((x - centre) / half.max(1.0)).clamp(-1.0, 1.0);
            let edge = (if x < centre { centre } else { cw - centre }) + over;
            let y = bell(t, config.band_curve, config.band_spread, config.band_skew)
                + tail_lift(
                    (x - centre).abs(),
                    edge,
                    tail,
                    config.band_tail_position,
                    config.band_tail_curve,
                );
            // While gathered, the line rides the corner arcs instead of
            // running straight into the radius and being clipped.
            let arc = if f.corner > 0.0 {
                corner_lift(x, cw, arc_radius, 0.0) * f.corner
            } else {
                0.0
            };
            (x, base - apex * y - arc)
        })
        .collect()
}

/// The band's stroke recipe for a frame (`drawBand`), `None` when its alpha
/// is under 0.005 or it has no thickness.
pub(crate) fn band_paint(config: &VoiceGlowConfig, level: f64, strength: f64) -> Option<BandPaint> {
    let alpha = (0.6 * config.band_strength * strength).min(1.0);
    if alpha < 0.005 || config.band_width <= 0.0 {
        return None;
    }
    let dark = config.theme.is_dark();
    let bw = config.band_width * (1.0 + 0.35 * level);
    let split = config.band_aberration * (0.35 + 0.65 * level);
    let dy = (4.0 + 12.0 * split) * config.scale;
    let dx = 4.0 * split * config.scale;
    let base = (if dark { 0.42 } else { 0.4 }) * alpha;
    let thickness = 14.0 * bw;
    let blur = (3.5 * config.band_width) / 2.0;
    // The old rim was a gradient: a thin ridge with a linear ramp across its
    // thickness (stacked strokes of shrinking width).
    let ramp: [(f64, f64); 4] = [(1.0, 0.16), (0.72, 0.2), (0.46, 0.26), (0.22, 0.34)];
    let ridge = |color: Rgb, a: f64, ox: f64, oy: f64| BandRidge {
        color,
        offset: (ox, oy),
        strokes: ramp.map(|(width_mul, alpha_mul)| BandStroke {
            width: (thickness * width_mul).max(0.6),
            alpha: base * a * alpha_mul,
        }),
    };
    let c = &config.band_colors;
    Some(BandPaint {
        alpha,
        thickness,
        halo: BandStroke {
            width: thickness * 2.2,
            alpha: base * 0.3,
        },
        halo_color: c.core,
        ridges: [
            ridge(c.above, 1.0, dx, -dy),
            ridge(c.mid, 0.55, dx * 0.35, -dy * 0.35),
            ridge(c.below, 1.0, -dx, dy),
            ridge(c.core, 0.9, 0.0, 0.0),
        ],
        // A short fade when the line reaches the corners (the tail should be
        // seen there), the old long one when it ends inside the host.
        end_fade: if config.band_tail > 0.0 { 0.015 } else { 0.18 },
        blur,
        halo_blur: blur * 3.0,
    })
}
