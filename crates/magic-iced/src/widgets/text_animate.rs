use super::text_element::TextElementRole;
use crate::primitives::{TextSegmentMode, segment_text};
use iced::time::Instant;
use iced::widget::{column, container, float, row, text};
use iced::{Color, Element, Length, Vector};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextAnimateBy {
    Text,
    #[default]
    Word,
    Character,
    Line,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextAnimatePreset {
    #[default]
    FadeIn,
    BlurIn,
    BlurInUp,
    BlurInDown,
    SlideUp,
    SlideDown,
    SlideLeft,
    SlideRight,
    ScaleUp,
    ScaleDown,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextAnimateVariantState {
    pub opacity: f32,
    pub x: f32,
    pub y: f32,
    pub scale: f32,
    pub rotation_deg: f32,
    pub blur: f32,
}

impl Default for TextAnimateVariantState {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            x: 0.0,
            y: 0.0,
            scale: 1.0,
            rotation_deg: 0.0,
            blur: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextAnimateVariants {
    pub hidden: TextAnimateVariantState,
    pub show: TextAnimateVariantState,
    pub exit: TextAnimateVariantState,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextAnimateProps {
    pub delay_secs: f32,
    pub duration_secs: f32,
    pub variants: Option<TextAnimateVariants>,
    pub role: TextElementRole,
    pub by: TextAnimateBy,
    pub start_on_view: bool,
    pub once: bool,
    pub animation: TextAnimatePreset,
    pub accessible: bool,
    pub font_size: Option<u16>,
}

impl Default for TextAnimateProps {
    fn default() -> Self {
        Self {
            delay_secs: 0.0,
            duration_secs: 0.3,
            variants: None,
            role: TextElementRole::P,
            by: TextAnimateBy::Word,
            start_on_view: true,
            once: false,
            animation: TextAnimatePreset::FadeIn,
            accessible: true,
            font_size: None,
        }
    }
}

impl TextAnimateProps {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TextAnimatePhase {
    Hidden,
    Showing,
    Visible,
    Hiding,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TextAnimateFingerprint {
    delay_secs: f32,
    duration_secs: f32,
    role: TextElementRole,
    by: TextAnimateBy,
    start_on_view: bool,
    once: bool,
    animation: TextAnimatePreset,
    font_size: Option<u16>,
}

impl From<&TextAnimateProps> for TextAnimateFingerprint {
    fn from(props: &TextAnimateProps) -> Self {
        Self {
            delay_secs: props.delay_secs,
            duration_secs: props.duration_secs,
            role: props.role,
            by: props.by,
            start_on_view: props.start_on_view,
            once: props.once,
            animation: props.animation,
            font_size: props.font_size,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TextAnimateState {
    phase: TextAnimatePhase,
    phase_started_at: Option<Instant>,
    auto_started: bool,
    played_once: bool,
    fingerprint: TextAnimateFingerprint,
}

impl TextAnimateState {
    pub fn new(props: &TextAnimateProps) -> Self {
        Self {
            phase: TextAnimatePhase::Hidden,
            phase_started_at: None,
            auto_started: false,
            played_once: false,
            fingerprint: TextAnimateFingerprint::from(props),
        }
    }

    pub fn tick(&mut self, now: Instant, props: &TextAnimateProps) {
        let fingerprint = TextAnimateFingerprint::from(props);
        if self.fingerprint != fingerprint {
            *self = Self::new(props);
        }

        if !self.auto_started && (!props.once || !self.played_once) {
            self.auto_started = true;
            self.show(now, props);
        }

        if let Some(start) = self.phase_started_at {
            let elapsed = now.saturating_duration_since(start).as_secs_f32();
            if elapsed >= timeline_duration(props, 1) {
                match self.phase {
                    TextAnimatePhase::Showing => {
                        self.phase = TextAnimatePhase::Visible;
                        self.played_once = true;
                    }
                    TextAnimatePhase::Hiding => {
                        self.phase = TextAnimatePhase::Hidden;
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn show(&mut self, now: Instant, props: &TextAnimateProps) {
        if props.once && self.played_once {
            return;
        }
        self.phase = TextAnimatePhase::Showing;
        self.phase_started_at = Some(now);
    }

    pub fn hide(&mut self, now: Instant) {
        self.phase = TextAnimatePhase::Hiding;
        self.phase_started_at = Some(now);
    }
}

pub fn text_animate<'a, Message: 'a>(
    content: &'a str,
    state: &TextAnimateState,
    now: Instant,
    props: &TextAnimateProps,
) -> Element<'a, Message> {
    let segments = segment_text(content, segment_mode(props.by));
    let variants = props
        .variants
        .unwrap_or_else(|| preset_variants(props.animation));
    let size = u32::from(
        props
            .font_size
            .unwrap_or(props.role.default_font_size())
            .max(1),
    );
    let segment_count = segments.len().max(1);
    let base_color = Color::WHITE;

    let mut items: Vec<Element<'a, Message>> = Vec::with_capacity(segment_count);
    for (index, segment) in segments.into_iter().enumerate() {
        let progress = segment_progress(state, now, props, index, segment_count);
        let transform = match state.phase {
            TextAnimatePhase::Hiding => interpolate(variants.show, variants.exit, progress),
            TextAnimatePhase::Hidden => variants.hidden,
            _ => interpolate(variants.hidden, variants.show, progress),
        };
        let color = Color {
            a: transform.opacity.clamp(0.0, 1.0),
            ..base_color
        };
        let node: Element<'a, Message> = float(
            container(
                text(segment)
                    .size((size as f32 * transform.scale.max(0.1)) as u32)
                    .style(move |_| iced::widget::text::Style { color: Some(color) }),
            )
            .width(Length::Shrink)
            .height(Length::Shrink),
        )
        .translate(move |_bounds, _viewport| Vector::new(transform.x, transform.y))
        .into();
        items.push(node);
    }

    match props.by {
        TextAnimateBy::Line => column(items).spacing(6).into(),
        _ => row(items).spacing(0).into(),
    }
}

fn segment_mode(by: TextAnimateBy) -> TextSegmentMode {
    match by {
        TextAnimateBy::Text => TextSegmentMode::Text,
        TextAnimateBy::Word => TextSegmentMode::Word,
        TextAnimateBy::Character => TextSegmentMode::Character,
        TextAnimateBy::Line => TextSegmentMode::Line,
    }
}

fn segment_progress(
    state: &TextAnimateState,
    now: Instant,
    props: &TextAnimateProps,
    index: usize,
    total: usize,
) -> f32 {
    let start = state.phase_started_at.unwrap_or(now);
    let elapsed = now.saturating_duration_since(start).as_secs_f32();
    let delay = props.delay_secs.max(0.0);
    let stagger = props.duration_secs.max(0.01) / total.max(1) as f32;
    let segment_start = delay + stagger * index as f32;
    let segment_duration = props.duration_secs.max(0.01);

    ((elapsed - segment_start) / segment_duration).clamp(0.0, 1.0)
}

fn timeline_duration(props: &TextAnimateProps, segment_count: usize) -> f32 {
    props.delay_secs.max(0.0)
        + props.duration_secs.max(0.01)
        + props.duration_secs.max(0.01) / segment_count.max(1) as f32 * segment_count as f32
}

fn preset_variants(preset: TextAnimatePreset) -> TextAnimateVariants {
    use TextAnimatePreset::*;

    let hidden = |opacity, x, y, scale, blur| TextAnimateVariantState {
        opacity,
        x,
        y,
        scale,
        rotation_deg: 0.0,
        blur,
    };

    match preset {
        FadeIn => TextAnimateVariants {
            hidden: hidden(0.0, 0.0, 20.0, 1.0, 0.0),
            show: TextAnimateVariantState::default(),
            exit: hidden(0.0, 0.0, 20.0, 1.0, 0.0),
        },
        BlurIn => TextAnimateVariants {
            hidden: hidden(0.0, 0.0, 0.0, 1.0, 10.0),
            show: TextAnimateVariantState::default(),
            exit: hidden(0.0, 0.0, 0.0, 1.0, 10.0),
        },
        BlurInUp => TextAnimateVariants {
            hidden: hidden(0.0, 0.0, 20.0, 1.0, 10.0),
            show: TextAnimateVariantState::default(),
            exit: hidden(0.0, 0.0, 20.0, 1.0, 10.0),
        },
        BlurInDown => TextAnimateVariants {
            hidden: hidden(0.0, 0.0, -20.0, 1.0, 10.0),
            show: TextAnimateVariantState::default(),
            exit: hidden(0.0, 0.0, -20.0, 1.0, 10.0),
        },
        SlideUp => TextAnimateVariants {
            hidden: hidden(0.0, 0.0, 20.0, 1.0, 0.0),
            show: TextAnimateVariantState::default(),
            exit: hidden(0.0, 0.0, -20.0, 1.0, 0.0),
        },
        SlideDown => TextAnimateVariants {
            hidden: hidden(0.0, 0.0, -20.0, 1.0, 0.0),
            show: TextAnimateVariantState::default(),
            exit: hidden(0.0, 0.0, 20.0, 1.0, 0.0),
        },
        SlideLeft => TextAnimateVariants {
            hidden: hidden(0.0, 20.0, 0.0, 1.0, 0.0),
            show: TextAnimateVariantState::default(),
            exit: hidden(0.0, -20.0, 0.0, 1.0, 0.0),
        },
        SlideRight => TextAnimateVariants {
            hidden: hidden(0.0, -20.0, 0.0, 1.0, 0.0),
            show: TextAnimateVariantState::default(),
            exit: hidden(0.0, 20.0, 0.0, 1.0, 0.0),
        },
        ScaleUp => TextAnimateVariants {
            hidden: hidden(0.0, 0.0, 0.0, 0.5, 0.0),
            show: TextAnimateVariantState::default(),
            exit: hidden(0.0, 0.0, 0.0, 0.5, 0.0),
        },
        ScaleDown => TextAnimateVariants {
            hidden: hidden(0.0, 0.0, 0.0, 1.5, 0.0),
            show: TextAnimateVariantState::default(),
            exit: hidden(0.0, 0.0, 0.0, 1.5, 0.0),
        },
    }
}

fn interpolate(
    from: TextAnimateVariantState,
    to: TextAnimateVariantState,
    progress: f32,
) -> TextAnimateVariantState {
    let progress = progress.clamp(0.0, 1.0);

    TextAnimateVariantState {
        opacity: from.opacity + (to.opacity - from.opacity) * progress,
        x: from.x + (to.x - from.x) * progress,
        y: from.y + (to.y - from.y) * progress,
        scale: from.scale + (to.scale - from.scale) * progress,
        rotation_deg: from.rotation_deg + (to.rotation_deg - from.rotation_deg) * progress,
        blur: from.blur + (to.blur - from.blur) * progress,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::time::Duration;

    #[test]
    fn text_animate_defaults_match_docs() {
        let props = TextAnimateProps::new();

        assert_eq!(props.duration_secs, 0.3);
        assert_eq!(props.by, TextAnimateBy::Word);
        assert!(props.start_on_view);
        assert!(!props.once);
    }

    #[test]
    fn text_animate_can_enter_and_exit() {
        let props = TextAnimateProps::new();
        let mut state = TextAnimateState::new(&props);
        let start = Instant::now();

        state.tick(start, &props);
        state.tick(start + Duration::from_millis(800), &props);
        assert_eq!(state.phase, TextAnimatePhase::Visible);

        state.hide(start + Duration::from_millis(900));
        state.tick(start + Duration::from_millis(1800), &props);
        assert_eq!(state.phase, TextAnimatePhase::Hidden);
    }
}
