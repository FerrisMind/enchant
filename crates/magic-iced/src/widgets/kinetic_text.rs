use crate::primitives::{
    AnimatedValue, EffectCanvasProps, EffectProgram, MotionSpec, effect_canvas,
};
use iced::advanced::graphics::text::{self as gfx_text, cosmic_text};
use iced::advanced::text::Alignment;
use iced::mouse;
use iced::time::Instant;
use iced::widget::canvas;
use iced::{Color, Element, Font, Length, Point, Rectangle, Renderer, Size, Vector};
use iced_shadcn::Theme;
use std::borrow::Cow;
use std::sync::OnceLock;
use twill::prelude::{Easing, TransitionDuration};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KineticTextRole {
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    P,
    Span,
}

impl KineticTextRole {
    fn default_font_size(self) -> u16 {
        match self {
            Self::H1 => 96,
            Self::H2 => 72,
            Self::H3 => 48,
            Self::H4 => 36,
            Self::H5 => 24,
            Self::H6 => 20,
            Self::P | Self::Span => 16,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KineticTextProps {
    pub role: KineticTextRole,
    pub font_size: Option<u16>,
    pub color: Option<Color>,
}

impl Default for KineticTextProps {
    fn default() -> Self {
        Self {
            role: KineticTextRole::H1,
            font_size: None,
            color: None,
        }
    }
}

impl KineticTextProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn role(mut self, role: KineticTextRole) -> Self {
        self.role = role;
        self
    }

    pub fn font_size(mut self, font_size: u16) -> Self {
        self.font_size = Some(font_size.max(1));
        self
    }
}

/// Retained for API compatibility; hover + animation live in the canvas state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KineticTextState {
    _private: (),
}

impl KineticTextState {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(clippy::unused_self)]
    pub fn set_hovered(&mut self, _index: Option<usize>) {}
}

const TRANSITION_MS: u16 = 400;
const GEIST_REGULAR: &[u8] = include_bytes!("../../assets/Geist/Geist-VariableFont_wght.ttf");
const GEIST_ITALIC: &[u8] = include_bytes!("../../assets/Geist/Geist-Italic-VariableFont_wght.ttf");
const GEIST_FONT: Font = Font::with_name("Geist");

fn kinetic_stroke_width(font_size: f32) -> f32 {
    font_size * 125.0 / 6_000.0
}

fn ensure_geist_fonts_loaded() {
    static GEIST_FONTS: OnceLock<()> = OnceLock::new();

    GEIST_FONTS.get_or_init(|| {
        let mut font_system = gfx_text::font_system().write().expect("Write font system");
        font_system.load_font(Cow::Borrowed(GEIST_REGULAR));
        font_system.load_font(Cow::Borrowed(GEIST_ITALIC));
    });
}

pub fn kinetic_text<'a, Message: 'a>(
    content: &'a str,
    _state: &KineticTextState,
    now: Instant,
    props: &KineticTextProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let font_size = props
        .font_size
        .unwrap_or(props.role.default_font_size())
        .max(1) as f32;
    let color = props.color.unwrap_or(theme.palette.foreground);
    let metrics = measure_kinetic_layout(content, font_size);

    effect_canvas::<_, Message>(
        KineticTextProgram {
            content: content.to_owned(),
            font_size,
            color,
            now,
            x_offset: metrics.x_offset,
            y_offset: metrics.y_offset,
        },
        EffectCanvasProps::new()
            .width(Length::Fixed(metrics.width.ceil().max(1.0)))
            .height(Length::Fixed(metrics.height.ceil().max(1.0))),
    )
    .into()
}

struct KineticTextProgram {
    content: String,
    font_size: f32,
    color: Color,
    now: Instant,
    x_offset: f32,
    y_offset: f32,
}

struct KineticLayoutMetrics {
    width: f32,
    height: f32,
    x_offset: f32,
    y_offset: f32,
}

