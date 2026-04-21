use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};
use iced::mouse;
use iced::widget::{canvas, container, stack};
use iced::{Color, Element, Length, Padding, Point, Rectangle, Renderer};
use iced_shadcn::{DecorativeSurfaceProps, Theme, decorative_surface};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MagicCardMode {
    #[default]
    Gradient,
    Orb,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MagicCardProps {
    pub mode: MagicCardMode,
    pub width: Length,
    pub height: Length,
    pub radius: f32,
    pub border_inset: f32,
    pub content_padding: Padding,
    pub gradient_size: f32,
    pub gradient_opacity: f32,
    pub glow_size: f32,
    pub glow_opacity: Option<f32>,
    pub background: Option<Color>,
    pub gradient_from: Option<Color>,
    pub gradient_to: Option<Color>,
    pub overlay_color: Option<Color>,
    pub glow_from: Option<Color>,
    pub glow_to: Option<Color>,
}

impl Default for MagicCardProps {
    fn default() -> Self {
        Self {
            mode: MagicCardMode::Gradient,
            width: Length::Shrink,
            height: Length::Shrink,
            radius: 24.0,
            border_inset: 1.0,
            content_padding: Padding::from(16.0),
            gradient_size: 200.0,
            gradient_opacity: 0.36,
            glow_size: 220.0,
            glow_opacity: None,
            background: None,
            gradient_from: None,
            gradient_to: None,
            overlay_color: None,
            glow_from: None,
            glow_to: None,
        }
    }
}

impl MagicCardProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mode(mut self, mode: MagicCardMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius.max(0.0);
        self
    }

    pub fn border_inset(mut self, border_inset: f32) -> Self {
        self.border_inset = border_inset.clamp(0.0, 16.0);
        self
    }

    pub fn content_padding(mut self, content_padding: impl Into<Padding>) -> Self {
        self.content_padding = content_padding.into();
        self
    }

    pub fn gradient_size(mut self, gradient_size: f32) -> Self {
        self.gradient_size = gradient_size.max(8.0);
        self
    }

    pub fn gradient_opacity(mut self, gradient_opacity: f32) -> Self {
        self.gradient_opacity = gradient_opacity.clamp(0.0, 1.0);
        self
    }

    pub fn glow_size(mut self, glow_size: f32) -> Self {
        self.glow_size = glow_size.max(16.0);
        self
    }

    pub fn glow_opacity(mut self, glow_opacity: f32) -> Self {
        self.glow_opacity = Some(glow_opacity.clamp(0.0, 1.0));
        self
    }

    pub fn background(mut self, background: Color) -> Self {
        self.background = Some(background);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ResolvedMagicCard {
    mode: MagicCardMode,
    radius: f32,
    inner_radius: f32,
    gradient_size: f32,
    gradient_from: Color,
    gradient_to: Color,
    overlay_color: Color,
    gradient_opacity: f32,
    glow_from: Color,
    glow_to: Color,
    glow_size: f32,
    glow_opacity: f32,
    background: Color,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MagicCardLayer {
    Border,
    Fill,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct MagicCardCanvasState {
    cursor: Option<Point>,
}

pub fn magic_card<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    props: MagicCardProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let resolved = resolve_magic_card(props, theme);
    let border_program = MagicCardProgram {
        resolved,
        layer: MagicCardLayer::Border,
    };
    let fill_program = MagicCardProgram {
        resolved,
        layer: MagicCardLayer::Fill,
    };

    let inner = container(stack(vec![
        container(effect_canvas::<_, Message>(
            fill_program,
            EffectCanvasProps::new()
                .width(Length::Fill)
                .height(Length::Fill),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .into(),
        container(content.into())
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(props.content_padding)
            .into(),
    ]))
    .width(Length::Fill)
    .height(Length::Fill)
    .style({
        let background = resolved.background;
        let radius = resolved.inner_radius;
        move |_| iced::widget::container::Style {
            background: Some(iced::Background::Color(background)),
            border: iced::Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: radius.into(),
            },
            ..iced::widget::container::Style::default()
        }
    });

    decorative_surface(
        inner,
        vec![
            container(effect_canvas::<_, Message>(
                border_program,
                EffectCanvasProps::new()
                    .width(Length::Fill)
                    .height(Length::Fill),
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
        ],
        Vec::new(),
        DecorativeSurfaceProps::new()
            .width(props.width)
            .height(props.height)
            .padding(props.border_inset)
            .clip(true)
            .radius(resolved.radius)
            .border_color(theme.palette.border)
            .border_width(1.0),
        theme,
    )
}

fn resolve_magic_card(props: MagicCardProps, theme: &Theme) -> ResolvedMagicCard {
    let glow_opacity = props
        .glow_opacity
        .or_else(|| theme.registry.number("magic.card.glow.opacity"))
        .unwrap_or(0.42);

    ResolvedMagicCard {
        mode: props.mode,
        radius: props.radius,
        inner_radius: (props.radius - props.border_inset).max(0.0),
        gradient_size: props.gradient_size,
        gradient_from: props
            .gradient_from
            .or_else(|| theme.registry.color("magic.gradient.from"))
            .unwrap_or(theme.palette.primary),
        gradient_to: props
            .gradient_to
            .or_else(|| theme.registry.color("magic.gradient.to"))
            .unwrap_or(theme.palette.accent),
        overlay_color: props
            .overlay_color
            .or_else(|| theme.registry.color("magic.button.shimmer"))
            .unwrap_or(theme.palette.foreground),
        gradient_opacity: props.gradient_opacity,
        glow_from: props
            .glow_from
            .or_else(|| theme.registry.color("magic.card.glow.from"))
            .unwrap_or(theme.palette.primary),
        glow_to: props
            .glow_to
            .or_else(|| theme.registry.color("magic.card.glow.to"))
            .unwrap_or(theme.palette.accent),
        glow_size: props.glow_size,
        glow_opacity,
        background: props.background.unwrap_or(theme.palette.card),
    }
}

struct MagicCardProgram {
    resolved: ResolvedMagicCard,
    layer: MagicCardLayer,
}

impl<Message> EffectProgram<Message> for MagicCardProgram {
    type State = MagicCardCanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Option<canvas::Action<Message>> {
        match event {
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let next = cursor.position_in(bounds);
                if state.cursor != next {
                    state.cursor = next;
                    return Some(canvas::Action::request_redraw());
                }
            }
            canvas::Event::Mouse(mouse::Event::CursorLeft) => {
                if state.cursor.take().is_some() {
                    return Some(canvas::Action::request_redraw());
                }
            }
            _ => {}
        }

        None
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
        let Some(cursor) = state.cursor else {
            return vec![frame.into_geometry()];
        };

        match self.resolved.mode {
            MagicCardMode::Gradient => {
                draw_gradient_layer(&mut frame, cursor, self.resolved, self.layer)
            }
            MagicCardMode::Orb => draw_orb_layer(&mut frame, cursor, self.resolved, self.layer),
        }

        vec![frame.into_geometry()]
    }
}

fn draw_gradient_layer(
    frame: &mut canvas::Frame,
    cursor: Point,
    resolved: ResolvedMagicCard,
    layer: MagicCardLayer,
) {
    let opacity_scale = match layer {
        MagicCardLayer::Border => 0.58,
        MagicCardLayer::Fill => 0.22,
    };

    for index in (0..14).rev() {
        let factor = index as f32 / 13.0;
        let radius = resolved.gradient_size * (0.18 + (1.0 - factor) * 0.82);
        let color = mix_color(resolved.gradient_from, resolved.gradient_to, 1.0 - factor);

        frame.fill(
            &canvas::Path::circle(cursor, radius),
            Color {
                a: resolved.gradient_opacity * opacity_scale * (1.0 - factor).powi(2),
                ..color
            },
        );
    }

    if matches!(layer, MagicCardLayer::Fill) {
        frame.fill(
            &canvas::Path::circle(cursor, resolved.gradient_size * 0.32),
            Color {
                a: resolved.gradient_opacity * 0.18,
                ..resolved.overlay_color
            },
        );
    }
}

fn draw_orb_layer(
    frame: &mut canvas::Frame,
    cursor: Point,
    resolved: ResolvedMagicCard,
    layer: MagicCardLayer,
) {
    let opacity_scale = match layer {
        MagicCardLayer::Border => 0.72,
        MagicCardLayer::Fill => 0.40,
    };

    for index in (0..18).rev() {
        let factor = index as f32 / 17.0;
        let radius = resolved.glow_size * (0.15 + (1.0 - factor) * 0.85);
        let color = mix_color(resolved.glow_from, resolved.glow_to, factor);

        frame.fill(
            &canvas::Path::circle(cursor, radius),
            Color {
                a: resolved.glow_opacity * opacity_scale * (1.0 - factor).powi(2),
                ..color
            },
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::canvas;

    #[test]
    fn magic_card_defaults_are_stable() {
        let props = MagicCardProps::new();

        assert_eq!(props.mode, MagicCardMode::Gradient);
        assert_eq!(props.radius, 24.0);
        assert_eq!(props.border_inset, 1.0);
        assert_eq!(props.gradient_size, 200.0);
    }

    #[test]
    fn resolve_magic_card_uses_phase0_theme_registry_and_local_fallbacks() {
        let theme = crate::theme::MagicTheme::dark();
        let resolved = resolve_magic_card(MagicCardProps::new(), &theme);

        assert_eq!(resolved.gradient_from, theme.palette.primary);
        assert_eq!(
            resolved.gradient_to,
            theme.registry.color("magic.gradient.to").unwrap()
        );
        assert_eq!(resolved.overlay_color, theme.palette.foreground);
        assert_eq!(resolved.glow_from, theme.palette.primary);
        assert_eq!(resolved.glow_to, theme.palette.accent);
        assert_eq!(resolved.glow_opacity, 0.42);
    }

    #[test]
    fn magic_card_canvas_updates_cursor_and_resets_on_leave() {
        let theme = crate::theme::MagicTheme::dark();
        let resolved = resolve_magic_card(MagicCardProps::new(), &theme);
        let program = MagicCardProgram {
            resolved,
            layer: MagicCardLayer::Fill,
        };
        let mut state = MagicCardCanvasState::default();
        let bounds = Rectangle::new(iced::Point::ORIGIN, iced::Size::new(200.0, 120.0));

        let moved: Option<canvas::Action<()>> = program.update(
            &mut state,
            &canvas::Event::Mouse(mouse::Event::CursorMoved {
                position: iced::Point::new(40.0, 30.0),
            }),
            bounds,
            mouse::Cursor::Available(iced::Point::new(40.0, 30.0)),
            &bounds,
        );
        assert!(moved.is_some());
        assert_eq!(state.cursor, Some(iced::Point::new(40.0, 30.0)));

        let left: Option<canvas::Action<()>> = program.update(
            &mut state,
            &canvas::Event::Mouse(mouse::Event::CursorLeft),
            bounds,
            mouse::Cursor::Unavailable,
            &bounds,
        );
        assert!(left.is_some());
        assert_eq!(state.cursor, None);
    }
}
