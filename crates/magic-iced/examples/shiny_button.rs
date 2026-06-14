mod _shared;

use iced::time::Instant;
use iced::widget::column;
use iced::{Element, Size, Subscription, Task};
use magic_iced::{ShinyButtonProps, shiny_button};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        ShinyButtonExample::new,
        ShinyButtonExample::update,
        ShinyButtonExample::view,
    )
    .title("Shiny Button")
    .window(iced::window::Settings {
        size: Size::new(900.0, 420.0),
        ..Default::default()
    })
    .subscription(ShinyButtonExample::subscription)
    .run()
}

struct ShinyButtonExample {
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

impl ShinyButtonExample {
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
        let transition = self.theme_shell.controller.is_transition_active(now);
        let body = shiny_button(
            "SHINY BUTTON",
            14,
            None,
            self.elapsed,
            ShinyButtonProps::new()
                .text_color(self.theme_shell.blended_shiny_text_color(now))
                .primary_color(self.theme_shell.blended_shiny_primary(now))
                .border_color(self.theme_shell.blended_shiny_border(now))
                .background_color(self.theme_shell.blended_shiny_background(now))
                .text_shine_enabled(!transition)
                .border_shine_enabled(!transition),
            self.theme_shell.theme(),
        );

        _shared::themed_shell(
            "Shiny Button",
            column![body].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
