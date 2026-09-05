//! Builder-first frame-based 2D grid (Magic UI `MotionGrid` + `MotionGridCells`).
//!
//! Each frame is a bitmask of active cells; the host supplies elapsed time for
//! frame index and cross-fade progress. Out-of-range dot coordinates in
//! [`MotionGrid::frames`] are dropped. Default width/height use [`Length::Shrink`]
//! with the grid's natural extent from cell size and gap.

use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};
use iced::widget::canvas::{self, Fill, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size};
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

/// Grid coordinate of an active cell in a single animation frame (`x`, `y`).
///
/// Maps Magic UI `MotionGridCells` dot tuples. Dots outside `cols` × `rows` are
/// ignored when frames are compiled.
pub type FrameDot = (u16, u16);

/// Easing applied to cross-fade progress within each frame interval.
///
/// Maps Magic UI `transition.ease` without exposing `twill` in the public API.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum MotionGridEase {
    /// Constant speed.
    Linear,
    /// Slow start.
    EaseIn,
    /// Slow end.
    EaseOut,
    /// Slow start and end (Magic UI default).
    #[default]
    EaseInOut,
}

/// Builder for an animated dot grid with per-frame cell masks.
///
/// # Examples
///
/// ```rust,no_run
/// use iced::Element;
/// use enchant_iced::MotionGrid;
/// use std::time::Duration;
///
/// fn grid<'a, Message: 'a>(elapsed: Duration) -> Element<'a, Message> {
///     MotionGrid::new(5, 5)
///         .frames([vec![(2, 2)], vec![(0, 0), (4, 4)]])
///         .cell_size(3.0)
///         .gap(2.0)
///         .into_element(elapsed)
/// }
/// ```
#[must_use = "builders do nothing unless turned into an iced Element"]
#[derive(Clone)]
pub struct MotionGrid {
    cols: u16,
    rows: u16,
    /// Per-frame bitmasks; word count = ceil(cols*rows / 64).
    frames: Arc<[Box<[u64]>]>,
    duration_ms: u64,
    animate: bool,
    gap: f32,
    cell_size: f32,
    /// `None` uses `cell_size * 0.5` (full circle) at draw time.
    cell_radius: Option<f32>,
    active_color: Color,
    inactive_color: Color,
    active_scale: f32,
    inactive_scale: f32,
    cell_border_width: f32,
    active_border_color: Option<Color>,
    inactive_border_color: Option<Color>,
    active_glow: Option<Color>,
    active_pulse: bool,
    ease: MotionGridEase,
    width: Length,
    height: Length,
}

impl MotionGrid {
    /// Creates a grid with Magic UI preview defaults (`cell_size` 3, `gap` 2, …).
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols: cols.max(1),
            rows: rows.max(1),
            frames: Arc::from(Vec::<Box<[u64]>>::new()),
            duration_ms: 200,
            animate: true,
            gap: 2.0,
            cell_size: 3.0,
            cell_radius: None,
            active_color: Color::from_rgba(1.0, 1.0, 1.0, 0.70),
            inactive_color: Color::from_rgba(1.0, 1.0, 1.0, 0.20),
            active_scale: 1.0,
            inactive_scale: 1.0,
            cell_border_width: 0.0,
            active_border_color: None,
            inactive_border_color: None,
            active_glow: None,
            active_pulse: false,
            ease: MotionGridEase::EaseInOut,
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    /// Per-frame active cell coordinates (Magic UI `frames` / `MotionGridCells`).
    ///
    /// Coordinates outside the grid bounds are dropped.
    pub fn frames(
        mut self,
        frames: impl IntoIterator<Item = impl IntoIterator<Item = FrameDot>>,
    ) -> Self {
        self.frames = compile_frames(self.cols, self.rows, frames);
        self
    }

    /// Milliseconds per frame step (Magic UI `duration`).
    pub fn duration_ms(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms.max(1);
        self
    }

