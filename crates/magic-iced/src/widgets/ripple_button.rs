use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};
use iced::mouse;
use iced::time::Instant;
use iced::widget::{canvas, container, mouse_area};
use iced::{Color, Element, Length, Point, Rectangle, Renderer};
use iced_shadcn::{ButtonProps, DecorativeSurfaceProps, Theme, button_content, decorative_surface};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RippleButtonAction {
    Move(Point),
    Exit,
    Press,
}

#[derive(Clone, Debug, PartialEq)]
struct RippleBurst {
    started_at: Instant,
    origin: Point,
}

#[derive(Clone, Debug, Default)]
pub struct RippleButtonState {
    pointer: Option<Point>,
    bursts: Vec<RippleBurst>,
}

impl RippleButtonState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick(&mut self, now: Instant, props: &RippleButtonProps) {
        self.bursts.retain(|burst| {
            now.saturating_duration_since(burst.started_at).as_millis() as u64
                <= props.duration_ms
        });
    }

    pub fn apply_action(
        &mut self,
        action: &RippleButtonAction,
        now: Instant,
        props: &RippleButtonProps,
    ) {
        match action {
            RippleButtonAction::Move(position) => self.pointer = Some(*position),
            RippleButtonAction::Exit => self.pointer = None,
            RippleButtonAction::Press => {
                let origin = self.pointer.unwrap_or(Point::new(0.5, 0.5));
                self.bursts.push(RippleBurst {
                    started_at: now,
                    origin,
                });
                self.tick(now, props);
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RippleButtonProps {
    pub button: ButtonProps,
    pub duration_ms: u64,
    pub ripple_color: Option<Color>,
    pub radius: f32,
}

impl Default for RippleButtonProps {
    fn default() -> Self {
        Self {
            button: ButtonProps::new(),
            duration_ms: 600,
            ripple_color: None,
            radius: 8.0,
        }
    }
}

impl RippleButtonProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn button(mut self, button: ButtonProps) -> Self {
        self.button = button;
        self
    }
}

pub fn ripple_button<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    state: &'a RippleButtonState,
    now: Instant,
    props: RippleButtonProps,
    on_action: impl Fn(RippleButtonAction) -> Message + Clone + 'a,
    theme: &Theme,
) -> Element<'a, Message> {
    let color = props
        .ripple_color
        .or_else(|| theme.registry.color("magic.button.ripple"))
        .unwrap_or(theme.palette.foreground);

    let ripples = container(effect_canvas::<_, Message>(
        RippleButtonProgram {
            now,
            bursts: &state.bursts,
            color,
            duration_ms: props.duration_ms,
        },
        EffectCanvasProps::new()
            .width(Length::Fill)
            .height(Length::Fill),
    ))
    .width(Length::Fill)
    .height(Length::Fill);

    let shell = decorative_surface(
        button_content(content, None::<Message>, props.button, theme),
        Vec::new(),
        vec![ripples.into()],
        DecorativeSurfaceProps::new()
            .clip(true)
            .radius(props.radius),
        theme,
    );

    mouse_area(shell)
        .on_move({
            let on_action = on_action.clone();
            move |position| on_action(RippleButtonAction::Move(position))
        })
        .on_exit(on_action(RippleButtonAction::Exit))
        .on_press(on_action(RippleButtonAction::Press))
        .interaction(mouse::Interaction::Pointer)
        .into()
}

struct RippleButtonProgram<'a> {
    now: Instant,
    bursts: &'a [RippleBurst],
    color: Color,
    duration_ms: u64,
}

impl<Message> EffectProgram<Message> for RippleButtonProgram<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let duration = self.duration_ms.max(1) as f32;

        for burst in self.bursts {
            let elapsed_ms = self
                .now
                .saturating_duration_since(burst.started_at)
                .as_millis() as f32;
            let progress = (elapsed_ms / duration).clamp(0.0, 1.0);
            let size = bounds.width.max(bounds.height) * (0.2 + progress * 1.2);
            let alpha = (1.0 - progress).powf(1.6) * 0.24;
            let origin = Point::new(
                bounds.width * burst.origin.x.clamp(0.0, 1.0),
                bounds.height * burst.origin.y.clamp(0.0, 1.0),
            );

            frame.stroke(
                &canvas::Path::circle(origin, size),
                canvas::Stroke::default()
                    .with_width(2.0)
                    .with_color(Color { a: alpha, ..self.color }),
            );
        }

        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn ripple_button_defaults_are_stable() {
        let props = RippleButtonProps::new();

        assert_eq!(props.duration_ms, 600);
        assert_eq!(props.radius, 8.0);
    }

    #[test]
    fn ripple_button_state_creates_and_prunes_bursts() {
        let props = RippleButtonProps::new();
        let now = Instant::now();
        let mut state = RippleButtonState::new();

        state.apply_action(&RippleButtonAction::Move(Point::new(0.25, 0.75)), now, &props);
        state.apply_action(&RippleButtonAction::Press, now, &props);

        assert_eq!(state.bursts.len(), 1);

        state.tick(now + Duration::from_millis(props.duration_ms + 1), &props);
        assert!(state.bursts.is_empty());
    }
}
