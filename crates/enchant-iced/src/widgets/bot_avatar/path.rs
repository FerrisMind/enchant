//! A small SVG path-data parser.
//!
//! The web package hands its outlines to the browser's `Path2D`, which speaks
//! SVG path data. The crate's renderer uses `tiny-skia`, whose `lyon` path
//! types do not include an SVG parser, so this module implements the subset
//! the package needs — and the whole of it, so a custom outline can be any
//! valid SVG path: `M`, `L`, `H`, `V`, `C`, `S`, `Q`, `T`, `A` and `Z`, in
//! absolute and relative form, with implicit repeated commands and the comma
//! or whitespace separators the grammar allows.
//!
//! Arcs are converted to cubic segments with the standard endpoint-to-centre
//! parameterisation; each segment spans at most a quarter turn, where the
//! cubic approximation is well below a device pixel at any avatar size.

use tiny_skia::{Path, PathBuilder};

/// Parse SVG path data into a `tiny-skia` path.
///
/// Returns [`None`] when the data is malformed or cannot form a path. An
/// empty string is a valid empty path.
#[must_use]
pub(crate) fn parse_svg_path(data: &str) -> Option<Path> {
    let mut parser = Parser::new(data);
    let mut builder = PathBuilder::new();
    parser.build(&mut builder)?;
    builder.finish()
}

/// Parse SVG path data into one path per subpath.
///
/// The package draws each thin part (an antenna, a rod, an ear) as its own
/// solid, so a list keeps the parts from being inflated toward one another.
#[must_use]
pub(crate) fn parse_svg_subpaths(data: &str) -> Vec<Path> {
    split_subpaths(data)
        .into_iter()
        .filter_map(parse_svg_path)
        .collect()
}

/// Split path data at every moveto, keeping the command with its subpath.
#[must_use]
pub(crate) fn split_subpaths(data: &str) -> Vec<&str> {
    let bytes = data.as_bytes();
    let mut starts = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'M' || bytes[i] == b'm' {
            starts.push(i);
        }
        i += 1;
    }
    let mut out = Vec::with_capacity(starts.len());
    for (n, &start) in starts.iter().enumerate() {
        let end = starts.get(n + 1).copied().unwrap_or(bytes.len());
        let piece = data[start..end].trim();
        if !piece.is_empty() {
            out.push(piece);
        }
    }
    out
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
    current: (f64, f64),
    start: (f64, f64),
    last_cubic: Option<(f64, f64)>,
    last_quad: Option<(f64, f64)>,
    previous: Option<u8>,
}

impl<'a> Parser<'a> {
    fn new(data: &'a str) -> Self {
        Self {
            bytes: data.as_bytes(),
            pos: 0,
            current: (0.0, 0.0),
            start: (0.0, 0.0),
            last_cubic: None,
            last_quad: None,
            previous: None,
        }
    }

