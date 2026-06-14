mod _shared;

use iced::time::Instant;
use iced::widget::column;
use iced::{Element, Size, Subscription, Task};
use magic_iced::{AuroraTextProps, aurora_text};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        AuroraTextExample::new,
        AuroraTextExample::update,
        AuroraTextExample::view,
    )
    .title("Aurora Text")
    .window(iced::window::Settings {
        size: Size::new(1000.0, 760.0),
        ..Default::default()
    })
    .subscription(AuroraTextExample::subscription)
    .run()
}

struct AuroraTextExample {
    theme_shell: _shared::ThemeShell,
    start: Instant,
    elapsed: Duration,
    props: AuroraTextProps,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(magic_iced::AnchorMeasurements),
    Tick(Instant),
}

impl AuroraTextExample {
    fn new() -> (Self, Task<Message>) {
        let start = Instant::now();
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
               start,
                elapsed: Duration::ZERO,
                props: AuroraTextProps::new()
                    .prefix("Ship ")
                    .font_size(72),
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
        let body = column![aurora_text::<Message>("beautiful", self.elapsed, &self.props)]
            .spacing(24)
            .align_x(iced::Alignment::Center);

        _shared::themed_shell("Aurora Text", body, &self.theme_shell, Message::ToggleTheme)
    }
}
