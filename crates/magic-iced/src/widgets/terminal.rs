use crate::primitives::{TextLoopMode, TextRunSpec, TextRunner};
use iced::Alignment;
use iced::Background;
use iced::Color;
use iced::Element;
use iced::Font;
use iced::Length;
use iced::time::Instant;
use iced::widget::text::Span;
use iced::widget::{Space, column, container, rich_text, row, text};
use iced_shadcn::{CodeBlockGroupProps, CodeBlockProps, Theme, code_block, code_block_group};

#[derive(Clone, Debug, PartialEq)]
pub struct TerminalSegment {
    pub text: String,
    pub color: Option<Color>,
}

impl TerminalSegment {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: None,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TerminalLineMode {
    #[default]
    Instant,
    Typing,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TerminalLine {
    pub mode: TerminalLineMode,
    pub segments: Vec<TerminalSegment>,
}

impl TerminalLine {
    pub fn instant(segments: Vec<TerminalSegment>) -> Self {
        Self {
            mode: TerminalLineMode::Instant,
            segments,
        }
    }

    pub fn typing(segments: Vec<TerminalSegment>) -> Self {
        Self {
            mode: TerminalLineMode::Typing,
            segments,
        }
    }

    fn full_text(&self) -> String {
        let mut text = String::new();
        for segment in &self.segments {
            text.push_str(&segment.text);
        }
        text
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerminalProps {
    pub typing_spec: TextRunSpec,
    pub padding: f32,
    pub line_gap: f32,
    pub max_width: f32,
    pub font_size: u16,
}

impl Default for TerminalProps {
    fn default() -> Self {
        Self {
            typing_spec: TextRunSpec::new().loop_mode(TextLoopMode::Once),
            padding: 16.0,
            line_gap: 6.0,
            max_width: 560.0,
            font_size: 14,
        }
    }
}

impl TerminalProps {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug)]
pub struct TerminalState {
    active_index: usize,
    typing_runners: Vec<Option<TextRunner>>,
}

impl TerminalState {
    pub fn new(lines: &[TerminalLine], props: &TerminalProps) -> Self {
        let typing_runners = lines
            .iter()
            .map(|line| match line.mode {
                TerminalLineMode::Instant => None,
                TerminalLineMode::Typing => {
                    Some(TextRunner::new(line.full_text(), props.typing_spec))
                }
            })
            .collect();

        Self {
            active_index: 0,
            typing_runners,
        }
    }

    pub fn tick(&mut self, now: Instant, lines: &[TerminalLine], props: &TerminalProps) {
        if self.typing_runners.len() != lines.len() {
            *self = Self::new(lines, props);
        }

        while self.active_index < lines.len() {
            match lines[self.active_index].mode {
                TerminalLineMode::Instant => {
                    self.active_index += 1;
                }
                TerminalLineMode::Typing => {
                    let Some(runner) = self.typing_runners[self.active_index].as_mut() else {
                        self.active_index += 1;
                        continue;
                    };

                    runner.advance(now);
                    if runner.visible_text().len() >= lines[self.active_index].full_text().len() {
                        self.active_index += 1;
                        continue;
                    }
                    break;
                }
            }
        }
    }

    pub fn is_complete(&self, lines: &[TerminalLine]) -> bool {
        self.active_index >= lines.len()
    }

    fn visible_bytes_for(&self, index: usize, lines: &[TerminalLine]) -> usize {
        if index < self.active_index {
            return lines[index].full_text().len();
        }

        if index > self.active_index {
            return 0;
        }

        self.typing_runners
            .get(index)
            .and_then(|runner| runner.as_ref())
            .map(|runner| runner.visible_text().len())
            .unwrap_or_else(|| lines[index].full_text().len())
    }
}

pub fn terminal<'a, Message: 'a>(
    state: &TerminalState,
    lines: &'a [TerminalLine],
    props: &TerminalProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let rendered_lines = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            let visible = state.visible_bytes_for(index, lines);
            if visible == 0 {
                return None;
            }

            Some(
                rich_text(render_line(line, visible, theme))
                    .font(Font::MONOSPACE)
                    .size(u32::from(props.font_size))
                    .into(),
            )
        })
        .collect::<Vec<Element<'a, Message>>>();

    let body = column(rendered_lines)
        .spacing(props.line_gap)
        .width(Length::Fill);
    let traffic = row![
        traffic_light(Color::from_rgb8(0xEF, 0x44, 0x44)),
        traffic_light(Color::from_rgb8(0xF5, 0x9E, 0x0B)),
        traffic_light(Color::from_rgb8(0x22, 0xC5, 0x5E)),
        text("FerrisMind native demo").size(12).style({
            let color = theme.palette.muted_foreground;
            move |_| iced::widget::text::Style { color: Some(color) }
        }),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let header = code_block_group(
        traffic,
        Space::new().width(Length::Shrink),
        CodeBlockGroupProps::new().border_color(theme.palette.border),
        theme,
    );

    code_block(
        column![header, container(body).padding(props.padding)]
            .spacing(0)
            .width(Length::Fill),
        CodeBlockProps::new()
            .padding(0.0)
            .max_width(props.max_width)
            .background(theme.palette.card)
            .border_color(theme.palette.border)
            .text_color(theme.palette.card_foreground)
            .radius(theme.radius.lg),
        theme,
    )
    .into()
}

fn render_line<'a>(line: &'a TerminalLine, visible_bytes: usize, theme: &Theme) -> Vec<Span<'a>> {
    let mut remaining = visible_bytes;
    let mut spans = Vec::new();

    for segment in &line.segments {
        if remaining == 0 {
            break;
        }

        let bytes = remaining.min(segment.text.len());
        let text = &segment.text[..bytes];
        remaining -= bytes;

        spans.push(Span::new(text).color(segment.color.unwrap_or(theme.palette.foreground)));
    }

    spans
}

fn traffic_light<'a, Message: 'a>(color: Color) -> Element<'a, Message> {
    container(
        Space::new()
            .width(Length::Fixed(10.0))
            .height(Length::Fixed(10.0)),
    )
    .style(move |_| iced::widget::container::Style {
        background: Some(Background::Color(color)),
        border: iced::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 9999.0.into(),
        },
        ..iced::widget::container::Style::default()
    })
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::time::Duration;

    #[test]
    fn terminal_sequence_progresses_in_order() {
        let lines = vec![
            TerminalLine::typing(vec![TerminalSegment::new("> cargo test")]),
            TerminalLine::instant(vec![TerminalSegment::new("ok")]),
        ];
        let props = TerminalProps {
            typing_spec: TextRunSpec::new()
                .typing_step(twill::prelude::TransitionDuration::CustomMs(50))
                .loop_mode(TextLoopMode::Once),
            ..TerminalProps::new()
        };
        let mut state = TerminalState::new(&lines, &props);
        let start = Instant::now();

        state.tick(start, &lines, &props);
        assert!(!state.is_complete(&lines));

        state.tick(start + Duration::from_secs(1), &lines, &props);
        assert!(state.is_complete(&lines));
    }

    #[test]
    fn terminal_visible_bytes_hide_future_lines() {
        let lines = vec![
            TerminalLine::instant(vec![TerminalSegment::new("one")]),
            TerminalLine::typing(vec![TerminalSegment::new("two")]),
        ];
        let props = TerminalProps::new();
        let state = TerminalState::new(&lines, &props);

        assert_eq!(state.visible_bytes_for(1, &lines), 0);
    }
}
