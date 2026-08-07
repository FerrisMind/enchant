mod _shared;

use iced::time::Instant;
use iced::widget::{column, text};
use iced::{Element, Size, Subscription, Task};
use enchant_iced::{RippleProps, ripple};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        RippleExample::new,
        RippleExample::update,
        RippleExample::view,
    )
    .title("Ripple")
    .window(iced::window::Settings {
        size: Size::new(1000.0, 760.0),
        ..Default::default()
    })
    .subscription(RippleExample::subscription)
    .run()
}

struct RippleExample {
    active: usize,
    theme_shell: _shared::ThemeShell,
    started_at: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Select(usize),
    Tick(Instant),
}

impl RippleExample {
    fn new() -> (Self, Task<Message>) {
        let started_at = Instant::now();
        (
            Self {
                active: 0,
                theme_shell: _shared::ThemeShell::new(),
                started_at,
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

            Message::Select(index) => self.active = index.min(1),
            Message::Tick(now) => {
                self.theme_shell.tick(now);
                self.elapsed = now.duration_since(self.started_at);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let labels = ["Calm", "Dense"];
        let props = if self.active == 0 {
            RippleProps {
                main_circle_size: 180.0,
                main_circle_opacity: 0.24,
                num_circles: 6,
                duration_secs: 1.8,
                color: None,
            }
        } else {
            RippleProps {
                main_circle_size: 240.0,
                main_circle_opacity: 0.18,
                num_circles: 10,
                duration_secs: 1.2,
                color: None,
            }
        };

        _shared::themed_shell(
            "Ripple",
            column![
                _shared::tabs(&labels, self.active, Message::Select),
                ripple(self.elapsed, props, self.theme_shell.theme()),
                text("A concentric background effect for hero surfaces."),
            ]
            .spacing(20),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
