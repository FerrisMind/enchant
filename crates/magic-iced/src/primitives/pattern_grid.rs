use crate::primitives::StableRng;
use iced::{Point, Rectangle, Size};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridPatternSpec {
    pub cell_width: f32,
    pub cell_height: f32,
    pub offset_x: f32,
    pub offset_y: f32,
}

impl Default for GridPatternSpec {
    fn default() -> Self {
        Self {
            cell_width: 40.0,
            cell_height: 40.0,
            offset_x: -1.0,
            offset_y: -1.0,
        }
    }
}

impl GridPatternSpec {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cell_width(mut self, cell_width: f32) -> Self {
        self.cell_width = cell_width.max(1.0);
        self
    }

    pub fn cell_height(mut self, cell_height: f32) -> Self {
        self.cell_height = cell_height.max(1.0);
        self
    }

    pub fn offset(mut self, offset_x: f32, offset_y: f32) -> Self {
        self.offset_x = offset_x;
        self.offset_y = offset_y;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridCellIndex {
    pub x: i32,
    pub y: i32,
}

pub fn grid_dimensions(bounds: Size, spec: GridPatternSpec) -> (usize, usize) {
    let columns = if bounds.width <= 0.0 {
        0
    } else {
        ((bounds.width - spec.offset_x).max(0.0) / spec.cell_width)
            .ceil()
            .max(0.0) as usize
    };
    let rows = if bounds.height <= 0.0 {
        0
    } else {
        ((bounds.height - spec.offset_y).max(0.0) / spec.cell_height)
            .ceil()
            .max(0.0) as usize
    };

    (columns, rows)
}

pub fn grid_cell_origin(index: GridCellIndex, spec: GridPatternSpec) -> Point {
    Point::new(
        spec.offset_x + index.x as f32 * spec.cell_width,
        spec.offset_y + index.y as f32 * spec.cell_height,
    )
}

pub fn grid_cell_rect(index: GridCellIndex, spec: GridPatternSpec) -> Rectangle {
    Rectangle::new(
        grid_cell_origin(index, spec),
        Size::new(spec.cell_width, spec.cell_height),
    )
}

pub fn stable_grid_cell(
    seed: u64,
    slot: usize,
    iteration: usize,
    columns: usize,
    rows: usize,
) -> GridCellIndex {
    let mut rng = StableRng::new(seed)
        .fork(slot as u64)
        .fork(iteration as u64);
    GridCellIndex {
        x: rng.range_usize(0, columns) as i32,
        y: rng.range_usize(0, rows) as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_dimensions_return_zero_for_zero_bounds() {
        let (columns, rows) = grid_dimensions(Size::new(0.0, 0.0), GridPatternSpec::new());

        assert_eq!(columns, 0);
        assert_eq!(rows, 0);
    }

    #[test]
    fn stable_grid_cell_is_repeatable() {
        let first = stable_grid_cell(12, 3, 4, 10, 8);
        let second = stable_grid_cell(12, 3, 4, 10, 8);

        assert_eq!(first, second);
        assert!(first.x >= 0 && first.x < 10);
        assert!(first.y >= 0 && first.y < 8);
    }
}