struct KineticLetterLayout {
    buffer: cosmic_text::Buffer,
    box_start: f32,
    box_width: f32,
    glyph_x: f32,
    baseline_offset: f32,
}

struct KineticLayout {
    letters: Vec<KineticLetterLayout>,
    width: f32,
    min_y: f32,
    max_y: f32,
}

#[derive(Clone, Debug)]
struct LetterAnimation {
    padding: AnimatedValue,
    stroke: AnimatedValue,
    weight: AnimatedValue,
}

impl LetterAnimation {
    fn new() -> Self {
        let motion = kinetic_motion();
        Self {
            padding: AnimatedValue::with_spec(0.0, motion),
            stroke: AnimatedValue::with_spec(0.0, motion),
            weight: AnimatedValue::with_spec(300.0, motion),
        }
    }

    fn set_targets(&mut self, padding: f32, stroke: f32, weight: f32, now: Instant) {
        self.padding.transition(padding, now);
        self.stroke.transition(stroke, now);
        self.weight.transition(weight, now);
    }

    fn weight_at(&self, now: Instant) -> f32 {
        self.weight.value_at(now).clamp(100.0, 900.0)
    }
}

#[derive(Clone, Debug, Default)]
struct KineticCanvasState {
    letters: Vec<LetterAnimation>,
    hovered_index: Option<usize>,
    content_key: String,
}

impl KineticCanvasState {
    fn ensure_content(&mut self, content: &str) {
        if self.content_key == content {
            return;
        }

        self.content_key = content.to_owned();
        self.letters = content.chars().map(|_| LetterAnimation::new()).collect();
        self.hovered_index = None;
    }

    fn set_hovered(&mut self, index: Option<usize>, font_size: f32, now: Instant) {
        if self.hovered_index == index {
            return;
        }

        self.hovered_index = index;
        let hover_padding = font_size / 12.0;

        for (letter_index, animation) in self.letters.iter_mut().enumerate() {
            let padding = kinetic_padding_target(letter_index, index, hover_padding);
            let stroke = if index == Some(letter_index) {
                1.0
            } else {
                0.0
            };
            let weight = kinetic_target_weight(letter_index, index);
            animation.set_targets(padding, stroke, weight, now);
        }
    }
}

impl<Message> EffectProgram<Message> for KineticTextProgram {
    type State = KineticCanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Option<canvas::Action<Message>> {
        state.ensure_content(&self.content);

        match event {
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let position = cursor.position_in(bounds)?;
                let next = hit_test_letter(
                    &self.content,
                    self.font_size,
                    state,
                    self.now,
                    position.x,
                    self.x_offset,
                );
                if next != state.hovered_index {
                    state.set_hovered(next, self.font_size, self.now);
                    return Some(canvas::Action::request_redraw());
                }
            }
            canvas::Event::Mouse(mouse::Event::CursorLeft) if state.hovered_index.is_some() => {
                state.set_hovered(None, self.font_size, self.now);
                return Some(canvas::Action::request_redraw());
            }
            _ => {}
        }

        if state.letters.iter().any(|letter| {
            letter.padding.is_animating(self.now)
                || letter.stroke.is_animating(self.now)
                || letter.weight.is_animating(self.now)
        }) {
            return Some(canvas::Action::request_redraw());
        }

        None
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> mouse::Interaction {
        mouse::Interaction::Pointer
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let stroke_width = kinetic_stroke_width(self.font_size);
        let letter_count = self.content.chars().count();
        let padding = kinetic_padding_values(state, self.now, letter_count);
        let stroke_amounts: Vec<f32> = (0..letter_count)
            .map(|index| {
                state
                    .letters
                    .get(index)
                    .map(|animation| animation.stroke.value_at(self.now))
                    .unwrap_or(0.0)
            })
            .collect();
        let weights = kinetic_weights(state, self.now, letter_count);
        let layout = build_kinetic_layout(&self.content, &weights, &padding, self.font_size);

        draw_layout_glyphs(
            &mut frame,
            &layout,
            &stroke_amounts,
            self.color,
            stroke_width,
            Vector::new(self.x_offset, self.y_offset),
        );

        vec![frame.into_geometry()]
    }
}

