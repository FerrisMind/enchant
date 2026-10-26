//! Shared metal-frame registry — iOS `MetalFrameStore` / web `MetalFxInstance`.
//!
//! [`MetalFx`] publishes under [`MetalFxOptions::anchor_id`]; [`MetalReflection`]
//! samples the latest snapshot to paint a mirrored catch on a neighbour.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

use super::geometry::MetalFxVariant;
use super::preset::{MetalFxResolvedTheme, MetalPresetMode};

/// One published metal frame an instance can reflect from.
#[derive(Clone, Debug)]
pub struct MetalReflectionAnchor {
    /// Absolute layout origin (iced layout bounds).
    pub origin: [f32; 2],
    pub width: f32,
    pub height: f32,
    pub radius: f32,
    pub variant: MetalFxVariant,
    pub mode: MetalPresetMode,
    pub theme: MetalFxResolvedTheme,
    pub clock: f32,
    pub strength: f32,
    pub ring_px: f32,
    pub shader_scale: f32,
    /// Fill / text / badge anchors mirror the whole sheet, not just the ring.
    pub is_sheet: bool,
}

impl MetalReflectionAnchor {
    /// Absolute AABB as `[x, y, w, h]`.
    #[must_use]
    pub fn rect(&self) -> [f32; 4] {
        [self.origin[0], self.origin[1], self.width, self.height]
    }
}

static STORE: LazyLock<Mutex<HashMap<String, MetalReflectionAnchor>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn lock() -> MutexGuard<'static, HashMap<String, MetalReflectionAnchor>> {
    STORE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Publishes (or replaces) an anchor snapshot.
pub fn register_reflection_anchor(id: impl Into<String>, anchor: MetalReflectionAnchor) {
    lock().insert(id.into(), anchor);
}

/// Drops an anchor if present.
pub fn unregister_reflection_anchor(id: &str) {
    lock().remove(id);
}

/// Latest snapshot for `id`, if any.
#[must_use]
pub fn reflection_anchor(id: &str) -> Option<MetalReflectionAnchor> {
    lock().get(id).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::metal_fx::preset::MetalFxPreset;

    #[test]
    fn round_trips() {
        let mode = MetalFxPreset::Silver.mode(MetalFxResolvedTheme::Dark);
        register_reflection_anchor(
            "t",
            MetalReflectionAnchor {
                origin: [10.0, 20.0],
                width: 140.0,
                height: 40.0,
                radius: 20.0,
                variant: MetalFxVariant::Button,
                mode,
                theme: MetalFxResolvedTheme::Dark,
                clock: 1.0,
                strength: 1.0,
                ring_px: 1.0,
                shader_scale: 1.6,
                is_sheet: false,
            },
        );
        assert!(reflection_anchor("t").is_some());
        unregister_reflection_anchor("t");
        assert!(reflection_anchor("t").is_none());
    }
}
