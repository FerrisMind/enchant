use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas, stable_noise};
use iced::widget::canvas::{self, Stroke};
use iced::widget::container;
use iced::{Alignment, Color, Element, Length, Padding, Point, Rectangle, Renderer};
use iced_shadcn::{DecorativeSurfaceProps, Theme, decorative_surface};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WarpBackgroundProps {
    pub perspective: f32,
    pub beams_per_side: usize,
    pub beam_size: f32,
    pub beam_delay_max: f32,
    pub beam_delay_min: f32,
    pub beam_duration: f32,
    pub grid_color: Option<Color>,
    pub seed: u64,
    pub padding: Padding,
    pub radius: f32,
    pub width: Length,
    pub height: Length,
    pub background: Option<Color>,
}

impl Default for WarpBackgroundProps {
    fn default() -> Self {
        Self {
            perspective: 100.0,
            beams_per_side: 3,
            beam_size: 5.0,
            beam_delay_max: 3.0,
            beam_delay_min: 0.0,
            beam_duration: 3.0,
            grid_color: None,
            seed: 19,
            padding: Padding::from(80.0),
            radius: 8.0,
            width: Length::Shrink,
            height: Length::Shrink,
            background: None,
        }
    }
}

impl WarpBackgroundProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn warp_background<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    elapsed: Duration,
    props: WarpBackgroundProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let grid_color = props.grid_color.unwrap_or(theme.palette.border);

    let mut surface_props = DecorativeSurfaceProps::new()
        .width(props.width)
        .height(props.height)
        .padding(props.padding)
        .clip(true)
        .radius(props.radius)
        .border_color(grid_color)
        .border_width(1.0);

    if let Some(background) = props.background {
        surface_props = surface_props.background(background);
    }

    decorative_surface(
        container(content.into())
            .align_x(Alignment::Center)
            .align_y(Alignment::Center),
        vec![
            container(effect_canvas::<_, Message>(
                WarpBackgroundProgram {
                    elapsed,
                    props,
                    grid_color,
                },
                EffectCanvasProps::new()
                    .width(Length::Fill)
                    .height(Length::Fill),
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
        ],
        Vec::new(),
        surface_props,
        theme,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Wall {
    Top,
    Right,
    Bottom,
    Left,
}

struct BeamSpec {
    cell_index: usize,
    delay: f32,
    hue: f32,
    aspect_ratio: f32,
}

struct WarpBackgroundProgram {
    elapsed: Duration,
    props: WarpBackgroundProps,
    grid_color: Color,
}

impl<Message> EffectProgram<Message> for WarpBackgroundProgram {
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
        let perspective = self.props.perspective.max(10.0);
        let max_dim = bounds.width.max(bounds.height);
        let beam_size = self.props.beam_size.max(1.0);
        let cell = max_dim * (beam_size / 100.0);
        let depth_extent = max_dim;
        let beam_size_frac = (beam_size / 100.0).max(0.01);

        for wall in [Wall::Top, Wall::Right, Wall::Bottom, Wall::Left] {
            draw_wall_grid(
                &mut frame,
                wall,
                bounds,
                depth_extent,
                perspective,
                cell,
                beam_size_frac,
                self.grid_color,
            );
        }

        let time = self.elapsed.as_secs_f32();
        for (side, wall) in [
            (0_u64, Wall::Top),
            (1, Wall::Right),
            (2, Wall::Bottom),
            (3, Wall::Left),
        ] {
            let beams = generate_beams(&self.props, side);
            for (beam_index, beam) in beams.iter().enumerate() {
                draw_beam(
                    &mut frame,
                    wall,
                    bounds,
                    depth_extent,
                    perspective,
                    beam,
                    time,
                    self.props.beam_duration.max(0.2),
                    beam_size_frac,
                    self.props.beam_size,
                    side,
                    beam_index,
                    self.props.seed,
                );
            }
        }

        vec![frame.into_geometry()]
    }
}

fn generate_beams(props: &WarpBackgroundProps, side: u64) -> Vec<BeamSpec> {
    let beam_size = props.beam_size.max(1.0);
    let cells_per_side = (100.0 / beam_size).floor() as usize;
    let beams_per_side = props.beams_per_side.max(1);
    let step = cells_per_side as f32 / beams_per_side as f32;

    (0..beams_per_side)
        .map(|index| {
            let cell_index = (index as f32 * step).floor() as usize;
            let delay = props.beam_delay_min
                + (props.beam_delay_max - props.beam_delay_min)
                    * stable_noise(props.seed + side + 17, index as u64 * 3);
            let hue = stable_noise(props.seed + side + 31, index as u64 * 7) * 360.0;
            let aspect_ratio =
                stable_noise(props.seed + side + 47, index as u64 * 11).mul_add(10.0, 1.0);
            BeamSpec {
                cell_index,
                delay,
                hue,
                aspect_ratio,
            }
        })
        .collect()
}

fn wall_point_3d(
    wall: Wall,
    bounds: Rectangle,
    depth_extent: f32,
    u: f32,
    depth: f32,
    clamp_depth: bool,
) -> (f32, f32, f32) {
    let u = u.clamp(0.0, 1.0);
    let depth = if clamp_depth {
        depth.clamp(0.0, 1.0)
    } else {
        depth
    };
    let z = depth * depth_extent;

    match wall {
        Wall::Top => (u * bounds.width, 0.0, z),
        Wall::Bottom => (u * bounds.width, bounds.height, z),
        Wall::Left => (0.0, u * bounds.height, z),
        Wall::Right => (bounds.width, u * bounds.height, z),
    }
}

fn project_point(point: (f32, f32, f32), bounds: Rectangle, perspective: f32) -> Point {
    let center = Point::new(bounds.width * 0.5, bounds.height * 0.5);
    let scale = perspective / (perspective + point.2);
    Point::new(
        center.x + (point.0 - center.x) * scale,
        center.y + (point.1 - center.y) * scale,
    )
}

fn wall_point(
    wall: Wall,
    bounds: Rectangle,
    depth_extent: f32,
    perspective: f32,
    u: f32,
    depth: f32,
) -> Point {
    project_point(
        wall_point_3d(wall, bounds, depth_extent, u, depth, true),
        bounds,
        perspective,
    )
}

fn wall_point_beam(
    wall: Wall,
    bounds: Rectangle,
    depth_extent: f32,
    perspective: f32,
    u: f32,
    depth: f32,
) -> Point {
    project_point(
        wall_point_3d(wall, bounds, depth_extent, u, depth, false),
        bounds,
        perspective,
    )
}

#[allow(clippy::too_many_arguments)]
fn draw_wall_grid(
    frame: &mut canvas::Frame,
    wall: Wall,
    bounds: Rectangle,
    depth_extent: f32,
    perspective: f32,
    cell: f32,
    beam_size_frac: f32,
    grid_color: Color,
) {
    let stroke = Stroke {
        width: 1.0,
        style: canvas::stroke::Style::Solid(grid_color),
        ..Stroke::default()
    };

    let mut z = 0.0;
    while z <= depth_extent + 0.5 {
        let depth = (z / depth_extent).min(1.0);
        let start = wall_point(wall, bounds, depth_extent, perspective, 0.0, depth);
        let end = wall_point(wall, bounds, depth_extent, perspective, 1.0, depth);
        frame.stroke(&canvas::Path::line(start, end), stroke);
        z += cell;
    }

    let mut u = 0.0;
    while u <= 1.0 + 0.001 {
        let near = wall_point(wall, bounds, depth_extent, perspective, u, 0.0);
        let far = wall_point(wall, bounds, depth_extent, perspective, u, 1.0);
        frame.stroke(&canvas::Path::line(near, far), stroke);
        u += beam_size_frac;
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_beam(
    frame: &mut canvas::Frame,
    wall: Wall,
    bounds: Rectangle,
    depth_extent: f32,
    perspective: f32,
    beam: &BeamSpec,
    time: f32,
    duration: f32,
    beam_size_frac: f32,
    beam_size: f32,
    side: u64,
    beam_index: usize,
    seed: u64,
) {
    let cycle = duration + beam.delay;
    let local = time.rem_euclid(cycle);
    if local < beam.delay {
        return;
    }

    let progress = ((local - beam.delay) / duration).clamp(0.0, 1.0);

    let u0 = beam.cell_index as f32 * beam_size_frac;
    let u1 = (u0 + beam_size_frac).min(1.0);

    // CSS: `aspect-[1/var(--aspect-ratio)]` → height = width × ar (larger ar = longer streak).
    let inline_span = match wall {
        Wall::Top | Wall::Bottom => bounds.width,
        Wall::Left | Wall::Right => bounds.height,
    };
    let beam_width = inline_span * (beam_size / 100.0);
    let beam_height = beam_width * beam.aspect_ratio.max(1.0);
    let beam_depth = (beam_height / depth_extent).clamp(0.06, 0.45);

    // depth: 0 = outer edge (user), 1 = vanishing point (behind card).
    // CSS: `y: 100cqmax → -100%` with `linear-gradient(hsl, transparent)` (bright at top).
    // Bright edge leads toward the user; transparent tail extends toward the center.
    let travel_range = 1.0 + beam_depth + 0.35;
    let head = 1.0 - progress * travel_range;
    let tail = head + beam_depth;

    if head <= 0.0 && tail <= 0.0 {
        return;
    }
    if head >= 1.0 + beam_depth {
        return;
    }

    let hue = beam.hue + stable_noise(seed + side, beam_index as u64 * 19) * 10.0;
    let depth_span = (tail - head).max(0.001);

    const SLICES: usize = 20;
    for slice in 0..SLICES {
        let slice_start = head + depth_span * (slice as f32 / SLICES as f32);
        let slice_end = head + depth_span * ((slice as f32 + 1.0) / SLICES as f32);

        // Clip to the visible tunnel — avoids full-length projection artifacts.
        let draw_start = slice_start.max(0.0);
        let draw_end = slice_end.min(1.0);
        if draw_end <= draw_start + 0.001 {
            continue;
        }

        let alpha = beam_slice_alpha(draw_start, draw_end, head, tail);
        if alpha <= 0.01 {
            continue;
        }

        let quad = [
            wall_point_beam(wall, bounds, depth_extent, perspective, u0, draw_start),
            wall_point_beam(wall, bounds, depth_extent, perspective, u1, draw_start),
            wall_point_beam(wall, bounds, depth_extent, perspective, u1, draw_end),
            wall_point_beam(wall, bounds, depth_extent, perspective, u0, draw_end),
        ];

        let path = canvas::Path::new(|builder| {
            builder.move_to(quad[0]);
            builder.line_to(quad[1]);
            builder.line_to(quad[2]);
            builder.line_to(quad[3]);
            builder.close();
        });

        frame.fill(
            &path,
            canvas::Fill::from(hsla_to_color(hue, 0.8, 0.6, alpha)),
        );
    }
}

/// Bright at the user-leading edge (`head`), fading toward the tunnel center (`tail`).
fn beam_slice_alpha(slice_start: f32, slice_end: f32, head: f32, tail: f32) -> f32 {
    let span = (tail - head).max(0.001);
    let slice_mid = (slice_start + slice_end) * 0.5;
    let t = ((slice_mid - head) / span).clamp(0.0, 1.0);
    // t=0 → user side (head), t=1 → center side (tail)
    (1.0 - smoothstep(0.0, 1.0, t)).powf(0.55) * 0.82
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    if edge0 >= edge1 {
        return if x >= edge1 { 1.0 } else { 0.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn hsla_to_color(h: f32, s: f32, l: f32, a: f32) -> Color {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = l - c / 2.0;

    let (r1, g1, b1) = match h as i32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    Color::from_rgba(r1 + m, g1 + m, b1 + m, a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warp_background_defaults_match_docs() {
        let props = WarpBackgroundProps::new();

        assert_eq!(props.perspective, 100.0);
        assert_eq!(props.beams_per_side, 3);
        assert_eq!(props.beam_size, 5.0);
        assert_eq!(props.beam_duration, 3.0);
        assert_eq!(props.padding, Padding::from(80.0));
    }

    #[test]
    fn generate_beams_matches_magicui_spacing() {
        let props = WarpBackgroundProps::new();
        let beams = generate_beams(&props, 0);

        assert_eq!(beams.len(), 3);
        assert_eq!(beams[0].cell_index, 0);
        assert_eq!(beams[1].cell_index, 6);
        assert_eq!(beams[2].cell_index, 13);
    }

    #[test]
    fn beam_starts_at_center_with_tail_extending_inward() {
        let beam_depth = 0.2_f32;
        let progress = 0.0_f32;
        let travel_range = 1.0 + beam_depth + 0.35;
        let head = 1.0 - progress * travel_range;
        let tail = head + beam_depth;

        assert!((head - 1.0).abs() < f32::EPSILON);
        assert!((tail - (1.0 + beam_depth)).abs() < f32::EPSILON);
        assert!(tail > head);
    }

    #[test]
    fn beam_alpha_is_brightest_toward_user_leading_edge() {
        assert!(beam_slice_alpha(0.52, 0.56, 0.5, 0.7) > beam_slice_alpha(0.66, 0.70, 0.5, 0.7));
    }

    #[test]
    fn beam_depth_grows_with_aspect_ratio() {
        let bounds = Rectangle::new(Point::ORIGIN, iced::Size::new(480.0, 400.0));
        let depth_extent = 480.0;
        let beam_size = 5.0;
        let beam_size_frac = 0.05;

        let short = {
            let beam = BeamSpec {
                cell_index: 0,
                delay: 0.0,
                hue: 0.0,
                aspect_ratio: 1.0,
            };
            let inline = bounds.width;
            let height = inline * (beam_size / 100.0) * beam.aspect_ratio;
            (height / depth_extent).clamp(0.06, 0.45)
        };
        let long = {
            let beam = BeamSpec {
                cell_index: 0,
                delay: 0.0,
                hue: 0.0,
                aspect_ratio: 8.0,
            };
            let inline = bounds.width;
            let height = inline * (beam_size / 100.0) * beam.aspect_ratio;
            (height / depth_extent).clamp(0.06, 0.45)
        };

        assert!(long > short);
        let _ = beam_size_frac;
    }

    #[test]
    fn perspective_projection_pulls_depth_toward_center() {
        let bounds = Rectangle::new(Point::ORIGIN, iced::Size::new(400.0, 300.0));
        let near = wall_point(Wall::Top, bounds, 400.0, 100.0, 0.5, 0.0);
        let far = wall_point(Wall::Top, bounds, 400.0, 100.0, 0.5, 1.0);

        assert!(far.y > near.y);
        assert!((far.x - 200.0).abs() < 0.01);
    }
}
