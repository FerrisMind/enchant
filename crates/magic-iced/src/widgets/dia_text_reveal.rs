use crate::primitives::{AnimatedValue, EffectCanvasProps, EffectProgram, effect_canvas, MotionSpec};
use iced::advanced::graphics::text::Paragraph as GraphicsParagraph;
use iced::advanced::text::{
    self as text_layout, Alignment, LineHeight, Paragraph as TextParagraph, Shaping, Wrapping,
};
use iced::alignment::Vertical;
use iced::time::Instant;
use iced::widget::{canvas, text};
use iced::{Color, Element, Font, Length, Pixels, Point, Rectangle, Renderer, Size};
use iced_shadcn::Theme;
use twill::prelude::{Easing, TransitionDuration};

const BAND_HALF: f32 = 17.0;
const SWEEP_START: f32 = -BAND_HALF;
const SWEEP_END: f32 = 100.0 + BAND_HALF;
/// Matches the reference `leading-[100%]` and keeps inline companions aligned.
const DIA_LINE_HEIGHT: LineHeight = LineHeight::Relative(1.0);
/// Animated span nudge (reference uses `-2px`; tuned to `-0.5px` for iced text metrics).
const DIA_REFERENCE_Y_NUDGE: f32 = -0.5;

pub const DEFAULT_DIA_COLORS: &[Color] = &[
    Color::from_rgb8(0xc6, 0x79, 0xc4),
    Color::from_rgb8(0xfa, 0x3d, 0x1d),
    Color::from_rgb8(0xff, 0xb0, 0x05),
    Color::from_rgb8(0xe1, 0xe1, 0xfe),
    Color::from_rgb8(0x03, 0x58, 0xf7),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlayPhase {
    Idle,
    Delaying,
    Sweeping,
    RepeatPause,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DiaTextRevealFingerprint {
    texts_key: u64,
    prefix_key: u64,
    colors_key: u64,
    duration_secs: f32,
    delay_secs: f32,
    repeat: bool,
    repeat_delay_secs: f32,
    start_on_view: bool,
    once: bool,
    fixed_width: bool,
    font_size: u16,
    reduced_motion: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct DiaTextRevealProps {
    pub texts: &'static [&'static str],
    pub colors: &'static [Color],
    pub text_color: Option<Color>,
    pub duration_secs: f32,
    pub delay_secs: f32,
    pub repeat: bool,
    pub repeat_delay_secs: f32,
    pub start_on_view: bool,
    pub once: bool,
    pub fixed_width: bool,
    pub font_size: u16,
    /// When `start_on_view` is true, the sweep begins after this becomes true.
    pub visible: bool,
    pub reduced_motion: bool,
    pub width_motion: MotionSpec,
    /// Static copy rendered before the animated text on the same baseline.
    pub prefix: Option<&'static str>,
}

impl Default for DiaTextRevealProps {
    fn default() -> Self {
        Self {
            texts: &["Magic UI"],
            colors: DEFAULT_DIA_COLORS,
            text_color: None,
            duration_secs: 1.5,
            delay_secs: 0.0,
            repeat: false,
            repeat_delay_secs: 0.5,
            start_on_view: true,
            once: true,
            fixed_width: false,
            font_size: 36,
            visible: false,
            reduced_motion: false,
            width_motion: MotionSpec::new()
                .duration(TransitionDuration::CustomMs(400))
                .easing(Easing::InOut),
            prefix: None,
        }
    }
}

impl DiaTextRevealProps {
    pub fn from_texts(texts: &'static [&'static str]) -> Self {
        Self {
            texts,
            ..Self::default()
        }
    }

    pub fn texts(mut self, texts: &'static [&'static str]) -> Self {
        self.texts = texts;
        self
    }

    pub fn colors(mut self, colors: &'static [Color]) -> Self {
        self.colors = colors;
        self
    }

    pub fn text_color(mut self, text_color: Color) -> Self {
        self.text_color = Some(text_color);
        self
    }

    pub fn duration_secs(mut self, duration_secs: f32) -> Self {
        self.duration_secs = duration_secs.max(0.05);
        self
    }

    pub fn delay_secs(mut self, delay_secs: f32) -> Self {
        self.delay_secs = delay_secs.max(0.0);
        self
    }

    pub fn repeat(mut self, repeat: bool) -> Self {
        self.repeat = repeat;
        self
    }

    pub fn repeat_delay_secs(mut self, repeat_delay_secs: f32) -> Self {
        self.repeat_delay_secs = repeat_delay_secs.max(0.0);
        self
    }

    pub fn start_on_view(mut self, start_on_view: bool) -> Self {
        self.start_on_view = start_on_view;
        self
    }

    pub fn once(mut self, once: bool) -> Self {
        self.once = once;
        self
    }

    pub fn fixed_width(mut self, fixed_width: bool) -> Self {
        self.fixed_width = fixed_width;
        self
    }

    pub fn font_size(mut self, font_size: u16) -> Self {
        self.font_size = font_size.max(1);
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub fn reduced_motion(mut self, reduced_motion: bool) -> Self {
        self.reduced_motion = reduced_motion;
        self
    }

    pub fn prefix(mut self, prefix: &'static str) -> Self {
        self.prefix = Some(prefix);
        self
    }
}

#[derive(Clone, Debug)]
pub struct DiaTextRevealState {
    active_index: usize,
    phase: PlayPhase,
    phase_started_at: Instant,
    width: AnimatedValue,
    has_played: bool,
    fingerprint: DiaTextRevealFingerprint,
}

impl DiaTextRevealState {
    pub fn new(props: &DiaTextRevealProps) -> Self {
        let width = initial_width(props, 0);
        Self {
            active_index: 0,
            phase: PlayPhase::Idle,
            phase_started_at: Instant::now(),
            width: AnimatedValue::with_spec(width, props.width_motion),
            has_played: false,
            fingerprint: fingerprint(props),
        }
    }

    pub fn active_index(&self) -> usize {
        self.active_index
    }

    pub fn sweep_position(&self, now: Instant, props: &DiaTextRevealProps) -> f32 {
        if props.reduced_motion || matches!(self.phase, PlayPhase::Done) {
            return SWEEP_END;
        }

        match self.phase {
            PlayPhase::Idle | PlayPhase::Delaying | PlayPhase::RepeatPause => SWEEP_START,
            PlayPhase::Sweeping => {
                let elapsed = now
                    .saturating_duration_since(self.phase_started_at)
                    .as_secs_f32();
                let progress = (elapsed / props.duration_secs.max(0.05)).clamp(0.0, 1.0);
                SWEEP_START + sweep_ease(progress) * (SWEEP_END - SWEEP_START)
            }
            PlayPhase::Done => SWEEP_END,
        }
    }

    pub fn tick(&mut self, now: Instant, props: &DiaTextRevealProps) {
        let next_fingerprint = fingerprint(props);
        if self.fingerprint != next_fingerprint {
            *self = Self::new(props);
        }

        if props.reduced_motion {
            self.phase = PlayPhase::Done;
            self.has_played = true;
            return;
        }

        match self.phase {
            PlayPhase::Idle => {
                if !self.should_start(props) {
                    return;
                }
                self.has_played = true;
                if props.delay_secs > f32::EPSILON {
                    self.phase = PlayPhase::Delaying;
                    self.phase_started_at = now;
                } else {
                    self.begin_sweep(now, props);
                }
            }
            PlayPhase::Delaying => {
                if now.saturating_duration_since(self.phase_started_at).as_secs_f32()
                    >= props.delay_secs
                {
                    self.begin_sweep(now, props);
                }
            }
            PlayPhase::Sweeping => {
                if now.saturating_duration_since(self.phase_started_at).as_secs_f32()
                    >= props.duration_secs
                {
                    self.complete_sweep(now, props);
                }
            }
            PlayPhase::RepeatPause => {
                if now.saturating_duration_since(self.phase_started_at).as_secs_f32()
                    >= props.repeat_delay_secs
                {
                    self.advance_text(now, props);
                    self.begin_sweep(now, props);
                }
            }
            PlayPhase::Done => {}
        }
    }

    fn should_start(&self, props: &DiaTextRevealProps) -> bool {
        if props.once && self.has_played {
            return false;
        }
        if props.start_on_view && !props.visible {
            return false;
        }
        true
    }

    fn begin_sweep(&mut self, now: Instant, _props: &DiaTextRevealProps) {
        self.phase = PlayPhase::Sweeping;
        self.phase_started_at = now;
    }

    fn complete_sweep(&mut self, now: Instant, props: &DiaTextRevealProps) {
        if props.repeat && props.texts.len() > 1 {
            self.phase = PlayPhase::RepeatPause;
            self.phase_started_at = now;
            return;
        }
        self.phase = PlayPhase::Done;
        let _ = now;
    }

    fn advance_text(&mut self, now: Instant, props: &DiaTextRevealProps) {
        if props.texts.is_empty() {
            return;
        }
        self.active_index = (self.active_index + 1) % props.texts.len();
        let target_width = initial_width(props, self.active_index);
        self.width.transition(target_width, now);
    }
}

/// Prefix/suffix copy that should match the settled color of `dia_text_reveal`.
pub fn dia_text_reveal_companion<'a, Message: 'a>(
    content: &'static str,
    font_size: u16,
    theme: &Theme,
) -> Element<'a, Message> {
    let color = theme.palette.foreground;
    text(content)
        .size(u32::from(font_size))
        .line_height(DIA_LINE_HEIGHT)
        .style(move |_| iced::widget::text::Style { color: Some(color) })
        .into()
}