    /// When `false`, holds frame 0 (Magic UI `animate`).
    pub fn animate(mut self, animate: bool) -> Self {
        self.animate = animate;
        self
    }

    /// Spacing between cell slots in logical pixels (Magic UI `gap`).
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap.max(0.0);
        self
    }

    /// Cell edge length in logical pixels (Magic UI `cellSize`).
    pub fn cell_size(mut self, cell_size: f32) -> Self {
        self.cell_size = cell_size.max(1.0);
        self
    }

    /// Corner radius for rounded cells (Magic UI `cellRadius`).
    ///
    /// When unset, radius follows `cell_size * 0.5` so dots stay circular after
    /// resizing. At draw time, `radius >= cell_size * 0.5` renders a circle.
    pub fn cell_radius(mut self, cell_radius: f32) -> Self {
        self.cell_radius = Some(cell_radius.max(0.0));
        self
    }

    /// Color of active dots (Magic UI `activeColor`).
    pub fn active_color(mut self, color: Color) -> Self {
        self.active_color = color;
        self
    }

    /// Color of inactive dots (Magic UI `inactiveColor`).
    pub fn inactive_color(mut self, color: Color) -> Self {
        self.inactive_color = color;
        self
    }

    /// Scale of active dots (Magic UI `activeScale`).
    pub fn active_scale(mut self, active_scale: f32) -> Self {
        self.active_scale = active_scale.max(0.0);
        self
    }

    /// Scale of inactive dots (Magic UI `inactiveScale`).
    pub fn inactive_scale(mut self, inactive_scale: f32) -> Self {
        self.inactive_scale = inactive_scale.max(0.0);
        self
    }

    /// Cell border stroke width in logical pixels (default `0`, no stroke).
    pub fn cell_border_width(mut self, cell_border_width: f32) -> Self {
        self.cell_border_width = cell_border_width.max(0.0);
        self
    }

    /// Border color when a cell is active (Magic UI `activeProps` border).
    ///
    /// Stroked only when [`Self::cell_border_width`] is greater than zero and
    /// this or [`Self::inactive_border_color`] is set.
    pub fn active_border_color(mut self, color: Color) -> Self {
        self.active_border_color = Some(color);
        self
    }

    /// Border color when a cell is inactive (Magic UI `inactiveProps` border).
    ///
    /// Stroked only when [`Self::cell_border_width`] is greater than zero and
    /// this or [`Self::active_border_color`] is set.
    pub fn inactive_border_color(mut self, color: Color) -> Self {
        self.inactive_border_color = Some(color);
        self
    }

    /// Soft outer glow drawn behind active cells (Magic UI emerald shadow).
    pub fn active_glow(mut self, color: Color) -> Self {
        self.active_glow = Some(color);
        self
    }

    /// When `true`, active cells pulse scale `1.0`↔`1.2` and opacity `0.8`↔`1.0`
    /// over each [`Self::duration_ms`] interval (Magic UI `activeProps.animate`).
    pub fn active_pulse(mut self, active_pulse: bool) -> Self {
        self.active_pulse = active_pulse;
        self
    }

    /// Cross-fade easing within each frame (Magic UI `transition.ease`).
    pub fn ease(mut self, ease: MotionGridEase) -> Self {
        self.ease = ease;
        self
    }

    /// Widget width (default [`Length::Shrink`] → natural grid width).
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Widget height (default [`Length::Shrink`] → natural grid height).
    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    /// Column count (minimum 1).
    pub fn cols(&self) -> u16 {
        self.cols
    }

    /// Row count (minimum 1).
    pub fn rows(&self) -> u16 {
        self.rows
    }

    /// Inspection helper: whether cell `(x, y)` is active in `frame_index`.
    pub fn cell_active_at(&self, frame_index: usize, x: u16, y: u16) -> bool {
        cell_active(self, frame_index, x, y)
    }

    /// Inspection helper: current frame index and in-frame progress in `[0, 1)`.
    ///
    /// When animation is disabled or no frames are set, returns `(0, 1.0)`.
    pub fn phase(&self, elapsed: Duration) -> (usize, f32) {
        let (index, progress, _) = frame_index_and_progress(self, elapsed);
        (index, progress)
    }

    /// Builds the canvas element for the given animation clock.
    pub fn into_element<'a, Message: 'a>(self, elapsed: Duration) -> Element<'a, Message> {
        let inset = overflow_inset(&self);
        let width = resolve_length(
            self.width,
            natural_extent(self.cols, self.cell_size, self.gap) + inset * 2.0,
        );
        let height = resolve_length(
            self.height,
            natural_extent(self.rows, self.cell_size, self.gap) + inset * 2.0,
        );

        effect_canvas::<_, Message>(
            MotionGridProgram {
                elapsed,
                config: self,
            },
            EffectCanvasProps::new().width(width).height(height),
        )
        .into()
    }

    /// Alias for [`Self::into_element`].
    pub fn view<'a, Message: 'a>(self, elapsed: Duration) -> Element<'a, Message> {
        self.into_element(elapsed)
    }
}

