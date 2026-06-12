use crate::primitives::{
    EffectCanvasProps, EffectProgram, StableRng, effect_canvas, image_grid_cells,
};
use iced::time::Instant;
use iced::widget::image;
use iced::{Color, Element, Length, Point, Rectangle, Renderer};
use iced_shadcn::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelGrid {
    pub rows: u16,
    pub cols: u16,
}

impl PixelGrid {
    pub fn new(rows: u16, cols: u16) -> Self {
        Self {
            rows: rows.max(1),
            cols: cols.max(1),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PixelGridPreset {
    SixByFour,
    #[default]
    EightByEight,
    EightByThree,
    FourBySix,
    ThreeByEight,
}

impl PixelGridPreset {
    fn resolve(self) -> PixelGrid {
        match self {
            PixelGridPreset::SixByFour => PixelGrid::new(4, 6),
            PixelGridPreset::EightByEight => PixelGrid::new(8, 8),
            PixelGridPreset::EightByThree => PixelGrid::new(3, 8),
            PixelGridPreset::FourBySix => PixelGrid::new(6, 4),
            PixelGridPreset::ThreeByEight => PixelGrid::new(8, 3),
        }
    }
}

#[derive(Clone)]
pub struct PixelImageProps {
    pub handle: image::Handle,
    pub width: Length,
    pub height: Length,
    pub grid: PixelGridPreset,
    pub custom_grid: Option<PixelGrid>,
    pub grayscale_animation: bool,
    pub pixel_fade_in_duration_ms: u64,
    pub max_animation_delay_ms: u64,
    pub color_reveal_delay_ms: u64,
    pub seed: u64,
}

impl PixelImageProps {
    pub fn new(handle: image::Handle) -> Self {
        Self {
            handle,
            width: Length::Fixed(384.0),
            height: Length::Fixed(384.0),
            grid: PixelGridPreset::SixByFour,
            custom_grid: None,
            grayscale_animation: true,
            pixel_fade_in_duration_ms: 1000,
            max_animation_delay_ms: 1200,
            color_reveal_delay_ms: 1300,
            seed: 37,
        }
    }

    pub fn resolved_grid(&self) -> PixelGrid {
        self.custom_grid.unwrap_or_else(|| self.grid.resolve())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PixelPiece {
    row: u16,
    col: u16,
    delay_ms: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PixelImageFingerprint {
    grid: PixelGrid,
    grayscale_animation: bool,
    pixel_fade_in_duration_ms: u64,
    max_animation_delay_ms: u64,
    color_reveal_delay_ms: u64,
    seed: u64,
}

impl From<&PixelImageProps> for PixelImageFingerprint {
    fn from(props: &PixelImageProps) -> Self {
        Self {
            grid: props.resolved_grid(),
            grayscale_animation: props.grayscale_animation,
            pixel_fade_in_duration_ms: props.pixel_fade_in_duration_ms,
            max_animation_delay_ms: props.max_animation_delay_ms,
            color_reveal_delay_ms: props.color_reveal_delay_ms,
            seed: props.seed,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PixelImageState {
    started_at: Option<Instant>,
    pieces: Box<[PixelPiece]>,
    fingerprint: PixelImageFingerprint,
}

impl PixelImageState {
    pub fn new(props: &PixelImageProps) -> Self {
        Self {
            started_at: None,
            pieces: build_pieces(
                props.resolved_grid(),
                props.max_animation_delay_ms,
                props.seed,
            ),
            fingerprint: PixelImageFingerprint::from(props),
        }
    }

    pub fn tick(&mut self, now: Instant, props: &PixelImageProps) {
        let fingerprint = PixelImageFingerprint::from(props);
        if self.fingerprint != fingerprint {
            *self = Self::new(props);
        }

        let _ = self.started_at.get_or_insert(now);
    }

    fn elapsed_ms(&self, now: Instant) -> u64 {
        self.started_at
            .map(|started| now.saturating_duration_since(started).as_millis() as u64)
            .unwrap_or(0)
    }

    fn pieces(&self) -> &[PixelPiece] {
        &self.pieces
    }
}

pub fn pixel_image<'a, Message: 'a>(
    state: &'a PixelImageState,
    now: Instant,
    props: &PixelImageProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let width = props.width;
    let height = props.height;
    effect_canvas::<_, Message>(
        PixelImageProgram {
            handle: props.handle.clone(),
            pieces: state.pieces(),
            grid: props.resolved_grid(),
            elapsed_ms: state.elapsed_ms(now),
            props: PixelImageProgramProps {
                pixel_fade_in_duration_ms: props.pixel_fade_in_duration_ms,
                grayscale_animation: props.grayscale_animation,
                color_reveal_delay_ms: props.color_reveal_delay_ms,
                background: theme.palette.background,
            },
        },
        EffectCanvasProps::new().width(width).height(height),
    )
    .into()
}

#[derive(Clone, Copy)]
struct PixelImageProgramProps {
    pixel_fade_in_duration_ms: u64,
    grayscale_animation: bool,
    color_reveal_delay_ms: u64,
    background: Color,
}

struct PixelImageProgram<'a> {
    handle: image::Handle,
    pieces: &'a [PixelPiece],
    grid: PixelGrid,
    elapsed_ms: u64,
    props: PixelImageProgramProps,
}

impl<Message> EffectProgram<Message> for PixelImageProgram<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<iced::widget::canvas::Geometry> {
        let mut frame = iced::widget::canvas::Frame::new(renderer, bounds.size());
        let cell_width = bounds.width / self.grid.cols as f32;
        let cell_height = bounds.height / self.grid.rows as f32;
        let color_mix = if self.props.grayscale_animation {
            (1.0 - ((self
                .elapsed_ms
                .saturating_sub(self.props.color_reveal_delay_ms) as f32)
                / self.props.pixel_fade_in_duration_ms.max(1) as f32))
                .clamp(0.0, 1.0)
        } else {
            0.0
        };

        for piece in self.pieces {
            let progress = ((self.elapsed_ms.saturating_sub(piece.delay_ms as u64) as f32)
                / self.props.pixel_fade_in_duration_ms.max(1) as f32)
                .clamp(0.0, 1.0);

            if progress <= 0.0 {
                continue;
            }

            let region = Rectangle::new(
                Point::new(
                    piece.col as f32 * cell_width,
                    piece.row as f32 * cell_height,
                ),
                iced::Size::new(cell_width, cell_height),
            );

            frame.with_clip(region, |frame| {
                frame.draw_image(Rectangle::with_size(bounds.size()), &self.handle);

                let veil = 1.0 - progress;
                if veil > 0.0 {
                    frame.fill(
                        &iced::widget::canvas::Path::rectangle(region.position(), region.size()),
                        Color {
                            a: veil,
                            ..self.props.background
                        },
                    );
                }

                if color_mix > 0.0 {
                    let gray = grayscale_overlay(color_mix);
                    frame.fill(
                        &iced::widget::canvas::Path::rectangle(region.position(), region.size()),
                        gray,
                    );
                }
            });
        }

        vec![frame.into_geometry()]
    }
}

fn build_pieces(grid: PixelGrid, max_delay_ms: u64, seed: u64) -> Box<[PixelPiece]> {
    let mut rng = StableRng::new(seed);
    let mut pieces = Vec::with_capacity(grid.rows as usize * grid.cols as usize);
    for cell in image_grid_cells(grid.rows, grid.cols).iter().copied() {
        pieces.push(PixelPiece {
            row: cell.row,
            col: cell.col,
            delay_ms: (rng.next_f32() * max_delay_ms as f32).round() as u16,
        });
    }
    pieces.into_boxed_slice()
}

fn grayscale_overlay(strength: f32) -> Color {
    Color {
        r: 0.4,
        g: 0.4,
        b: 0.4,
        a: 0.55 * strength.clamp(0.0, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_image_uses_custom_grid_when_present() {
        let props = PixelImageProps {
            handle: image::Handle::from_rgba(1, 1, vec![255, 255, 255, 255]),
            custom_grid: Some(PixelGrid::new(4, 6)),
            ..PixelImageProps::new(image::Handle::from_rgba(1, 1, vec![255, 255, 255, 255]))
        };

        assert_eq!(props.resolved_grid(), PixelGrid::new(4, 6));
    }

    #[test]
    fn pixel_image_defaults_match_docs() {
        let props = PixelImageProps::new(image::Handle::from_rgba(1, 1, vec![255, 255, 255, 255]));

        assert_eq!(props.resolved_grid(), PixelGrid::new(4, 6));
        assert_eq!(props.color_reveal_delay_ms, 1300);
        assert_eq!(props.pixel_fade_in_duration_ms, 1000);
        assert_eq!(props.max_animation_delay_ms, 1200);
    }

    #[test]
    fn pixel_piece_delays_are_stable() {
        let first = build_pieces(PixelGrid::new(4, 6), 1200, 42);
        let second = build_pieces(PixelGrid::new(4, 6), 1200, 42);

        assert_eq!(first, second);
    }

    #[test]
    fn pixel_pieces_follow_shared_grid_geometry() {
        let pieces = build_pieces(PixelGrid::new(4, 6), 1200, 42);

        assert_eq!(pieces.len(), 24);
        assert_eq!(pieces[0].row, 0);
        assert_eq!(pieces[0].col, 0);
        assert_eq!(pieces[23].row, 3);
        assert_eq!(pieces[23].col, 5);
    }
}
