//! Border beam, ported from Libraries.dev `border-beam` v1.4.1.
//!
//! One widget, five size presets and eight palettes:
//!
//! | [`BorderBeamSize`] | Reference | What it draws |
//! | --- | --- | --- |
//! | [`Sm`](BorderBeamSize::Sm) | `sm` | Rotating conic border + inner glow on a 32px radius |
//! | [`Md`](BorderBeamSize::Md) | default | Rotating conic border + inner glow + bloom ring |
//! | [`Line`](BorderBeamSize::Line) | `line` | A bright segment travelling along the bottom edge with a wide bloom |
//! | [`PulseInner`](BorderBeamSize::PulseInner) | `pulse-inner` | Nine desynced oscillators driving an inner glow ring + bloom |
//! | [`PulseOutside`](BorderBeamSize::PulseOutside) | `pulse-outside` | The pulse outside the host, with a 1px ring and a big halo |
//!
//! Every non-DOM reference prop is available: [`static_colors`],
//! [`duration`](BorderBeamOptions::duration), [`active`](BorderBeamOptions::active)
//! with the 0.6s/0.5s fade lifecycle and `on_activate`/`on_deactivate`, the
//! radius (per-size default, overridable), [`border_width`](BorderBeamOptions::border_width),
//! [`brightness`](BorderBeamOptions::brightness), [`saturation`](BorderBeamOptions::saturation),
//! [`hue_range`](BorderBeamOptions::hue_range) (capped at 13° for `line`),
//! [`glow_size`](BorderBeamOptions::glow_size), [`strength`](BorderBeamOptions::strength)
//! and reduced motion. Light and dark presets follow [`BorderBeamTheme`];
//! [`BorderBeamTheme::Auto`] resolves against the iced theme.
//!
//! [`static_colors`]: BorderBeamOptions::static_colors
//!
//! # Rendering model
//!
//! The reference is a stack of CSS layers: a conic stroke ring (`::after`
//! with a mask), an inner glow (`::before`), a bloom element behind or around
//! it, `hue-rotate`/`brightness`/`saturate` filters, gaussian blurs and CSS
//! opacity. iced offers no equivalent CSS compositor, so this port reproduces
//! the layer stack analytically:
//!
//! - [`BorderBeamFrame`] samples every time-dependent value (travel, breathing,
//!   spikes, the 17 oscillators, hue and angle) from the reference keyframes.
//! - [`BorderBeamRaster`] paints each layer into a premultiplied float plane,
//!   applies the same masks (analytic conic/ellipse/rounded-rect coverage,
//!   XOR ring, 28px edge bands), blurs with the browser's three-box gaussian
//!   approximation, applies the CSS colour matrices and emits straight RGBA8.
//! - The widget caches one frame per redraw as an [`iced::advanced::image::Handle`]
//!   and draws it above (and, for `pulse-outside`, below) the child.
//!
//! `BorderBeamOptions::render_at` exposes the same pipeline headlessly, which
//! is what the parity tests compare against captured browser frames.
//!
//! # Backends
//!
//! [`BorderBeam`] is the image-widget backend: it measures its child, owns the
//! animation clock, applies the fade lifecycle and draws the rasterized layers
//! above and below the child. [`BorderBeamCanvas`] is the canvas backend for
//! callers that already drive a clock: same [`BorderBeamOptions`], same
//! [`BorderBeamFrame`] sampling and the same [`BorderBeamRaster`], emitted as a
//! [`canvas::Geometry`](iced::widget::canvas::Geometry) with the
//! `pulse-outside` halo outside the layout bounds. Reach for the frame and
//! raster types directly only when integrating with a larger canvas scene.
//!
//! iced cannot read a child's corner radius the way the reference reads
//! `border-top-left-radius`, so both backends take an explicit radius (or use
//! the size preset's default); nothing else from the reference needs DOM
//! access. Reduced motion freezes the effect at a representative phase and
//! snaps the fade in/out instead of animating it.
//!
//! # Sizes and defaults
//!
//! Durations and radii mirror the reference: 1.96s and 16px for `md`/`sm`
//! (`sm` uses 32px), 3.1s for `line`, 2.3s for both pulse sizes. Fade-in is
//! 600ms and fade-out 500ms, both with CSS `ease`. `pulse-outside` forces a
//! 1px ring, as upstream does.
//!
//! # Deliberate deviation
//!
//! Libraries.dev v1.4.1 ships malformed `rgba(r, g, b,, a)` declarations for
//! `forest`, `candy`, `ice` and `gold` (112 occurrences in `styles.ts`, e.g.
//! the spike, small-inner and line-bloom tables). Browsers drop those
//! declarations, so upstream loses most of the glow for those palettes. This
//! port repairs the extra comma and keeps the authored colours; the
//! difference is intentional and documented here.
//!
//! # Examples
//!
//! ```rust
//! use enchant_iced::{
//!     BeamHost, BorderBeamOptions, BorderBeamPalette, BorderBeamSize,
//!     BeamRenderOptions, ResolvedBeamTheme,
//! };
//! use std::time::Duration;
//!
//! let layers = BorderBeamOptions::new()
//!     .size(BorderBeamSize::Line)
//!     .palette(BorderBeamPalette::Colorful)
//!     .render_at(
//!         Duration::from_secs_f32(1.55),
//!         BeamHost::new(348.0, 122.0),
//!         ResolvedBeamTheme::Dark,
//!         &BeamRenderOptions::new().density(2.0),
//!     );
//! assert!(layers.above().is_some());
//! ```

mod blur;
mod canvas;
mod data;
mod frame;
mod options;
mod paint;
mod palette;
mod raster;
mod widget;

pub use canvas::{BorderBeamCanvas, BorderBeamCanvasState};
pub use frame::BorderBeamFrame;
pub use options::{BeamRenderOptions, BorderBeamOptions, BorderBeamSpec};
pub use palette::{
    BeamHost, BorderBeamPalette, BorderBeamSize, BorderBeamTheme, ResolvedBeamTheme,
};
pub use raster::{BeamBitmap, BeamLayers, BorderBeamRaster};
pub use widget::BorderBeam;
