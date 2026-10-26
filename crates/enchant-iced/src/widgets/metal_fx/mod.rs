//! Metal FX, ported from Libraries.dev `metal-fx` v2.
//!
//! A liquid-metal ring around any element: Paper Shaders' `liquidMetal`
//! material carved into a band along the element's edge, a wandering halo that
//! hunts the brightest point of the ring, an optional inner-shadow rim, the
//! wrapper surface and the web's hairlines. [`MetalFx`] wraps a child and draws
//! the whole stack with the web's layering.
//!
//! | here | upstream |
//! |------|----------|
//! | [`MetalFx`], [`MetalFxOptions`] | `MetalFx.tsx`, `MetalFxProps` (`types.ts`) |
//! | [`MetalFxVariant`], [`MetalFxPreset`], [`MetalFxTheme`] | `MetalFxVariant`, `MetalFxPreset`, `MetalFxTheme` |
//! | [`MetalPresetMode`], [`MetalFxPreset::mode`] | `PRESETS`, `PresetMode` (`presets.ts`) |
//! | [`MetalMaterial`], [`sheet_uvs`], [`SheetMapping`] | Paper `liquidMetal` fragment + sizing vertex stage, `copyShaderToInstance` |
//! | [`round_rect_outline`], [`sample_at_arc`], [`arc_at_point`] | `renderer/outline.ts`, `glow/geometry.ts` |
//! | [`GlowState`], [`MetalGlowConfig`] (+ [`glow_config`]) | `glow/glow.ts`, `GLOW` (`glow/config.ts`) |
//! | [`bake_halo`], [`bake_extra`] | `glow/bake.ts` |
//! | [`MetalRimOptions`] | `rim.ts` (`RimOptions`) |
//! | [`Tween`], [`Ease`] | `tween.ts` |
//! | [`MetalFxPipeline`] | the shared GL renderer (`renderer/core.ts`, `loop.ts`) |
//!
//! # Rendering model
//!
//! The web draws the material once onto a shared 192 px GL sheet and crops a
//! window onto every instance's 2D canvas. Here the shader evaluates the same
//! material per fragment from the same sheet coordinate (`SheetMapping`), so
//! there is no readback and no per-instance canvas: [`MetalFxPipeline`] is one
//! shared wgpu pipeline (a normal and a `multiply` render pipeline over one
//! shader module) and each layer of each instance is a primitive drawn from a
//! recycled slot. The glow is the reference's baked-sprite approach: white
//! alpha sprites ([`bake_halo`], [`bake_extra`]) are sampled at the hotspot by
//! the same shader, tinted, rotated to the outline's tangent and clipped to the
//! ring band (full strength) and the surround (half strength). The CPU keeps a
//! twin of the material to find the hotspot, which also powers headless
//! rendering ([`MetalFxOptions::render_at`]).
//!
//! # Differences from the web version
//!
//! - The shader clock is continuous (about 60 fps) instead of the web's
//!   15 fps throttle, and the material sheet is a fixed 192 px regardless of
//!   DPR (the web caps at DPR 2).
//! - The ring is fully rounded unless [`MetalFxOptions::border_radius`] is set
//!   (no computed CSS to read).
//! - The metal layers draw over the child by default
//!   ([`MetalFxLayer::OverChild`]); iced cannot normalise a child's own
//!   chrome like `normalizeHostStyles`.
//! - The rim is evaluated analytically per fragment instead of rasterised
//!   and box-blurred on a canvas.
//! - Stripe anti-aliasing uses finite differences over one sheet texel where
//!   the GLSL uses `fwidth`; the mask shape is fixed to Paper's `none`.
//! - Masked (text) glow, bend, cursor light and reflections are separate
//!   features and not part of this module yet.
//!
//! # Examples
//!
//! ```rust
//! use std::time::Duration;
//! use enchant_iced::widgets::metal_fx::{
//!     MetalFxHost, MetalFxOptions, MetalFxPreset, MetalFxResolvedTheme, MetalFxTheme,
//!     MetalRenderOptions,
//! };
//!
//! let options = MetalFxOptions::new()
//!     .preset(MetalFxPreset::Gold)
//!     .theme(MetalFxTheme::Dark)
//!     .inner_shadow(true);
//! let bitmap = options.render_at(
//!     Duration::from_millis(1200),
//!     MetalFxHost::new(134.0, 40.0),
//!     MetalFxResolvedTheme::Dark,
//!     &MetalRenderOptions::new().density(2.0),
//! );
//! assert!(bitmap.width() > 268);
//! ```

mod bake;
mod bend;
mod bend_config;
mod color;
mod geometry;
mod glow;
mod glow_config;
mod mask;
mod material;
mod metal_badge;
mod metal_text;
mod options;
mod pipeline;
mod preset;
mod primitive;
mod raster;
mod rim;
mod scene;
mod text_mask;
mod tween;
mod widget;

pub use bake::{GlowSprite, SPRITE_DENSITY, bake_extra, bake_halo};
pub use bend::{MetalBendField, MetalBendState, pointer_targets};
pub use bend_config::{
    MetalBendApplyTo, MetalBendConfig, bend_config, bend_config_revision, reset_bend_config,
    set_bend_config,
};
pub use color::{hex_to_rgb, hex_to_rgba};
pub use geometry::{
    ARC_SEGMENTS, Deform, EDGE_STEP, MetalFxVariant, arc_at_point, round_rect_outline,
    rr_perimeter, sample_at_arc, shape_perimeter, smoothstep, tangent_angle_at_arc,
};
pub use glow::{GlowFrame, GlowSampler, GlowState, GlowTick, MaterialSampler, PERIMETER_SAMPLES};
pub use glow_config::{
    MetalGlowConfig, glow_config, glow_config_revision, reset_glow_config, set_glow_config,
    update_glow_config,
};
pub use mask::{AlphaMask, MetalFxMask, MetalGlowMode};
pub use material::{
    CANONICAL_HEIGHT, CANONICAL_WIDTH, MetalMaterial, SHEET_PIXEL_RATIO, SHEET_RESOLUTION,
    SheetMapping, SheetUvs, sheet_uvs,
};
pub use metal_badge::{METAL_BADGE_DEFAULTS, MetalBadge, MetalBadgeCore, MetalBadgeDefaults};
pub use metal_text::MetalText;
pub use options::{MetalFxLayer, MetalFxOptions, MetalFxSurface};
pub use pipeline::{MetalFxPipeline, WGSL as METAL_FX_WGSL};
pub use preset::{MetalFxPreset, MetalFxResolvedTheme, MetalFxTheme, MetalPresetMode, ShaderFit};
pub use raster::{MetalFxBitmap, MetalFxHost, MetalFxLayerKind, MetalRenderOptions};
pub use rim::MetalRimOptions;
pub use text_mask::{
    METAL_TEXT_DEFAULTS, MetalTextDefaults, MetalTextFont, TextInnerShadow, rasterize_text_mask,
    text_inner_shadow_mask,
};
pub use tween::{Ease, Tween};
pub use widget::MetalFx;