impl fmt::Debug for MotionGrid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MotionGrid")
            .field("cols", &self.cols)
            .field("rows", &self.rows)
            .field("frame_count", &self.frames.len())
            .field("duration_ms", &self.duration_ms)
            .field("animate", &self.animate)
            .field("gap", &self.gap)
            .field("cell_size", &self.cell_size)
            .field("cell_radius", &self.cell_radius)
            .field("active_color", &self.active_color)
            .field("inactive_color", &self.inactive_color)
            .field("active_scale", &self.active_scale)
            .field("inactive_scale", &self.inactive_scale)
            .field("cell_border_width", &self.cell_border_width)
            .field("active_border_color", &self.active_border_color)
            .field("inactive_border_color", &self.inactive_border_color)
            .field("active_glow", &self.active_glow)
            .field("active_pulse", &self.active_pulse)
            .field("ease", &self.ease)
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

struct MotionGridProgram {
    elapsed: Duration,
    config: MotionGrid,
}

impl<Message> EffectProgram<Message> for MotionGridProgram {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let config = &self.config;

        if config.cols == 0 || config.rows == 0 {
            return vec![frame.into_geometry()];
        }

        let (index, t, tick) = frame_index_and_progress(config, self.elapsed);
        let cell_radius = config.cell_radius.unwrap_or(config.cell_size * 0.5);
        let slot = config.cell_size + config.gap;
        let inset = overflow_inset(config);
        let eased_t = apply_ease(t, config.ease);
        let pulse = active_pulse_modulation(config, self.elapsed);

        for row in 0..config.rows {
            for col in 0..config.cols {
                let curr_active = cell_active(config, index, col, row);
                let prev_active = if tick == 0 {
                    false
                } else {
                    let n = config.frames.len();
                    let prev_index = (index + n - 1) % n;
                    cell_active(config, prev_index, col, row)
                };

                let amount = if prev_active == curr_active {
                    f32::from(curr_active)
                } else if curr_active {
                    eased_t
                } else {
                    1.0 - eased_t
                };

                let mut color = lerp_color(config.inactive_color, config.active_color, amount);
                let mut scale = lerp_f32(config.inactive_scale, config.active_scale, amount);

                if config.active_pulse && amount > 0.0 {
                    let pulse_scale = lerp_f32(1.0, pulse.scale, amount);
                    let pulse_opacity = lerp_f32(1.0, pulse.opacity, amount);
                    scale *= pulse_scale;
                    color.a *= pulse_opacity;
                }

                let draw_size = config.cell_size * scale;
                let half_draw = draw_size * 0.5;

                let slot_x = inset + col as f32 * slot;
                let slot_y = inset + row as f32 * slot;
                let center = Point::new(
                    slot_x + config.cell_size * 0.5,
                    slot_y + config.cell_size * 0.5,
                );

                let path = cell_path(center, draw_size, cell_radius, config.cell_size);

                if let Some(glow) = config.active_glow.filter(|_| amount > 0.001) {
                    let glow_alpha = glow.a * amount * 0.55;
                    let glow_radius = half_draw * GLOW_RADIUS_FACTOR;
                    let glow_path = canvas::Path::circle(center, glow_radius);
                    frame.fill(
                        &glow_path,
                        Fill::from(Color {
                            a: glow_alpha,
                            ..glow
                        }),
                    );
                }

                frame.fill(&path, Fill::from(color));

                if config.cell_border_width > 0.0
                    && (config.active_border_color.is_some()
                        || config.inactive_border_color.is_some())
                {
                    let border_from = config.inactive_border_color.unwrap_or(Color::TRANSPARENT);
                    let border_to = config.active_border_color.unwrap_or(Color::TRANSPARENT);
                    let border_color = lerp_color(border_from, border_to, amount);
                    if border_color.a > 0.001 {
                        frame.stroke(
                            &path,
                            Stroke::default()
                                .with_width(config.cell_border_width)
                                .with_color(border_color),
                        );
                    }
                }
            }
        }

