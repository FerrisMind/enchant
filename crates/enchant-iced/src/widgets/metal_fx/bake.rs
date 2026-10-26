//! Pre-rendered glow sprites, from `engine/glow/bake.ts`.
//!
//! The halo is four blurred round-capped strokes and the catch-light two tight
//! ones with a radial fade. Rendered once, in white, into small alpha bitmaps;
//! the shader then only samples them at the hotspot. Blur is the reference's
//! 3-pass box blur (a close gaussian approximation).
//!
//! Sprites are cached by everything that shapes them (half-length, scale and
//! the baked glow-config values) and shared by every instance with the same
//! key. They are baked at [`SPRITE_DENSITY`] device px per CSS px and sampled
//! with a linear filter at any display scale.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, PoisonError};

use super::glow_config::MetalGlowConfig;

/// Supersampling the sprites are baked at (device px per CSS px).
pub const SPRITE_DENSITY: f32 = 2.0;

/// A white, alpha-only glow bitmap.
///
/// The segment runs along `+x` through the anchor; the anchor is the segment's
/// centre, so rotating about it and translating to the hotspot is exact.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::{bake_halo, MetalGlowConfig};
///
/// let halo = bake_halo(7.8, 1.0, &MetalGlowConfig::DEFAULTS);
/// let (ax, ay) = halo.anchor();
/// assert!(halo.size().0 > 2.0 * 7.8 && ax > 30.0 && ay > 30.0);
/// ```
#[derive(Clone, Debug)]
pub struct GlowSprite {
    id: u64,
    px_width: u32,
    px_height: u32,
    alpha: Vec<u8>,
    width: f32,
    height: f32,
    anchor: [f32; 2],
}

impl GlowSprite {
    /// A stable identity of the cache key (what the GPU texture cache uses).
    #[must_use]
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Bitmap size in device px.
    #[must_use]
    pub fn pixel_size(&self) -> (u32, u32) {
        (self.px_width, self.px_height)
    }

    /// Alpha per device px, row-major.
    #[must_use]
    pub fn alpha(&self) -> &[u8] {
        &self.alpha
    }

    /// CSS-px size to draw it at.
    #[must_use]
    pub fn size(&self) -> (f32, f32) {
        (self.width, self.height)
    }

    /// CSS-px offset from the top-left to the anchor.
    #[must_use]
    pub fn anchor(&self) -> (f32, f32) {
        (self.anchor[0], self.anchor[1])
    }

    /// Bilinear alpha (`0..=1`) at a CSS-px offset from the anchor, `0` outside.
    #[must_use]
    pub fn sample(&self, dx: f32, dy: f32) -> f32 {
        let u = (dx + self.anchor[0]) / self.width * self.px_width as f32 - 0.5;
        let v = (dy + self.anchor[1]) / self.height * self.px_height as f32 - 0.5;
        if dx + self.anchor[0] < 0.0
            || dy + self.anchor[1] < 0.0
            || dx + self.anchor[0] > self.width
            || dy + self.anchor[1] > self.height
        {
            return 0.0;
        }
        let at = |x: i64, y: i64| {
            let x = x.clamp(0, i64::from(self.px_width) - 1) as usize;
            let y = y.clamp(0, i64::from(self.px_height) - 1) as usize;
            f32::from(self.alpha[y * self.px_width as usize + x]) / 255.0
        };
        let (x0, y0) = (u.floor(), v.floor());
        let (fx, fy) = (u - x0, v - y0);
        let (x0, y0) = (x0 as i64, y0 as i64);
        let top = at(x0, y0) * (1.0 - fx) + at(x0 + 1, y0) * fx;
        let bottom = at(x0, y0 + 1) * (1.0 - fx) + at(x0 + 1, y0 + 1) * fx;
        top * (1.0 - fy) + bottom * fy
    }
}

#[derive(Clone, Copy)]
struct Layer {
    stroke: f32,
    blur: f32,
    opacity: f32,
}

