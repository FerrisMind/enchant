//! The pointer raster that gravity bends.

use std::error::Error;
use std::fmt;
use std::sync::Arc;

/// Why a [`CursorSprite`] could not be built.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorSpriteError {
    /// The raster has a zero side or the logical size is not positive/finite.
    EmptySize,
    /// `rgba` is not exactly `px_width * px_height * 4` bytes.
    LengthMismatch {
        /// Bytes required by the declared raster size.
        expected: usize,
        /// Bytes supplied.
        actual: usize,
    },
}

impl fmt::Display for CursorSpriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySize => f.write_str("cursor sprite size must be positive and finite"),
            Self::LengthMismatch { expected, actual } => write!(
                f,
                "cursor sprite needs {expected} RGBA bytes for its raster size, got {actual}"
            ),
        }
    }
}

impl Error for CursorSpriteError {}

/// A raster of the platform's real pointer.
///
/// Gravity hides the OS pointer and draws this in its place, warped toward the
/// orb, so it has to match the pointer pixel for pixel or the swap shows. The
/// raster may be denser than the logical size (a 2x or 3x sprite for a hi-dpi
/// screen); `width`/`height` and the hotspot are in logical pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct CursorSprite {
    rgba: Arc<[u8]>,
    px_width: u32,
    px_height: u32,
    width: f32,
    height: f32,
    hot_x: f32,
    hot_y: f32,
}

impl CursorSprite {
    /// Builds a sprite from straight-alpha RGBA8 pixels.
    ///
    /// `width`/`height` are the sprite's logical size and `(hot_x, hot_y)` the
    /// click point inside it, both in logical pixels.
    ///
    /// # Errors
    ///
    /// Returns [`CursorSpriteError::EmptySize`] for a zero raster side or a
    /// non-positive logical size, and [`CursorSpriteError::LengthMismatch`]
    /// when `rgba` does not hold `px_width * px_height * 4` bytes.
    pub fn new(
        rgba: impl Into<Arc<[u8]>>,
        px_width: u32,
        px_height: u32,
        width: f32,
        height: f32,
        hot_x: f32,
        hot_y: f32,
    ) -> Result<Self, CursorSpriteError> {
        let rgba = rgba.into();
        if px_width == 0
            || px_height == 0
            || !(width.is_finite() && width > 0.0)
            || !(height.is_finite() && height > 0.0)
        {
            return Err(CursorSpriteError::EmptySize);
        }
        let expected = px_width as usize * px_height as usize * 4;
        if rgba.len() != expected {
            return Err(CursorSpriteError::LengthMismatch {
                expected,
                actual: rgba.len(),
            });
        }
        Ok(Self {
            rgba,
            px_width,
            px_height,
            width,
            height,
            hot_x: if hot_x.is_finite() { hot_x } else { 0.0 },
            hot_y: if hot_y.is_finite() { hot_y } else { 0.0 },
        })
    }

    /// A procedurally drawn classic arrow (black fill, white outline) rastered
    /// at `scale` pixels per logical pixel, hotspot at the tip.
    ///
    /// It matches the stock arrow of most desktops closely but not exactly, so
    /// it is a convenient default rather than a substitute for the real pointer
    /// raster where the swap must be invisible.
    pub fn classic_arrow(scale: u32) -> Self {
        let scale = scale.clamp(1, 8);
        let px_w = ARROW_W * scale;
        let px_h = ARROW_H * scale;
        let mut rgba = vec![0u8; px_w as usize * px_h as usize * 4];
        const SUPER: u32 = 4;
        let samples = f64::from(SUPER * SUPER);
        for py in 0..px_h {
            for px in 0..px_w {
                let (mut black, mut white) = (0.0, 0.0);
                for sy in 0..SUPER {
                    for sx in 0..SUPER {
                        let x = (f64::from(px) + (f64::from(sx) + 0.5) / f64::from(SUPER))
                            / f64::from(scale);
                        let y = (f64::from(py) + (f64::from(sy) + 0.5) / f64::from(SUPER))
                            / f64::from(scale);
                        if inside_arrow(x, y) {
                            black += 1.0;
                        } else if distance_to_arrow(x, y) <= 1.0 {
                            white += 1.0;
                        }
                    }
                }
                let a = (black + white) / samples;
                let i = (py as usize * px_w as usize + px as usize) * 4;
                if a > 0.0 {
                    let g = (white / (black + white) * 255.0).round() as u8;
                    rgba[i] = g;
                    rgba[i + 1] = g;
                    rgba[i + 2] = g;
                    rgba[i + 3] = (a * 255.0).round() as u8;
                }
            }
        }
        Self {
            rgba: rgba.into(),
            px_width: px_w,
            px_height: px_h,
            width: ARROW_W as f32,
            height: ARROW_H as f32,
            hot_x: 0.0,
            hot_y: 0.0,
        }
    }