        vec![frame.into_geometry()]
    }
}

struct PulseModulation {
    scale: f32,
    opacity: f32,
}

fn active_pulse_modulation(grid: &MotionGrid, elapsed: Duration) -> PulseModulation {
    if !grid.active_pulse {
        return PulseModulation {
            scale: 1.0,
            opacity: 1.0,
        };
    }

    let elapsed_ms = elapsed.as_millis() as u64;
    let t = (elapsed_ms % grid.duration_ms) as f32 / grid.duration_ms as f32;
    let scale = if t < 0.5 {
        lerp_f32(1.0, 1.2, t * 2.0)
    } else {
        lerp_f32(1.2, 1.0, (t - 0.5) * 2.0)
    };
    let opacity = if t < 0.5 {
        lerp_f32(0.8, 1.0, t * 2.0)
    } else {
        lerp_f32(1.0, 0.8, (t - 0.5) * 2.0)
    };

    PulseModulation { scale, opacity }
}

fn cell_path(center: Point, draw_size: f32, cell_radius: f32, cell_size: f32) -> canvas::Path {
    let half_draw = draw_size * 0.5;
    if cell_radius >= cell_size * 0.5 {
        canvas::Path::circle(center, half_draw)
    } else {
        let top_left = Point::new(center.x - half_draw, center.y - half_draw);
        canvas::Path::rounded_rectangle(
            top_left,
            Size::new(draw_size, draw_size),
            cell_radius.into(),
        )
    }
}

fn words_for(cells: usize) -> usize {
    cells.div_ceil(64).max(1)
}

fn set_bit(mask: &mut [u64], index: usize) {
    let w = index / 64;
    let b = index % 64;
    if let Some(word) = mask.get_mut(w) {
        *word |= 1u64 << b;
    }
}

fn bit_is_set(mask: &[u64], index: usize) -> bool {
    let w = index / 64;
    let b = index % 64;
    mask.get(w).is_some_and(|word| (word & (1u64 << b)) != 0)
}

fn compile_frames(
    cols: u16,
    rows: u16,
    frames: impl IntoIterator<Item = impl IntoIterator<Item = FrameDot>>,
) -> Arc<[Box<[u64]>]> {
    let cells = (cols as usize) * (rows as usize);
    let word_count = words_for(cells);

    let compiled: Vec<Box<[u64]>> = frames
        .into_iter()
        .map(|frame_dots| {
            let mut mask = vec![0u64; word_count];
            for (x, y) in frame_dots {
                if x < cols && y < rows {
                    let index = (y as usize) * (cols as usize) + (x as usize);
                    set_bit(&mut mask, index);
                }
            }
            mask.into_boxed_slice()
        })
        .collect();

    Arc::from(compiled)
}

