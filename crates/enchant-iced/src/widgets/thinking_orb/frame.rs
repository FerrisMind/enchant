//! The draw list of one rendered instant.
//!
//! A frame is *final*: culled, radius-clamped and z-sorted into draw order, so
//! a renderer only walks the list. That is what lets the golden-vector tests
//! compare numbers instead of pixels.

/// One dot of a frame, in orb-local pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dot {
    x: f64,
    y: f64,
    z: f64,
    r: f64,
    white: f64,
    alpha: f64,
}

impl Dot {
    /// A fully opaque dot. `white` is the ink value: `0` is the darkest ink on
    /// paper, mirrored on dark themes.
    pub fn new(x: f64, y: f64, z: f64, r: f64, white: f64) -> Self {
        Self {
            x,
            y,
            z,
            r,
            white,
            alpha: 1.0,
        }
    }

    /// Sets the dot's opacity.
    pub fn with_alpha(mut self, alpha: f64) -> Self {
        self.alpha = alpha;
        self
    }

    /// Horizontal centre.
    pub fn x(&self) -> f64 {
        self.x
    }

    /// Vertical centre.
    pub fn y(&self) -> f64 {
        self.y
    }

    /// Depth; frames are sorted far to near on this.
    pub fn z(&self) -> f64 {
        self.z
    }

    /// Radius.
    pub fn r(&self) -> f64 {
        self.r
    }

    /// Ink value.
    pub fn white(&self) -> f64 {
        self.white
    }

    /// Opacity.
    pub fn alpha(&self) -> f64 {
        self.alpha
    }
}

/// A stroked edge between two projected points (the `connecting` web).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    white: f64,
    alpha: f64,
    width: f64,
}

impl Line {
    /// A fully opaque edge.
    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64, white: f64, width: f64) -> Self {
        Self {
            x1,
            y1,
            x2,
            y2,
            white,
            alpha: 1.0,
            width,
        }
    }

    /// Sets the edge's opacity.
    pub fn with_alpha(mut self, alpha: f64) -> Self {
        self.alpha = alpha;
        self
    }

    /// Start point.
    pub fn start(&self) -> (f64, f64) {
        (self.x1, self.y1)
    }

    /// End point.
    pub fn end(&self) -> (f64, f64) {
        (self.x2, self.y2)
    }

    /// Ink value, same convention as [`Dot::white`].
    pub fn white(&self) -> f64 {
        self.white
    }

    /// Opacity.
    pub fn alpha(&self) -> f64 {
        self.alpha
    }

    /// Stroke width.
    pub fn width(&self) -> f64 {
        self.width
    }
}

/// A complete set of draw instructions for one instant.
///
/// `dots` are in draw order (far to near); `lines` are drawn first, so nodes
/// sit on top of their edges.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OrbFrame {
    dots: Vec<Dot>,
    lines: Vec<Line>,
}

impl OrbFrame {
    /// A frame from already-final lists. Use this from custom frame functions;
    /// the built-in modes go through the culling and sorting step instead.
    pub fn new(dots: Vec<Dot>, lines: Vec<Line>) -> Self {
        Self { dots, lines }
    }

    /// Turns raw mode output into a finished frame: drops invisible marks,
    /// clamps radii to `r_min` and z-sorts far to near into draw order.
    pub fn finalize(mut dots: Vec<Dot>, lines: Vec<Line>, r_min: f64) -> Self {
        dots.retain(|dot| dot.alpha >= 0.02);
        for dot in &mut dots {
            dot.r = dot.r.max(r_min);
        }
        // `sort_by` is stable, matching the reference's `Array.sort`.
        dots.sort_by(|a, b| a.z.total_cmp(&b.z));
        let lines = lines
            .into_iter()
            .filter(|line| line.alpha >= 0.02)
            .collect();
        Self { dots, lines }
    }

    /// The dots, in draw order.
    pub fn dots(&self) -> &[Dot] {
        &self.dots
    }

    /// The edges, drawn before the dots.
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finalize_culls_clamps_and_sorts() {
        let dots = vec![
            Dot::new(0.0, 0.0, 2.0, 0.1, 0.5),
            Dot::new(1.0, 1.0, -1.0, 2.0, 0.5),
            Dot::new(2.0, 2.0, 0.0, 1.0, 0.5).with_alpha(0.01),
        ];
        let lines = vec![
            Line::new(0.0, 0.0, 1.0, 1.0, 0.1, 1.0).with_alpha(0.5),
            Line::new(0.0, 0.0, 1.0, 1.0, 0.1, 1.0).with_alpha(0.0),
        ];
        let frame = OrbFrame::finalize(dots, lines, 0.3);
        assert_eq!(frame.dots().len(), 2);
        assert_eq!(frame.lines().len(), 1);
        assert_eq!(frame.dots()[0].z(), -1.0);
        assert_eq!(frame.dots()[1].r(), 0.3);
    }

    #[test]
    fn finalize_is_stable_for_equal_depth() {
        let dots = (0..10)
            .map(|i| Dot::new(f64::from(i), 0.0, 0.0, 1.0, 0.0))
            .collect();
        let frame = OrbFrame::finalize(dots, vec![], 0.3);
        let xs: Vec<f64> = frame.dots().iter().map(Dot::x).collect();
        assert_eq!(xs, (0..10).map(f64::from).collect::<Vec<_>>());
    }
}
