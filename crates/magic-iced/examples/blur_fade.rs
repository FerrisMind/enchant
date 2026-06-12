mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::{column, container, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::primitives::MotionSpec;
use magic_iced::{BlurFadeProps, BlurFadeState, blur_fade};
use twill::prelude::TransitionDuration;

const TILE_COUNT: usize = 9;

struct TileConfig {
    label: &'static str,
    width: f32,
    height: f32,
    color: Color,
    delay_ms: u16,
}

struct BlurFadeExample {
    theme: Theme,
    now: Instant,
    states: [BlurFadeState; TILE_COUNT],
    tiles: [TileConfig; TILE_COUNT],
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl BlurFadeExample {
    fn new() -> (Self, Task<Message>) {
        let tiles = [
            TileConfig {
                label: "01",
                width: 200.0,
                height: 150.0,
                color: Color::from_rgb8(0x60, 0xA5, 0xFA),
                delay_ms: 250,
            },
            TileConfig {
                label: "02",
                width: 160.0,
                height: 200.0,
                color: Color::from_rgb8(0x34, 0xD3, 0x99),
                delay_ms: 300,
            },
            TileConfig {
                label: "03",
                width: 200.0,
                height: 150.0,
                color: Color::from_rgb8(0xF4, 0x72, 0xB6),
                delay_ms: 350,
            },
            TileConfig {
                label: "04",
                width: 160.0,
                height: 200.0,
                color: Color::from_rgb8(0xFB, 0xBF, 0x24),
                delay_ms: 400,
            },
            TileConfig {
                label: "05",
                width: 200.0,
                height: 150.0,
                color: Color::from_rgb8(0xA7, 0x8B, 0xFA),
                delay_ms: 450,
            },
            TileConfig {
                label: "06",
                width: 160.0,
                height: 200.0,
                color: Color::from_rgb8(0x38, 0xBD, 0xF8),
                delay_ms: 500,
            },
            TileConfig {
                label: "07",
                width: 200.0,
                height: 150.0,
                color: Color::from_rgb8(0x4A, 0xDE, 0x80),
                delay_ms: 550,
            },
            TileConfig {
                label: "08",
                width: 160.0,
                height: 200.0,
                color: Color::from_rgb8(0xF8, 0x71, 0x71),
                delay_ms: 600,
            },
            TileConfig {
                label: "09",
                width: 200.0,
                height: 150.0,
                color: Color::from_rgb8(0x2D, 0xD4, 0xBF),
                delay_ms: 650,
            },
        ];

        let states = std::array::from_fn(|index| {
            let props = BlurFadeProps::new().motion(
                MotionSpec::new()
                    .duration(TransitionDuration::CustomMs(400))
                    .delay(TransitionDuration::CustomMs(tiles[index].delay_ms)),
            );
            BlurFadeState::new(&props)
        });

        (
            Self {
                theme: Theme::default(),
                now: Instant::now(),
                states,
                tiles,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        if let Message::Tick(now) = message {
            self.now = now;
            for (index, state) in self.states.iter_mut().enumerate() {
                let props = BlurFadeProps::new().motion(
                    MotionSpec::new()
                        .duration(TransitionDuration::CustomMs(400))
                        .delay(TransitionDuration::CustomMs(self.tiles[index].delay_ms)),
                );
                state.tick(now, &props);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn tile(&self, index: usize) -> Element<'_, Message> {
        let tile = &self.tiles[index];
        let props = BlurFadeProps::new().motion(
            MotionSpec::new()
                .duration(TransitionDuration::CustomMs(400))
                .delay(TransitionDuration::CustomMs(tile.delay_ms)),
        );
        let content = container(text(tile.label).size(28))
            .width(Length::Fixed(tile.width))
            .height(Length::Fixed(tile.height))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(move |_| container::Style {
                background: Some(Background::Color(tile.color)),
                border: Border {
                    radius: 12.0.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            });

        blur_fade(content, &self.states[index], self.now, &props, &self.theme)
    }

    fn view(&self) -> Element<'_, Message> {
        let grid = column![
            row![self.tile(0), self.tile(1), self.tile(2)].spacing(16),
            row![self.tile(3), self.tile(4), self.tile(5)].spacing(16),
            row![self.tile(6), self.tile(7), self.tile(8)].spacing(16),
        ]
        .spacing(16)
        .align_x(Alignment::Center);

        _shared::shell("Blur Fade", column![grid].spacing(24))
    }
}

pub fn main() -> iced::Result {
    iced::application(
        BlurFadeExample::new,
        BlurFadeExample::update,
        BlurFadeExample::view,
    )
    .title("Blur Fade")
    .window(iced::window::Settings {
        size: Size::new(900.0, 720.0),
        ..Default::default()
    })
    .subscription(BlurFadeExample::subscription)
    .run()
}
