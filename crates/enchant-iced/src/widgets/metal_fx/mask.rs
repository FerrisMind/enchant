//! Custom alpha masks for sheet-fill metal (`mask` / `glowMode` on the web).
//!
//! When a mask is set the ring punch is skipped and the material is kept only
//! where the mask is opaque — metal-filled text and the badge's full pill.

use std::sync::Arc;

/// How the wandering glow is placed when a [`MetalFxMask`](MetalFxMask) is set.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::MetalGlowMode;
///
/// assert_eq!(MetalGlowMode::default(), MetalGlowMode::Mask);
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MetalGlowMode {
    /// Sample and clip the halo inside the mask (metal text).
    #[default]
    Mask,
    /// Keep the rounded-rect perimeter halo (full-fill masks like a badge).
    Ring,
}

/// An R8 alpha bitmap in device px, origin at the box's top-left.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::AlphaMask;
///
/// let mask = AlphaMask::solid(8, 4);
/// assert_eq!(mask.sample(0.5, 0.5), 1.0);
/// assert_eq!(mask.sample(100.0, 0.0), 0.0);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct AlphaMask {
    width: u32,
    height: u32,
    /// Device px per CSS px the mask was rasterised at.
    density: f32,
    alpha: Arc<[u8]>,
    id: u64,
}

impl AlphaMask {
    /// Fully opaque `width × height` mask at density 1.
    #[must_use]
    pub fn solid(width: u32, height: u32) -> Self {
        Self::from_alpha(width, height, 1.0, vec![255; (width * height) as usize])
    }

    /// Builds a mask from row-major alpha bytes.
    ///
    /// # Panics
    ///
    /// Panics if `alpha.len() != width * height` or either side is zero.
    #[must_use]
    pub fn from_alpha(width: u32, height: u32, density: f32, alpha: Vec<u8>) -> Self {
        assert!(width > 0 && height > 0, "mask size must be non-zero");
        assert_eq!(
            alpha.len(),
            (width * height) as usize,
            "mask alpha length must equal width*height"
        );
        let density = if density.is_finite() && density > 0.0 {
            density
        } else {
            1.0
        };
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::{Hash, Hasher};
        width.hash(&mut hasher);
        height.hash(&mut hasher);
        density.to_bits().hash(&mut hasher);
        alpha.hash(&mut hasher);
        Self {
            width,
            height,
            density,
            alpha: Arc::from(alpha),
            id: hasher.finish(),
        }
    }

    /// Stable GPU-cache identity.
    #[must_use]
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Device-px size.
    #[must_use]
    pub fn pixel_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Device px per CSS px.
    #[must_use]
    pub fn density(&self) -> f32 {
        self.density
    }

    /// Row-major alpha bytes.
    #[must_use]
    pub fn alpha(&self) -> &[u8] {
        &self.alpha
    }

    /// Bilinear alpha (`0..=1`) at a CSS-px point; `0` outside.
    #[must_use]
    pub fn sample(&self, x: f32, y: f32) -> f32 {
        let fx = x * self.density - 0.5;
        let fy = y * self.density - 0.5;
        if !fx.is_finite() || !fy.is_finite() {
            return 0.0;
        }
        let x0 = fx.floor() as i32;
        let y0 = fy.floor() as i32;
        let tx = fx - x0 as f32;
        let ty = fy - y0 as f32;
        let a00 = self.texel(x0, y0);
        let a10 = self.texel(x0 + 1, y0);
        let a01 = self.texel(x0, y0 + 1);
        let a11 = self.texel(x0 + 1, y0 + 1);
        let a0 = a00 + (a10 - a00) * tx;
        let a1 = a01 + (a11 - a01) * tx;
        a0 + (a1 - a0) * ty
    }

    fn texel(&self, x: i32, y: i32) -> f32 {
        if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
            return 0.0;
        }
        f32::from(self.alpha[(y as u32 * self.width + x as u32) as usize]) / 255.0
    }

    /// Points inside the mask on a CSS-px grid of `step` (for glow hunting).
    #[must_use]
    pub fn interior_points(&self, step: f32) -> Vec<[f32; 2]> {
        let step = step.max(1.0);
        let css_w = self.width as f32 / self.density;
        let css_h = self.height as f32 / self.density;
        let mut pts = Vec::new();
        let mut y = step * 0.5;
        while y < css_h {
            let mut x = step * 0.5;
            while x < css_w {
                if self.sample(x, y) > 0.5 {
                    pts.push([x, y]);
                }
                x += step;
            }
            y += step;
        }
        pts
    }
}

/// What the metal layer fills: the default ring band, or a custom mask.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::MetalFxMask;
///
/// assert_eq!(MetalFxMask::default(), MetalFxMask::Ring);
/// ```
#[non_exhaustive]
#[derive(Clone, Debug, Default, PartialEq)]
pub enum MetalFxMask {
    /// Rounded-rect ring with the inner hole punched (default).
    #[default]
    Ring,
    /// Full rounded-rect fill (badge-style). Pair with [`MetalGlowMode::Ring`].
    Fill,
    /// Arbitrary alpha mask (text glyphs). Pair with [`MetalGlowMode::Mask`].
    Custom(AlphaMask),
}

impl MetalFxMask {
    /// Whether the metal covers the full shape instead of a ring band.
    #[must_use]
    pub fn is_fill(&self) -> bool {
        !matches!(self, Self::Ring)
    }

    /// Optional custom alpha mask.
    #[must_use]
    pub fn custom(&self) -> Option<&AlphaMask> {
        match self {
            Self::Custom(m) => Some(m),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solid_is_opaque_inside() {
        let m = AlphaMask::solid(4, 4);
        assert!((m.sample(1.0, 1.0) - 1.0).abs() < 1e-5);
        assert_eq!(m.sample(-1.0, 0.0), 0.0);
    }

    #[test]
    fn interior_points_skip_empty_mask() {
        let m = AlphaMask::from_alpha(4, 4, 1.0, vec![0; 16]);
        assert!(m.interior_points(1.0).is_empty());
    }
}
