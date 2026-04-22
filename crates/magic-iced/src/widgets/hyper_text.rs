use super::text_element::TextElementRole;
use crate::primitives::{TextSegmentMode, segment_text};
use iced::Font;
use iced::time::Instant;
use iced::widget::{mouse_area, text};
use iced::{Color, Element};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HyperTextAction {
    Trigger,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HyperTextProps {
    pub duration_ms: u64,
    pub delay_ms: u64,
    pub role: TextElementRole,
    pub start_on_view: bool,
    pub animate_on_hover: bool,
    pub character_set: Vec<char>,
    pub font_size: Option<u16>,
    pub color: Option<Color>,
}

impl Default for HyperTextProps {
    fn default() -> Self {
        Self {
            duration_ms: 800,
            delay_ms: 0,
            role: TextElementRole::Div,
            start_on_view: false,
            animate_on_hover: true,
            character_set: "ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars().collect(),
            font_size: None,
            color: None,
        }
    }
}

impl HyperTextProps {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug, PartialEq)]
struct HyperTextFingerprint {
    duration_ms: u64,
    delay_ms: u64,
    start_on_view: bool,
    animate_on_hover: bool,
    character_set: Vec<char>,
}

impl From<&HyperTextProps> for HyperTextFingerprint {
    fn from(props: &HyperTextProps) -> Self {
        Self {
            duration_ms: props.duration_ms,
            delay_ms: props.delay_ms,
            start_on_view: props.start_on_view,
            animate_on_hover: props.animate_on_hover,
            character_set: props.character_set.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct HyperTextState {
    scheduled_at: Option<Instant>,
    auto_started: bool,
    fingerprint: HyperTextFingerprint,
}

impl HyperTextState {
    pub fn new(props: &HyperTextProps) -> Self {
        Self {
            scheduled_at: None,
            auto_started: false,
            fingerprint: HyperTextFingerprint::from(props),
        }
    }

    pub fn tick(&mut self, now: Instant, props: &HyperTextProps) {
        let fingerprint = HyperTextFingerprint::from(props);
        if self.fingerprint != fingerprint {
            *self = Self::new(props);
        }

        if !self.auto_started {
            self.auto_started = true;
            self.scheduled_at = Some(now + std::time::Duration::from_millis(props.delay_ms));
        }
    }

    pub fn apply_action(&mut self, action: &HyperTextAction, now: Instant, props: &HyperTextProps) {
        match action {
            HyperTextAction::Trigger => {
                self.scheduled_at = Some(now + std::time::Duration::from_millis(props.delay_ms));
            }
        }
    }

    fn progress(&self, now: Instant, props: &HyperTextProps) -> Option<f32> {
        let start = self.scheduled_at?;
        if now < start {
            return None;
        }

        let duration = props.duration_ms.max(1) as f32;
        let elapsed = now.saturating_duration_since(start).as_millis() as f32;
        Some((elapsed / duration).clamp(0.0, 1.0))
    }
}

pub fn hyper_text<'a, Message: Clone + 'a>(
    content: &'a str,
    state: &HyperTextState,
    now: Instant,
    props: &HyperTextProps,
    on_action: impl Fn(HyperTextAction) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let display = build_display_text(content, state.progress(now, props), props);
    let size = u32::from(
        props
            .font_size
            .unwrap_or(props.role.default_font_size())
            .max(1),
    );
    let color = props.color.unwrap_or(Color::WHITE);
    let body: Element<'a, Message> = text(display)
        .size(size)
        .font(Font::MONOSPACE)
        .style(move |_| iced::widget::text::Style { color: Some(color) })
        .into();

    if props.animate_on_hover {
        mouse_area(body)
            .on_enter(on_action(HyperTextAction::Trigger))
            .into()
    } else {
        body
    }
}

fn build_display_text(content: &str, progress: Option<f32>, props: &HyperTextProps) -> String {
    let units = segment_text(content, TextSegmentMode::Character);
    let mut rendered = String::with_capacity(content.len());
    let character_set = if props.character_set.is_empty() {
        &['X'][..]
    } else {
        props.character_set.as_slice()
    };

    match progress {
        None => content.to_uppercase(),
        Some(progress) => {
            let threshold = progress * units.len() as f32;
            let frame = (progress * 60.0).floor() as usize;

            for (index, unit) in units.into_iter().enumerate() {
                let reveal_unit = unit.trim().is_empty()
                    || unit.chars().count() != 1
                    || index as f32 <= threshold;
                if reveal_unit {
                    rendered.push_str(&unit.to_uppercase());
                } else {
                    let random_index = (frame + index) % character_set.len();
                    rendered.push(character_set[random_index]);
                }
            }

            rendered
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::time::Duration;

    #[test]
    fn hyper_text_defaults_match_docs() {
        let props = HyperTextProps::new();

        assert_eq!(props.duration_ms, 800);
        assert!(props.animate_on_hover);
        assert_eq!(props.character_set.len(), 26);
    }

    #[test]
    fn hyper_text_trigger_restarts_animation() {
        let props = HyperTextProps::new();
        let mut state = HyperTextState::new(&props);
        let start = Instant::now();

        state.tick(start, &props);
        state.apply_action(
            &HyperTextAction::Trigger,
            start + Duration::from_millis(100),
            &props,
        );

        let progress = state.progress(start + Duration::from_millis(200), &props);
        assert!(progress.is_some());
    }

    #[test]
    fn hyper_text_splits_emoji_sequences_into_codepoints() {
        let props = HyperTextProps::new();
        let units = segment_text("A👨‍👩‍👧‍👦B", TextSegmentMode::Character);
        let display = build_display_text("A👨‍👩‍👧‍👦B", Some(0.0), &props);

        assert_eq!(units[1], "👨");
        assert!(display.starts_with('A'));
        assert!(!display.contains("👨‍👩‍👧‍👦"));
    }
}
