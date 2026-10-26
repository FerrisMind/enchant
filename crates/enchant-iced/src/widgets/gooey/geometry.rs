//! Rounded-rectangle geometry for the goo silhouette.
//!
//! The reference draws SVG `<rect rx>` blobs (plus a path for bent bodies)
//! and lets CSS resolve the corner radii. Here every shape is rasterized from
//! an analytic signed-distance field, with coverage antialiasing and CSS-style
//! radius clamping, so the same numbers produce the same silhouette.

use iced::{Point, Rectangle, Size};

/// Per-corner radii in logical pixels, clockwise from the top-left.
///
/// `border-radius: 999px` is represented by simply using a large radius: the
/// rasterizer clamps it to half the short side, exactly like CSS and the
/// reference's `pillRadius`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CornerRadii {
    /// Top-left radius.
    pub top_left: f32,
    /// Top-right radius.
    pub top_right: f32,
    /// Bottom-right radius.
    pub bottom_right: f32,
    /// Bottom-left radius.
    pub bottom_left: f32,
}

impl CornerRadii {
    /// Equal radii on all four corners.
    #[must_use]
    pub const fn uniform(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    /// Four explicit radii.
    #[must_use]
    pub const fn new(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self {
        Self {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }

    /// Clears negative (invalid) radii.
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self::new(
            non_negative(self.top_left),
            non_negative(self.top_right),
            non_negative(self.bottom_right),
            non_negative(self.bottom_left),
        )
    }

    /// Scales every radius by `factor` (used to move from logical pixels to
    /// raster pixels).
    #[must_use]
    pub(crate) fn scaled(self, factor: f32) -> Self {
        Self::new(
            self.top_left * factor,
            self.top_right * factor,
            self.bottom_right * factor,
            self.bottom_left * factor,
        )
    }

    /// Equal radii.
    #[must_use]
    pub fn all(radius: f32) -> Self {
        Self::uniform(non_negative(radius))
    }

    /// CSS-style overlap clamping against a `width x height` box.
    #[must_use]
    pub fn clamped(self, width: f32, height: f32) -> Self {
        let [tl, tr, br, bl] = [
            non_negative(self.top_left),
            non_negative(self.top_right),
            non_negative(self.bottom_right),
            non_negative(self.bottom_left),
        ];
        let f = 1.0_f32
            .min(width / (tl + tr).max(1e-6))
            .min(width / (bl + br).max(1e-6))
            .min(height / (tl + bl).max(1e-6))
            .min(height / (tr + br).max(1e-6))
            .min(1.0);
        Self::new(tl * f, tr * f, br * f, bl * f)
    }
}

impl From<f32> for CornerRadii {
    fn from(radius: f32) -> Self {
        Self::uniform(radius)
    }
}

impl From<u16> for CornerRadii {
    fn from(radius: u16) -> Self {
        Self::uniform(f32::from(radius))
    }
}

impl From<i32> for CornerRadii {
    fn from(radius: i32) -> Self {
        Self::uniform(radius.max(0) as f32)
    }
}

impl From<[f32; 4]> for CornerRadii {
    fn from([top_left, top_right, bottom_right, bottom_left]: [f32; 4]) -> Self {
        Self::new(top_left, top_right, bottom_right, bottom_left)
    }
}

impl From<(f32, f32, f32, f32)> for CornerRadii {
    fn from((top_left, top_right, bottom_right, bottom_left): (f32, f32, f32, f32)) -> Self {
        Self::new(top_left, top_right, bottom_right, bottom_left)
    }
}

impl From<iced::border::Radius> for CornerRadii {
    fn from(radius: iced::border::Radius) -> Self {
        Self::new(
            radius.top_left,
            radius.top_right,
            radius.bottom_right,
            radius.bottom_left,
        )
    }
}

impl From<CornerRadii> for iced::border::Radius {
    fn from(radius: CornerRadii) -> Self {
        Self {
            top_left: radius.top_left,
            top_right: radius.top_right,
            bottom_right: radius.bottom_right,
            bottom_left: radius.bottom_left,
        }
    }
}

/// A rectangle in raster pixel space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RectPx {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl RectPx {
    #[cfg(test)]
    pub(crate) fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub(crate) fn from_rect(rect: Rectangle, scale: f32) -> Self {
        Self {
            x: rect.x * scale,
            y: rect.y * scale,
            w: rect.width * scale,
            h: rect.height * scale,
        }
    }

    pub(crate) fn right(self) -> f32 {
        self.x + self.w
    }

    pub(crate) fn bottom(self) -> f32 {
        self.y + self.h
    }

    pub(crate) fn is_empty(self) -> bool {
        self.w <= 0.0 || self.h <= 0.0
    }
}

/// Coverage of a rounded rectangle at one sample point, `0..=1`.
pub(crate) fn coverage(px: f32, py: f32, rect: RectPx, radii: CornerRadii) -> f32 {
    sdf(px, py, rect, radii)
        .map(|d| (0.5 - d).clamp(0.0, 1.0))
        .unwrap_or(0.0)
}

/// Signed distance to a rounded rectangle (negative inside), after the CSS
/// corner-overlap clamp. Returns `None` for a degenerate box.
pub(crate) fn sdf(px: f32, py: f32, rect: RectPx, radii: CornerRadii) -> Option<f32> {
    if rect.is_empty() {
        return None;
    }
    let radii = radii.clamped(rect.w, rect.h);
    let hx = rect.w / 2.0;
    let hy = rect.h / 2.0;
    let cx = rect.x + hx;
    let cy = rect.y + hy;
    let dx = px - cx;
    let dy = py - cy;
    let (rx, ry) = if dx < 0.0 {
        if dy < 0.0 {
            (radii.top_left, radii.top_left)
        } else {
            (radii.bottom_left, radii.bottom_left)
        }
    } else if dy < 0.0 {
        (radii.top_right, radii.top_right)
    } else {
        (radii.bottom_right, radii.bottom_right)
    };
    let qx = dx.abs() - (hx - rx);
    let qy = dy.abs() - (hy - ry);
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    Some(qx.max(qy).min(0.0) + outside - rx)
}

/// Union-fills a rounded rectangle into an alpha plane.
pub(crate) fn fill_rrect(
    plane: &mut [f32],
    width: usize,
    height: usize,
    rect: RectPx,
    radii: CornerRadii,
) {
    let x0 = rect.x.floor().max(0.0) as usize;
    let y0 = rect.y.floor().max(0.0) as usize;
    let x1 = (rect.right().ceil().max(0.0) as usize).min(width);
    let y1 = (rect.bottom().ceil().max(0.0) as usize).min(height);
    if x0 >= width || y0 >= height || x0 >= x1 || y0 >= y1 {
        return;
    }
    for y in y0..y1 {
        let py = y as f32 + 0.5;
        for x in x0..x1 {
            let px = x as f32 + 0.5;
            let c = coverage(px, py, rect, radii);
            if c > 0.0 {
                let slot = &mut plane[y * width + x];
                *slot = slot.max(c);
            }
        }
    }
}

/// Union-fills a circle into an alpha plane.
pub(crate) fn fill_circle(
    plane: &mut [f32],
    width: usize,
    height: usize,
    cx: f32,
    cy: f32,
    radius: f32,
) {
    if radius <= 0.0 {
        return;
    }
    let x0 = (cx - radius - 1.0).floor().max(0.0) as usize;
    let y0 = (cy - radius - 1.0).floor().max(0.0) as usize;
    let x1 = ((cx + radius + 1.0).ceil().max(0.0) as usize).min(width);
    let y1 = ((cy + radius + 1.0).ceil().max(0.0) as usize).min(height);
    for y in y0..y1 {
        let py = y as f32 + 0.5;
        for x in x0..x1 {
            let px = x as f32 + 0.5;
            let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt() - radius;
            let c = (0.5 - d).clamp(0.0, 1.0);
            if c > 0.0 {
                let slot = &mut plane[y * width + x];
                *slot = slot.max(c);
            }
        }
    }
}

/// Union-fills a closed polygon with 2×2 supersampling.
pub(crate) fn fill_polygon(plane: &mut [f32], width: usize, height: usize, points: &[Point]) {
    if points.len() < 3 {
        return;
    }
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for p in points {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }
    let x0 = min_x.floor().max(0.0) as usize;
    let y0 = min_y.floor().max(0.0) as usize;
    let x1 = (max_x.ceil().max(0.0) as usize).min(width);
    let y1 = (max_y.ceil().max(0.0) as usize).min(height);
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    let samples = [
        (0.25_f32, 0.25_f32),
        (0.75, 0.25),
        (0.25, 0.75),
        (0.75, 0.75),
    ];
    for y in y0..y1 {
        for x in x0..x1 {
            let mut hits = 0_u32;
            for (ox, oy) in samples {
                if point_in_polygon(x as f32 + ox, y as f32 + oy, points) {
                    hits += 1;
                }
            }
            if hits > 0 {
                let c = hits as f32 / 4.0;
                let slot = &mut plane[y * width + x];
                *slot = slot.max(c);
            }
        }
    }
}

fn point_in_polygon(x: f32, y: f32, points: &[Point]) -> bool {
    let mut inside = false;
    let mut j = points.len() - 1;
    for i in 0..points.len() {
        let pi = points[i];
        let pj = points[j];
        if ((pi.y > y) != (pj.y > y)) && (x < (pj.x - pi.x) * (y - pi.y) / (pj.y - pi.y) + pi.x) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// The upstream bend path as a sampled polygon.
///
/// `bend_y` is the smoothed vertical bow (px), `bend_x` the smoothed cap
/// deformation (px). The path itself is a direct transcription of the
/// reference's `bendD` construction: quadratic top/bottom edges and
/// asymmetric cap radii (`rxL`/`rxR`).
pub(crate) fn bend_polygon(
    rect: RectPx,
    radii: CornerRadii,
    bend_y: f32,
    bend_x: f32,
) -> Vec<Point> {
    let radii = radii.clamped(rect.w, rect.h);
    let r = radii.top_left.min(rect.w / 2.0).min(rect.h / 2.0);
    let rx_r = (r * 3.0).min((r - 0.8 * bend_x).max(r * 0.2));
    let rx_l = (r * 3.0).min((r + 1.6 * bend_x).max(r * 0.2));
    let rx_r = rx_r.min(rect.w / 2.0);
    let rx_l = rx_l.min(rect.w / 2.0);
    let k = 0.552_3;
    let w = rect.w;
    let h = rect.h;
    let mut points = Vec::with_capacity(64);
    let push_quad = |points: &mut Vec<Point>, a: Point, c: Point, b: Point, steps: usize| {
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let u = 1.0 - t;
            points.push(Point::new(
                u * u * a.x + 2.0 * u * t * c.x + t * t * b.x,
                u * u * a.y + 2.0 * u * t * c.y + t * t * b.y,
            ));
        }
    };
    let push_cubic = |points: &mut Vec<Point>,
                      a: Point,
                      c1: Point,
                      c2: Point,
                      b: Point,
                      steps: usize| {
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let u = 1.0 - t;
            points.push(Point::new(
                u * u * u * a.x + 3.0 * u * u * t * c1.x + 3.0 * u * t * t * c2.x + t * t * t * b.x,
                u * u * u * a.y + 3.0 * u * u * t * c1.y + 3.0 * u * t * t * c2.y + t * t * t * b.y,
            ));
        }
    };

    // Top edge, left -> right (middle leads into the motion).
    push_quad(
        &mut points,
        Point::new(rx_l, 0.0),
        Point::new(w / 2.0, bend_y * 2.0),
        Point::new(w - rx_r, 0.0),
        18,
    );
    push_cubic(
        &mut points,
        Point::new(w - rx_r, 0.0),
        Point::new(w - rx_r + k * rx_r, 0.0),
        Point::new(w, r - k * r),
        Point::new(w, r),
        6,
    );
    points.push(Point::new(w, h - r));
    push_cubic(
        &mut points,
        Point::new(w, h - r),
        Point::new(w, h - r + k * r),
        Point::new(w - rx_r + k * rx_r, h),
        Point::new(w - rx_r, h),
        6,
    );
    push_quad(
        &mut points,
        Point::new(w - rx_r, h),
        Point::new(w / 2.0, h + bend_y * 2.0),
        Point::new(rx_l, h),
        18,
    );
    push_cubic(
        &mut points,
        Point::new(rx_l, h),
        Point::new(rx_l - k * rx_l, h),
        Point::new(0.0, h - r + k * r),
        Point::new(0.0, h - r),
        6,
    );
    points.push(Point::new(0.0, r));
    push_cubic(
        &mut points,
        Point::new(0.0, r),
        Point::new(0.0, r - k * r),
        Point::new(rx_l - k * rx_l, 0.0),
        Point::new(rx_l, 0.0),
        6,
    );

    points
        .into_iter()
        .map(|p| Point::new(p.x + rect.x, p.y + rect.y))
        .collect()
}

/// Applies an anisotropic stretch along the motion axis, in place.
///
/// Equivalent to transforming every point by `R(-a) · S(along, across) · R(a)`
/// around `center`: the body grows where it leads and squeezes where it
/// trails, which is what upstream's wrapper transform does with CSS.
pub(crate) fn stretch_points(
    points: &mut [Point],
    center: Point,
    angle: f32,
    along: f32,
    across: f32,
) {
    let (sin, cos) = angle.sin_cos();
    for point in points {
        let dx = point.x - center.x;
        let dy = point.y - center.y;
        let axial = dx * cos + dy * sin;
        let lateral = -dx * sin + dy * cos;
        let axial = axial * along;
        let lateral = lateral * across;
        point.x = center.x + axial * cos - lateral * sin;
        point.y = center.y + axial * sin + lateral * cos;
    }
}

/// Scales a logical-pixel rectangle into a raster of `size` pixels.
pub(crate) fn resize(size: Size, density: f32) -> (usize, usize) {
    (
        (size.width * density).round().max(1.0) as usize,
        (size.height * density).round().max(1.0) as usize,
    )
}

fn non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radii_clamp_like_css() {
        let r = CornerRadii::uniform(999.0).clamped(40.0, 20.0);
        assert_eq!(r.top_left, 10.0);
        let r = CornerRadii::new(10.0, 10.0, 0.0, 0.0).clamped(10.0, 10.0);
        assert_eq!(r.top_left, 5.0);
    }

