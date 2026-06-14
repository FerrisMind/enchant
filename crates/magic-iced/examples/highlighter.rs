mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::{button, column, row, svg, text, tooltip};
use iced::{Alignment, Color, Element, Length, Size, Subscription, Task};
use magic_iced::{HighlighterAction, HighlighterProps, HighlighterState, highlighter};

pub fn main() -> iced::Result {
    iced::application(
        HighlighterExample::new,
        HighlighterExample::update,
        HighlighterExample::view,
    )
    .title("Highlighter")
    .window(iced::window::Settings {
        size: Size::new(900.0, 360.0),
        ..Default::default()
    })
    .subscription(HighlighterExample::subscription)
    .run()
}

struct HighlighterExample {
    theme_shell: _shared::ThemeShell,
    underline_state: HighlighterState,
    highlight_state: HighlighterState,
    underline_props: HighlighterProps,
    highlight_props: HighlighterProps,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    ResetAnimations,
    AnchorsMeasured(magic_iced::AnchorMeasurements),
    Tick(Instant),
}

impl HighlighterExample {
    fn new() -> (Self, Task<Message>) {
        let underline_props = HighlighterProps {
            action: HighlighterAction::Underline,
            color: Color::from_rgb8(0xFF, 0x98, 0x00),
            ..HighlighterProps::new()
        };
        let highlight_props = HighlighterProps {
            action: HighlighterAction::Highlight,
            color: Color::from_rgb8(0x87, 0xCE, 0xFA),
            ..HighlighterProps::new()
        };
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                underline_state: HighlighterState::new(&underline_props),
                highlight_state: HighlighterState::new(&highlight_props),
                underline_props,
                highlight_props,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleTheme => {
                return _shared::ThemeShell::measure_toggle_anchor().map(Message::AnchorsMeasured);
            }
            Message::ResetAnimations => {
                let now = self.theme_shell.now;
                self.underline_state.reset(now, &self.underline_props);
                self.highlight_state.reset(now, &self.highlight_props);
            }
            Message::AnchorsMeasured(measurements) => {
                self.theme_shell.apply_anchor_and_toggle(&measurements);
            }
            Message::Tick(now) => {
                self.theme_shell.tick(now);
                self.underline_state.tick(now, &self.underline_props);
                self.highlight_state.tick(now, &self.highlight_props);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let now = self.theme_shell.now;
        let foreground = self.theme_shell.blended_foreground(now);
        let theme = self.theme_shell.blended_theme();

        let body = row![
            body_text("The ", foreground),
            highlighter(
                body_text("Magic UI Highlighter", foreground),
                &self.underline_state,
                now,
                &self.underline_props,
                theme,
            ),
            body_text(" makes important ", foreground),
            highlighter(
                body_text("text stand out", foreground),
                &self.highlight_state,
                now,
                &self.highlight_props,
                theme,
            ),
            body_text(" effortlessly.", foreground),
        ]
        .spacing(0);

        let controls = row![
            reset_button(foreground),
            text("Replay with new random strokes")
                .size(13)
                .style(move |_| iced::widget::text::Style {
                    color: Some(foreground),
                }),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        _shared::themed_shell(
            "Highlighter",
            column![controls, body].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}

fn body_text<'a>(label: &'static str, foreground: Color) -> Element<'a, Message> {
    text(label)
        .style(move |_| iced::widget::text::Style {
            color: Some(foreground),
        })
        .into()
}

fn reset_button(foreground: Color) -> Element<'static, Message> {
    let icon = svg(reset_icon_handle())
        .width(Length::Fixed(18.0))
        .height(Length::Fixed(18.0))
        .style(move |_, _| svg::Style {
            color: Some(foreground),
            ..svg::Style::default()
        });

    tooltip(
        button(icon).padding(8).on_press(Message::ResetAnimations),
        "New random strokes",
        tooltip::Position::Bottom,
    )
    .into()
}

fn reset_icon_handle() -> svg::Handle {
    const ROTATE_CCW: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/></svg>"#;
    svg::Handle::from_memory(ROTATE_CCW.as_bytes())
}
