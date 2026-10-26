//! Cursor light — `engine/cursor/light.ts` (live singleton).
//!
//! Three effects keyed off pointer distance to the nearest ring outline:
//! - **cursor** — lights a supplied pointer sprite ([`set_cursor_sprite`]);
//! - **spill** — soft glint under the pointer ([`MetalCursorSpill`]);
//! - **catch** — ring catch-light faces the pointer ([`MetalCursorLight`] → glow).

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use super::geometry::MetalFxVariant;

/// Live cursor-light tuning (`CursorLightConfig`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MetalCursorLightConfig {
    pub enabled: bool,
    pub reach: f32,
    pub fade_ms: f32,
    pub cursor: bool,
    pub cursor_distance: f32,
    pub cursor_strength: f32,
    pub cursor_diffuse: f32,
    pub cursor_falloff: f32,
    pub cursor_depth: f32,
    pub cursor_edge: f32,
    pub cursor_reach: f32,
    pub cursor_blur: f32,
    pub cursor_zoom: f32,
    pub spill: bool,
    pub spill_radius: f32,
    pub spill_strength: f32,
    pub spill_offset: f32,
    pub spill_lum_gain: f32,
    pub spill_saturation: f32,
    pub spill_inside: f32,
    pub spill_blur: f32,
    pub catch_light: bool,
    pub catch_follow: f32,
    pub catch_gain: f32,
}

impl MetalCursorLightConfig {
    /// Upstream `CURSOR_LIGHT_DEFAULTS`.
    pub const DEFAULTS: Self = Self {
        enabled: true,
        reach: 56.0,
        fade_ms: 200.0,
        cursor: true,
        cursor_distance: 186.0,
        cursor_strength: 3.35,
        cursor_diffuse: 1.4,
        cursor_falloff: 37.0,
        cursor_depth: 0.4,
        cursor_edge: 0.0,
        cursor_reach: 11.5,
        cursor_blur: 0.5,
        cursor_zoom: 3.0,
        spill: false,
        spill_radius: 48.0,
        spill_strength: 0.55,
        spill_offset: 0.35,
        spill_lum_gain: 0.7,
        spill_saturation: 1.3,
        spill_inside: 0.5,
        spill_blur: 0.0,
        catch_light: false,
        catch_follow: 0.25,
        catch_gain: 1.0,
    };
}

impl Default for MetalCursorLightConfig {
    fn default() -> Self {
        Self::DEFAULTS
    }
}

/// Per-frame catch-light sample handed to [`super::GlowState`] (`inst.cursorLight`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MetalCursorLight {
    /// Nearest outline point, box-local CSS px.
    pub x: f32,
    /// Nearest outline point, box-local CSS px.
    pub y: f32,
    /// Proximity envelope `0..=1` (`w` in the reference).
    pub weight: f32,
}

/// Soft page glint under the pointer (`spill` effect).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MetalCursorSpill {
    /// Centre in box-local CSS px.
    pub position: [f32; 2],
    /// Radius in CSS px.
    pub radius: f32,
    /// Peak opacity `0..=1`.
    pub opacity: f32,
    /// Tint `0..=1` per channel.
    pub tint: [f32; 3],
}

/// Raster cursor sprite for the lit-pointer path (`CursorSprite`).
#[derive(Clone, Debug)]
pub struct MetalCursorSprite {
    /// Hotspot in image px.
    pub hotspot: [f32; 2],
    /// Optional body centre override (default: image centre).
    pub center: Option<[f32; 2]>,
    /// Premultiplied RGBA8 row-major.
    pub rgba: Arc<[u8]>,
    pub width: u32,
    pub height: u32,
}

static CURSOR_LIGHT: Mutex<MetalCursorLightConfig> = Mutex::new(MetalCursorLightConfig::DEFAULTS);
static CURSOR_SPRITE: Mutex<Option<MetalCursorSprite>> = Mutex::new(None);
static CURSOR_REV: Mutex<u64> = Mutex::new(0);

fn lock_cfg() -> MutexGuard<'static, MetalCursorLightConfig> {
    CURSOR_LIGHT.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Current live cursor-light config.
