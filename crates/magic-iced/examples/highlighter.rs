mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::{column, row, text};
use iced::{Color, Element, Size, Subscription, Task};
use iced_shadcn::Theme;
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
    theme: Theme,
    underline_state: HighlighterState,
    highlight_state: HighlighterState,
    underline_props: HighlighterProps,
    highlight_props: HighlighterProps,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
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
                theme: Theme::default(),
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
        if let Message::Tick(now) = message {
            self.now = now;
            self.underline_state.tick(now, &self.underline_props);
            self.highlight_state.tick(now, &self.highlight_props);
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
                &self.theme,
            ),
            text(" makes important "),
            highlighter(
                text("text stand out"),
                &self.highlight_state,
                self.now,
                &self.highlight_props,
                &self.theme,
            ),
            text(" effortlessly."),
        ]
        .spacing(4);

        _shared::shell("Highlighter", column![body].spacing(24))
    }
}
