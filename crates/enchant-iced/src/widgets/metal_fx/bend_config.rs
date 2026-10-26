//! Live bend (cursor liquid-dent) config — `engine/bend/config.ts`.

use std::sync::{Mutex, MutexGuard, PoisonError};

/// What the bend deforms (`BendConfig.applyTo`).
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MetalBendApplyTo {
    /// Only the metal ring / sheet.
    #[default]
    Ring,
    /// The whole MetalFx stack.
    All,
}

/// Cursor liquid-dent tuning (`BendConfig`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MetalBendConfig {
    pub enabled: bool,
    pub apply_to: MetalBendApplyTo,
    pub strength: f32,
    pub fade_in_ms: f32,
    pub fade_out_ms: f32,
    pub smooth_ms: f32,
    pub reach: f32,
    pub blob: f32,
    pub liquid_blob: f32,
    pub max_disp: f32,
    pub gain: f32,
    pub press_gain: f32,
    pub pull_gain: f32,
    pub press: f32,
    pub liquid: f32,
    pub liquid_reach: f32,
    pub liquid_stiffness: f32,
    pub liquid_damping: f32,
    pub stiffness: f32,
    pub damping: f32,
    pub mass: f32,
    pub follow: f32,
    pub map_res: f32,
    pub smooth: f32,
}

impl MetalBendConfig {
    /// Upstream `BEND_DEFAULTS`.
    pub const DEFAULTS: Self = Self {
        enabled: true,
        apply_to: MetalBendApplyTo::Ring,
        strength: 0.74,
        fade_in_ms: 200.0,
        fade_out_ms: 350.0,
        smooth_ms: 140.0,
        reach: 36.0,
        blob: 13.0,
        liquid_blob: 10.0,
        max_disp: 9.0,
        gain: 0.6,
        press_gain: 0.55,
        pull_gain: 0.49,
        press: 5.0,
        liquid: 7.5,
        liquid_reach: 8.0,
        liquid_stiffness: 53.0,
        liquid_damping: 9.0,
        stiffness: 260.0,
        damping: 13.0,
        mass: 1.0,
        follow: 0.32,
        map_res: 2.0,
        smooth: 0.25,
    };
}

impl Default for MetalBendConfig {
    fn default() -> Self {
        Self::DEFAULTS
    }
}

static BEND: Mutex<MetalBendConfig> = Mutex::new(MetalBendConfig::DEFAULTS);
static BEND_REV: Mutex<u64> = Mutex::new(0);

fn lock(m: &Mutex<MetalBendConfig>) -> MutexGuard<'_, MetalBendConfig> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Current live bend config (clone).
#[must_use]
pub fn bend_config() -> MetalBendConfig {
    *lock(&BEND)
}

/// Live config revision (bumped on every set/reset).
#[must_use]
pub fn bend_config_revision() -> u64 {
    *BEND_REV.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Patches the live bend config.
pub fn set_bend_config(patch: impl FnOnce(&mut MetalBendConfig)) {
    let mut g = lock(&BEND);
    patch(&mut g);
    *BEND_REV.lock().unwrap_or_else(PoisonError::into_inner) += 1;
}

/// Restores `BEND_DEFAULTS`.
pub fn reset_bend_config() {
    *lock(&BEND) = MetalBendConfig::DEFAULTS;
    *BEND_REV.lock().unwrap_or_else(PoisonError::into_inner) += 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_config_ts() {
        let d = MetalBendConfig::DEFAULTS;
        assert!(d.enabled);
        assert_eq!(d.apply_to, MetalBendApplyTo::Ring);
        assert!((d.strength - 0.74).abs() < 1e-6);
        assert!((d.stiffness - 260.0).abs() < 1e-6);
        assert!((d.liquid - 7.5).abs() < 1e-6);
        assert!((d.follow - 0.32).abs() < 1e-6);
    }

    #[test]
    fn live_config_round_trips() {
        let before = bend_config_revision();
        set_bend_config(|c| c.strength = 0.5);
        assert!((bend_config().strength - 0.5).abs() < 1e-6);
        assert!(bend_config_revision() > before);
        reset_bend_config();
        assert!((bend_config().strength - 0.74).abs() < 1e-6);
    }
}
