mod _shared;

use iced::time::Instant;
use iced::widget::column;
use iced::{Color, Element, Length, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{
    AnimatedGradientTextProps, NeonColors, NeonGradientCardProps, animated_gradient_text,
    neon_gradient_card,
};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        NeonGradientCardExample::new,
        NeonGradientCardExample::update,
        NeonGradientCardExample::view,
    )
    .title("Neon Gradient Card")
    .window(iced::window::Settings {
        size: Size::new(900.0, 480.0),
        ..Default::default()
    })
    .subscription(NeonGradientCardExample::subscription)
    .run()
}

struct NeonGradientCardExample {
    theme: Theme,
    start: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl NeonGradientCardExample {
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
        let title = animated_gradient_text(
            "Neon Gradient Card",
            self.elapsed,
            AnimatedGradientTextProps::new()
                .font_size(48)
                .color_from(Color::from_rgb8(0xFF, 0x29, 0x75))
                .color_to(Color::from_rgb8(0x00, 0xFF, 0xF1)),
        );

        let card = neon_gradient_card(
            title,
            self.elapsed,
            NeonGradientCardProps {
                width: Length::Fixed(384.0),
                height: Length::Fixed(220.0),
                neon_colors: Some(NeonColors {
                    first_color: Color::from_rgb8(0xFF, 0x29, 0x75),
                    second_color: Color::from_rgb8(0x00, 0xFF, 0xF1),
                }),
                ..NeonGradientCardProps::new()
            },
            &self.theme,
        );

        _shared::shell("Neon Gradient Card", column![card].spacing(24))
    }
}
