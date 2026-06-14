use super::border_track::RoundedRectTrack;
use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};
use iced::widget::canvas::{LineCap, Stroke};
use iced::widget::{button, canvas, container, stack, text};
use iced::{Border, Color, Element, Length, Padding, Point, Rectangle, Renderer, Shadow, Size};
use iced_shadcn::{DecorativeSurfaceProps, Theme, decorative_surface};
use std::time::Duration;

const BORDER_WIDTH: f32 = 1.0;
const BEAM_STROKE_WIDTH: f32 = 1.0;
const BEAM_SEGMENTS: usize = 180;
const TEXT_MASK_STEP: f32 = 1.5;
/// Magic UI `linear-gradient(-75deg, …)`.
const SHINE_CSS_ANGLE: f32 = -75.0;
const SHINE_BAND_START: f32 = 0.20;
const SHINE_BAND_END: f32 = 0.30;

#[derive(Clone, Copy, Debug)]
pub struct ShinyButtonProps {
    pub radius: f32,
    pub duration_secs: f32,
    pub repeat_delay_secs: f32,
    pub text_color: Option<Color>,
    pub primary_color: Option<Color>,
    pub border_color: Option<Color>,
    pub background_color: Option<Color>,
    /// When false, text stays fully opaque (use during theme transitions).
    pub text_shine_enabled: bool,
    /// When false, border beam is hidden (use during theme transitions).
    pub border_shine_enabled: bool,
}

impl Default for ShinyButtonProps {
    fn default() -> Self {
        Self {
            radius: 8.0,
            duration_secs: 2.4,
            repeat_delay_secs: 1.0,
            text_color: None,
            primary_color: None,
            border_color: None,
            background_color: None,
            text_shine_enabled: true,
            border_shine_enabled: true,
        }
    }
}

impl ShinyButtonProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    pub fn primary_color(mut self, color: Color) -> Self {
        self.primary_color = Some(color);
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    pub fn text_shine_enabled(mut self, enabled: bool) -> Self {
        self.text_shine_enabled = enabled;
        self
    }

    pub fn border_shine_enabled(mut self, enabled: bool) -> Self {
        self.border_shine_enabled = enabled;
        self
    }
}

pub fn shiny_button<'a, Message: Clone + 'a>(
    label: &'a str,
    font_size: u16,
    on_press: Option<Message>,
    elapsed: Duration,
    props: ShinyButtonProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let resolved = resolve_shiny_button(props, theme);
    let x = shiny_x_fraction(elapsed, resolved.duration_secs, resolved.repeat_delay_secs);

    let label = shiny_button_label(
        label,
        font_size,
        resolved.text_color,
        resolved.background,
        x,
        props.text_shine_enabled,
    );

    let mut control = button(label).padding(Padding::from([8.0, 24.0])).style({
        let radius = resolved.radius;
        move |_, status| shiny_button_style(radius, status)
    });

    if let Some(message) = on_press {
        control = control.on_press(message);
    }

    let mut surface_props = DecorativeSurfaceProps::new()
        .clip(true)
        .radius(resolved.radius)
        .border_color(resolved.border_color)
        .border_width(BORDER_WIDTH);

    if resolved.background.a > f32::EPSILON {
        surface_props = surface_props.background(resolved.background);
    }

    let mut overlays = Vec::new();

    if props.border_shine_enabled {
        overlays.push(
            container(effect_canvas::<_, Message>(
                ShinyButtonBorderProgram {
                    x,
                    primary: resolved.primary,
                    radius: resolved.radius,
                },
                EffectCanvasProps::new()
                    .width(Length::Fill)
                    .height(Length::Fill),
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
        );
    }

    decorative_surface(control, Vec::new(), overlays, surface_props, theme)
}

fn shiny_button_label<'a, Message: 'a>(
    label: &'a str,
    font_size: u16,
    text_color: Color,
    background: Color,
    x: f32,
    text_shine_enabled: bool,
) -> Element<'a, Message> {
    let size = u32::from(font_size.max(1));
    let label = text(label)
        .size(size)
        .style(move |_| iced::widget::text::Style {
            color: Some(text_color),
        });

    if !text_shine_enabled || background.a <= f32::EPSILON {
        return label.into();
    }

    stack![
        label,
        container(effect_canvas::<_, Message>(
            ShinyButtonTextMaskProgram { x, background },
            EffectCanvasProps::new()
                .width(Length::Fill)
                .height(Length::Fill),
        ))
        .width(Length::Fill)
        .height(Length::Fill),
    ]
    .width(Length::Shrink)
    .height(Length::Shrink)
    .into()
}

/// Diagonal `maskImage` slit over the label bounds — paints button background inside the band.
struct ShinyButtonTextMaskProgram {
    x: f32,
    background: Color,
}

impl<Message> EffectProgram<Message> for ShinyButtonTextMaskProgram {
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
        let width = bounds.width.max(1.0);
        let height = bounds.height.max(1.0);
        let step = TEXT_MASK_STEP;

        let mut y = 0.0;
        while y < height {
            let mut x = 0.0;
            while x < width {
                let nx = (x + step * 0.5) / width;
                let ny = (y + step * 0.5) / height;
                let projection = shine_projection(nx, ny);
                let alpha = text_mask_alpha(projection, self.x);

                if alpha > f32::EPSILON {
                    frame.fill_rectangle(
                        Point::new(x, y),
                        Size::new(step, step),
                        Color {
                            a: self.background.a * alpha,
                            ..self.background
                        },
                    );
                }

                x += step;
            }
            y += step;
        }

        vec![frame.into_geometry()]
    }
}

