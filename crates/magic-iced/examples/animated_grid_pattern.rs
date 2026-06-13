mod _shared;

use iced::time::Instant;
use iced::widget::{column, container};
use iced::{Alignment, Background, Border, Element, Length, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{AnimatedGridPatternProps, animated_grid_pattern};
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
    theme: Theme,
    start: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl AnimatedGridPatternExample {
    fn new() -> (Self, Task<Message>) {
        let start = Instant::now();
        (
            Self {
                theme: Theme::default(),
                start,
                elapsed: Duration::ZERO,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        if let Message::Tick(now) = message {
            self.elapsed = now.duration_since(self.start);
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
            &self.theme,
        ))
        .clip(true)
        .width(Length::Fixed(720.0))
        .height(Length::Fixed(500.0))
        .padding(80.0)
        .style(move |_| container::Style {
            background: Some(Background::Color(self.theme.palette.background)),
            border: Border {
                width: 1.0,
                radius: 12.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        });

        _shared::shell(
            "Animated Grid Pattern",
            column![stage].spacing(24).align_x(Alignment::Center),
        )
    }
}