    fn build(&mut self, builder: &mut PathBuilder) -> Option<()> {
        while let Some(command) = self.command()? {
            let relative = command.is_ascii_lowercase();
            let cmd = command.to_ascii_uppercase();
            let (mut cx, mut cy) = self.current;
            match cmd {
                b'M' => {
                    let mut first = true;
                    while let Some((x, y)) = self.pair()? {
                        let (x, y) = if relative { (cx + x, cy + y) } else { (x, y) };
                        if first {
                            builder.move_to(x as f32, y as f32);
                            self.start = (x, y);
                            first = false;
                        } else {
                            builder.line_to(x as f32, y as f32);
                        }
                        cx = x;
                        cy = y;
                    }
                }
                b'L' => {
                    while let Some((x, y)) = self.pair()? {
                        let (x, y) = if relative { (cx + x, cy + y) } else { (x, y) };
                        builder.line_to(x as f32, y as f32);
                        cx = x;
                        cy = y;
                    }
                }
                b'H' => {
                    while let Some(x) = self.number()? {
                        let x = if relative { cx + x } else { x };
                        builder.line_to(x as f32, cy as f32);
                        cx = x;
                    }
                }
                b'V' => {
                    while let Some(y) = self.number()? {
                        let y = if relative { cy + y } else { y };
                        builder.line_to(cx as f32, y as f32);
                        cy = y;
                    }
                }
                b'C' => {
                    while let Some((x1, y1, x2, y2, x, y)) = self.sextet()? {
                        let (x1, y1) = if relative { (cx + x1, cy + y1) } else { (x1, y1) };
                        let (x2, y2) = if relative { (cx + x2, cy + y2) } else { (x2, y2) };
                        let (x, y) = if relative { (cx + x, cy + y) } else { (x, y) };
                        builder.cubic_to(
                            x1 as f32, y1 as f32, x2 as f32, y2 as f32, x as f32, y as f32,
                        );
                        self.last_cubic = Some((x2, y2));
                        cx = x;
                        cy = y;
                    }
                }
                b'S' => {
                    let mut reflect = matches!(self.previous, Some(b'C' | b'S'));
                    while let Some((x2, y2, x, y)) = self.quad_pair()? {
                        let (x2, y2) = if relative { (cx + x2, cy + y2) } else { (x2, y2) };
                        let (x, y) = if relative { (cx + x, cy + y) } else { (x, y) };
                        let (x1, y1) = if reflect {
                            self.last_cubic
                                .map_or((cx, cy), |(px, py)| (2.0 * cx - px, 2.0 * cy - py))
                        } else {
                            (cx, cy)
                        };
                        reflect = true;
                        builder.cubic_to(
                            x1 as f32, y1 as f32, x2 as f32, y2 as f32, x as f32, y as f32,
                        );
                        self.last_cubic = Some((x2, y2));
                        cx = x;
                        cy = y;
                    }
                }
                b'Q' => {
                    while let Some((x1, y1, x, y)) = self.quad_pair()? {
                        let (x1, y1) = if relative { (cx + x1, cy + y1) } else { (x1, y1) };
                        let (x, y) = if relative { (cx + x, cy + y) } else { (x, y) };
                        builder.quad_to(x1 as f32, y1 as f32, x as f32, y as f32);
                        self.last_quad = Some((x1, y1));
                        cx = x;
                        cy = y;
                    }
                }
                b'T' => {
                    let mut reflect = matches!(self.previous, Some(b'Q' | b'T'));
                    while let Some((x, y)) = self.pair()? {
                        let (x, y) = if relative { (cx + x, cy + y) } else { (x, y) };
                        let (x1, y1) = if reflect {
                            self.last_quad
                                .map_or((cx, cy), |(px, py)| (2.0 * cx - px, 2.0 * cy - py))
                        } else {
                            (cx, cy)
                        };
                        reflect = true;
                        builder.quad_to(x1 as f32, y1 as f32, x as f32, y as f32);
                        self.last_quad = Some((x1, y1));
                        cx = x;
                        cy = y;
                    }
                }
                b'A' => {
                    while let Some((rx, ry, rotation, large, sweep, x, y)) = self.arc_args()? {
                        let (x, y) = if relative { (cx + x, cy + y) } else { (x, y) };
                        arc_to(builder, cx, cy, rx, ry, rotation, large, sweep, x, y);
                        cx = x;
                        cy = y;
                    }
                }
                b'Z' => {
                    builder.close();
                    cx = self.start.0;
                    cy = self.start.1;
                }
                _ => return None,
            }
            self.current = (cx, cy);
            if !matches!(cmd, b'C' | b'S') {
                self.last_cubic = None;
            }
            if !matches!(cmd, b'Q' | b'T') {
                self.last_quad = None;
            }
            self.previous = Some(cmd);
        }
        Some(())
    }

    /// The next command letter, or `None` at the end.
    ///
    /// Repeated coordinates after a command are folded into that command: the
    /// caller's loops read them until the next letter. A bare number where a
    /// command should be is therefore an error, which SVG treats the same way.
    fn command(&mut self) -> Option<Option<u8>> {
        self.skip_separators();
        if self.pos >= self.bytes.len() {
            return Some(None);
        }
        let b = self.bytes[self.pos];
        if b.is_ascii_alphabetic() {
            self.pos += 1;
            Some(Some(b))
        } else {
            None
        }
    }

    fn skip_separators(&mut self) {
        while self.pos < self.bytes.len()
            && matches!(self.bytes[self.pos], b' ' | b'\t' | b'\n' | b'\r' | b',')
        {
            self.pos += 1;
        }
    }

