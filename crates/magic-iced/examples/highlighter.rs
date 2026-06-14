mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::{column, row, text};
use iced::{Color, Element, Size, Subscription, Task};
use magic_iced::{
    HighlighterAction, HighlighterProps, HighlighterState, highlighter,
};

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
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
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
                now: Instant::now(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleTheme => {
                return _shared::ThemeShell::measure_toggle_anchor()
                    .map(Message::AnchorsMeasured);
            }
            Message::AnchorsMeasured(measurements) => {
                self.theme_shell.apply_anchor_and_toggle(&measurements);
            }
            Message::Tick(now) => {
self.theme_shell.tick(now);

            self.now = now;
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
        let body = row![
            text("The "),
            highlighter(
                text("Magic UI Highlighter"),
                &self.underline_state,
                self.now,
                &self.underline_props,
                self.theme_shell.theme(),
            ),
            text(" makes important "),
            highlighter(
                text("text stand out"),
                &self.highlight_state,
                self.now,
                &self.highlight_props,
                self.theme_shell.theme(),
            ),
            text(" effortlessly."),
        ]
        .spacing(4);

        _shared::themed_shell("Highlighter", column![body].spacing(24), &self.theme_shell, Message::ToggleTheme)
    }
}
