mod _shared;

use iced::alignment::Vertical;
use iced::time::Instant;
use iced::widget::{column, row, text};
use iced::{Color, Element, Size, Subscription, Task};
use magic_iced::{LineShadowTextProps, line_shadow_text};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        LineShadowTextExample::new,
        LineShadowTextExample::update,
        LineShadowTextExample::view,
    )
    .title("Line Shadow Text")
    .window(iced::window::Settings {
        size: Size::new(1000.0, 420.0),
        ..Default::default()
    })
    .subscription(LineShadowTextExample::subscription)
    .run()
}

struct LineShadowTextExample {
    theme_shell: _shared::ThemeShell,
    start: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(magic_iced::AnchorMeasurements),
    Tick(Instant),
}

impl LineShadowTextExample {
    fn new() -> (Self, Task<Message>) {
        let start = Instant::now();
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
               start,
                elapsed: Duration::ZERO,
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

            self.elapsed = now.duration_since(self.start);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let font_size = 72_u16;
        let headline = row![
            text("Ship ").size(f32::from(font_size)),
            line_shadow_text(
                "Fast",
                self.elapsed,
                LineShadowTextProps {
                    font_size: Some(font_size),
                    shadow_color: Color::BLACK,
                    ..LineShadowTextProps::new()
                },
            ),
        ]
        .spacing(0)
        .align_y(Vertical::Bottom);

        _shared::themed_shell("Line Shadow Text", column![headline].spacing(24), &self.theme_shell, Message::ToggleTheme)
    }
}