pub fn dia_text_reveal<'a, Message: 'a>(
    state: &DiaTextRevealState,
    now: Instant,
    props: &DiaTextRevealProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let text_color = props.text_color.unwrap_or(theme.palette.foreground);
    let active_text = props
        .texts
        .get(state.active_index)
        .copied()
        .unwrap_or("");
    let prefix = props.prefix.unwrap_or("");
    let prefix_metrics = measure_text_size(prefix, props.font_size as f32);
    let metrics = measure_text_size(active_text, props.font_size as f32);
    let clip_width = if props.texts.len() > 1 {
        if props.fixed_width {
            max_text_width(props)
        } else {
            state.width.value_at(now).max(1.0)
        }
    } else {
        metrics.width
    };
    let content_top_pad = DIA_REFERENCE_Y_NUDGE.abs();
    let block_height = prefix_metrics
        .height
        .max(metrics.height)
        .max(dia_line_box_height(props.font_size as f32))
        + content_top_pad;
    let canvas_width = prefix_metrics.width + clip_width;

    effect_canvas::<_, Message>(
        DiaTextRevealProgram {
            prefix: prefix.to_owned(),
            content: active_text.to_owned(),
            font_size: props.font_size as f32,
            text_color,
            colors: props.colors,
            sweep_pos: state.sweep_position(now, props),
            text_size: metrics,
            clip_width,
            prefix_width: prefix_metrics.width,
        },
        EffectCanvasProps::new()
            .width(Length::Fixed(canvas_width.ceil().max(1.0)))
            .height(Length::Fixed(block_height)),
    )
    .into()
}

