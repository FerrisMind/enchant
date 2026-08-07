//! Animated glyph matrix adapted from the MIT-licensed Magic UI component.

use crate::primitives::{EffectCanvasProps, EffectProgram, StableRng, effect_canvas};
use iced::advanced::text::LineHeight;
use iced::widget::canvas;
use iced::{Color, Element, Font, Length, Pixels, Point, Rectangle, Renderer, Vector};
use std::borrow::Cow;
use std::cell::RefCell;
use std::time::Duration;

const DEFAULT_GLYPHS: &str = "01·•+*/\\<>=";
const DEFAULT_COLOR: Color = Color {
    r: 0x6B as f32 / 255.0,
    g: 0x72 as f32 / 255.0,
    b: 0x80 as f32 / 255.0,
    a: 1.0,
};
const MATRIX_SEED: u64 = 0x4759_4C50_485F_4D58;
const MAX_CATCH_UP_TICKS: u64 = 256;

/// Configuration for [`glyph_matrix`].
///
/// The visual defaults match Magic UI's `GlyphMatrix`: a 14 px monospace
/// grid, a 90 ms mutation interval, and a muted gray glyph color.
#[derive(Clone, Debug, PartialEq)]
pub struct GlyphMatrixProps {
    width: Length,
    height: Length,
    glyphs: Cow<'static, str>,
    cell_size: f32,
    mutation_rate: f32,
    interval: Duration,
    fade_bottom: f32,
    color: Color,
}

impl Default for GlyphMatrixProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            height: Length::Fixed(220.0),
            glyphs: Cow::Borrowed(DEFAULT_GLYPHS),
            cell_size: 14.0,
            mutation_rate: 0.04,
            interval: Duration::from_millis(90),
            fade_bottom: 0.6,
            color: DEFAULT_COLOR,
        }
    }
}

impl GlyphMatrixProps {
    /// Creates a configuration with the defaults documented by Magic UI.
    ///
    /// # Example
    ///
    /// ```
    /// use enchant_iced::GlyphMatrixProps;
    ///
    /// let _props = GlyphMatrixProps::new().cell_size(12.0);
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the layout width of the canvas.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the layout height of the canvas.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the characters used by the matrix.
    pub fn glyphs(mut self, glyphs: impl Into<Cow<'static, str>>) -> Self {
        self.glyphs = glyphs.into();
        self
    }

    /// Sets the cell size in pixels and the glyph font size.
    pub fn cell_size(mut self, cell_size: f32) -> Self {
        self.cell_size = normalized_cell_size(cell_size);
        self
    }

    /// Sets the fraction of cells that may mutate on each interval.
    pub fn mutation_rate(mut self, mutation_rate: f32) -> Self {
        self.mutation_rate = normalized_mutation_rate(mutation_rate);
        self
    }

    /// Sets the time between mutation ticks.
    pub fn interval(mut self, interval: Duration) -> Self {
        self.interval = normalized_interval(interval);
        self
    }

    /// Sets the time between mutation ticks in milliseconds.
    pub fn interval_ms(mut self, interval_ms: u64) -> Self {
        self.interval = Duration::from_millis(interval_ms.max(1));
        self
    }

    /// Sets the fade strength toward the bottom of the matrix.
    pub fn fade_bottom(mut self, fade_bottom: f32) -> Self {
        self.fade_bottom = normalized_fade_bottom(fade_bottom);
        self
    }

    /// Sets the glyph color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

/// Draw an animated matrix of subtly shifting glyphs.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use enchant_iced::{GlyphMatrixProps, glyph_matrix};
/// use std::time::Duration;
///
/// let _element: Element<'_, ()> = glyph_matrix(Duration::ZERO, GlyphMatrixProps::new());
/// ```
pub fn glyph_matrix<'a, Message: 'a>(
    elapsed: Duration,
    props: GlyphMatrixProps,
) -> Element<'a, Message> {
    let width = props.width;
    let height = props.height;
    effect_canvas::<_, Message>(
        GlyphMatrixProgram { elapsed, props },
        EffectCanvasProps::new().width(width).height(height),
    )
    .into()
}

struct GlyphMatrixProgram {
    elapsed: Duration,
    props: GlyphMatrixProps,
}

#[derive(Default)]
struct GlyphMatrixState {
    matrix: RefCell<MatrixCells>,
}