/// Returns frame index, in-frame progress `[0, 1)`, and completed frame ticks.
fn frame_index_and_progress(grid: &MotionGrid, elapsed: Duration) -> (usize, f32, u64) {
    if !grid.animate || grid.frames.is_empty() {
        return (0, 1.0, 0);
    }

    let elapsed_ms = elapsed.as_millis() as u64;
    let tick = elapsed_ms / grid.duration_ms;
    let t = (elapsed_ms % grid.duration_ms) as f32 / grid.duration_ms as f32;
    let n = grid.frames.len();
    let index = (tick as usize) % n;
    (index, t, tick)
}

fn cell_active(grid: &MotionGrid, frame_index: usize, x: u16, y: u16) -> bool {
    if x >= grid.cols || y >= grid.rows {
        return false;
    }

    let cell_index = (y as usize) * (grid.cols as usize) + (x as usize);
    grid.frames
        .get(frame_index)
        .is_some_and(|mask| bit_is_set(mask, cell_index))
}

fn apply_ease(t: f32, ease: MotionGridEase) -> f32 {
    let t = t.clamp(0.0, 1.0);
    match ease {
        MotionGridEase::Linear => t,
        MotionGridEase::EaseIn => t * t,
        MotionGridEase::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
        MotionGridEase::EaseInOut => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
            }
        }
    }
}

fn lerp_color(from: Color, to: Color, progress: f32) -> Color {
    let progress = progress.clamp(0.0, 1.0);
    Color {
        r: from.r + (to.r - from.r) * progress,
        g: from.g + (to.g - from.g) * progress,
        b: from.b + (to.b - from.b) * progress,
        a: from.a + (to.a - from.a) * progress,
    }
}

fn lerp_f32(from: f32, to: f32, progress: f32) -> f32 {
    let progress = progress.clamp(0.0, 1.0);
    from + (to - from) * progress
}

/// Peak active-pulse scale (Magic UI `scale: [1, 1.2, 1]`).
const PULSE_MAX_SCALE: f32 = 1.2;
/// Glow radius relative to the scaled cell half-size.
const GLOW_RADIUS_FACTOR: f32 = 1.85;

fn natural_extent(cells: u16, cell_size: f32, gap: f32) -> f32 {
    (cells as f32) * cell_size + (cells.saturating_sub(1) as f32) * gap
}

/// Extra padding so scaled / pulsed / glowing edge cells are not clipped.
fn overflow_inset(config: &MotionGrid) -> f32 {
    let max_scale = config.active_scale.max(config.inactive_scale)
        * if config.active_pulse {
            PULSE_MAX_SCALE
        } else {
            1.0
        };
    let scaled_half = config.cell_size * max_scale * 0.5;
    let base_half = config.cell_size * 0.5;
    let scale_overhang = (scaled_half - base_half).max(0.0);

    let glow_overhang = if config.active_glow.is_some() {
        let glow_radius = scaled_half * GLOW_RADIUS_FACTOR;
        (glow_radius - base_half).max(0.0)
    } else {
        0.0
    };

    scale_overhang.max(glow_overhang) + config.cell_border_width * 0.5
}

