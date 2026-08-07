use crate::primitives::{EffectCanvasProps, EffectProgram, PointerMotion, effect_canvas};
use iced::mouse;
use iced::time::Instant;
use iced::widget::{mouse_area, stack};
use iced::{Color, Element, Rectangle};
use iced_shadcn::Theme;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GlareHoverAction {
    Move(iced::Point),
    Exit,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct GlareHoverState {
    pointer: Option<iced::Point>,
    hovered: bool,
    started_at: Option<Instant>,
    motion: PointerMotion,
}

impl GlareHoverState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_action(&mut self, action: &GlareHoverAction, now: Instant) {
        match action {
            GlareHoverAction::Move(position) => {
                if !self.hovered {
                    self.started_at = Some(now);
                }
                self.hovered = true;
                self.pointer = Some(*position);
                self.motion.update(*position, now);
            }
            GlareHoverAction::Exit => {
                self.hovered = false;
                self.pointer = None;
                self.motion.reset();
            }
        }
    }

    fn progress(&self, now: Instant, props: &GlareHoverProps) -> f32 {
        let Some(started_at) = self.started_at else {
            return 0.0;
        };
        let duration = (props.duration_ms.max(1) as f32 / 1000.0).max(0.05);
        let local = now.saturating_duration_since(started_at).as_secs_f32() / duration;
        if props.play_once {
            local.clamp(0.0, 1.0)
        } else {
            local.fract()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlareHoverProps {
    pub background: Option<Color>,
    pub color: Option<Color>,
    pub opacity: f32,
    pub angle_deg: f32,
    pub size: f32,
    pub duration_ms: u64,
    pub play_once: bool,
}

impl Default for GlareHoverProps {
    fn default() -> Self {
        Self {
            background: None,
            color: None,
            opacity: 0.5,
            angle_deg: -45.0,
            size: 250.0,
            duration_ms: 650,
            play_once: false,
        }
    }
}

impl GlareHoverProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn glare_hover<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    state: &GlareHoverState,
    now: Instant,
    props: GlareHoverProps,
    on_action: impl Fn(GlareHoverAction) -> Message + Clone + 'a,
    theme: &Theme,
) -> Element<'a, Message> {
    let overlay: Element<'a, Message> = effect_canvas::<_, Message>(
        GlareHoverProgram {
            pointer: state.pointer,
            progress: state.progress(now, &props),
            props,
            color: props
                .color
                .or_else(|| theme.registry.color("magic.glare.color"))
                .unwrap_or(Color::WHITE),
            background: props.background.unwrap_or(theme.palette.background),
        },
        EffectCanvasProps::new(),
    )
    .into();

    mouse_area(stack(vec![content.into(), overlay]))
        .on_move({
            let on_action = on_action.clone();
            move |position| on_action(GlareHoverAction::Move(position))
        })
        .on_exit(on_action(GlareHoverAction::Exit))
        .interaction(mouse::Interaction::Pointer)
        .into()
}

struct GlareHoverProgram {
    pointer: Option<iced::Point>,
    progress: f32,
    props: GlareHoverProps,
    color: Color,
    background: Color,
}

impl<Message> EffectProgram<Message> for GlareHoverProgram {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<iced::widget::canvas::Geometry> {
        let mut frame = iced::widget::canvas::Frame::new(renderer, bounds.size());
        let Some(pointer) = self.pointer else {
            return vec![frame.into_geometry()];
        };

        let sweep = (self.progress * 2.0 - 1.0) * self.props.size.max(80.0);
        frame.with_save(|frame| {
            frame.translate(iced::Vector::new(pointer.x + sweep, pointer.y));
            frame.rotate(self.props.angle_deg.to_radians());

            let band = iced::widget::canvas::Path::rectangle(
                iced::Point::new(-self.props.size * 0.5, -bounds.height),
                iced::Size::new(self.props.size, bounds.height * 2.0),
            );

            for layer in 0..3 {
                let alpha = self.props.opacity.clamp(0.0, 1.0) * (0.18 / (layer + 1) as f32);
                frame.fill(
                    &band,
                    Color {
                        a: alpha,
                        ..self.color
                    },
                );
            }
        });

        let _ = self.background;

        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glare_hover_defaults_match_docs() {
        let props = GlareHoverProps::new();

        assert_eq!(props.opacity, 0.5);
        assert_eq!(props.angle_deg, -45.0);
        assert_eq!(props.size, 250.0);
        assert_eq!(props.duration_ms, 650);
    }

    #[test]
    fn glare_hover_state_starts_and_resets() {
        let mut state = GlareHoverState::new();
        let now = Instant::now();
        state.apply_action(&GlareHoverAction::Move(iced::Point::new(5.0, 6.0)), now);
        assert!(state.hovered);
        assert_eq!(state.pointer, Some(iced::Point::new(5.0, 6.0)));

        state.apply_action(&GlareHoverAction::Exit, now);
        assert!(!state.hovered);
        assert_eq!(state.pointer, None);
    }
}
