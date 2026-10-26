//! Live-tunable glow parameters, from `engine/glow/config.ts`.
//!
//! Every number the halo + catch-light overlay used to hard-code lives in one
//! process-wide [`MetalGlowConfig`] so a tuning surface can drive it at
//! runtime. Two classes of field:
//!
//! - *runtime* fields are read every tick; a change shows on the next frame.
//! - *markup* fields (stroke widths, blur radii, per-layer opacities, blob
//!   lengths) are baked into the glow sprites; a change rebakes them, which the
//!   widgets notice through [`glow_config_revision`].
//!
//! [`MetalGlowConfig::DEFAULTS`] are the shipped values, so
//! [`reset_glow_config`] is an exact restore.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{PoisonError, RwLock};

/// The glow tuning knobs (`GlowConfig` in `config.ts`).
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::{
///     glow_config, reset_glow_config, update_glow_config, MetalGlowConfig,
/// };
///
/// assert_eq!(glow_config(), MetalGlowConfig::DEFAULTS);
/// update_glow_config(|c| c.peak_op = 0.5);
/// assert_eq!(glow_config().peak_op, 0.5);
/// reset_glow_config();
/// assert_eq!(glow_config(), MetalGlowConfig::DEFAULTS);
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MetalGlowConfig {
    // ── runtime ──────────────────────────────────────────────────────────
    /// Overall halo group opacity multiplier.
    pub halo_op_mul: f32,
    /// Catch-light group intensity multiplier (clamped to 1 after multiply).
    pub extra_intensity: f32,
    /// Glow opacity at a fully-lit perimeter point.
    pub peak_op: f32,
    /// Glow opacity at a dark perimeter point.
    pub base_op: f32,
    /// Perpendicular inset of the halo from the ring, CSS px at 1×.
    pub inset: f32,
    /// Extra outward offset for the catch-light, CSS px at 1×.
    pub extra_outward: f32,
    /// Random wander amplitude along the perimeter at the 140×40 reference.
    pub wander_range: f32,
    /// Per-tick lerp toward the wander target (at the 15 fps reference rate).
    pub wander_lerp: f32,
    /// Per-tick lerp of opacity toward its luminance-driven target.
    pub fade_rate: f32,
    /// Luminance below which the glow sits at `base_op`.
    pub lum_lo: f32,
    /// Luminance above which the glow reaches `peak_op`.
    pub lum_hi: f32,
    /// Minimum time at a hotspot before a brighter rival can steal it, ms.
    pub min_dwell_ms: f32,
    /// Appear duration, ms. A relocation is a full fade-out at the old spot
    /// then a full fade-in at the new one; the glow never slides.
    pub reloc_fade_ms: f32,
    /// Disappear duration, ms.
    pub reloc_fade_out_ms: f32,
    /// Point-mode (masked text/glyph) intensity multiplier.
    pub point_gain: f32,

    // ── markup (rebakes the sprites) ─────────────────────────────────────
    /// Half-length of the halo stroke path at the reference pill.
    pub halo_half_len: f32,
    /// Half-length of the catch-light stroke path at the reference pill.
    pub extra_half_len: f32,
    /// Halo stroke widths, widest first.
    pub halo_stroke: [f32; 4],
    /// Halo gaussian blur sigmas, widest first.
    pub halo_blur: [f32; 4],
    /// Halo per-layer opacities, widest first.
    pub halo_op: [f32; 4],
    /// Catch-light outer stroke width.
    pub extra_stroke_outer: f32,
    /// Catch-light core stroke width.
    pub extra_stroke_core: f32,
    /// Catch-light outer blur sigma.
    pub extra_blur_outer: f32,
    /// Catch-light core blur sigma.
    pub extra_blur_core: f32,
    /// Radius of the radial mask that fades the catch-light's ends.
    pub extra_fade_r: f32,
    /// Opacity of the catch-light's outer stroke.
    pub extra_op_outer: f32,
}

impl MetalGlowConfig {
    /// The values that shipped before the config existed.
    pub const DEFAULTS: Self = Self {
        halo_op_mul: 2.0,
        extra_intensity: 3.51,
        peak_op: 0.85,
        base_op: 0.34,
        inset: 1.5,
        extra_outward: 1.0,
        wander_range: 15.0,
        wander_lerp: 0.0075,
        fade_rate: 0.00875,
        lum_lo: 0.08,
        lum_hi: 0.32,
        min_dwell_ms: 1500.0,
        reloc_fade_ms: 300.0,
        reloc_fade_out_ms: 450.0,
        point_gain: 2.5,
        halo_half_len: 7.8,
        extra_half_len: 9.13952 / 3.0,
        halo_stroke: [26.4, 15.6, 7.2, 3.0],
        halo_blur: [8.4, 4.8, 2.1, 0.9],
        halo_op: [0.385, 0.595, 0.70, 0.70],
        extra_stroke_outer: 4.0 / 3.0,
        extra_stroke_core: 2.0 / 3.0,
        extra_blur_outer: 2.0 / 3.0,
        extra_blur_core: 1.35 / 3.0,
        extra_fade_r: 13.0 / 3.0,
        extra_op_outer: 0.85,
    };

    /// Whether any *markup* field differs from `other` (the sprites need a
    /// rebake).
    #[must_use]
    pub fn markup_differs(&self, other: &Self) -> bool {
        self.halo_half_len != other.halo_half_len
            || self.extra_half_len != other.extra_half_len
            || self.halo_stroke != other.halo_stroke
            || self.halo_blur != other.halo_blur
            || self.halo_op != other.halo_op
            || self.extra_stroke_outer != other.extra_stroke_outer
            || self.extra_stroke_core != other.extra_stroke_core
            || self.extra_blur_outer != other.extra_blur_outer
            || self.extra_blur_core != other.extra_blur_core
            || self.extra_fade_r != other.extra_fade_r
            || self.extra_op_outer != other.extra_op_outer
    }
}

impl Default for MetalGlowConfig {
    fn default() -> Self {
        Self::DEFAULTS
    }
}

static GLOW: RwLock<MetalGlowConfig> = RwLock::new(MetalGlowConfig::DEFAULTS);
static REVISION: AtomicU64 = AtomicU64::new(0);

/// A snapshot of the live glow configuration (`GLOW`).
#[must_use]
pub fn glow_config() -> MetalGlowConfig {
    *GLOW.read().unwrap_or_else(PoisonError::into_inner)
}

/// Replaces the live glow configuration (`setGlowConfig`).
pub fn set_glow_config(config: MetalGlowConfig) {
    let mut live = GLOW.write().unwrap_or_else(PoisonError::into_inner);
    if *live != config {
        *live = config;
        REVISION.fetch_add(1, Ordering::Release);
    }
}

/// Edits the live glow configuration in place (the partial `setGlowConfig`).
pub fn update_glow_config(edit: impl FnOnce(&mut MetalGlowConfig)) {
    let mut next = glow_config();
    edit(&mut next);
    set_glow_config(next);
}

/// Restores [`MetalGlowConfig::DEFAULTS`] (`resetGlowConfig`).
pub fn reset_glow_config() {
    set_glow_config(MetalGlowConfig::DEFAULTS);
}

/// Bumped on every real change of the live configuration; widgets compare it
/// between frames (the poll form of `subscribeGlowConfig`).
#[must_use]
pub fn glow_config_revision() -> u64 {
    REVISION.load(Ordering::Acquire)
}

#[cfg(test)]
pub(crate) static CONFIG_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_config_ts() {
        let d = MetalGlowConfig::DEFAULTS;
        assert_eq!(
            (d.halo_op_mul, d.extra_intensity, d.peak_op, d.base_op),
            (2.0, 3.51, 0.85, 0.34)
        );
        assert_eq!((d.inset, d.extra_outward, d.wander_range), (1.5, 1.0, 15.0));
        assert_eq!(
            (d.wander_lerp, d.fade_rate, d.lum_lo, d.lum_hi),
            (0.0075, 0.00875, 0.08, 0.32)
        );
        assert_eq!(
            (
                d.min_dwell_ms,
                d.reloc_fade_ms,
                d.reloc_fade_out_ms,
                d.point_gain
            ),
            (1500.0, 300.0, 450.0, 2.5)
        );
        assert_eq!(d.halo_half_len, 7.8);
        assert!((d.extra_half_len - 3.046_506_7).abs() < 1e-6);
        assert_eq!(d.halo_stroke, [26.4, 15.6, 7.2, 3.0]);
        assert_eq!(d.halo_blur, [8.4, 4.8, 2.1, 0.9]);
        assert_eq!(d.halo_op, [0.385, 0.595, 0.70, 0.70]);
        assert!((d.extra_stroke_outer - 4.0 / 3.0).abs() < 1e-6);
        assert!((d.extra_stroke_core - 2.0 / 3.0).abs() < 1e-6);
        assert!((d.extra_blur_outer - 2.0 / 3.0).abs() < 1e-6);
        assert!((d.extra_blur_core - 0.45).abs() < 1e-6);
        assert!((d.extra_fade_r - 13.0 / 3.0).abs() < 1e-6);
        assert_eq!(d.extra_op_outer, 0.85);
    }

    #[test]
    fn live_config_round_trips_and_bumps_the_revision() {
        let _guard = CONFIG_TEST_LOCK
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        reset_glow_config();
        let before = glow_config_revision();
        update_glow_config(|c| c.wander_range = 3.0);
        assert_eq!(glow_config().wander_range, 3.0);
        assert!(glow_config_revision() > before);
        let after = glow_config_revision();
        update_glow_config(|c| c.wander_range = 3.0);
        assert_eq!(
            glow_config_revision(),
            after,
            "no-op writes keep the revision"
        );
        reset_glow_config();
        assert_eq!(glow_config(), MetalGlowConfig::DEFAULTS);
    }

    #[test]
    fn markup_differs_only_for_baked_fields() {
        let a = MetalGlowConfig::DEFAULTS;
        let mut b = a;
        b.peak_op = 0.1;
        b.wander_range = 1.0;
        assert!(!a.markup_differs(&b));
        b.halo_blur[2] = 3.0;
        assert!(a.markup_differs(&b));
    }
}
