mod _shared;

use iced::time::Instant;
use iced::widget::{column, container};
use iced::{Background, Border, Color, Element, Size, Subscription, Task};
use magic_iced::{AnimatedShinyTextProps, animated_shiny_text};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        AnimatedShinyTextExample::new,
        AnimatedShinyTextExample::update,
        AnimatedShinyTextExample::view,
    )
    .title("Animated Shiny Text")
    .window(iced::window::Settings {
        size: Size::new(900.0, 420.0),
        ..Default::default()
    })
    .subscription(AnimatedShinyTextExample::subscription)
    .run()
}

struct AnimatedShinyTextExample {
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

impl AnimatedShinyTextExample {
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
        let pill = container(animated_shiny_text(
            "✨ Introducing Magic UI",
            self.elapsed,
            AnimatedShinyTextProps {
                font_size: Some(16),
                base_color: Color::from_rgba(1.0, 1.0, 1.0, 0.55),
                shine_color: Color::WHITE,
                ..AnimatedShinyTextProps::new()
            },
        ))
        .padding([8.0, 18.0])
        .style(|_| container::Style {
            background: Some(Background::Color(Color::from_rgb8(0x17, 0x17, 0x17))),
            border: Border {
                radius: 999.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        });

        _shared::themed_shell(
            "Animated Shiny Text",
            column![pill].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