#[must_use]
pub fn cursor_light_config() -> MetalCursorLightConfig {
    *lock_cfg()
}

/// Live config revision.
#[must_use]
pub fn cursor_light_config_revision() -> u64 {
    *CURSOR_REV.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Patches the live cursor-light config.
pub fn set_cursor_light_config(patch: impl FnOnce(&mut MetalCursorLightConfig)) {
    let mut g = lock_cfg();
    patch(&mut g);
    *CURSOR_REV.lock().unwrap_or_else(PoisonError::into_inner) += 1;
}

/// Restores `CURSOR_LIGHT_DEFAULTS`.
pub fn reset_cursor_light_config() {
    *lock_cfg() = MetalCursorLightConfig::DEFAULTS;
    *CURSOR_REV.lock().unwrap_or_else(PoisonError::into_inner) += 1;
}

/// Installs the pointer sprite used when `cursor` lighting is on.
pub fn set_cursor_sprite(sprite: Option<MetalCursorSprite>) {
    *CURSOR_SPRITE.lock().unwrap_or_else(PoisonError::into_inner) = sprite;
    *CURSOR_REV.lock().unwrap_or_else(PoisonError::into_inner) += 1;
}

/// Currently installed cursor sprite, if any.
#[must_use]
pub fn cursor_sprite() -> Option<MetalCursorSprite> {
    CURSOR_SPRITE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// Signed distance from a box-local point to the ring's outer outline
/// (positive outside), plus the nearest outline point — `nearestOutlinePoint`.
#[must_use]
pub fn nearest_outline_point(
    p: [f32; 2],
    w: f32,
    h: f32,
    radius: f32,
    kind: MetalFxVariant,
) -> (f32, [f32; 2]) {
    let rr = match kind {
        MetalFxVariant::Circle => w.min(h) * 0.5,
        MetalFxVariant::Button => radius.min(w.min(h) * 0.5).max(0.0),
    };
    let cx = w * 0.5;
    let cy = h * 0.5;
    let hx = (w * 0.5 - rr).max(0.0);
    let hy = (h * 0.5 - rr).max(0.0);
    let qx = (p[0] - cx).clamp(-hx, hx);
    let qy = (p[1] - cy).clamp(-hy, hy);
    let dx = p[0] - cx - qx;
    let dy = p[1] - cy - qy;
    let len = dx.hypot(dy);
    if len > 1e-6 {
        let out = [cx + qx + (dx / len) * rr, cy + qy + (dy / len) * rr];
        return (len - rr, out);
    }
    // Inside the straight-edged core: nearest side.
    let (dl, dr, dt, db) = (p[0], w - p[0], p[1], h - p[1]);
    let m = dl.min(dr).min(dt).min(db);
    let out = if (m - dl).abs() < 1e-6 {
        [0.0, p[1]]
    } else if (m - dr).abs() < 1e-6 {
        [w, p[1]]
    } else if (m - dt).abs() < 1e-6 {
        [p[0], 0.0]
    } else {
        [p[0], h]
    };
    (-m, out)
}

/// Smoothstep proximity weight for absolute outline distance `ad` within `reach`.
#[must_use]
pub fn proximity_weight(ad: f32, reach: f32) -> f32 {
    let reach = reach.max(1.0);
    if ad > reach {
        return 0.0;
    }
    let t = 1.0 - ad / reach;
    t * t * (3.0 - 2.0 * t)
}

/// Envelope lerp factor matching the reference (`fadeMs` ≈ 95% settle).
#[must_use]
pub fn fade_alpha(dt_secs: f32, fade_ms: f32) -> f32 {
    let tau = fade_ms.max(1.0) / 3.0;
    1.0 - (-(dt_secs * 1000.0) / tau).exp()
}

/// Soft radial spill bitmap matching the web's
/// `radial-gradient(closest-side, rgba 0% → 0.35@45% → 0@100%)`.
///
/// Returns straight RGBA8 (`width == height == 2 * radius_px`).
#[must_use]
pub fn bake_spill_rgba(radius_px: u32, tint: [f32; 3], opacity: f32) -> (u32, u32, Vec<u8>) {
    let radius_px = radius_px.max(1);
    let side = radius_px * 2;
    let mut rgba = vec![0u8; (side * side * 4) as usize];
    let r = radius_px as f32;
    let cx = r - 0.5;
    let cy = r - 0.5;
    let op = opacity.clamp(0.0, 1.0);
    let tr = tint[0].clamp(0.0, 1.0);
    let tg = tint[1].clamp(0.0, 1.0);
    let tb = tint[2].clamp(0.0, 1.0);
    for y in 0..side {
        for x in 0..side {
            let d = ((x as f32 - cx).hypot(y as f32 - cy) / r).clamp(0.0, 1.0);
            // Piecewise match of the CSS stops: 1 @ 0, 0.35 @ 0.45, 0 @ 1.
            let a = if d <= 0.45 {
                1.0 - (1.0 - 0.35) * (d / 0.45)
            } else {
                0.35 * (1.0 - (d - 0.45) / 0.55)
            }
            .clamp(0.0, 1.0)
                * op;
            if a < 0.002 {
                continue;
            }
            let i = ((y * side + x) * 4) as usize;
            rgba[i] = (tr * 255.0).round() as u8;
            rgba[i + 1] = (tg * 255.0).round() as u8;
            rgba[i + 2] = (tb * 255.0).round() as u8;
            rgba[i + 3] = (a * 255.0).round() as u8;
        }
    }
    (side, side, rgba)
}

/// Builds a spill frame from proximity, sample colour and luminance.
#[must_use]
pub fn spill_frame(
    cfg: &MetalCursorLightConfig,
    pointer: [f32; 2],
    outline: [f32; 2],
    weight: f32,
    rgb: [f32; 3],
    lum: f32,
    inside: bool,
) -> Option<MetalCursorSpill> {
    if !cfg.spill || weight < 0.002 {
        return None;
    }
    let peak = rgb[0].max(rgb[1]).max(rgb[2]).max(1e-6);
    // Saturate toward peak, then boost.
    let nr = rgb[0] / peak;
    let ng = rgb[1] / peak;
    let nb = rgb[2] / peak;
    let sat = cfg.spill_saturation;
    let mid = (nr + ng + nb) / 3.0;
    let tint = [
        (mid + (nr - mid) * sat).clamp(0.0, 1.0),
        (mid + (ng - mid) * sat).clamp(0.0, 1.0),
        (mid + (nb - mid) * sat).clamp(0.0, 1.0),
    ];
    let lf = (lum / 0.3).clamp(0.0, 1.0);
    let lum_mix = 1.0 - cfg.spill_lum_gain + cfg.spill_lum_gain * lf;
    let mut op = cfg.spill_strength * weight * lum_mix;
    if inside {
        op *= cfg.spill_inside;
    }
    let position = [
        pointer[0] + (outline[0] - pointer[0]) * cfg.spill_offset,
        pointer[1] + (outline[1] - pointer[1]) * cfg.spill_offset,
    ];
    Some(MetalCursorSpill {
        position,
        radius: cfg.spill_radius.max(1.0),
        opacity: op.clamp(0.0, 1.0),
        tint,
    })
}

/// Applies a simplified diffuse rim onto a cursor sprite using ring colour
/// (`drawCursor` diffuse term; specular mirror needs a live ring framebuffer).
///
/// `toward_ring` is the vector from the sprite body centre to the lit outline
/// point (CSS px).
#[must_use]
pub fn light_cursor_sprite(
    sprite: &MetalCursorSprite,
    cfg: &MetalCursorLightConfig,
    toward_ring: [f32; 2],
    ring_rgb: [f32; 3],
    ring_lum: f32,
    env: f32,
) -> Vec<u8> {
    let w = sprite.width as usize;
    let h = sprite.height as usize;
    let mut out = sprite.rgba.as_ref().to_vec();
    if out.len() < w * h * 4 || env < 0.002 || !cfg.cursor {
        return out;
    }
    let body = sprite
        .center
        .unwrap_or([sprite.width as f32 * 0.5, sprite.height as f32 * 0.5]);
    let dist = toward_ring[0].hypot(toward_ring[1]).max(0.01);
    let ux = toward_ring[0] / dist;
    let uy = toward_ring[1] / dist;
    let e = (sprite.width.min(sprite.height) as f32 * 0.35) + cfg.cursor_edge;
    let d_edge = (dist - e).max(0.0);
    let f0 = cfg.cursor_falloff.max(1.0);
    let falloff = 1.0 / (1.0 + (d_edge / f0) * (d_edge / f0));
    let diff = cfg.cursor_diffuse * falloff * (0.5 + 0.5 * (ring_lum / 0.5).min(1.0)) * env;
    if diff < 0.005 {
        return out;
    }
    let peak = ring_rgb[0].max(ring_rgb[1]).max(ring_rgb[2]).max(1e-6);
    let cr = ring_rgb[0] / peak;
    let cg = ring_rgb[1] / peak;
    let cb = ring_rgb[2] / peak;
    let fl = cfg.cursor_reach.max(1.0);
    let mx = body[0] + e * ux;
    let my = body[1] + e * uy;
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            let a = out[i + 3];
            if a == 0 {
                continue;
            }
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            // Distance into the body away from the mirror plane (facing the ring).
            let behind = -((px - mx) * ux + (py - my) * uy);
            let fade = if behind <= 0.0 {
                1.0
            } else {
                (1.0 - behind / (fl * 1.2)).clamp(0.0, 1.0)
            };
            let k = (diff * fade).clamp(0.0, 1.0);
            if k < 1e-4 {
                continue;
            }
            let af = f32::from(a) / 255.0;
            let add = |c: u8, t: f32| -> u8 {
                let v = f32::from(c) / 255.0 + t * k * af;
                (v.clamp(0.0, 1.0) * 255.0).round() as u8
            };
            out[i] = add(out[i], cr);
            out[i + 1] = add(out[i + 1], cg);
            out[i + 2] = add(out[i + 2], cb);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_light_ts() {
        let d = MetalCursorLightConfig::DEFAULTS;
        assert!(d.enabled);
        assert!((d.reach - 56.0).abs() < 1e-6);
        assert!((d.cursor_strength - 3.35).abs() < 1e-6);
        assert!(!d.catch_light);
        assert!(!d.spill);
    }

    #[test]
    fn outline_outside_is_positive() {
        let (d, pt) =
            nearest_outline_point([200.0, 20.0], 140.0, 40.0, 20.0, MetalFxVariant::Button);
        assert!(d > 0.0, "outside: {d}");
        assert!((pt[0] - 140.0).abs() < 1.0, "pt={pt:?}");
    }

    #[test]
    fn outline_inside_is_negative() {
        let (d, _) = nearest_outline_point([70.0, 20.0], 140.0, 40.0, 20.0, MetalFxVariant::Button);
        assert!(d < 0.0, "inside: {d}");
    }

    #[test]
    fn sprite_round_trips() {
        set_cursor_sprite(Some(MetalCursorSprite {
            hotspot: [1.0, 2.0],
            center: None,
            rgba: Arc::from(vec![255u8; 16]),
            width: 2,
            height: 2,
        }));
        assert!(cursor_sprite().is_some());
        set_cursor_sprite(None);
        assert!(cursor_sprite().is_none());
    }

    #[test]
    fn spill_radial_is_soft_not_a_hard_disc() {
        let (w, h, rgba) = bake_spill_rgba(32, [1.0, 1.0, 1.0], 1.0);
        assert_eq!(w, 64);
        assert_eq!(h, 64);
        let center = ((32 * 64 + 32) * 4) as usize;
        let edge = ((32 * 64 + 63) * 4) as usize;
        let mid = ((32 * 64 + 32 + 14) * 4) as usize; // ~45% of radius
        assert!(rgba[center + 3] > 240, "core opaque");
        assert!(
            rgba[mid + 3] > 60 && rgba[mid + 3] < 120,
            "mid stop ~0.35: {}",
            rgba[mid + 3]
        );
        assert!(rgba[edge + 3] < 8, "rim fades out: {}", rgba[edge + 3]);
    }
}
