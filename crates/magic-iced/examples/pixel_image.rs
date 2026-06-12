mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::column;
use iced::{Element, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{PixelGrid, PixelImageProps, PixelImageState, pixel_image};

fn demo_image() -> iced::widget::image::Handle {
    let width = 24;
    let height = 16;
    let mut pixels = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        for x in 0..width {
            let t = (x as f32 / width as f32 + y as f32 / height as f32) * 0.5;
            pixels.extend_from_slice(&[
                (40.0 + t * 180.0) as u8,
                (80.0 + (1.0 - t) * 120.0) as u8,
                (200.0 - t * 80.0) as u8,
                255,
            ]);
        }
    }
    iced::widget::image::Handle::from_rgba(width as u32, height as u32, pixels)
}

pub fn main() -> iced::Result {
    iced::application(
        PixelImageExample::new,
        PixelImageExample::update,
        PixelImageExample::view,
    )
    .title("Pixel Image")
    .window(iced::window::Settings {
        size: Size::new(900.0, 520.0),
        ..Default::default()
    })
    .subscription(PixelImageExample::subscription)
    .run()
}

struct PixelImageExample {
    theme: Theme,
    props: PixelImageProps,
    state: PixelImageState,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl PixelImageExample {
    fn new() -> (Self, Task<Message>) {
        let props = PixelImageProps {
            handle: demo_image(),
            custom_grid: Some(PixelGrid { rows: 4, cols: 6 }),
            grayscale_animation: true,
            ..PixelImageProps::new(demo_image())
        };
        let state = PixelImageState::new(&props);
        (
            Self {
                theme: Theme::default(),
                props,
                state,
                now: Instant::now(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        if let Message::Tick(now) = message {
            self.now = now;
            self.state.tick(now, &self.props);
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let body = pixel_image(&self.state, self.now, &self.props, &self.theme);
        _shared::shell("Pixel Image", column![body].spacing(24))
    }
}
