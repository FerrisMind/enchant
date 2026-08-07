mod _shared;

use iced::time::Instant;
use iced::widget::column;
use iced::{Element, Size, Subscription, Task};
use enchant_iced::{LineShadowTextProps, line_shadow_text};
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
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
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
                return _shared::ThemeShell::measure_toggle_anchor().map(Message::AnchorsMeasured);
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
        let now = self.theme_shell.now;
        let font_size = 72_u16;
        let foreground = self.theme_shell.blended_foreground(now);
        let shadow = self.theme_shell.blended_line_shadow(now);

        let headline = line_shadow_text(
            "Fast",
            self.elapsed,
            LineShadowTextProps {
                prefix: Some("Ship"),
                font_size: Some(font_size),
                text_color: Some(foreground),
                shadow_color: Some(shadow),
                italic: true,
                semibold: true,
                ..LineShadowTextProps::new()
            },
            self.theme_shell.blended_theme(),
        );

        _shared::themed_shell(
            "Line Shadow Text",
            column![headline].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
