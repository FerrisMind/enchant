use crate::primitives::{
    AnimatedValue, EffectCanvasProps, EffectProgram, MotionSpec, effect_canvas,
};
use iced::time::Instant;
use iced::widget::{canvas, container};
use iced::{Color, Element, Length, Padding, Rectangle, Renderer};
use iced_shadcn::{DecorativeSurfaceProps, Theme, decorative_surface};
use twill::prelude::{Easing, TransitionDuration};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HighlighterAction {
    #[default]
    Highlight,
    Underline,
    Box,
    Circle,
    StrikeThrough,
    CrossedOff,
    Bracket,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HighlighterProps {
    pub action: HighlighterAction,
    pub color: Color,
    pub stroke_width: f32,
    pub animation_duration_ms: u64,
    pub iterations: u8,
    pub padding: f32,
    pub multiline: bool,
    pub is_view: bool,
}

impl Default for HighlighterProps {
    fn default() -> Self {
        Self {
            action: HighlighterAction::Highlight,
            color: Color::from_rgb8(0xFF, 0xD1, 0xDC),
            stroke_width: 1.5,
            animation_duration_ms: 600,
            iterations: 2,
            padding: 2.0,
            multiline: true,
            is_view: false,
        }
    }
}

impl HighlighterProps {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct HighlighterFingerprint {
    action: HighlighterAction,
    color: Color,
    stroke_width: f32,
    animation_duration_ms: u64,
    iterations: u8,
    padding: f32,
    multiline: bool,
    is_view: bool,
}

impl From<&HighlighterProps> for HighlighterFingerprint {
    fn from(props: &HighlighterProps) -> Self {
        Self {
            action: props.action,
            color: props.color,
            stroke_width: props.stroke_width,
            animation_duration_ms: props.animation_duration_ms,
            iterations: props.iterations,
            padding: props.padding,
            multiline: props.multiline,
            is_view: props.is_view,
        }
    }
}

#[derive(Clone, Debug)]
pub struct HighlighterState {
    progress: AnimatedValue,
    started: bool,
    fingerprint: HighlighterFingerprint,
}

impl HighlighterState {
    pub fn new(props: &HighlighterProps) -> Self {
        Self {
            progress: AnimatedValue::with_spec(
                0.0,
                MotionSpec::new()
                    .duration(TransitionDuration::CustomMs(
                        props.animation_duration_ms.min(u16::MAX as u64) as u16,
                    ))
                    .easing(Easing::Out),
            ),
            started: false,
            fingerprint: HighlighterFingerprint::from(props),
        }
    }

    pub fn tick(&mut self, now: Instant, props: &HighlighterProps) {
        let fingerprint = HighlighterFingerprint::from(props);
        if self.fingerprint != fingerprint {
            *self = Self::new(props);
        }

        self.progress.set_spec(
            MotionSpec::new()
                .duration(TransitionDuration::CustomMs(
                    props.animation_duration_ms.min(u16::MAX as u64) as u16,
                ))
                .easing(Easing::Out),
        );

        if !self.started {
            self.started = true;
            self.progress.transition(1.0, now);
        }
    }

    pub fn progress_at(&self, now: Instant) -> f32 {
        self.progress.value_at(now).clamp(0.0, 1.0)
    }
}

pub fn highlighter<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    state: &HighlighterState,
    now: Instant,
    props: &HighlighterProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let build_overlay = || {
        container(effect_canvas::<_, Message>(
            HighlighterProgram {
                progress: state.progress_at(now),
                props: *props,
            },
            EffectCanvasProps::new()
                .width(Length::Fill)
                .height(Length::Fill),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    };

    let underlays = if matches!(props.action, HighlighterAction::Highlight) {
        vec![build_overlay()]
    } else {
        Vec::new()
    };
    let overlays = if matches!(props.action, HighlighterAction::Highlight) {
        Vec::new()
    } else {
        vec![build_overlay()]
    };

    decorative_surface(
        container(content.into())
            .padding(Padding::from(props.padding))
            .width(Length::Shrink)
            .height(Length::Shrink),
        underlays,
        overlays,
        DecorativeSurfaceProps::new().clip(false),
        theme,
    )
}

struct HighlighterProgram {
    progress: f32,
    props: HighlighterProps,
}

impl<Message> EffectProgram<Message> for HighlighterProgram {
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
        let progress = self.progress.clamp(0.0, 1.0);

        for iteration in 0..self.props.iterations.max(1) {
            let local = (progress - iteration as f32 * 0.08).clamp(0.0, 1.0);
            if local <= 0.0 {
                continue;
            }

            match self.props.action {
                HighlighterAction::Highlight => {
                    frame.fill_rectangle(
                        iced::Point::ORIGIN,
                        bounds.size(),
                        Color {
                            a: 0.14 * local,
                            ..self.props.color
                        },
                    );
                }
                HighlighterAction::Underline => draw_partial_line(
                    &mut frame,
                    iced::Point::new(0.0, bounds.height - self.props.stroke_width),
                    iced::Point::new(bounds.width, bounds.height - self.props.stroke_width),
                    self.props.color,
                    self.props.stroke_width,
                    local,
                ),
                HighlighterAction::StrikeThrough => draw_partial_line(
                    &mut frame,
                    iced::Point::new(0.0, bounds.height * 0.5),
                    iced::Point::new(bounds.width, bounds.height * 0.5),
                    self.props.color,
                    self.props.stroke_width,
                    local,
                ),
                HighlighterAction::CrossedOff => {
                    draw_partial_line(
                        &mut frame,
                        iced::Point::new(0.0, 0.0),
                        iced::Point::new(bounds.width, bounds.height),
                        self.props.color,
                        self.props.stroke_width,
                        local,
                    );
                    draw_partial_line(
                        &mut frame,
                        iced::Point::new(bounds.width, 0.0),
                        iced::Point::new(0.0, bounds.height),
                        self.props.color,
                        self.props.stroke_width,
                        local,
                    );
                }
                HighlighterAction::Box => {
                    frame.stroke(
                        &canvas::Path::rectangle(iced::Point::ORIGIN, bounds.size()),
                        canvas::Stroke::default()
                            .with_width(self.props.stroke_width)
                            .with_color(Color {
                                a: local,
                                ..self.props.color
                            }),
                    );
                }
                HighlighterAction::Circle => {
                    frame.stroke(
                        &canvas::Path::circle(
                            iced::Point::new(bounds.width * 0.5, bounds.height * 0.5),
                            bounds.width.max(bounds.height) * 0.5,
                        ),
                        canvas::Stroke::default()
                            .with_width(self.props.stroke_width)
                            .with_color(Color {
                                a: local,
                                ..self.props.color
                            }),
                    );
                }
                HighlighterAction::Bracket => {
                    let x = self.props.stroke_width * 2.0;
                    let y = self.props.stroke_width;
                    draw_partial_line(
                        &mut frame,
                        iced::Point::new(x, y),
                        iced::Point::new(x, bounds.height - y),
                        self.props.color,
                        self.props.stroke_width,
                        local,
                    );
                    draw_partial_line(
                        &mut frame,
                        iced::Point::new(bounds.width - x, y),
                        iced::Point::new(bounds.width - x, bounds.height - y),
                        self.props.color,
                        self.props.stroke_width,
                        local,
                    );
                }
            }
        }

        vec![frame.into_geometry()]
    }
}

fn draw_partial_line(
    frame: &mut canvas::Frame,
    from: iced::Point,
    to: iced::Point,
    color: Color,
    width: f32,
    progress: f32,
) {
    let end = iced::Point::new(
        from.x + (to.x - from.x) * progress,
        from.y + (to.y - from.y) * progress,
    );
    frame.stroke(
        &canvas::Path::line(from, end),
        canvas::Stroke::default()
            .with_width(width.max(1.0))
            .with_color(color),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::time::Duration;

    #[test]
    fn highlighter_defaults_match_docs() {
        let props = HighlighterProps::new();

        assert_eq!(props.action, HighlighterAction::Highlight);
        assert_eq!(props.stroke_width, 1.5);
        assert_eq!(props.animation_duration_ms, 600);
        assert_eq!(props.iterations, 2);
    }

    #[test]
    fn highlighter_progress_advances() {
        let props = HighlighterProps::new();
        let mut state = HighlighterState::new(&props);
        let start = Instant::now();

        state.tick(start, &props);
        assert!(state.progress_at(start + Duration::from_millis(700)) > 0.9);
    }
}
