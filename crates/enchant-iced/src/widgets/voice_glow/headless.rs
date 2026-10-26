//! Headless rendering: a glow at a fixed time, level and host size straight
//! to RGBA, for goldens, previews and the screenshot comparison tool.

use std::time::Duration;

use super::frame::{HostSize, LayerOpacity};
use super::options::{ResolvedTheme, VoiceGlow};
use super::paint::sd_round_rect;
use super::raster::{GlowBitmap, VoiceGlowRaster};

/// How [`VoiceGlow::render_at`] paints.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::RenderOptions;
///
/// let options = RenderOptions::new().density(2.0).background([10, 10, 14, 255]);
/// assert_eq!(options.density, 2.0);
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderOptions {
    /// Device pixels per logical px (default 1).
    pub density: f32,
    /// A colour (straight RGBA) the host's rounded rectangle is filled with
    /// under the glow; `None` leaves it transparent.
    pub background: Option<[u8; 4]>,
    /// A colour for the pixels outside the rounded corners; `None` leaves them
    /// transparent.
    pub page: Option<[u8; 4]>,
    /// Paint only this layer (every other layer's opacity is zeroed), for
    /// comparing layers one by one against the browser. `None` paints all.
    pub solo: Option<GlowLayer>,
}

/// One compositing layer of the glow, see [`RenderOptions::solo`].
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::{GlowLayer, RenderOptions};
///
/// let options = RenderOptions::new().solo(GlowLayer::Bloom);
/// assert_eq!(options.solo, Some(GlowLayer::Bloom));
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GlowLayer {
    /// The crisp edge stroke (`::after`).
    Stroke,
    /// The soft inner light (`::before`).
    Inner,
    /// The blurred bloom.
    Bloom,
    /// The band canvases.
    Band,
    /// The epicentre wash.
    CoreLight,
}

impl std::str::FromStr for GlowLayer {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "stroke" => Ok(Self::Stroke),
            "inner" => Ok(Self::Inner),
            "bloom" => Ok(Self::Bloom),
            "band" => Ok(Self::Band),
            "core-light" | "core" => Ok(Self::CoreLight),
            other => Err(format!("unknown layer {other:?}")),
        }
    }
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            density: 1.0,
            background: None,
            page: None,
            solo: None,
        }
    }
}

impl RenderOptions {
    /// The defaults: density 1, no background.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Device pixels per logical px.
    #[must_use]
    pub fn density(mut self, density: f32) -> Self {
        self.density = density;
        self
    }

    /// The host's fill colour under the glow.
    #[must_use]
    pub fn background(mut self, rgba: [u8; 4]) -> Self {
        self.background = Some(rgba);
        self
    }

    /// The colour outside the host's rounded corners.
    #[must_use]
    pub fn page(mut self, rgba: [u8; 4]) -> Self {
        self.page = Some(rgba);
        self
    }

    /// Paints only `layer`.
    #[must_use]
    pub fn solo(mut self, layer: GlowLayer) -> Self {
        self.solo = Some(layer);
        self
    }
}

/// Straight-alpha source-over of `src` onto `dst`, per channel in `0..=1`.
fn over_straight(dst: [f32; 4], src: [f32; 4]) -> [f32; 4] {
    let a = src[3] + dst[3] * (1.0 - src[3]);
    if a <= 0.0 {
        return [0.0; 4];
    }
    let mut out = [0.0; 4];
    for c in 0..3 {
        out[c] = (src[c] * src[3] + dst[c] * dst[3] * (1.0 - src[3])) / a;
    }
    out[3] = a;
    out
}

impl VoiceGlow {
    /// Renders the glow `elapsed` after it started (replayed at 60 Hz, see
    /// [`VoiceGlow::frame_at`]) for a host of `host` logical px.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use enchant_iced::widgets::voice_glow::{
    ///     HostSize, RenderOptions, ResolvedTheme, VoiceGlow,
    /// };
    ///
    /// let bitmap = VoiceGlow::new().level(0.8).render_at(
    ///     Duration::from_secs(2),
    ///     HostSize::new(200.0, 60.0),
    ///     ResolvedTheme::Dark,
    ///     &RenderOptions::new(),
    /// );
    /// assert_eq!((bitmap.width, bitmap.height), (200, 60));
    /// assert!(bitmap.rgba.chunks(4).any(|p| p[3] > 0));
    /// ```
    #[must_use]
    pub fn render_at(
        &self,
        elapsed: Duration,
        host: HostSize,
        system: ResolvedTheme,
        options: &RenderOptions,
    ) -> GlowBitmap {
        let config = self.config(system);
        let mut frame = self.frame_at(elapsed, host, system);
        if let Some(only) = options.solo {
            let keep = |layer: GlowLayer, v: f64| if only == layer { v } else { 0.0 };
            let o = frame.layers;
            frame.layers = LayerOpacity {
                stroke: keep(GlowLayer::Stroke, o.stroke),
                inner: keep(GlowLayer::Inner, o.inner),
                bloom: keep(GlowLayer::Bloom, o.bloom),
                band: keep(GlowLayer::Band, o.band),
                core_light: keep(GlowLayer::CoreLight, o.core_light),
            };
        }
        let mut raster = VoiceGlowRaster::new();
        let mut bitmap = raster.render(&frame, &config, options.density).clone();
        if bitmap.width == 0 || options.background.is_none() && options.page.is_none() {
            return bitmap;
        }
        let (w, h) = (bitmap.width as usize, bitmap.height as usize);
        let k = (w as f32 / host.width as f32).min(h as f32 / host.height as f32);
        let radius = config.style.border_radius as f32;
        let unit = |c: [u8; 4]| c.map(|v| f32::from(v) / 255.0);
        for j in 0..h {
            let y = (j as f32 + 0.5) / h as f32 * host.height as f32;
            for i in 0..w {
                let x = (i as f32 + 0.5) / w as f32 * host.width as f32;
                let inside = (0.5
                    - sd_round_rect(
                        x,
                        y,
                        0.0,
                        0.0,
                        host.width as f32,
                        host.height as f32,
                        radius,
                    ) * k)
                    .clamp(0.0, 1.0);
                let idx = (j * w + i) * 4;
                let glow = unit([
                    bitmap.rgba[idx],
                    bitmap.rgba[idx + 1],
                    bitmap.rgba[idx + 2],
                    bitmap.rgba[idx + 3],
                ]);
                let mut px = options.page.map_or([0.0; 4], unit);
                if let Some(bg) = options.background {
                    let mut bg = unit(bg);
                    bg[3] *= inside;
                    px = over_straight(px, bg);
                }
                // The glow is already clipped to the host.
                px = over_straight(px, glow);
                for (dst, v) in bitmap.rgba[idx..idx + 4].iter_mut().zip(px) {
                    *dst = (v * 255.0).round().clamp(0.0, 255.0) as u8;
                }
            }
        }
        bitmap
    }
}