fn kinetic_motion() -> MotionSpec {
    MotionSpec::new()
        .duration(TransitionDuration::CustomMs(TRANSITION_MS))
        .easing(Easing::InOut)
}

fn draw_layout_glyphs(
    frame: &mut canvas::Frame,
    layout: &KineticLayout,
    stroke_amounts: &[f32],
    color: Color,
    stroke_width: f32,
    layout_offset: Vector,
) {
    let mut swash_cache = cosmic_text::SwashCache::new();
    let mut outline_context = swash::scale::ScaleContext::new();
    let mut font_system = gfx_text::font_system().write().expect("Write font system");

    for (letter_index, letter) in layout.letters.iter().enumerate() {
        let stroke_amount = stroke_amounts.get(letter_index).copied().unwrap_or(0.0);

        for run in letter.buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                let physical_glyph = glyph.physical((0.0, 0.0), 1.0);
                let start_x = letter.glyph_x + glyph.x + glyph.x_offset;
                let start_y = letter.baseline_offset + glyph.y_offset + run.line_y;
                let offset = Vector::new(start_x, start_y) + layout_offset;

                if let Some(commands) = variable_outline_commands(
                    font_system.raw(),
                    &mut outline_context,
                    physical_glyph.cache_key,
                ) {
                    let path = canvas::Path::new(|path| {
                        for command in commands {
                            match command {
                                cosmic_text::Command::MoveTo(point) => {
                                    path.move_to(Point::new(point.x, -point.y) + offset);
                                }
                                cosmic_text::Command::LineTo(point) => {
                                    path.line_to(Point::new(point.x, -point.y) + offset);
                                }
                                cosmic_text::Command::CurveTo(control_a, control_b, to) => {
                                    path.bezier_curve_to(
                                        Point::new(control_a.x, -control_a.y) + offset,
                                        Point::new(control_b.x, -control_b.y) + offset,
                                        Point::new(to.x, -to.y) + offset,
                                    );
                                }
                                cosmic_text::Command::QuadTo(control, to) => {
                                    path.quadratic_curve_to(
                                        Point::new(control.x, -control.y) + offset,
                                        Point::new(to.x, -to.y) + offset,
                                    );
                                }
                                cosmic_text::Command::Close => {
                                    path.close();
                                }
                            }
                        }
                    });

                    frame.fill(&path, color);

                    if stroke_amount > f32::EPSILON {
                        frame.stroke(
                            &path,
                            canvas::Stroke::default()
                                .with_width(stroke_width * (1.0 + stroke_amount))
                                .with_color(Color {
                                    a: color.a * stroke_amount,
                                    ..color
                                }),
                        );
                    }
                } else {
                    let [r, g, b, a] = color.into_rgba8();

                    swash_cache.with_pixels(
                        font_system.raw(),
                        physical_glyph.cache_key,
                        cosmic_text::Color::rgba(r, g, b, a),
                        |x, y, pixel| {
                            frame.fill(
                                &canvas::Path::rectangle(
                                    Point::new(x as f32, y as f32) + offset,
                                    Size::new(1.0, 1.0),
                                ),
                                Color::from_rgba8(
                                    pixel.r(),
                                    pixel.g(),
                                    pixel.b(),
                                    pixel.a() as f32 / 255.0,
                                ),
                            );
                        },
                    );
                }
            }
        }
    }
}

