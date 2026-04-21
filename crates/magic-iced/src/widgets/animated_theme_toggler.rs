use iced::time::Instant;
use iced::{Color, Element, Length};
use iced_shadcn::{LightSwitchProps, Theme, light_switch};

use crate::primitives::{AnimatedValue, MotionSpec};
use crate::theme::MagicTheme;
use twill::prelude::{Easing, TransitionDuration};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimatedThemeTogglerProps {
    pub duration_ms: u64,
    pub width: Length,
    pub height: Length,
}

impl Default for AnimatedThemeTogglerProps {
    fn default() -> Self {
        Self {
            duration_ms: 400,
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }
}

impl AnimatedThemeTogglerProps {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug)]
pub struct AnimatedThemeTogglerController {
    dark_mode: bool,
    reveal: AnimatedValue,
    started: Option<Instant>,
}

impl AnimatedThemeTogglerController {
    pub fn new(dark_mode: bool, props: &AnimatedThemeTogglerProps) -> Self {
        Self {
            dark_mode,
            reveal: AnimatedValue::with_spec(1.0, motion_spec(props.duration_ms)),
            started: None,
        }
    }

    pub fn dark_mode(&self) -> bool {
        self.dark_mode
    }

    pub fn sync(&mut self, dark_mode: bool) {
        self.dark_mode = dark_mode;
    }

    pub fn toggle(&mut self, now: Instant, props: &AnimatedThemeTogglerProps) {
        self.dark_mode = !self.dark_mode;
        self.reveal = AnimatedValue::with_spec(0.0, motion_spec(props.duration_ms));
        self.reveal.transition(1.0, now);
        self.started = Some(now);
    }

    pub fn tick(&mut self, _now: Instant, props: &AnimatedThemeTogglerProps) {
        self.reveal.set_spec(motion_spec(props.duration_ms));
    }

    pub fn reveal_at(&self, now: Instant) -> f32 {
        self.reveal.value_at(now).clamp(0.0, 1.0)
    }

    pub fn theme(&self) -> Theme {
        if self.dark_mode {
            MagicTheme::dark()
        } else {
            MagicTheme::light()
        }
    }
}

pub fn animated_theme_toggler<'a, Message: Clone + 'a>(
    dark_mode: bool,
    on_toggle: Message,
    _props: AnimatedThemeTogglerProps,
    theme: &Theme,
) -> Element<'a, Message> {
    light_switch(LightSwitchProps::new(dark_mode), on_toggle, theme)
}

pub fn animated_theme_toggler_with_controller<'a, Message: Clone + 'a>(
    controller: &AnimatedThemeTogglerController,
    now: Instant,
    on_toggle: Message,
    props: AnimatedThemeTogglerProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let reveal = controller.reveal_at(now);
    let base = animated_theme_toggler(controller.dark_mode(), on_toggle, props, theme);
    let highlight_color = if controller.dark_mode() {
        theme.palette.primary
    } else {
        theme.palette.secondary
    };

    let highlight = iced::widget::container(iced::widget::Space::new())
        .width(Length::Fixed(44.0 * reveal.max(0.05)))
        .height(Length::Fixed(24.0))
        .style(move |_| iced::widget::container::Style {
            background: Some(iced::Background::Color(Color {
                a: 0.18 * reveal,
                ..highlight_color
            })),
            border: iced::Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 12.0.into(),
            },
            ..iced::widget::container::Style::default()
        });

    iced::widget::stack(vec![highlight.into(), base]).into()
}

fn motion_spec(duration_ms: u64) -> MotionSpec {
    MotionSpec::new()
        .duration(TransitionDuration::CustomMs(
            duration_ms.clamp(1, u16::MAX as u64) as u16,
        ))
        .easing(Easing::InOut)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn animated_theme_toggler_defaults_match_docs() {
        let props = AnimatedThemeTogglerProps::new();
        assert_eq!(props.duration_ms, 400);
    }

    #[test]
    fn animated_theme_toggler_controller_toggles_mode() {
        let props = AnimatedThemeTogglerProps::new();
        let mut controller = AnimatedThemeTogglerController::new(false, &props);
        let now = Instant::now();

        controller.toggle(now, &props);
        assert!(controller.dark_mode());
        assert!(controller.reveal_at(now + Duration::from_millis(200)) > 0.0);
    }
}