#[derive(Default)]
struct MatrixCells {
    columns: usize,
    rows: usize,
    glyph_source: Cow<'static, str>,
    glyphs: Vec<char>,
    glyph_paths: Vec<Vec<canvas::Path>>,
    cells: Vec<usize>,
    alphas: Vec<f32>,
    cell_size: f32,
    last_tick: u64,
    initialized: bool,
}

impl MatrixCells {
    fn reset(
        &mut self,
        columns: usize,
        rows: usize,
        cell_size: f32,
        glyph_source: Cow<'static, str>,
    ) {
        let glyphs = normalized_glyphs(&glyph_source);
        self.columns = columns;
        self.rows = rows;
        self.glyph_source.clone_from(&glyph_source);
        self.glyphs.clone_from(&glyphs);
        self.glyph_paths.clear();
        self.glyph_paths.reserve(glyphs.len());
        for glyph in &glyphs {
            self.glyph_paths.push(glyph_paths(*glyph, cell_size));
        }
        self.cells.clear();
        self.alphas.clear();
        self.cell_size = cell_size;

        let mut rng = StableRng::new(MATRIX_SEED ^ ((columns as u64) << 32) ^ rows as u64);
        let total = columns.saturating_mul(rows);
        self.cells.reserve(total);
        self.alphas.reserve(total);
        for _ in 0..total {
            self.cells.push(rng.range_usize(0, glyphs.len()));
            self.alphas.push(rng.range_f32(0.05, 0.40));
        }

        self.last_tick = 0;
        self.initialized = true;
    }

    fn advance(&mut self, tick: u64, mutation_rate: f32) {
        if tick <= self.last_tick || self.cells.is_empty() {
            return;
        }

        let total = self.cells.len();
        let rate = normalized_mutation_rate(mutation_rate);
        let mutations = if rate == 0.0 {
            0
        } else {
            ((total as f32 * rate).floor() as usize).max(1)
        };

        let first_tick = if tick.saturating_sub(self.last_tick) > MAX_CATCH_UP_TICKS {
            tick - MAX_CATCH_UP_TICKS + 1
        } else {
            self.last_tick.saturating_add(1)
        };

        for current_tick in first_tick..=tick {
            let mut rng = StableRng::new(MATRIX_SEED ^ current_tick.rotate_left(17));
            for _ in 0..mutations {
                let index = rng.range_usize(0, total);
                self.cells[index] = rng.range_usize(0, self.glyphs.len());
                self.alphas[index] = rng.range_f32(0.05, 0.50);
            }
        }

        self.last_tick = tick;
    }
}

impl<Message> EffectProgram<Message> for GlyphMatrixProgram {
    type State = GlyphMatrixState;

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<canvas::Geometry> {
        let cell_size = normalized_cell_size(self.props.cell_size);
        let columns = (bounds.width / cell_size).ceil().max(1.0) as usize;
        let rows = (bounds.height / cell_size).ceil().max(1.0) as usize;
        let tick = elapsed_tick(self.elapsed, self.props.interval);

        let mut matrix = state.matrix.borrow_mut();
        if !matrix.initialized
            || matrix.columns != columns
            || matrix.rows != rows
            || matrix.glyph_source != self.props.glyphs
            || matrix.cell_size != cell_size
            || tick < matrix.last_tick
        {
            matrix.reset(columns, rows, cell_size, self.props.glyphs.clone());
        }
        matrix.advance(tick, self.props.mutation_rate);

        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let fade_bottom = normalized_fade_bottom(self.props.fade_bottom);

        for row in 0..rows {
            let fade = (1.0 - (row as f32 / rows as f32) * fade_bottom).clamp(0.0, 1.0);
            for column in 0..columns {
                let index = row * columns + column;
                let color = Color {
                    a: self.props.color.a * matrix.alphas[index] * fade,
                    ..self.props.color
                };

                frame.with_save(|frame| {
                    frame.translate(Vector::new(
                        column as f32 * cell_size,
                        row as f32 * cell_size,
                    ));
                    for path in &matrix.glyph_paths[matrix.cells[index]] {
                        frame.fill(path, color);
                    }
                });
            }
        }

        vec![frame.into_geometry()]
    }
}

fn normalized_glyphs(glyphs: &str) -> Vec<char> {
    let glyphs = glyphs.chars().collect::<Vec<_>>();
    if glyphs.is_empty() {
        DEFAULT_GLYPHS.chars().collect()
    } else {
        glyphs
    }
}