// Adapted from cosmic-text (MIT OR Apache-2.0) to preserve the `wght` axis.
fn variable_outline_commands(
    font_system: &mut cosmic_text::FontSystem,
    outline_context: &mut swash::scale::ScaleContext,
    cache_key: cosmic_text::CacheKey,
) -> Option<Box<[cosmic_text::Command]>> {
    use swash::zeno::PathData as _;

    let font = font_system.get_font(cache_key.font_id, cache_key.font_weight)?;
    let variable_weight = font
        .as_swash()
        .variations()
        .find_by_tag(swash::Tag::from_be_bytes(*b"wght"));
    let mut scaler = outline_context
        .builder(font.as_swash())
        .size(f32::from_bits(cache_key.font_size_bits))
        .hint(
            !cache_key
                .flags
                .contains(cosmic_text::CacheKeyFlags::DISABLE_HINTING),
        );

    if let Some(variation) = variable_weight {
        scaler = scaler.variations(std::iter::once(swash::Setting {
            tag: swash::Tag::from_be_bytes(*b"wght"),
            value: f32::from(cache_key.font_weight.0)
                .clamp(variation.min_value(), variation.max_value()),
        }));
    }

    let mut scaler = scaler.build();
    let mut outline = scaler
        .scale_outline(cache_key.glyph_id)
        .or_else(|| scaler.scale_color_outline(cache_key.glyph_id))?;

    if cache_key
        .flags
        .contains(cosmic_text::CacheKeyFlags::FAKE_ITALIC)
    {
        outline.transform(&swash::zeno::Transform::skew(
            swash::zeno::Angle::from_degrees(14.0),
            swash::zeno::Angle::from_degrees(0.0),
        ));
    }

    Some(outline.path().commands().collect())
}

fn hit_test_letter(
    content: &str,
    font_size: f32,
    state: &KineticCanvasState,
    now: Instant,
    x: f32,
    x_offset: f32,
) -> Option<usize> {
    let letter_count = content.chars().count();
    let padding = kinetic_padding_values(state, now, letter_count);
    let weights = kinetic_weights(state, now, letter_count);
    let layout = build_kinetic_layout(content, &weights, &padding, font_size);
    let x = x - x_offset;

    for index in 0..layout.letters.len() {
        if let Some((start, end)) = letter_hit_bounds(index, &layout)
            && (start..end).contains(&x)
        {
            return Some(index);
        }
    }

    None
}

fn kinetic_target_weight(index: usize, hovered: Option<usize>) -> f32 {
    match hovered {
        None => 300.0,
        Some(active) if index == active => 900.0,
        Some(active) if index.abs_diff(active) == 1 => 600.0,
        Some(active) if index.abs_diff(active) == 2 => 400.0,
        _ => 300.0,
    }
}

fn kinetic_padding_target(index: usize, hovered: Option<usize>, hover_padding: f32) -> f32 {
    match hovered {
        Some(active) if index.abs_diff(active) <= 1 => hover_padding,
        _ => 0.0,
    }
}

fn letter_string(letter: char) -> String {
    if letter == ' ' {
        "\u{00A0}".to_string()
    } else {
        letter.to_string()
    }
}

fn build_kinetic_layout(
    content: &str,
    weights: &[f32],
    padding: &[f32],
    font_size: f32,
) -> KineticLayout {
    ensure_geist_fonts_loaded();

    let mut letters = Vec::with_capacity(content.chars().count());
    let mut box_start = 0.0;
    let (reference_buffer, _) = build_letter_buffer("M", 300.0, font_size);
    let reference_baseline = buffer_baseline(&reference_buffer, font_size);

    for (index, letter) in content.chars().enumerate() {
        let content = letter_string(letter);
        let (buffer, content_width) = build_letter_buffer(
            &content,
            weights.get(index).copied().unwrap_or(300.0),
            font_size,
        );
        let padding = padding.get(index).copied().unwrap_or(0.0);
        let box_width = content_width + padding * 2.0;
        let baseline_offset = reference_baseline - buffer_baseline(&buffer, font_size);

        letters.push(KineticLetterLayout {
            buffer,
            box_start,
            box_width,
            glyph_x: box_start + padding,
            baseline_offset,
        });
        box_start += box_width;
    }

    let mut min_y = 0.0_f32;
    let mut max_y = font_size;
    for letter in &letters {
        let (top, bottom) = buffer_glyph_extent(&letter.buffer);
        min_y = min_y.min(top + letter.baseline_offset);
        max_y = max_y.max(bottom + letter.baseline_offset);
    }

    KineticLayout {
        letters,
        width: box_start,
        min_y,
        max_y,
    }
}