/// Box sizes for `n` passes approximating a gaussian of `sigma` (Kutskir).
fn boxes_for_gauss(sigma: f32, n: usize) -> Vec<usize> {
    let nf = n as f32;
    let w_ideal = ((12.0 * sigma * sigma) / nf + 1.0).sqrt();
    let mut wl = w_ideal.floor() as i64;
    if wl % 2 == 0 {
        wl -= 1;
    }
    let wu = wl + 2;
    let wlf = wl as f32;
    let m_ideal =
        (12.0 * sigma * sigma - nf * wlf * wlf - 4.0 * nf * wlf - 3.0 * nf) / (-4.0 * wlf - 4.0);
    let m = m_ideal.round() as i64;
    (0..n as i64)
        .map(|i| if i < m { wl } else { wu }.max(1) as usize)
        .collect()
}

fn box_blur_h(src: &[f32], dst: &mut [f32], w: usize, h: usize, r: usize) {
    let iarr = 1.0 / (2 * r + 1) as f32;
    for y in 0..h {
        let row = y * w;
        let mut acc = 0.0;
        for x in -(r as i64)..=(r as i64) {
            acc += src[row + x.clamp(0, w as i64 - 1) as usize];
        }
        for x in 0..w {
            dst[row + x] = acc * iarr;
            let out = row + x.saturating_sub(r);
            let inn = row + (x + r + 1).min(w - 1);
            acc += src[inn] - src[out];
        }
    }
}

fn box_blur_v(src: &[f32], dst: &mut [f32], w: usize, h: usize, r: usize) {
    let iarr = 1.0 / (2 * r + 1) as f32;
    for x in 0..w {
        let mut acc = 0.0;
        for y in -(r as i64)..=(r as i64) {
            acc += src[y.clamp(0, h as i64 - 1) as usize * w + x];
        }
        for y in 0..h {
            dst[y * w + x] = acc * iarr;
            let out = y.saturating_sub(r) * w + x;
            let inn = (y + r + 1).min(h - 1) * w + x;
            acc += src[inn] - src[out];
        }
    }
}

/// The reference's `gaussBlur`: three box blurs on `a` (in place).
pub(crate) fn gauss_blur(a: &mut [f32], w: usize, h: usize, sigma: f32) {
    if sigma <= 0.05 || w == 0 || h == 0 {
        return;
    }
    let mut tmp = vec![0.0; a.len()];
    for size in boxes_for_gauss(sigma, 3) {
        let r = (size - 1) / 2;
        box_blur_h(a, &mut tmp, w, h, r);
        box_blur_v(&tmp, a, w, h, r);
    }
}

/// Alpha of a horizontal round-capped line, analytically anti-aliased.
fn stroke_alpha(
    half_len: f32,
    stroke_w: f32,
    w: usize,
    h: usize,
    dpr: f32,
    ax: f32,
    ay: f32,
) -> Vec<f32> {
    let mut out = vec![0.0; w * h];
    let half = stroke_w / 2.0;
    for y in 0..h {
        let cy = (y as f32 + 0.5) / dpr;
        for x in 0..w {
            let cx = (x as f32 + 0.5) / dpr;
            let dx = ((cx - ax).abs() - half_len).max(0.0);
            let d = dx.hypot(cy - ay);
            out[y * w + x] = (0.5 + (half - d) * dpr).clamp(0.0, 1.0);
        }
    }
    out
}

fn compose(layers: &[Layer], half_len: f32, s: f32, dpr: f32, fade: f32, id: u64) -> GlowSprite {
    let pad_max = layers
        .iter()
        .map(|l| (l.stroke / 2.0 + 3.0 * l.blur) * s)
        .fold(0.0_f32, f32::max);
    let pad = pad_max.ceil() + 1.0;
    let cw = 2.0 * half_len + 2.0 * pad;
    let ch = 2.0 * pad;
    let w = (cw * dpr).ceil() as usize;
    let h = (ch * dpr).ceil() as usize;
    let mut acc = vec![0.0_f32; w * h];
    for layer in layers {
        let mut a = stroke_alpha(half_len, layer.stroke * s, w, h, dpr, pad, pad);
        gauss_blur(&mut a, w, h, layer.blur * s * dpr);
        for (dst, src) in acc.iter_mut().zip(&a) {
            let la = src * layer.opacity;
            *dst += la * (1.0 - *dst);
        }
    }
    if fade > 0.0 {
        // SVG luminance mask: white to 0.30, #404040 (0.25) at 0.65, black at 1.
        let (cx, cy, radius) = (pad * dpr, pad * dpr, fade * s * dpr);
        for y in 0..h {
            for x in 0..w {
                let t = (x as f32 + 0.5 - cx).hypot(y as f32 + 0.5 - cy) / radius;
                let m = if t <= 0.3 {
                    1.0
                } else if t <= 0.65 {
                    1.0 - ((t - 0.3) / 0.35) * 0.75
                } else if t < 1.0 {
                    0.25 * (1.0 - (t - 0.65) / 0.35)
                } else {
                    0.0
                };
                acc[y * w + x] *= m;
            }
        }
    }
    let alpha = acc
        .iter()
        .map(|v| (v.min(1.0) * 255.0).round() as u8)
        .collect();
    GlowSprite {
        id,
        px_width: w as u32,
        px_height: h as u32,
        alpha,
        width: cw,
        height: ch,
        anchor: [pad, pad],
    }
}

