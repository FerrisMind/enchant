//! Live reflection occluder config — `REFLECTION_OCCLUDER` in `paint.ts`.

use std::sync::{Mutex, MutexGuard, PoisonError};

/// Cursor shadow cut from a proximity reflection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MetalReflectionOccluderConfig {
    pub enabled: bool,
    pub radius: f32,
    pub strength: f32,
    pub penumbra: f32,
    pub falloff: f32,
    pub edge_fade: f32,
    pub softness: f32,
    pub repaint_ms: f32,
}

impl MetalReflectionOccluderConfig {
    /// Upstream `REFLECTION_OCCLUDER_DEFAULTS`.
    pub const DEFAULTS: Self = Self {
        enabled: true,
        radius: 11.5,
        strength: 0.57,
        penumbra: 0.55,
        falloff: 0.21,
        edge_fade: 0.7,
        softness: 0.24,
        repaint_ms: 36.0,
    };
}

impl Default for MetalReflectionOccluderConfig {
    fn default() -> Self {
        Self::DEFAULTS
    }
}

static OCCLUDER: Mutex<MetalReflectionOccluderConfig> =
    Mutex::new(MetalReflectionOccluderConfig::DEFAULTS);
static REV: Mutex<u64> = Mutex::new(0);

fn lock() -> MutexGuard<'static, MetalReflectionOccluderConfig> {
    OCCLUDER.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Current live occluder config.
#[must_use]
pub fn reflection_occluder_config() -> MetalReflectionOccluderConfig {
    *lock()
}

/// Live config revision.
#[must_use]
pub fn reflection_occluder_config_revision() -> u64 {
    *REV.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Patches the live occluder config.
pub fn set_reflection_occluder_config(patch: impl FnOnce(&mut MetalReflectionOccluderConfig)) {
    let mut g = lock();
    patch(&mut g);
    *REV.lock().unwrap_or_else(PoisonError::into_inner) += 1;
}

/// Restores defaults.
pub fn reset_reflection_occluder_config() {
    *lock() = MetalReflectionOccluderConfig::DEFAULTS;
    *REV.lock().unwrap_or_else(PoisonError::into_inner) += 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_paint_ts() {
        let d = MetalReflectionOccluderConfig::DEFAULTS;
        assert!(d.enabled);
        assert!((d.radius - 11.5).abs() < 1e-6);
        assert!((d.strength - 0.57).abs() < 1e-6);
    }
}