fn shiny_button_style(radius: f32, status: button::Status) -> button::Style {
    let mut style = button::Style {
        background: None,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius.into(),
        },
        text_color: Color::BLACK,
        shadow: Shadow::default(),
        snap: false,
    };

    match status {
        button::Status::Hovered => {
            style.shadow = Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.06),
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 8.0,
            };
        }
        button::Status::Pressed | button::Status::Active => {}
        button::Status::Disabled => {
            style.text_color = Color::from_rgba(0.0, 0.0, 0.0, 0.35);
        }
    }

    style
}

fn resolve_shiny_button(props: ShinyButtonProps, theme: &Theme) -> ResolvedShinyButton {
    let dark_mode = theme.variant().is_dark();

    ResolvedShinyButton {
        radius: props.radius,
        duration_secs: props.duration_secs,
        repeat_delay_secs: props.repeat_delay_secs,
        text_color: props
            .text_color
            .unwrap_or_else(|| shiny_text_color_for_mode(dark_mode)),
        primary: props.primary_color.unwrap_or(theme.palette.primary),
        border_color: props.border_color.unwrap_or(theme.palette.border),
        background: props.background_color.unwrap_or(if dark_mode {
            Color::TRANSPARENT
        } else {
            theme.palette.background
        }),
    }
}

#[derive(Clone, Copy, Debug)]
struct ResolvedShinyButton {
    radius: f32,
    duration_secs: f32,
    repeat_delay_secs: f32,
    text_color: Color,
    primary: Color,
    border_color: Color,
    background: Color,
}

pub fn shiny_text_color_for_mode(dark_mode: bool) -> Color {
    if dark_mode {
        Color::from_rgba(1.0, 1.0, 1.0, 0.90)
    } else {
        Color::from_rgba(0.0, 0.0, 0.0, 0.65)
    }
}

struct ShinyButtonBorderProgram {
    x: f32,
    primary: Color,
    radius: f32,
}

impl<Message> EffectProgram<Message> for ShinyButtonBorderProgram {
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
        let track = RoundedRectTrack::new(bounds.size(), self.radius);
        let size = bounds.size();
        let last = BEAM_SEGMENTS.saturating_sub(1);

        for index in 0..BEAM_SEGMENTS {
            let t0 = index as f32 / BEAM_SEGMENTS as f32;
            let t1 = (index + 1) as f32 / BEAM_SEGMENTS as f32;
            let p0 = track.sample(t0).point;
            let p1 = track.sample(t1).point;
            let mid = track.sample((t0 + t1) * 0.5);
            let projection = shine_projection(
                mid.point.x / size.width.max(1.0),
                mid.point.y / size.height.max(1.0),
            );
            let opacity = border_shine_opacity(projection, self.x);

            if opacity <= f32::EPSILON {
                continue;
            }

            let segment = canvas::Path::line(p0, p1);
            let line_cap = if index == 0 || index == last {
                LineCap::Round
            } else {
                LineCap::Butt
            };

            frame.stroke(
                &segment,
                Stroke::default()
                    .with_width(BEAM_STROKE_WIDTH)
                    .with_line_cap(line_cap)
                    .with_color(Color {
                        a: opacity,
                        ..self.primary
                    }),
            );
        }

