mod _shared;

use iced::time::Instant;
use iced::widget::{column, container, stack, text};
use iced::{Alignment, Border, Element, Length, Size, Subscription, Task};
use magic_iced::{LightRaysProps, light_rays};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        LightRaysExample::new,
        LightRaysExample::update,
        LightRaysExample::view,
    )
    .title("Light Rays")
    .window(iced::window::Settings {
        size: Size::new(900.0, 620.0),
        ..Default::default()
    })
    .subscription(LightRaysExample::subscription)
    .run()
}

struct LightRaysExample {
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

impl LightRaysExample {
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

            Message::Tick(now) => self.elapsed = now.duration_since(self.start),
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let copy = column![
            text("Ambient glow").size(12),
            text("Light Rays").size(40),
            text(
                "Drop this component into any container and it will fill the space with softly animated light rays shining from above."
            )
            .size(14)
            .width(Length::Fixed(360.0)),
        ]
        .spacing(12)
        .align_x(Alignment::Center);

        let stage = container(stack(vec![
            container(light_rays(
                self.elapsed,
                LightRaysProps {
                    width: Length::Fill,
                    height: Length::Fill,
                    ..LightRaysProps::new()
                },
                self.theme_shell.theme(),
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
            container(copy)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into(),
        ]))
        .width(Length::Fixed(720.0))
        .height(Length::Fixed(500.0))
        .style(|_| container::Style {
            border: Border {
                width: 1.0,
                radius: 12.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        });

        _shared::themed_shell(
            "Light Rays",
            column![stage].spacing(24).align_x(Alignment::Center),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
