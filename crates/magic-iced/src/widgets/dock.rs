use crate::primitives::{pointer_distance, pointer_strength};
use iced::mouse;
use iced::widget::{container, mouse_area, row};
use iced::{Background, Color, Element, Length};
use iced_shadcn::Theme;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DockAction {
    Move(f32),
    Exit,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct DockState {
    pointer_x: Option<f32>,
}

impl DockState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_action(&mut self, action: &DockAction) {
        match action {
            DockAction::Move(x) => self.pointer_x = Some(*x),
            DockAction::Exit => self.pointer_x = None,
        }
    }

    pub fn pointer_x(&self) -> Option<f32> {
        self.pointer_x
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DockDirection {
    Top,
    #[default]
    Middle,
    Bottom,
}

pub struct DockIcon<'a, Message> {
    pub content: Element<'a, Message>,
    pub on_press: Option<Message>,
    pub background: Option<Color>,
}

impl<'a, Message> DockIcon<'a, Message> {
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    pub fn background(mut self, background: Color) -> Self {
        self.background = Some(background);
        self
    }
}

pub fn dock_icon<'a, Message>(content: impl Into<Element<'a, Message>>) -> DockIcon<'a, Message> {
    DockIcon {
        content: content.into(),
        on_press: None,
        background: None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DockProps {
    pub icon_size: f32,
    pub icon_magnification: f32,
    pub disable_magnification: bool,
    pub icon_distance: f32,
    pub direction: DockDirection,
    pub gap: f32,
    pub padding: f32,
    pub background: Option<Color>,
}

impl Default for DockProps {
    fn default() -> Self {
        Self {
            icon_size: 40.0,
            icon_magnification: 60.0,
            disable_magnification: false,
            icon_distance: 140.0,
            direction: DockDirection::Middle,
            gap: 8.0,
            padding: 8.0,
            background: None,
        }
    }
}

impl DockProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn dock<'a, Message: Clone + 'a>(
    items: Vec<DockIcon<'a, Message>>,
    state: &DockState,
    props: DockProps,
    on_action: impl Fn(DockAction) -> Message + Clone + 'a,
    theme: &Theme,
) -> Element<'a, Message> {
    let max_extent = props.icon_size.max(if props.disable_magnification {
        props.icon_size
    } else {
        props.icon_magnification
    });
    let slot_extent = max_extent + props.padding * 2.0;
    let surface_background = props
        .background
        .or_else(|| theme.registry.color("magic.dock.surface"))
        .unwrap_or(Color {
            a: if theme.palette.background.a < 1.0 {
                0.18
            } else {
                0.12
            },
            ..theme.palette.background
        });
    let border_color = Color {
        a: 0.2,
        ..theme.palette.border
    };
    let radius = theme.radius.lg;

    let mut content = row![].spacing(props.gap).padding(props.padding);

    for (index, item) in items.into_iter().enumerate() {
        let center = slot_center(index, slot_extent, props.gap);
        let size = dock_icon_size(state.pointer_x(), center, props);
        let scale = size / max_extent.max(1.0);
        let background = item
            .background
            .or_else(|| theme.registry.color("magic.dock.icon.background"))
            .unwrap_or(Color {
                a: 0.12,
                ..theme.palette.foreground
            });

        let shell = container(item.content)
            .width(Length::Fixed(max_extent))
            .height(Length::Fixed(max_extent))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(background)),
                border: iced::Border {
                    color: Color {
                        a: 0.24,
                        ..Color::WHITE
                    },
                    width: 1.0,
                    radius: (max_extent / 2.0).into(),
                },
                ..iced::widget::container::Style::default()
            });

        let scaled: Element<'a, Message> = iced::widget::float(shell).scale(scale).into();

        let slot: Element<'a, Message> = container(scaled)
            .width(Length::Fixed(slot_extent))
            .height(Length::Fixed(slot_extent))
            .center_x(Length::Fill)
            .center_y(match props.direction {
                DockDirection::Top => Length::Shrink,
                DockDirection::Middle => Length::Fill,
                DockDirection::Bottom => Length::Shrink,
            })
            .into();

        let slot = if let Some(message) = item.on_press {
            mouse_area(slot)
                .on_press(message)
                .interaction(mouse::Interaction::Pointer)
                .into()
        } else {
            slot
        };

        content = content.push(slot);
    }

    mouse_area(
        container(content).style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(surface_background)),
            border: iced::Border {
                color: border_color,
                width: 1.0,
                radius: radius.into(),
            },
            ..iced::widget::container::Style::default()
        }),
    )
    .on_move({
        let on_action = on_action.clone();
        move |position| on_action(DockAction::Move(position.x))
    })
    .on_exit(on_action(DockAction::Exit))
    .interaction(mouse::Interaction::Pointer)
    .into()
}

fn slot_center(index: usize, slot_extent: f32, gap: f32) -> f32 {
    index as f32 * (slot_extent + gap) + slot_extent / 2.0
}

fn dock_icon_size(pointer_x: Option<f32>, center: f32, props: DockProps) -> f32 {
    if props.disable_magnification {
        return props.icon_size;
    }

    let Some(pointer_x) = pointer_x else {
        return props.icon_size;
    };

    let strength = pointer_strength(
        pointer_distance(pointer_x, center),
        props.icon_distance.max(1.0),
    );

    props.icon_size + (props.icon_magnification - props.icon_size) * strength
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dock_size_peaks_at_cursor_center() {
        let props = DockProps::new();
        let center = 32.0;

        assert_eq!(
            dock_icon_size(Some(center), center, props),
            props.icon_magnification
        );
        assert_eq!(dock_icon_size(None, center, props), props.icon_size);
    }
}