struct DiaTextRevealProgram {
    prefix: String,
    content: String,
    font_size: f32,
    text_color: Color,
    colors: &'static [Color],
    sweep_pos: f32,
    text_size: Size,
    clip_width: f32,
    prefix_width: f32,
}

impl<Message> EffectProgram<Message> for DiaTextRevealProgram {
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

        let content_top_pad = DIA_REFERENCE_Y_NUDGE.abs();

        if !self.prefix.is_empty() {
            let prefix_text = dia_canvas_text(
                &self.prefix,
                Point::new(0.0, content_top_pad),
                self.font_size,
                self.text_size.width,
            );
            prefix_text.draw_with(|path, _color| {
                frame.fill(&path, self.text_color);
            });
        }

        let animated_text = dia_canvas_text(
            &self.content,
            Point::new(
                self.prefix_width,
                content_top_pad + DIA_REFERENCE_Y_NUDGE,
            ),
            self.font_size,
            self.text_size.width,
        );

        let mut glyph_paths = Vec::new();
        animated_text.draw_with(|path, _color| glyph_paths.push(path));

        if glyph_paths.is_empty() {
            return vec![frame.into_geometry()];
        }

        let animated_clip = Rectangle::new(
            Point::new(self.prefix_width, 0.0),
            Size::new(
                self.clip_width.min(bounds.width - self.prefix_width),
                bounds.height,
            ),
        );
        let text_width = self.text_size.width.max(1.0);
        let column_count = text_width.ceil() as i32;

