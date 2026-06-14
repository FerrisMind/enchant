mod _shared;

use iced::time::Instant;
use iced::widget::{column, container, stack, text};
use iced::{Alignment, Border, Element, Length, Size, Subscription, Task};
use magic_iced::{BorderBeamProps, border_beam};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        BorderTrackExample::new,
        BorderTrackExample::update,
        BorderTrackExample::view,
    )
    .title("Border Track")
    .window(iced::window::Settings {
        size: Size::new(900.0, 520.0),
        ..Default::default()
    })
    .subscription(BorderTrackExample::subscription)
    .run()
}

struct BorderTrackExample {
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

impl BorderTrackExample {
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

    fn tracked_card(&self, label: &'static str, radius: f32) -> Element<'static, Message> {
        let border_radius = radius;
        container(stack(vec![
            container(text(label).size(16))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into(),
            container(border_beam(
                self.elapsed,
                BorderBeamProps {
                    radius,
                    size: 80.0,
                    duration_secs: 6.0,
                    ..BorderBeamProps::new()
                },
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
        ]))
        .width(Length::Fixed(220.0))
        .height(Length::Fixed(120.0))
        .style(move |_| container::Style {
            border: Border {
                width: 1.0,
                radius: border_radius.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
    }

    fn view(&self) -> Element<'_, Message> {
        let body = column![
            text("Rounded-rect perimeter sampling powers Border Beam and Shine Border.").size(14),
            column![
                self.tracked_card("radius 12", 12.0),
                self.tracked_card("radius 24", 24.0),
                self.tracked_card("radius 36", 36.0),
            ]
            .spacing(16)
            .align_x(Alignment::Center),
        ]
        .spacing(20)
        .align_x(Alignment::Center);

        _shared::themed_shell(
            "Border Track",
            body,
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