    /// Logical width.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Logical height.
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Hotspot (click point), logical pixels from the top-left.
    pub fn hotspot(&self) -> (f32, f32) {
        (self.hot_x, self.hot_y)
    }

    /// Raster size in pixels.
    pub fn raster_size(&self) -> (u32, u32) {
        (self.px_width, self.px_height)
    }

    /// Straight-alpha RGBA8 pixels, row-major.
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    /// Raster pixels per logical pixel.
    pub(crate) fn pixels_per_unit(&self) -> f64 {
        f64::from(self.px_width) / f64::from(self.width)
    }

    pub(crate) fn same_as(&self, other: &CursorSprite) -> bool {
        Arc::ptr_eq(&self.rgba, &other.rgba)
            && self.px_width == other.px_width
            && self.px_height == other.px_height
    }
}

const ARROW_W: u32 = 12;
const ARROW_H: u32 = 19;
const ARROW: [(f64, f64); 7] = [
    (0.0, 0.0),
    (0.0, 16.0),
    (3.6, 12.6),
    (6.4, 18.6),
    (8.6, 17.6),
    (5.9, 11.8),
    (10.6, 11.8),
];

fn inside_arrow(x: f64, y: f64) -> bool {
    let mut inside = false;
    let mut j = ARROW.len() - 1;
    for (i, &(xi, yi)) in ARROW.iter().enumerate() {
        let (xj, yj) = ARROW[j];
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn distance_to_arrow(x: f64, y: f64) -> f64 {
    let mut best = f64::INFINITY;
    for (i, &(ax, ay)) in ARROW.iter().enumerate() {
        let (bx, by) = ARROW[(i + 1) % ARROW.len()];
        let (dx, dy) = (bx - ax, by - ay);
        let len2 = dx * dx + dy * dy;
        let t = (((x - ax) * dx + (y - ay) * dy) / len2).clamp(0.0, 1.0);
        best = best.min((x - (ax + t * dx)).hypot(y - (ay + t * dy)));
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_sizes_and_lengths() {
        assert_eq!(
            CursorSprite::new(vec![0u8; 4], 0, 1, 1.0, 1.0, 0.0, 0.0),
            Err(CursorSpriteError::EmptySize)
        );
        assert_eq!(
            CursorSprite::new(vec![0u8; 4], 1, 1, 0.0, 1.0, 0.0, 0.0),
            Err(CursorSpriteError::EmptySize)
        );
        assert_eq!(
            CursorSprite::new(vec![0u8; 5], 1, 1, 1.0, 1.0, 0.0, 0.0),
            Err(CursorSpriteError::LengthMismatch {
                expected: 4,
                actual: 5
            })
        );
        assert!(CursorSprite::new(vec![0u8; 16], 2, 2, 2.0, 2.0, 0.0, 0.0).is_ok());
    }

    #[test]
    fn classic_arrow_is_opaque_at_the_tip_and_clear_far_away() {
        let sprite = CursorSprite::classic_arrow(2);
        let (w, h) = sprite.raster_size();
        assert_eq!((w, h), (24, 38));
        // just inside the tip
        let i = (3 * w as usize + 2) * 4;
        assert!(sprite.rgba()[i + 3] > 200);
        // far bottom-right corner is transparent
        let corner = ((h as usize - 1) * w as usize + (w as usize - 1)) * 4;
        assert_eq!(sprite.rgba()[corner + 3], 0);
        assert!((sprite.pixels_per_unit() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn errors_display() {
        assert!(!CursorSpriteError::EmptySize.to_string().is_empty());
    }
}