    fn number(&mut self) -> Option<Option<f64>> {
        self.skip_separators();
        if self.pos >= self.bytes.len() || self.bytes[self.pos].is_ascii_alphabetic() {
            return Some(None);
        }
        let start = self.pos;
        if self.bytes[self.pos] == b'+' || self.bytes[self.pos] == b'-' {
            self.pos += 1;
        }
        let mut digits = 0;
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
            self.pos += 1;
            digits += 1;
        }
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b'.' {
            self.pos += 1;
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
                self.pos += 1;
                digits += 1;
            }
        }
        if digits == 0 {
            return None;
        }
        if self.pos < self.bytes.len() && matches!(self.bytes[self.pos], b'e' | b'E') {
            let mark = self.pos;
            self.pos += 1;
            if self.pos < self.bytes.len() && matches!(self.bytes[self.pos], b'+' | b'-') {
                self.pos += 1;
            }
            let mut exp_digits = 0;
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
                self.pos += 1;
                exp_digits += 1;
            }
            if exp_digits == 0 {
                self.pos = mark;
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos]).ok()?;
        text.parse().ok().map(Some)
    }

    /// One coordinate pair. `None` inside the option means "no more numbers";
    /// an outer `None` means malformed.
    fn pair(&mut self) -> Option<Option<(f64, f64)>> {
        let Some(x) = self.number()? else {
            return Some(None);
        };
        let Some(y) = self.number()? else {
            return None;
        };
        Some(Some((x, y)))
    }

    fn triplet(&mut self) -> Option<Option<(f64, f64, f64)>> {
        let Some(x) = self.number()? else {
            return Some(None);
        };
        let Some(y) = self.number()? else {
            return None;
        };
        let Some(z) = self.number()? else {
            return None;
        };
        Some(Some((x, y, z)))
    }

    fn sextet(&mut self) -> Option<Option<(f64, f64, f64, f64, f64, f64)>> {
        let Some(a) = self.number()? else {
            return Some(None);
        };
        let Some(b) = self.number()? else {
            return None;
        };
        let Some(c) = self.number()? else {
            return None;
        };
        let Some(d) = self.number()? else {
            return None;
        };
        let Some(e) = self.number()? else {
            return None;
        };
        let Some(f) = self.number()? else {
            return None;
        };
        Some(Some((a, b, c, d, e, f)))
    }

    fn quad_pair(&mut self) -> Option<Option<(f64, f64, f64, f64)>> {
        let Some(a) = self.number()? else {
            return Some(None);
        };
        let Some(b) = self.number()? else {
            return None;
        };
        let Some(c) = self.number()? else {
            return None;
        };
        let Some(d) = self.number()? else {
            return None;
        };
        Some(Some((a, b, c, d)))
    }

    /// The seven arguments of an arc. Flags are the characters `0` or `1`,
    /// which may run together with the numbers that follow (`A5 5 0 0110 10`).
    fn arc_args(&mut self) -> Option<Option<(f64, f64, f64, bool, bool, f64, f64)>> {
        let Some(rx) = self.number()? else {
            return Some(None);
        };
        let Some(ry) = self.number()? else {
            return None;
        };
        let Some(rotation) = self.number()? else {
            return None;
        };
        let Some(large) = self.flag()? else {
            return None;
        };
        let Some(sweep) = self.flag()? else {
            return None;
        };
        let Some(x) = self.number()? else {
            return None;
        };
        let Some(y) = self.number()? else {
            return None;
        };
        Some(Some((rx, ry, rotation, large, sweep, x, y)))
    }

    fn flag(&mut self) -> Option<Option<bool>> {
        self.skip_separators();
        if self.pos >= self.bytes.len() {
            return Some(None);
        }
        let v = match self.bytes[self.pos] {
            b'0' => false,
            b'1' => true,
            _ => return None,
        };
        self.pos += 1;
        Some(Some(v))
    }
}