fn resolve_length(length: Length, natural: f32) -> Length {
    if length == Length::Shrink {
        Length::Fixed(natural)
    } else {
        length
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn effective_radius(grid: &MotionGrid) -> f32 {
        grid.cell_radius.unwrap_or(grid.cell_size * 0.5)
    }

    #[test]
    fn motion_grid_defaults_match_preview() {
        let g = MotionGrid::new(5, 5);
        assert_eq!(g.duration_ms, 200);
        assert!(g.animate);
        assert_eq!(g.cell_size, 3.0);
        assert_eq!(g.gap, 2.0);
        assert_eq!(g.cell_radius, None);
        assert!((effective_radius(&g) - 1.5).abs() < f32::EPSILON);
        assert_eq!(g.ease, MotionGridEase::EaseInOut);
        assert_eq!(g.cell_border_width, 0.0);
        assert_eq!(g.active_border_color, None);
        assert_eq!(g.inactive_border_color, None);
        assert_eq!(g.active_glow, None);
        assert!(!g.active_pulse);
    }

    #[test]
    fn cell_radius_follows_cell_size_when_unset() {
        let g = MotionGrid::new(5, 5).cell_size(8.0);
        assert_eq!(g.cell_radius, None);
        assert!((effective_radius(&g) - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn frames_compile_active_bits_y_cols_plus_x() {
        let g = MotionGrid::new(5, 5).frames([vec![(2, 2)], vec![(0, 0), (4, 4)]]);
        assert!(g.cell_active_at(0, 2, 2));
        assert!(!g.cell_active_at(0, 0, 0));
        assert!(g.cell_active_at(1, 0, 0));
        assert!(g.cell_active_at(1, 4, 4));
    }

    #[test]
    fn frame_index_advances_every_duration_ms() {
        let g = MotionGrid::new(5, 5).frames([[(0, 0)], [(1, 1)], [(2, 2)]]);
        let (i0, t0) = g.phase(Duration::from_millis(0));
        let (i1, _) = g.phase(Duration::from_millis(200));
        let (i2, _) = g.phase(Duration::from_millis(400));
        assert_eq!(i0, 0);
        assert_eq!(i1, 1);
        assert_eq!(i2, 2);
        assert!((t0 - 0.0).abs() < 0.001);
    }

    #[test]
    fn animate_false_freezes_at_frame_zero() {
        let g = MotionGrid::new(5, 5)
            .frames([[(0, 0)], [(1, 1)]])
            .animate(false);
        let (i, t) = g.phase(Duration::from_millis(9999));
        assert_eq!(i, 0);
        assert!((t - 1.0).abs() < 0.001);
    }

    #[test]
    fn ease_in_out_is_symmetric() {
        let a = apply_ease(0.25, MotionGridEase::EaseInOut);
        let b = apply_ease(0.75, MotionGridEase::EaseInOut);
        assert!((a + b - 1.0).abs() < 0.001);
    }

    #[test]
    fn active_pulse_modulates_over_duration() {
        let g = MotionGrid::new(5, 5).duration_ms(200).active_pulse(true);
        let start = active_pulse_modulation(&g, Duration::from_millis(0));
        let mid = active_pulse_modulation(&g, Duration::from_millis(100));
        assert!((start.scale - 1.0).abs() < 0.001);
        assert!((start.opacity - 0.8).abs() < 0.001);
        assert!((mid.scale - 1.2).abs() < 0.001);
        assert!((mid.opacity - 1.0).abs() < 0.001);
    }

    #[test]
    fn cell_border_width_validates_non_negative() {
        let g = MotionGrid::new(5, 5).cell_border_width(-3.0);
        assert_eq!(g.cell_border_width, 0.0);
    }

    #[test]
    fn overflow_inset_grows_for_scale_pulse_and_glow() {
        let base = MotionGrid::new(5, 5);
        assert_eq!(overflow_inset(&base), 0.0);

        let scaled = MotionGrid::new(5, 5).active_scale(1.1);
        assert!(overflow_inset(&scaled) > 0.0);

        let pulsed = MotionGrid::new(5, 5).active_pulse(true);
        assert!(overflow_inset(&pulsed) > 0.0);

        let glowing = MotionGrid::new(5, 5).active_glow(Color::WHITE);
        assert!(overflow_inset(&glowing) > overflow_inset(&scaled));
    }

    #[test]
    fn setters_validate_ranges() {
        let g = MotionGrid::new(0, 0)
            .duration_ms(0)
            .cell_size(-1.0)
            .gap(-2.0)
            .active_scale(-0.5);
        assert_eq!(g.cols(), 1);
        assert_eq!(g.rows(), 1);
        assert_eq!(g.duration_ms, 1);
        assert_eq!(g.cell_size, 1.0);
        assert_eq!(g.gap, 0.0);
        assert_eq!(g.active_scale, 0.0);
    }
}