        frame.with_clip(animated_clip, |frame| {
            for column in 0..column_count {
                let pct = (column as f32 + 0.5) / text_width * 100.0;
                let color = reference_sweep_color_at_pct(
                    pct,
                    self.sweep_pos,
                    self.colors,
                    self.text_color,
                );

                if color.a <= f32::EPSILON {
                    continue;
                }

                let strip = Rectangle::new(
                    Point::new(self.prefix_width + column as f32, 0.0),
                    Size::new(1.0, bounds.height),
                );

                frame.with_clip(strip, |frame| {
                    for path in &glyph_paths {
                        frame.fill(path, color);
                    }
                });
            }
        });

        vec![frame.into_geometry()]
    }
}

fn dia_canvas_text(
    content: &str,
    position: Point,
    font_size: f32,
    max_width: f32,
) -> canvas::Text {
    canvas::Text {
        content: content.to_owned(),
        position,
        max_width,
        color: Color::TRANSPARENT,
        size: Pixels(font_size),
        line_height: DIA_LINE_HEIGHT,
        font: Font::DEFAULT,
        align_x: Alignment::Left,
        align_y: Vertical::Top,
        ..canvas::Text::default()
    }
}

/// Samples the sweep gradient at a horizontal percentage of the text width,
/// matching `buildGradient` from the Magic UI reference.
fn reference_sweep_color_at_pct(
    pct: f32,
    sweep_pos: f32,
    colors: &[Color],
    text_color: Color,
) -> Color {
    let band_start = sweep_pos - BAND_HALF;
    let band_end = sweep_pos + BAND_HALF;

    if band_start >= 100.0 {
        return text_color;
    }

    if band_start > 0.0 && pct < band_start {
        return text_color;
    }

    if band_end < 100.0 && pct > band_end {
        return Color::TRANSPARENT;
    }

    match colors.len() {
        0 => text_color,
        1 => colors[0],
        count => {
            let mut stop_pcts = Vec::with_capacity(count);
            for (index, &color) in colors.iter().enumerate() {
                let stop_pct =
                    band_start + (index as f32 / (count as f32 - 1.0)) * BAND_HALF * 2.0;
                stop_pcts.push((stop_pct, color));
            }

            if pct <= stop_pcts[0].0 {
                return stop_pcts[0].1;
            }

            let last = stop_pcts[count - 1];
            if pct >= last.0 {
                return last.1;
            }

            for window in stop_pcts.windows(2) {
                let (left_pct, left_color) = window[0];
                let (right_pct, right_color) = window[1];
                if pct >= left_pct && pct <= right_pct {
                    let span = right_pct - left_pct;
                    let t = if span <= f32::EPSILON {
                        0.0
                    } else {
                        (pct - left_pct) / span
                    };
                    return mix_color(left_color, right_color, t);
                }
            }

            last.1
        }
    }
}

fn mix_color(from: Color, to: Color, progress: f32) -> Color {
    let progress = progress.clamp(0.0, 1.0);

    Color {
        r: from.r + (to.r - from.r) * progress,
        g: from.g + (to.g - from.g) * progress,
        b: from.b + (to.b - from.b) * progress,
        a: from.a + (to.a - from.a) * progress,
    }
}

fn dia_line_box_height(font_size: f32) -> f32 {
    DIA_LINE_HEIGHT.to_absolute(Pixels(font_size)).0
}

fn measure_text_size(content: &str, font_size: f32) -> Size {
    if content.is_empty() {
        return Size::ZERO;
    }

    let paragraph = GraphicsParagraph::with_text(text_layout::Text {
        content,
        bounds: Size::new(f32::INFINITY, f32::INFINITY),
        size: Pixels(font_size),
        line_height: DIA_LINE_HEIGHT,
        font: Font::DEFAULT,
        align_x: Alignment::Left,
        align_y: Vertical::Top,
        shaping: Shaping::Advanced,
        wrapping: Wrapping::None,
    });

    paragraph.min_bounds()
}