    #[test]
    fn coverage_is_one_inside_and_zero_outside() {
        let rect = RectPx::new(10.0, 10.0, 20.0, 20.0);
        let r = CornerRadii::uniform(4.0);
        assert_eq!(coverage(20.0, 20.0, rect, r), 1.0);
        assert_eq!(coverage(5.0, 5.0, rect, r), 0.0);
        assert!((coverage(10.0, 20.0, rect, r) - 0.5).abs() < 1e-3);
        assert_eq!(coverage(10.5, 10.5, rect, r), 0.0);
    }

    #[test]
    fn fill_rrect_marks_the_box() {
        let (w, h) = (32, 16);
        let mut plane = vec![0.0; w * h];
        fill_rrect(
            &mut plane,
            w,
            h,
            RectPx::new(2.0, 2.0, 12.0, 8.0),
            CornerRadii::uniform(2.0),
        );
        assert_eq!(plane[6 * w + 8], 1.0);
        assert_eq!(plane[0], 0.0);
    }

    #[test]
    fn bend_polygon_stays_within_growth_bounds() {
        let rect = RectPx::new(0.0, 0.0, 100.0, 40.0);
        let pts = bend_polygon(rect, CornerRadii::uniform(20.0), 6.0, 4.0);
        assert!(pts.iter().all(|p| p.y > -20.0 && p.y < 60.0));
        assert!(pts.iter().all(|p| p.x > -2.0 && p.x < 102.0));
    }

