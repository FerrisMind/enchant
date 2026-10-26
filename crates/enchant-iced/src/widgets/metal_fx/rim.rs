//! The inner-shadow rim on the metal ring, from `engine/rim.ts`.
//!
//! A hairline of light along the top inside edge of the band, the same
//! treatment the "Pro" metal text carries (Figma: white 90 %, offset 0/1, blur
//! 0.5): band coverage minus the same coverage shifted down by the offset
//! leaves exactly the top rim (outer edge on the ring's upper half, inner edge
//! on its lower half, what a light from above does to a torus). It is blurred,
//! tinted and clipped back to the band.
//!
//! The reference computes this on a canvas and box-blurs it; here the shader
//! evaluates the same difference analytically per fragment and blurs it with
//! gaussian taps, so it needs no bitmap and follows every resize for free.

/// Inner-shadow tuning (`RimOptions`).
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::MetalRimOptions;
///
/// let rim = MetalRimOptions::new().offset_y(2.0).alpha(0.5);
/// assert_eq!(rim.offset_y_px(), 2.0);
/// assert_eq!(rim.blur_px(), 0.5);
/// assert_eq!(MetalRimOptions::default(), MetalRimOptions::DEFAULTS);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
#[must_use = "rim options do nothing unless passed to MetalFxOptions::inner_shadow_with"]
pub struct MetalRimOptions {
    offset_y: f32,
    blur: f32,
    alpha: f32,
    color: [f32; 3],
}

impl MetalRimOptions {
    /// The design defaults (`RIM_DEFAULTS`): white 90 %, offset 1 px, blur
    /// 0.5 px.
    pub const DEFAULTS: Self = Self {
        offset_y: 1.0,
        blur: 0.5,
        alpha: 0.9,
        color: [1.0, 1.0, 1.0],
    };

    /// The design defaults.
    pub fn new() -> Self {
        Self::DEFAULTS
    }

    /// Shadow offset, CSS px (positive = light from above). Non-finite
    /// values are ignored.
    pub fn offset_y(mut self, offset: f32) -> Self {
        if offset.is_finite() {
            self.offset_y = offset;
        }
        self
    }

    /// Blur, CSS px (clamped to `>= 0`; non-finite values are ignored).
    pub fn blur(mut self, blur: f32) -> Self {
        if blur.is_finite() {
            self.blur = blur.max(0.0);
        }
        self
    }

    /// Peak opacity of the rim (clamped to `0..=1`; non-finite values are
    /// ignored).
    pub fn alpha(mut self, alpha: f32) -> Self {
        if alpha.is_finite() {
            self.alpha = alpha.clamp(0.0, 1.0);
        }
        self
    }

    /// Rim colour.
    pub fn color(mut self, color: iced::Color) -> Self {
        self.color = [color.r, color.g, color.b];
        self
    }

    /// Shadow offset, CSS px.
    #[must_use]
    pub fn offset_y_px(&self) -> f32 {
        self.offset_y
    }

    /// Blur, CSS px.
    #[must_use]
    pub fn blur_px(&self) -> f32 {
        self.blur
    }

    /// Peak opacity.
    #[must_use]
    pub fn alpha_value(&self) -> f32 {
        self.alpha
    }

    /// Rim colour, `0..=1` per channel.
    #[must_use]
    pub fn color_rgb(&self) -> [f32; 3] {
        self.color
    }
}

impl Default for MetalRimOptions {
    fn default() -> Self {
        Self::DEFAULTS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_rim_ts() {
        let d = MetalRimOptions::DEFAULTS;
        assert_eq!(
            (d.offset_y_px(), d.blur_px(), d.alpha_value()),
            (1.0, 0.5, 0.9)
        );
        assert_eq!(d.color_rgb(), [1.0, 1.0, 1.0]);
    }

    #[test]
    fn setters_validate() {
        let r = MetalRimOptions::new()
            .alpha(7.0)
            .blur(-3.0)
            .offset_y(f32::NAN);
        assert_eq!(r.alpha_value(), 1.0);
        assert_eq!(r.blur_px(), 0.0);
        assert_eq!(r.offset_y_px(), 1.0);
    }
}