type Cache = HashMap<Vec<u32>, Arc<GlowSprite>>;
static CACHE: Mutex<Option<Cache>> = Mutex::new(None);
const CACHE_LIMIT: usize = 48;

fn cached(key: Vec<u32>, build: impl FnOnce(u64) -> GlowSprite) -> Arc<GlowSprite> {
    let mut guard = CACHE.lock().unwrap_or_else(PoisonError::into_inner);
    let cache = guard.get_or_insert_with(HashMap::new);
    if let Some(hit) = cache.get(&key) {
        return Arc::clone(hit);
    }
    if cache.len() >= CACHE_LIMIT {
        cache.clear();
    }
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    let sprite = Arc::new(build(hasher.finish()));
    cache.insert(key, Arc::clone(&sprite));
    sprite
}

fn bits(values: &[f32]) -> impl Iterator<Item = u32> + '_ {
    values.iter().map(|v| v.to_bits())
}

/// The wide halo: four blurred strokes stacked at `half_len` half-length
/// (`bakeHalo`). `scale` is the master scale of absolute units.
#[must_use]
pub fn bake_halo(half_len: f32, scale: f32, cfg: &MetalGlowConfig) -> Arc<GlowSprite> {
    let mut key = vec![0, half_len.to_bits(), scale.to_bits()];
    key.extend(bits(&cfg.halo_stroke));
    key.extend(bits(&cfg.halo_blur));
    key.extend(bits(&cfg.halo_op));
    cached(key, |id| {
        let layers: Vec<Layer> = (0..4)
            .map(|i| Layer {
                stroke: cfg.halo_stroke[i],
                blur: cfg.halo_blur[i],
                opacity: cfg.halo_op[i],
            })
            .collect();
        compose(&layers, half_len, scale, SPRITE_DENSITY, 0.0, id)
    })
}

