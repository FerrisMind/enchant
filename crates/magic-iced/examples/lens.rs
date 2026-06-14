mod _shared;

use iced::time::Instant;
use std::time::Duration;

use iced::widget::{column, container, image, text};
use iced::{Element, Length, Size, Task};
use magic_iced::{LensAction, LensProps, LensState, lens};

pub fn main() -> iced::Result {
    iced::application(LensExample::new, LensExample::update, LensExample::view)
        .title("Lens")
        .window(iced::window::Settings {
            size: Size::new(900.0, 520.0),
            ..Default::default()
        })
        .subscription(LensExample::subscription)
        .run()
}

struct LensExample {
    theme_shell: _shared::ThemeShell,
    state: LensState,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    Tick(Instant),
    AnchorsMeasured(magic_iced::AnchorMeasurements),
    Lens(LensAction),
}

fn sample_image() -> image::Handle {
    let mut pixels = Vec::with_capacity(4 * 4 * 4);
    for y in 0..4 {
        for x in 0..4 {
            let r = (x * 60 + 40) as u8;
            let g = (y * 50 + 30) as u8;
            pixels.extend_from_slice(&[r, g, 200, 255]);
        }
    }
    image::Handle::from_rgba(4, 4, pixels)
}

impl LensExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                state: LensState::new(),
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
            }
            Message::Lens(action) => {
                self.state.apply_action(&action);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let content = container(
            column![
                image(sample_image())
                    .width(Length::Fixed(360.0))
                    .height(Length::Fixed(240.0)),
                text("Hover to magnify").size(14),
            ]
            .spacing(12)
            .align_x(iced::Alignment::Center),
        )
        .padding(16);

        let body = lens(
            content,
            &self.state,
            LensProps {
                width: Length::Fixed(400.0),
                height: Length::Fixed(320.0),
                lens_size: 170.0,
                zoom_factor: 1.3,
                ..LensProps::new()
            },
            Message::Lens,
        );

        _shared::themed_shell(
            "Lens",
            column![body].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