fn glyph_paths(glyph: char, cell_size: f32) -> Vec<canvas::Path> {
    let mut content = String::with_capacity(glyph.len_utf8());
    content.push(glyph);
    let mut paths = Vec::new();
    canvas::Text {
        content,
        position: Point::ORIGIN,
        max_width: f32::INFINITY,
        color: Color::TRANSPARENT,
        size: Pixels((cell_size - 2.0).max(1.0)),
        line_height: LineHeight::Relative(1.0),
        font: Font::MONOSPACE,
        align_x: iced::advanced::text::Alignment::Left,
        align_y: iced::alignment::Vertical::Top,
        ..canvas::Text::default()
    }
    .draw_with(|path, _| paths.push(path));
    paths
}

fn normalized_cell_size(cell_size: f32) -> f32 {
    if cell_size.is_finite() {
        cell_size.max(1.0)
    } else {
        1.0
    }
}

fn normalized_mutation_rate(mutation_rate: f32) -> f32 {
    if mutation_rate.is_finite() {
        mutation_rate.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn normalized_interval(interval: Duration) -> Duration {
    if interval.as_millis() == 0 {
        Duration::from_millis(1)
    } else {
        interval
    }
}

fn normalized_fade_bottom(fade_bottom: f32) -> f32 {
    if fade_bottom.is_finite() {
        fade_bottom.max(0.0)
    } else {
        0.0
    }
}

fn elapsed_tick(elapsed: Duration, interval: Duration) -> u64 {
    let interval_ms = normalized_interval(interval).as_millis();
    (elapsed.as_millis() / interval_ms).min(u64::MAX as u128) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_matrix_defaults_match_magic_ui_docs() {
        let props = GlyphMatrixProps::new();

        assert_eq!(props.glyphs, Cow::Borrowed(DEFAULT_GLYPHS));
        assert_eq!(props.cell_size, 14.0);
        assert_eq!(props.interval, Duration::from_millis(90));
    }

    #[test]
    fn empty_glyph_sets_fall_back_to_the_documented_default() {
        assert_eq!(
            normalized_glyphs(""),
            DEFAULT_GLYPHS.chars().collect::<Vec<_>>()
        );
        assert_eq!(normalized_glyphs("ab"), vec!['a', 'b']);
    }

    #[test]
    fn invalid_cell_sizes_and_intervals_are_safe_for_layout() {
        assert_eq!(normalized_cell_size(0.0), 1.0);
        assert_eq!(normalized_cell_size(f32::NAN), 1.0);
        assert_eq!(normalized_cell_size(12.0), 12.0);
        assert_eq!(elapsed_tick(Duration::from_millis(4), Duration::ZERO), 4);
        assert_eq!(
            elapsed_tick(Duration::from_millis(181), Duration::from_millis(90)),
            2
        );
    }

    #[test]
    fn builders_normalize_values_at_the_api_boundary() {
        let props = GlyphMatrixProps::new()
            .cell_size(0.0)
            .mutation_rate(f32::NAN)
            .interval(Duration::from_micros(1))
            .fade_bottom(-1.0);

        assert_eq!(props.cell_size, 1.0);
        assert_eq!(props.mutation_rate, 0.0);
        assert_eq!(props.interval, Duration::from_millis(1));
        assert_eq!(props.fade_bottom, 0.0);
    }

    #[test]
    fn zero_mutation_rate_keeps_cells_stable() {
        let mut matrix = MatrixCells::default();
        matrix.reset(2, 2, 14.0, Cow::Borrowed("01"));
        let initial = matrix.cells.clone();

        matrix.advance(10, 0.0);

        assert_eq!(matrix.cells, initial);
    }

    #[test]
    fn empty_matrices_do_not_mutate_or_panic() {
        let mut matrix = MatrixCells::default();

        matrix.advance(1, 1.0);

        assert_eq!(matrix.last_tick, 0);
    }

    #[test]
    fn large_elapsed_jumps_are_bounded() {
        let mut matrix = MatrixCells::default();
        matrix.reset(2, 2, 14.0, Cow::Borrowed("01"));

        matrix.advance(u64::MAX, 0.04);

        assert_eq!(matrix.last_tick, u64::MAX);
    }
}