        vec![frame.into_geometry()]
    }
}

/// Map a point to CSS gradient stop space (0 = bottom-right, 1 = top-left for -75deg).
pub(crate) fn shine_projection(nx: f32, ny: f32) -> f32 {
    let rad = SHINE_CSS_ANGLE.to_radians();
    let raw = nx * rad.sin() + ny * (-rad.cos());
    let min = raw_corner_projection(1.0, 1.0);
    let max = raw_corner_projection(0.0, 0.0);
    let span = (max - min).max(f32::EPSILON);
    (raw - min) / span
}

fn raw_corner_projection(nx: f32, ny: f32) -> f32 {
    let rad = SHINE_CSS_ANGLE.to_radians();
    nx * rad.sin() + ny * (-rad.cos())
}

fn text_mask_alpha(projection: f32, x: f32) -> f32 {
    if projection >= x + SHINE_BAND_START && projection <= x + SHINE_BAND_END {
        let center = x + (SHINE_BAND_START + SHINE_BAND_END) * 0.5;
        let half = (SHINE_BAND_END - SHINE_BAND_START) * 0.5;
        let distance = ((projection - center).abs() / half).clamp(0.0, 1.0);
        1.0 - distance * distance
    } else {
        0.0
    }
}

fn shiny_x_fraction(elapsed: Duration, duration_secs: f32, repeat_delay_secs: f32) -> f32 {
    let duration = duration_secs.max(0.01);
    let delay = repeat_delay_secs.max(0.0);
    let cycle = duration + delay;
    let time = elapsed.as_secs_f32().rem_euclid(cycle);

    if time > duration {
        return -1.0;
    }

    let progress = (time / duration).clamp(0.0, 1.0);
    let eased = spring_like_ease(progress);
    1.0 - eased * 2.0
}

fn spring_like_ease(progress: f32) -> f32 {
    let t = progress.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

fn border_shine_opacity(projection: f32, x: f32) -> f32 {
    sample_stops(
        &[
            (x + SHINE_BAND_START, 0.10),
            (x + 0.25, 0.50),
            (x + 1.00, 0.10),
        ],
        projection,
    )
}

fn sample_stops(stops: &[(f32, f32)], position: f32) -> f32 {
    if stops.is_empty() {
        return 0.0;
    }

    if position <= stops[0].0 {
        return stops[0].1;
    }

    for window in stops.windows(2) {
        let (left_pos, left_value) = window[0];
        let (right_pos, right_value) = window[1];

        if position <= right_pos {
            let span = (right_pos - left_pos).max(f32::EPSILON);
            let factor = (position - left_pos) / span;
            return left_value + (right_value - left_value) * factor;
        }
    }

    stops.last().map(|(_, value)| *value).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shiny_button_defaults_are_stable() {
        let props = ShinyButtonProps::new();

        assert_eq!(props.duration_secs, 2.4);
        assert_eq!(props.repeat_delay_secs, 1.0);
        assert_eq!(props.radius, 8.0);
    }

    #[test]
    fn shine_projection_spans_unit_square() {
        let top_left = shine_projection(0.0, 0.0);
        let bottom_right = shine_projection(1.0, 1.0);

        assert!((top_left - 1.0).abs() < 0.01);
        assert!(bottom_right.abs() < 0.01);
    }

    #[test]
    fn shiny_x_fraction_sweeps_from_right_to_left() {
        let start = shiny_x_fraction(Duration::from_secs(0), 2.4, 1.0);
        let mid = shiny_x_fraction(Duration::from_secs_f32(1.2), 2.4, 1.0);
        let end = shiny_x_fraction(Duration::from_secs_f32(2.4), 2.4, 1.0);

        assert!((start - 1.0).abs() < 0.001);
        assert!(mid < 0.2 && mid > -0.2);
        assert!((end - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn border_shine_matches_original_stops() {
        let p = 0.55;
        let before = border_shine_opacity(p, 0.8);
        let during = border_shine_opacity(p, 0.3);

        assert!((before - 0.10).abs() < 0.01);
        assert!(during > 0.30);
    }

    #[test]
    fn text_mask_only_covers_the_band() {
        assert_eq!(text_mask_alpha(0.35, 0.2), 0.0);
        assert!(text_mask_alpha(0.45, 0.2) > 0.0);
    }
}