    #[test]
    fn stretch_scales_along_the_motion_axis_only() {
        let center = Point::new(50.0, 50.0);
        let mut points = vec![
            Point::new(60.0, 50.0), // 10 px ahead
            Point::new(50.0, 60.0), // 10 px to the side
            Point::new(40.0, 50.0), // 10 px behind
        ];
        stretch_points(&mut points, center, 0.0, 2.0, 0.5);
        assert!((points[0].x - 70.0).abs() < 1e-4);
        assert!((points[0].y - 50.0).abs() < 1e-4);
        assert!((points[1].x - 50.0).abs() < 1e-4);
        assert!((points[1].y - 55.0).abs() < 1e-4);
        assert!((points[2].x - 30.0).abs() < 1e-4);
    }

    #[test]
    fn stretch_rotates_with_the_motion_axis() {
        let center = Point::new(0.0, 0.0);
        let mut points = vec![Point::new(0.0, 10.0)];
        let half_pi = std::f32::consts::FRAC_PI_2;
        stretch_points(&mut points, center, half_pi, 2.0, 1.0);
        assert!(points[0].x.abs() < 1e-4);
        assert!((points[0].y - 20.0).abs() < 1e-4);
    }

    #[test]
    fn polygon_rasterizes_centrally() {
        let (w, h) = (20, 20);
        let mut plane = vec![0.0; w * h];
        let pts = vec![
            Point::new(2.0, 2.0),
            Point::new(18.0, 2.0),
            Point::new(18.0, 18.0),
            Point::new(2.0, 18.0),
        ];
        fill_polygon(&mut plane, w, h, &pts);
        assert_eq!(plane[10 * w + 10], 1.0);
        assert_eq!(plane[0], 0.0);
    }
}
