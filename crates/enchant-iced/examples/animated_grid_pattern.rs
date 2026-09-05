mod _shared;

use enchant_iced::{AnimatedGridPatternProps, animated_grid_pattern};
use iced::time::Instant;
use iced::widget::{column, container};
use iced::{Alignment, Background, Border, Element, Length, Size, Subscription, Task};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        AnimatedGridPatternExample::new,
        AnimatedGridPatternExample::update,
        AnimatedGridPatternExample::view,
    )
    .title("Animated Grid Pattern")
    .window(iced::window::Settings {
        size: Size::new(900.0, 620.0),
        ..Default::default()
    })
    .subscription(AnimatedGridPatternExample::subscription)
    .run()
}

struct AnimatedGridPatternExample {
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

impl AnimatedGridPatternExample {
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
        let stage = container(animated_grid_pattern(
            self.elapsed,
            AnimatedGridPatternProps {
                width: Length::Fill,
                height: Length::Fill,
                num_squares: 30,
                max_opacity: 0.3,
                duration_secs: 3.0,
                repeat_delay_secs: 1.0,
                skew_x_deg: 12.0,
                pattern_height_scale: 2.0,
                pattern_offset_y_ratio: -0.3,
                mask_radius: 250.0,
                ..AnimatedGridPatternProps::new()
            },
            self.theme_shell.theme(),
        ))
        .clip(true)
        .width(Length::Fixed(720.0))
        .height(Length::Fixed(500.0))
        .padding(80.0)
        .style(move |_| container::Style {
            background: Some(Background::Color(
                self.theme_shell.theme().palette.background,
            )),
            border: Border {
                width: 1.0,
                radius: 12.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        });

        _shared::themed_shell(
            "Animated Grid Pattern",
            column![stage].spacing(24).align_x(Alignment::Center),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