/// The catch-light: two tight strokes with a radial fade at the ends
/// (`bakeExtra`).
#[must_use]
pub fn bake_extra(half_len: f32, scale: f32, cfg: &MetalGlowConfig) -> Arc<GlowSprite> {
    let mut key = vec![1, half_len.to_bits(), scale.to_bits()];
    key.extend(bits(&[
        cfg.extra_stroke_outer,
        cfg.extra_stroke_core,
        cfg.extra_blur_outer,
        cfg.extra_blur_core,
        cfg.extra_fade_r,
        cfg.extra_op_outer,
    ]));
    cached(key, |id| {
        let layers = [
            Layer {
                stroke: cfg.extra_stroke_outer,
                blur: cfg.extra_blur_outer,
                opacity: cfg.extra_op_outer,
            },
            Layer {
                stroke: cfg.extra_stroke_core,
                blur: cfg.extra_blur_core,
                opacity: 1.0,
            },
        ];
        compose(
            &layers,
            half_len,
            scale,
            SPRITE_DENSITY,
            cfg.extra_fade_r,
            id,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_sizes_follow_kutskir() {
        // sigma 8.4: wIdeal = sqrt(12*70.56/3 + 1) = 16.8 -> wl 15 (odd), wu 17.
        let b = boxes_for_gauss(8.4, 3);
        assert_eq!(b.len(), 3);
        assert!(b.iter().all(|s| s % 2 == 1));
        // Variance of the three boxes approximates sigma^2.
        let var: f32 = b.iter().map(|s| ((s * s) as f32 - 1.0) / 12.0).sum();
        assert!(
            (var - 8.4 * 8.4).abs() / (8.4 * 8.4) < 0.05,
            "variance {var}"
        );
    }

    #[test]
    fn blur_conserves_mass_away_from_edges() {
        let (w, h) = (64, 64);
        let mut a = vec![0.0; w * h];
        a[32 * w + 32] = 1.0;
        gauss_blur(&mut a, w, h, 3.0);
        let sum: f32 = a.iter().sum();
        assert!((sum - 1.0).abs() < 1e-3, "mass {sum}");
        let peak = a.iter().cloned().fold(0.0, f32::max);
        // Peak of a 2-D gaussian: 1/(2 pi sigma^2) ~ 0.0177
        assert!((peak - 0.0177).abs() < 0.004, "peak {peak}");
    }

    #[test]
    fn sigma_below_threshold_is_a_no_op() {
        let mut a = vec![0.0, 1.0, 0.0, 0.0];
        gauss_blur(&mut a, 2, 2, 0.04);
        assert_eq!(a, vec![0.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn halo_sprite_is_a_soft_horizontal_blob() {
        let cfg = MetalGlowConfig::DEFAULTS;
        let halo = bake_halo(7.8, 1.0, &cfg);
        let (w, h) = halo.size();
        // pad = ceil((26.4/2 + 3*8.4) * 1) + 1 = 40
        assert_eq!(halo.anchor(), (40.0, 40.0));
        assert!((w - (2.0 * 7.8 + 80.0)).abs() < 1e-4 && (h - 80.0).abs() < 1e-4);
        assert_eq!(
            halo.pixel_size(),
            (
                (w * SPRITE_DENSITY).ceil() as u32,
                (h * SPRITE_DENSITY).ceil() as u32
            )
        );
        let centre = halo.sample(0.0, 0.0);
        assert!(
            centre > 0.9,
            "core of the halo is near-opaque, got {centre}"
        );
        // Wider along x (segment) than y at the same distance.
        assert!(halo.sample(14.0, 0.0) > halo.sample(0.0, 14.0));
        // Fades to zero well outside.
        assert!(halo.sample(0.0, 38.0) < 0.02);
        assert_eq!(halo.sample(500.0, 0.0), 0.0);
    }

    #[test]
    fn extra_sprite_has_the_radial_end_fade() {
        let cfg = MetalGlowConfig::DEFAULTS;
        let extra = bake_extra(cfg.extra_half_len, 1.0, &cfg);
        // Two tight blurred strokes: 1-(1-.54)(1-.58) ~ .8 at the core.
        let core = extra.sample(0.0, 0.0);
        assert!(core > 0.7 && core < 0.95, "core {core}");
        // Mask hits zero at fadeR = 13/3 from the anchor.
        assert!(extra.sample(cfg.extra_fade_r + 0.5, 0.0) < 0.02);
        // Between 0.3R and 0.65R the mask is partial.
        let mid = extra.sample(0.5 * cfg.extra_fade_r, 0.0);
        assert!(mid > 0.1 && mid < 0.95, "mid {mid}");
    }

    #[test]
    fn sprites_are_cached_by_key() {
        let cfg = MetalGlowConfig::DEFAULTS;
        let a = bake_halo(9.0, 1.0, &cfg);
        let b = bake_halo(9.0, 1.0, &cfg);
        assert!(Arc::ptr_eq(&a, &b));
        let c = bake_halo(9.5, 1.0, &cfg);
        assert!(!Arc::ptr_eq(&a, &c));
        assert_ne!(a.id(), c.id());
        let mut tuned = cfg;
        tuned.halo_blur[0] = 5.0;
        assert!(!Arc::ptr_eq(&a, &bake_halo(9.0, 1.0, &tuned)));
        // Runtime-only fields don't affect the key.
        let mut runtime = cfg;
        runtime.peak_op = 0.1;
        assert!(Arc::ptr_eq(&a, &bake_halo(9.0, 1.0, &runtime)));
    }

    #[test]
    fn scale_grows_the_sprite() {
        let cfg = MetalGlowConfig::DEFAULTS;
        let one = bake_extra(3.0, 1.0, &cfg);
        let two = bake_extra(3.0, 2.0, &cfg);
        assert!(two.size().1 > one.size().1);
    }
}
