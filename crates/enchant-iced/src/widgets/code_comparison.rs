use iced::widget::{column, container, row, scrollable, text};
use iced::{Color, Element, Length};
use iced_shadcn::Theme;

#[derive(Clone, Debug, PartialEq)]
pub struct CodeComparisonProps {
    pub filename: String,
    pub width: Length,
    pub height: Length,
    pub highlight_color: Color,
}

impl Default for CodeComparisonProps {
    fn default() -> Self {
        Self {
            filename: String::from("example.tsx"),
            width: Length::Fill,
            height: Length::Shrink,
            highlight_color: Color::from_rgb8(0xFF, 0x33, 0x33),
        }
    }
}

impl CodeComparisonProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn code_comparison<'a, Message: 'a>(
    before_code: impl AsRef<str>,
    after_code: impl AsRef<str>,
    props: CodeComparisonProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let before = render_pane("before", before_code.as_ref(), props.highlight_color, theme);
    let after = render_pane("after", after_code.as_ref(), props.highlight_color, theme);

    container(
        row![before, after]
            .spacing(0)
            .width(props.width)
            .height(props.height),
    )
    .width(props.width)
    .height(props.height)
    .into()
}

fn render_pane<'a, Message: 'a>(
    label: &'a str,
    code: &str,
    highlight_color: Color,
    theme: &Theme,
) -> Element<'a, Message> {
    let lines: Vec<&str> = code.lines().collect();
    let pane = column![
        container(row![text("▣").size(16), text(label).size(12)].spacing(8)).padding([8.0, 10.0]),
        container(
            column(
                lines
                    .into_iter()
                    .enumerate()
                    .map(|(index, line)| {
                        let accent = if line.starts_with('+') || line.starts_with('-') {
                            highlight_color
                        } else {
                            theme.palette.foreground
                        };
                        row![
                            text(format!("{:>3}", index + 1)).size(11).style({
                                let color = Color {
                                    a: 0.45,
                                    ..theme.palette.muted_foreground
                                };
                                move |_| iced::widget::text::Style { color: Some(color) }
                            }),
                            text(line.to_owned()).size(12).style(move |_| {
                                iced::widget::text::Style {
                                    color: Some(accent),
                                }
                            })
                        ]
                        .spacing(10)
                        .into()
                    })
                    .collect::<Vec<_>>(),
            )
            .spacing(4),
        )
        .padding([10.0, 12.0])
    ]
    .spacing(8);

    container(scrollable(pane)).width(Length::Fill).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_comparison_defaults_are_stable() {
        let props = CodeComparisonProps::new();

        assert_eq!(props.filename, "example.tsx");
    }
}