fn sweep_ease(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

fn initial_width(props: &DiaTextRevealProps, index: usize) -> f32 {
    if props.fixed_width {
        return max_text_width(props);
    }

    props
        .texts
        .get(index)
        .map(|text| measure_text_size(text, props.font_size as f32).width)
        .unwrap_or(1.0)
}

fn max_text_width(props: &DiaTextRevealProps) -> f32 {
    props
        .texts
        .iter()
        .map(|text| measure_text_size(text, props.font_size as f32).width)
        .fold(1.0, f32::max)
}

fn fingerprint(props: &DiaTextRevealProps) -> DiaTextRevealFingerprint {
    DiaTextRevealFingerprint {
        texts_key: hash_texts(props.texts),
        prefix_key: hash_text(props.prefix.unwrap_or("")),
        colors_key: hash_colors(props.colors),
        duration_secs: props.duration_secs,
        delay_secs: props.delay_secs,
        repeat: props.repeat,
        repeat_delay_secs: props.repeat_delay_secs,
        start_on_view: props.start_on_view,
        once: props.once,
        fixed_width: props.fixed_width,
        font_size: props.font_size,
        reduced_motion: props.reduced_motion,
    }
}

fn hash_texts(texts: &[&str]) -> u64 {
    texts.iter().fold(0_u64, |acc, text| hash_text_with_seed(acc, text))
}

fn hash_text(text: &str) -> u64 {
    hash_text_with_seed(0, text)
}

fn hash_text_with_seed(seed: u64, text: &str) -> u64 {
    text.bytes().fold(seed, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(u64::from(byte))
    })
}

fn hash_colors(colors: &[Color]) -> u64 {
    colors.iter().fold(0_u64, |acc, color| {
        acc.wrapping_mul(31)
            .wrapping_add(color.r.to_bits() as u64)
            .wrapping_add(color.g.to_bits() as u64)
            .wrapping_add(color.b.to_bits() as u64)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::time::Duration;

    #[test]
    fn dia_text_reveal_defaults_match_reference_palette() {
        let props = DiaTextRevealProps::from_texts(&["Magic UI"]);
        assert_eq!(props.duration_secs, 1.5);
        assert_eq!(props.colors.len(), 5);
        assert_eq!(props.repeat_delay_secs, 0.5);
        assert!(props.start_on_view);
        assert!(props.once);
    }

    #[test]
    fn sweep_ease_matches_reference_midpoint() {
        assert!((sweep_ease(0.5) - 0.5).abs() < 0.01);
        assert!(sweep_ease(0.0) < sweep_ease(0.25));
        assert!(sweep_ease(0.75) > sweep_ease(0.5));
    }

    #[test]
    fn completed_sweep_uses_flat_text_color() {
        let text_color = Color::from_rgb8(0x11, 0x11, 0x11);
        assert!(SWEEP_END - BAND_HALF >= 100.0);
        for pct in [0.0, 25.0, 50.0, 99.0, 100.0] {
            let color =
                reference_sweep_color_at_pct(pct, SWEEP_END, DEFAULT_DIA_COLORS, text_color);
            assert_eq!(color, text_color, "pct {pct}");
        }
    }

    #[test]
    fn unrevealed_trailing_region_is_transparent() {
        let color = reference_sweep_color_at_pct(80.0, 50.0, DEFAULT_DIA_COLORS, Color::WHITE);
        assert_eq!(color.a, 0.0);
    }

    #[test]
    fn band_remains_visible_past_right_text_edge() {
        let color = reference_sweep_color_at_pct(100.0, 100.0, DEFAULT_DIA_COLORS, Color::WHITE);
        assert!(
            color.a > 0.0,
            "right edge should stay visible while the band crosses it"
        );
    }

    #[test]
    fn sweep_past_right_edge_reveals_text_before_settling() {
        let text_color = Color::WHITE;
        let mid = reference_sweep_color_at_pct(50.0, 110.0, DEFAULT_DIA_COLORS, text_color);
        let settled =
            reference_sweep_color_at_pct(50.0, SWEEP_END, DEFAULT_DIA_COLORS, text_color);
        assert_eq!(settled, text_color);
        assert!(mid.a > 0.0);
    }

    #[test]
    fn repeat_cycles_active_index() {
        let props = DiaTextRevealProps::from_texts(&["build faster", "ship smarter", "scale easier"])
            .repeat(true)
            .repeat_delay_secs(0.1)
            .duration_secs(0.1)
            .start_on_view(false)
            .visible(true);
        let mut state = DiaTextRevealState::new(&props);
        let start = Instant::now();

        state.tick(start, &props);
        state.tick(start + Duration::from_millis(120), &props);
        state.tick(start + Duration::from_millis(220), &props);
        state.tick(start + Duration::from_millis(340), &props);

        assert_eq!(state.active_index(), 1);
    }
}
