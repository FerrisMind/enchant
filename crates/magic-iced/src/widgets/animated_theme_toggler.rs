use iced::mouse;
use iced::time::Instant;
use iced::widget::{button, canvas, container, svg};
use iced::{Color, Element, Length, Point, Rectangle, Renderer};
use iced_shadcn::Theme;

use crate::primitives::{AnimatedValue, MotionSpec};
use crate::theme::MagicTheme;
use twill::prelude::{Easing, TransitionDuration};

const ICON_SIZE: f32 = 24.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimatedThemeTogglerProps {
    pub duration_ms: u64,
    pub width: Length,
    pub height: Length,
    /// When true, the reveal expands from the viewport center.
    pub from_center: bool,
}

impl Default for AnimatedThemeTogglerProps {
    fn default() -> Self {
        Self {
            duration_ms: 400,
            width: Length::Shrink,
            height: Length::Shrink,
            from_center: false,
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
    origin: Option<Point>,
}

impl AnimatedThemeTogglerController {
    pub fn new(dark_mode: bool, props: &AnimatedThemeTogglerProps) -> Self {
        Self {
            dark_mode,
            reveal: AnimatedValue::with_spec(1.0, motion_spec(props.duration_ms)),
            started: None,
            origin: None,
        }
    }

    pub fn dark_mode(&self) -> bool {
        self.dark_mode
    }

    pub fn sync(&mut self, dark_mode: bool) {
        self.dark_mode = dark_mode;
    }

    pub fn set_origin(&mut self, origin: Point) {
        self.origin = Some(origin);
    }

    pub fn toggle(&mut self, now: Instant, props: &AnimatedThemeTogglerProps) {
        self.dark_mode = !self.dark_mode;
        self.reveal = AnimatedValue::with_spec(0.0, motion_spec(props.duration_ms));
        self.reveal.transition(1.0, now);
        self.started = Some(now);
    }

    pub fn tick(&mut self, now: Instant, props: &AnimatedThemeTogglerProps) {
        self.reveal.set_spec(motion_spec(props.duration_ms));
        if self.started.is_some() && !self.reveal.is_animating(now) {
            self.started = None;
        }
    }

    pub fn reveal_at(&self, now: Instant) -> f32 {
        self.reveal.value_at(now).clamp(0.0, 1.0)
    }

    pub fn is_transition_active(&self, now: Instant) -> bool {
        self.started.is_some() && self.reveal.is_animating(now)
    }

    /// Theme painted by the page while the transition runs (old theme until reveal ends).
    pub fn display_theme(&self, now: Instant) -> Theme {
        if self.is_transition_active(now) {
            theme_for_mode(!self.dark_mode)
        } else {
            theme_for_mode(self.dark_mode)
        }
    }

    /// Theme used for the toggler icon after click (matches post-toggle mode).
    pub fn icon_theme(&self) -> Theme {
        theme_for_mode(self.dark_mode)
    }

    pub fn theme(&self) -> Theme {
        theme_for_mode(self.dark_mode)
    }

    pub fn transition_background(&self) -> Color {
        theme_for_mode(self.dark_mode).palette.background
    }
}

/// Magic UI `AnimatedThemeToggler` — plain `<button>` with Lucide `Sun` / `Moon`.
pub fn animated_theme_toggler<'a, Message: Clone + 'a>(
    dark_mode: bool,
    on_toggle: Message,
    props: AnimatedThemeTogglerProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let icon = theme_icon::<Message>(dark_mode, theme.palette.foreground);
    let foreground = theme.palette.foreground;

    let control = button(icon)
        .on_press(on_toggle)
        .padding(0)
        .style(move |_, _| button::Style {
            background: None,
            border: iced::Border::default(),
            text_color: foreground,
            shadow: iced::Shadow::default(),
            snap: false,
        });

    container(control)
        .width(props.width)
        .height(props.height)
        .into()
}

pub fn animated_theme_toggler_with_controller<'a, Message: Clone + 'a>(
    controller: &AnimatedThemeTogglerController,
    _now: Instant,
    on_toggle: Message,
    props: AnimatedThemeTogglerProps,
    theme: &Theme,
) -> Element<'a, Message> {
    animated_theme_toggler(controller.dark_mode(), on_toggle, props, theme)
}

/// Circle reveal for the new theme (`::view-transition-new(root)`, circle variant).
pub fn theme_transition_overlay<'a, Message: 'a>(
    controller: &AnimatedThemeTogglerController,
    now: Instant,
    props: &AnimatedThemeTogglerProps,
) -> Option<Element<'a, Message>> {
    if !controller.is_transition_active(now) {
        return None;
    }

    let reveal = controller.reveal_at(now);
    let background = controller.transition_background();

    Some(
        container(
            canvas(ThemeTransitionProgram {
                reveal,
                background,
                origin: controller.origin,
                from_center: props.from_center,
            })
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into(),
    )
}

#[derive(Clone, Copy, Debug)]
struct ThemeTransitionProgram {
    reveal: f32,
    background: Color,
    origin: Option<Point>,
    from_center: bool,
}

impl<Message> canvas::Program<Message> for ThemeTransitionProgram {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let (cx, cy) = transition_center(bounds, self.origin, self.from_center);

        let max_radius = hypot(
            cx.max(bounds.width - cx),
            cy.max(bounds.height - cy),
        );
        let radius = max_radius * self.reveal;

        frame.fill(
            &canvas::Path::circle(Point::new(cx, cy), radius.max(0.0)),
            self.background,
        );

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &(),
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        mouse::Interaction::None
    }
}

fn transition_center(bounds: Rectangle, origin: Option<Point>, from_center: bool) -> (f32, f32) {
    if from_center {
        return (bounds.width / 2.0, bounds.height / 2.0);
    }

    if let Some(origin) = origin {
        return (origin.x, origin.y);
    }

    (bounds.width / 2.0, bounds.height / 2.0)
}

fn theme_for_mode(dark_mode: bool) -> Theme {
    if dark_mode {
        MagicTheme::dark()
    } else {
        MagicTheme::light()
    }
}

fn theme_icon<'a, Message: 'a>(dark_mode: bool, color: Color) -> Element<'a, Message> {
    let handle = if dark_mode {
        sun_icon(color)
    } else {
        moon_icon(color)
    };

    svg(handle)
        .width(Length::Fixed(ICON_SIZE))
        .height(Length::Fixed(ICON_SIZE))
        .into()
}

fn sun_icon(color: Color) -> svg::Handle {
    let [r, g, b, _] = color.into_rgba8();
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="rgb({r},{g},{b})" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/></svg>"#
    );
    svg::Handle::from_memory(svg.into_bytes())
}

fn moon_icon(color: Color) -> svg::Handle {
    let [r, g, b, _] = color.into_rgba8();
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="rgb({r},{g},{b})" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"/></svg>"#
    );
    svg::Handle::from_memory(svg.into_bytes())
}

fn hypot(x: f32, y: f32) -> f32 {
    (x * x + y * y).sqrt()
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
        assert!(!props.from_center);
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

    #[test]
    fn display_theme_stays_old_until_transition_finishes() {
        let props = AnimatedThemeTogglerProps::new();
        let mut controller = AnimatedThemeTogglerController::new(false, &props);
        let now = Instant::now();

        controller.toggle(now, &props);

        assert!(
            !controller
                .display_theme(now + Duration::from_millis(100))
                .variant()
                .is_dark()
        );
        assert!(
            controller
                .display_theme(now + Duration::from_millis(500))
                .variant()
                .is_dark()
        );
    }
}
