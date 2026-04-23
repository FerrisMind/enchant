use iced::widget::{container, row, text};
use iced::{Element, Length};
use iced_shadcn::{ButtonProps, ButtonSize, ButtonVariant, Theme, button_content};

#[derive(Clone, Copy, Debug)]
pub struct InteractiveHoverButtonProps {
    pub button: ButtonProps,
    pub dot_size: f32,
    pub radius: f32,
    pub gap: f32,
}

impl Default for InteractiveHoverButtonProps {
    fn default() -> Self {
        Self {
            button: ButtonProps::new()
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Size2),
            dot_size: 8.0,
            radius: 999.0,
            gap: 8.0,
        }
    }
}

impl InteractiveHoverButtonProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn button(mut self, button: ButtonProps) -> Self {
        self.button = button;
        self
    }
}

pub fn interactive_hover_button<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    on_press: Option<Message>,
    props: InteractiveHoverButtonProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let dot = container(text(" "))
        .width(Length::Fixed(props.dot_size))
        .height(Length::Fixed(props.dot_size))
        .style({
            let color = theme.palette.primary;
            move |_| iced::widget::container::Style {
                background: Some(iced::Background::Color(color)),
                border: iced::Border {
                    color,
                    width: 0.0,
                    radius: props.radius.into(),
                },
                ..iced::widget::container::Style::default()
            }
        });

    let arrow = text("→");

    button_content(
        row![dot, content.into(), arrow].spacing(props.gap),
        on_press,
        props.button,
        theme,
    )
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_button_defaults_match_shell_shape() {
        let props = InteractiveHoverButtonProps::new();

        assert_eq!(props.dot_size, 8.0);
        assert_eq!(props.radius, 999.0);
    }
}