/// Append an SVG elliptical arc as cubic segments.
#[allow(clippy::too_many_arguments)]
fn arc_to(
    builder: &mut PathBuilder,
    x0: f64,
    y0: f64,
    rx: f64,
    ry: f64,
    rotation_deg: f64,
    large: bool,
    sweep: bool,
    x: f64,
    y: f64,
) {
    if rx == 0.0 || ry == 0.0 || (x0 == x && y0 == y) {
        builder.line_to(x as f32, y as f32);
        return;
    }
    let mut rx = rx.abs();
    let mut ry = ry.abs();
    let phi = rotation_deg.to_radians();
    let (sin_phi, cos_phi) = phi.sin_cos();
    let dx = (x0 - x) / 2.0;
    let dy = (y0 - y) / 2.0;
    let x1 = cos_phi * dx + sin_phi * dy;
    let y1 = -sin_phi * dx + cos_phi * dy;
    let lambda = x1 * x1 / (rx * rx) + y1 * y1 / (ry * ry);
    if lambda > 1.0 {
        let s = lambda.sqrt();
        rx *= s;
        ry *= s;
    }
    let sign = if large == sweep { -1.0 } else { 1.0 };
    let num = (rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1).max(0.0);
    let den = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let coef = if den == 0.0 { 0.0 } else { sign * (num / den).sqrt() };
    let cx1 = coef * rx * y1 / ry;
    let cy1 = -coef * ry * x1 / rx;
    let cx = cos_phi * cx1 - sin_phi * cy1 + (x0 + x) / 2.0;
    let cy = sin_phi * cx1 + cos_phi * cy1 + (y0 + y) / 2.0;
    let theta1 = ((y1 - cy1) / ry).atan2((x1 - cx1) / rx);
    let theta2 = ((-y1 - cy1) / ry).atan2((-x1 - cx1) / rx);
    let mut delta = theta2 - theta1;
    if !sweep && delta > 0.0 {
        delta -= std::f64::consts::TAU;
    } else if sweep && delta < 0.0 {
        delta += std::f64::consts::TAU;
    }
    let segments = (delta.abs() / std::f64::consts::FRAC_PI_2).ceil().max(1.0) as usize;
    let step = delta / segments as f64;
    let alpha = 4.0 / 3.0 * (step / 4.0).tan();
    let point = |t: f64| {
        let (s, c) = t.sin_cos();
        (
            cx + cos_phi * rx * c - sin_phi * ry * s,
            cy + sin_phi * rx * c + cos_phi * ry * s,
        )
    };
    let tangent = |t: f64| {
        let (s, c) = t.sin_cos();
        (
            -cos_phi * rx * s - sin_phi * ry * c,
            -sin_phi * rx * s + cos_phi * ry * c,
        )
    };
    let mut t0 = theta1;
    for _ in 0..segments {
        let t1 = t0 + step;
        let (px, py) = point(t0);
        let (qx, qy) = point(t1);
        let (dx0, dy0) = tangent(t0);
        let (dx1, dy1) = tangent(t1);
        builder.cubic_to(
            (px + alpha * dx0) as f32,
            (py + alpha * dy0) as f32,
            (qx - alpha * dx1) as f32,
            (qy - alpha * dy1) as f32,
            qx as f32,
            qy as f32,
        );
        t0 = t1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(data: &str) -> (f32, f32, f32, f32) {
        let path = parse_svg_path(data).expect("path");
        let b = path.bounds();
        (b.left(), b.top(), b.right(), b.bottom())
    }

    #[test]
    fn parses_lines_and_closes() {
        let (l, t, r, b) = bounds("M10 20 L30 20 L30 40 Z");
        assert_eq!((l, t, r, b), (10.0, 20.0, 30.0, 40.0));
    }

    #[test]
    fn parses_relative_commands_and_implicit_repeats() {
        let (l, t, r, b) = bounds("m10 10 h20 v20 h-20 z");
        assert_eq!((l, t, r, b), (10.0, 10.0, 30.0, 30.0));
        let (l, t, r, b) = bounds("M10 10 20 20 30 10");
        assert_eq!((l, t, r, b), (10.0, 10.0, 30.0, 20.0));
    }

    #[test]
    fn smooth_commands_reflect_the_previous_control() {
        let a = bounds("M0 0 C10 0 20 10 30 10 S50 20 60 20");
        let b = bounds("M0 0 C10 0 20 10 30 10 C40 10 50 20 60 20");
        assert_eq!(a, b);
        let q = bounds("M0 0 Q10 0 20 10 T40 20");
        let q2 = bounds("M0 0 Q10 0 20 10 Q30 20 40 20");
        assert_eq!(q, q2);
    }

    #[test]
    fn parses_arcs_in_all_four_quadrants() {
        let (l, t, r, b) = bounds("M50 8A42 42 0 1 1 50 92A42 42 0 1 1 50 8Z");
        assert!((l - 8.0).abs() < 0.05, "{l}");
        assert!((t - 8.0).abs() < 0.05, "{t}");
        assert!((r - 92.0).abs() < 0.05, "{r}");
        assert!((b - 92.0).abs() < 0.05, "{b}");
    }

    #[test]
    fn arc_flags_may_run_together_with_numbers() {
        let glued = bounds("M50 8A42 42 0 1 1 50 92Z");
        let loose = bounds("M50 8 A 42 42 0 1 1 50 92 Z");
        assert_eq!(glued, loose);
    }

    #[test]
    fn splits_parts_per_moveto() {
        let parts = split_subpaths(
            "M47.5 14H52.5V32H47.5ZM43 11A7 7 0 1 1 57 11A7 7 0 1 1 43 11Z",
        );
        assert_eq!(parts.len(), 2);
        let paths = parse_svg_subpaths(
            "M47.5 14H52.5V32H47.5ZM43 11A7 7 0 1 1 57 11A7 7 0 1 1 43 11Z",
        );
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn parses_all_bundled_shapes() {
        for ty in crate::widgets::bot_avatar::types::BotAvatarType::ALL {
            let d = crate::widgets::bot_avatar::data::bot_avatar_shape(ty);
            let path = parse_svg_path(d).unwrap_or_else(|| panic!("{ty:?}"));
            assert!(path.bounds().width() > 50.0, "{ty:?}");
        }
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_svg_path("nonsense").is_none());
        assert!(parse_svg_path("M10").is_none());
        assert!(parse_svg_path("M10 10 L20 20").is_some());
    }
}