fn build_letter_buffer(content: &str, weight: f32, font_size: f32) -> (cosmic_text::Buffer, f32) {
    let mut font_system = gfx_text::font_system().write().expect("Write font system");
    let mut buffer = cosmic_text::Buffer::new(
        font_system.raw(),
        cosmic_text::Metrics::new(font_size, font_size),
    );
    buffer.set_size(font_system.raw(), Some(f32::INFINITY), Some(f32::INFINITY));
    buffer.set_wrap(font_system.raw(), cosmic_text::Wrap::None);

    let attributes = gfx_text::to_attributes(GEIST_FONT).weight(cosmic_text::Weight(
        weight.round().clamp(100.0, 900.0) as u16,
    ));
    buffer.set_text(
        font_system.raw(),
        content,
        &attributes,
        cosmic_text::Shaping::Advanced,
        None,
    );
    let content_width = gfx_text::align(&mut buffer, font_system.raw(), Alignment::Left).width;

    (buffer, content_width)
}

fn kinetic_weights(state: &KineticCanvasState, now: Instant, len: usize) -> Vec<f32> {
    (0..len)
        .map(|index| {
            state
                .letters
                .get(index)
                .map(|animation| animation.weight_at(now))
                .unwrap_or(300.0)
        })
        .collect()
}

fn kinetic_target_weights(len: usize, hovered: Option<usize>) -> Vec<f32> {
    (0..len)
        .map(|index| kinetic_target_weight(index, hovered))
        .collect()
}

fn kinetic_padding_values(state: &KineticCanvasState, now: Instant, len: usize) -> Vec<f32> {
    (0..len)
        .map(|index| {
            state
                .letters
                .get(index)
                .map(|animation| animation.padding.value_at(now))
                .unwrap_or(0.0)
        })
        .collect()
}

fn letter_hit_bounds(index: usize, layout: &KineticLayout) -> Option<(f32, f32)> {
    let letter = layout.letters.get(index)?;
    Some((letter.box_start, letter.box_start + letter.box_width))
}

fn buffer_glyph_extent(buffer: &cosmic_text::Buffer) -> (f32, f32) {
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for run in buffer.layout_runs() {
        for glyph in run.glyphs.iter() {
            let top = run.line_y + glyph.y_offset;
            let bottom = top + glyph.line_height_opt.unwrap_or(run.line_height);
            min_y = min_y.min(top);
            max_y = max_y.max(bottom);
        }
    }

    if min_y.is_infinite() {
        (0.0, 0.0)
    } else {
        (min_y, max_y)
    }
}

fn measure_kinetic_layout(content: &str, font_size: f32) -> KineticLayoutMetrics {
    let chars: Vec<char> = content.chars().collect();
    let hover_padding = font_size / 12.0;
    let stroke_inset = kinetic_stroke_width(font_size);
    let mut max_width: f32 = 0.0;
    let mut min_y = 0.0_f32;
    let mut max_y = font_size;

    let hover_states = if chars.is_empty() {
        vec![None]
    } else {
        std::iter::once(None)
            .chain((0..chars.len()).map(Some))
            .collect()
    };

    for hovered in hover_states {
        let weights = kinetic_target_weights(chars.len(), hovered);
        let padding: Vec<f32> = (0..chars.len())
            .map(|index| kinetic_padding_target(index, hovered, hover_padding))
            .collect();
        let layout = build_kinetic_layout(content, &weights, &padding, font_size);

        max_width = max_width.max(layout.width);
        min_y = min_y.min(layout.min_y);
        max_y = max_y.max(layout.max_y);
    }

    KineticLayoutMetrics {
        width: max_width + stroke_inset * 2.0,
        height: (max_y - min_y).max(1.0) + stroke_inset * 2.0,
        x_offset: stroke_inset,
        y_offset: -min_y + stroke_inset,
    }
}

