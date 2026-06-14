mod _shared;

use iced::time::Instant;
use std::time::Duration;

use iced::widget::{column, container, image, text};
use iced::{Border, Element, Length, Size, Task};
use magic_iced::{BacklightProps, backlight};

fn hero_image() -> image::Handle {
    let width = 48;
    let height = 32;
    let mut pixels = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        for x in 0..width {
            let t = x as f32 / width as f32;
            pixels.extend_from_slice(&[
                (80.0 + t * 140.0) as u8,
                (40.0 + (1.0 - t) * 90.0) as u8,
                (180.0 - t * 60.0) as u8,
                255,
            ]);
        }
    }
    image::Handle::from_rgba(width as u32, height as u32, pixels)
}

pub fn main() -> iced::Result {
    iced::application(
        BacklightExample::new,
        BacklightExample::update,
        BacklightExample::view,
    )
    .title("Backlight")
    .window(iced::window::Settings {
        size: Size::new(900.0, 520.0),
        ..Default::default()
    })
    .subscription(BacklightExample::subscription)
        
    .run()
}

struct BacklightExample {
    theme_shell: _shared::ThemeShell,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    Tick(Instant),
    AnchorsMeasured(magic_iced::AnchorMeasurements),
}

impl BacklightExample {
    fn new() -> (Self, Task<Message>) {
        (Self { theme_shell: _shared::ThemeShell::new() }, Task::none())
    }

    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }


    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let image_card = container(image(hero_image()))
            .width(Length::Fixed(480.0))
            .height(Length::Fixed(320.0))
            .style(|_| container::Style {
                border: Border {
                    radius: 16.0.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            });

        let body = backlight(image_card, BacklightProps::new(), self.theme_shell.theme());

        _shared::themed_shell("Backlight", column![body, text("Fancy gradient backlight around media").size(14)].spacing(16),
            &self.theme_shell, Message::ToggleTheme)
    }
}
