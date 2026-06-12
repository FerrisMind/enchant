mod _shared;

use iced::widget::{column, container, image, text};
use iced::{Border, Element, Length, Size, Task};
use iced_shadcn::Theme;
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
    .run()
}

struct BacklightExample {
    theme: Theme,
}

#[derive(Debug, Clone)]
enum Message {}

impl BacklightExample {
    fn new() -> (Self, Task<Message>) {
        (Self { theme: Theme::default() }, Task::none())
    }

    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
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

        let body = backlight(image_card, BacklightProps::new(), &self.theme);

        _shared::shell(
            "Backlight",
            column![body, text("Fancy gradient backlight around media").size(14)].spacing(16),
        )
    }
}
