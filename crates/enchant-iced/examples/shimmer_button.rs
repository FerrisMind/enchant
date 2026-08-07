mod _shared;

use iced::time::Instant;
use iced::widget::{column, text};
use iced::{Element, Size, Subscription, Task};
use enchant_iced::ShimmerButton;
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        ShimmerButtonExample::new,
        ShimmerButtonExample::update,
        ShimmerButtonExample::view,
    )
    .title("Shimmer Button")
    .window(iced::window::Settings {
        size: Size::new(900.0, 420.0),
        ..Default::default()
    })
    .subscription(ShimmerButtonExample::subscription)
    .run()
}

struct ShimmerButtonExample {
    theme_shell: _shared::ThemeShell,
    start: Instant,
    elapsed: Duration,
    presses: u32,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Tick(Instant),
    Pressed,
}

impl ShimmerButtonExample {
    fn new() -> (Self, Task<Message>) {
        let start = Instant::now();
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                start,
                elapsed: Duration::ZERO,
                presses: 0,
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
            Message::Pressed => {
                self.presses = self.presses.saturating_add(1);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let body = ShimmerButton::new("Shimmer Button")
            .on_press(Message::Pressed)
            .into_element(self.elapsed);

        let feedback = text(format!("clicks: {}", self.presses))
            .size(14)
            .color(self.theme_shell.blended_foreground(self.theme_shell.now));

        _shared::themed_shell(
            "Shimmer Button",
            column![body, feedback].spacing(16).align_x(iced::Alignment::Center),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