fn buffer_baseline(buffer: &cosmic_text::Buffer, fallback: f32) -> f32 {
    buffer
        .layout_runs()
        .next()
        .map_or(fallback, |run| run.line_y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::time::Duration;

    #[test]
    fn weight_transition_keeps_intermediate_variable_font_weights() {
        let start = Instant::now();
        let mut letter = LetterAnimation::new();

        letter.set_targets(0.0, 0.0, 900.0, start);

        assert_eq!(letter.weight_at(start + Duration::from_millis(200)), 600.0);
        assert_eq!(letter.weight_at(start + Duration::from_millis(400)), 900.0);
    }

    #[test]
    fn variable_font_buffers_preserve_nonstandard_weights() {
        let font_size = 96.0;

        for weight in [317.0, 563.0, 841.0] {
            let layout = build_kinetic_layout("N", &[weight], &[0.0], font_size);
            let glyph_weight = layout.letters[0]
                .buffer
                .layout_runs()
                .next()
                .expect("letter buffer has a layout run")
                .glyphs
                .first()
                .expect("letter buffer has a glyph")
                .physical((0.0, 0.0), 1.0)
                .cache_key
                .font_weight
                .0;

            assert_eq!(glyph_weight, weight as u16);
        }
    }

    #[test]
    fn variable_font_outlines_change_with_weight() {
        let light = build_kinetic_layout("N", &[300.0], &[0.0], 96.0);
        let black = build_kinetic_layout("N", &[900.0], &[0.0], 96.0);
        let light_key = first_glyph_cache_key(&light);
        let black_key = first_glyph_cache_key(&black);
        let mut font_system = gfx_text::font_system().write().expect("Write font system");
        let mut outline_context = swash::scale::ScaleContext::new();
        let light_outline =
            variable_outline_commands(font_system.raw(), &mut outline_context, light_key)
                .expect("light glyph has an outline");
        let black_outline =
            variable_outline_commands(font_system.raw(), &mut outline_context, black_key)
                .expect("black glyph has an outline");

        assert_ne!(light_outline, black_outline);
    }

    #[test]
    fn kinetic_weight_matches_reference_hierarchy() {
        assert_eq!(kinetic_target_weight(4, Some(4)), 900.0);
        assert_eq!(kinetic_target_weight(3, Some(4)), 600.0);
        assert_eq!(kinetic_target_weight(2, Some(4)), 400.0);
        assert_eq!(kinetic_target_weight(5, Some(4)), 600.0);
        assert_eq!(kinetic_target_weight(6, Some(4)), 400.0);
        assert_eq!(kinetic_target_weight(0, Some(4)), 300.0);
        assert_eq!(kinetic_target_weight(0, None), 300.0);
    }

    #[test]
    fn kinetic_padding_applies_to_neighbors_only() {
        assert_eq!(kinetic_padding_target(3, Some(4), 8.0), 8.0);
        assert_eq!(kinetic_padding_target(4, Some(4), 8.0), 8.0);
        assert_eq!(kinetic_padding_target(5, Some(4), 8.0), 8.0);
        assert_eq!(kinetic_padding_target(2, Some(4), 8.0), 0.0);
    }

    #[test]
    fn every_character_uses_local_reference_states_and_hit_bounds() {
        let content = "Nostalgia";
        let font_size = 96.0;
        let start = Instant::now();
        let settled = start + Duration::from_millis(u64::from(TRANSITION_MS));
        let len = content.chars().count();
        let metrics = measure_kinetic_layout(content, font_size);

        for active in 0..len {
            let mut state = KineticCanvasState::default();
            state.ensure_content(content);
            state.set_hovered(Some(active), font_size, start);

            let weights = kinetic_weights(&state, settled, len);
            let padding = kinetic_padding_values(&state, settled, len);
            let layout = build_kinetic_layout(content, &weights, &padding, font_size);

            for index in 0..len {
                let expected_weight = if index == active {
                    900.0
                } else if index.abs_diff(active) == 1 {
                    600.0
                } else if index.abs_diff(active) == 2 {
                    400.0
                } else {
                    300.0
                };
                let expected_padding = if index.abs_diff(active) <= 1 {
                    font_size / 12.0
                } else {
                    0.0
                };

                assert_eq!(weights[index], expected_weight);
                assert_eq!(padding[index], expected_padding);

                let (left, right) = letter_hit_bounds(index, &layout).unwrap();
                assert_eq!(
                    hit_test_letter(
                        content,
                        font_size,
                        &state,
                        settled,
                        (left + right) / 2.0 + metrics.x_offset,
                        metrics.x_offset,
                    ),
                    Some(index),
                    "active {active}, glyph {index} has incorrect hit bounds"
                );
            }
        }
    }

    #[test]
    fn each_character_has_an_independent_flex_box() {
        let content = "Nostalgia";
        let font_size = 96.0;
        let active = 1;
        let weights = kinetic_target_weights(content.chars().count(), Some(active));
        let padding: Vec<f32> = (0..content.chars().count())
            .map(|index| kinetic_padding_target(index, Some(active), font_size / 12.0))
            .collect();
        let layout = build_kinetic_layout(content, &weights, &padding, font_size);
        let mut previous_end = 0.0;

        assert_eq!(layout.letters.len(), content.chars().count());

        for (index, letter) in layout.letters.iter().enumerate() {
            assert!(
                (letter.box_start - previous_end).abs() < 0.01,
                "letter {index} does not begin at the previous flex-box boundary"
            );
            assert!(
                (letter.glyph_x - (letter.box_start + padding[index])).abs() < 0.01,
                "letter {index} glyph origin is not inside its own padding"
            );
            assert!(
                (letter.box_width - (buffer_width(&letter.buffer) + padding[index] * 2.0)).abs()
                    < 0.01,
                "letter {index} box width does not match its independent content width"
            );

            previous_end = letter.box_start + letter.box_width;
        }

        assert!((layout.width - previous_end).abs() < 0.01);
    }

    #[test]
    fn every_layout_state_stays_within_measured_horizontal_bounds() {
        let content = "Nostalgia";
        let font_size = 96.0;
        let metrics = measure_kinetic_layout(content, font_size);
        let len = content.chars().count();

        for hovered in std::iter::once(None).chain((0..len).map(Some)) {
            let weights = kinetic_target_weights(len, hovered);
            let padding: Vec<f32> = (0..len)
                .map(|index| kinetic_padding_target(index, hovered, font_size / 12.0))
                .collect();
            let layout = build_kinetic_layout(content, &weights, &padding, font_size);
            let (min_x, max_x) = layout_glyph_x_extent(&layout);
            let shifted_min = min_x + metrics.x_offset;
            let shifted_max = max_x + metrics.x_offset;

            assert!(
                shifted_min >= -0.5,
                "hovered {hovered:?} extends left of the canvas origin ({shifted_min})"
            );
            assert!(
                shifted_max <= metrics.width + 0.5,
                "hovered {hovered:?} extends beyond the measured width ({shifted_max} > {})",
                metrics.width
            );
        }
    }

    #[test]
    fn independent_layouts_preserve_character_count() {
        let content = "Nostalgia";
        let weights = kinetic_target_weights(content.chars().count(), None);
        let padding = vec![0.0; content.chars().count()];
        let layout = build_kinetic_layout(content, &weights, &padding, 96.0);
        let glyph_counts = layout
            .letters
            .iter()
            .map(|letter| {
                letter
                    .buffer
                    .layout_runs()
                    .map(|run| run.glyphs.len())
                    .sum::<usize>()
            })
            .collect::<Vec<_>>();

        assert_eq!(layout.letters.len(), content.chars().count());
        assert_eq!(glyph_counts, vec![1; content.chars().count()]);
    }

    #[test]
    fn letter_baselines_stay_stable_when_hover_changes() {
        let content = "Nostalgia";
        let idle_weights = kinetic_target_weights(content.chars().count(), None);
        let hovered_weights = kinetic_target_weights(content.chars().count(), Some(4));
        let padding = vec![0.0; content.chars().count()];
        let idle = build_kinetic_layout(content, &idle_weights, &padding, 96.0);
        let hovered = build_kinetic_layout(content, &hovered_weights, &padding, 96.0);

        for (index, (idle_letter, hovered_letter)) in
            idle.letters.iter().zip(&hovered.letters).enumerate()
        {
            let idle_y = letter_baseline(idle_letter);
            let hovered_y = letter_baseline(hovered_letter);
            assert!(
                (idle_y - hovered_y).abs() < 0.01,
                "letter {index} baseline shifted ({idle_y} vs {hovered_y})"
            );
        }
    }

    #[test]
    fn every_layout_state_fits_inside_measured_canvas_height() {
        let content = "Nostalgia";
        let font_size = 96.0;
        let metrics = measure_kinetic_layout(content, font_size);
        let len = content.chars().count();

        for hovered in std::iter::once(None).chain((0..len).map(Some)) {
            let weights = kinetic_target_weights(len, hovered);
            let padding: Vec<f32> = (0..len)
                .map(|index| kinetic_padding_target(index, hovered, font_size / 12.0))
                .collect();
            let layout = build_kinetic_layout(content, &weights, &padding, font_size);
            let (min_y, max_y) = layout_glyph_y_extent(&layout);
            let shifted_min = min_y + metrics.y_offset;
            let shifted_max = max_y + metrics.y_offset;

            assert!(
                shifted_min >= -0.5,
                "hovered {hovered:?} extends above canvas origin ({shifted_min})"
            );
            assert!(
                shifted_max <= metrics.height + 0.5,
                "hovered {hovered:?} extends below canvas height ({shifted_max} > {})",
                metrics.height
            );
        }
    }

    fn letter_baseline(letter: &KineticLetterLayout) -> f32 {
        letter
            .buffer
            .layout_runs()
            .next()
            .expect("letter paragraph has a layout run")
            .line_y
            + letter.baseline_offset
    }

    fn layout_glyph_x_extent(layout: &KineticLayout) -> (f32, f32) {
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;

        for letter in &layout.letters {
            for run in letter.buffer.layout_runs() {
                for glyph in run.glyphs.iter() {
                    let left = letter.glyph_x + glyph.x + glyph.x_offset;
                    let right = left + glyph.w;
                    min_x = min_x.min(left);
                    max_x = max_x.max(right);
                }
            }
        }

        if min_x.is_infinite() {
            (0.0, 0.0)
        } else {
            (min_x, max_x)
        }
    }

    fn buffer_width(buffer: &cosmic_text::Buffer) -> f32 {
        buffer
            .layout_runs()
            .map(|run| run.line_w)
            .fold(0.0_f32, f32::max)
    }

    fn first_glyph_cache_key(layout: &KineticLayout) -> cosmic_text::CacheKey {
        layout.letters[0]
            .buffer
            .layout_runs()
            .next()
            .expect("letter buffer has a layout run")
            .glyphs
            .first()
            .expect("letter buffer has a glyph")
            .physical((0.0, 0.0), 1.0)
            .cache_key
    }

    fn layout_glyph_y_extent(layout: &KineticLayout) -> (f32, f32) {
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for letter in &layout.letters {
            for run in letter.buffer.layout_runs() {
                for glyph in run.glyphs.iter() {
                    let top = letter.baseline_offset + run.line_y + glyph.y_offset;
                    let bottom = top + glyph.line_height_opt.unwrap_or(run.line_height);
                    min_y = min_y.min(top);
                    max_y = max_y.max(bottom);
                }
            }
        }

        if min_y.is_infinite() {
            (0.0, 0.0)
        } else {
            (min_y, max_y)
        }
    }
}
