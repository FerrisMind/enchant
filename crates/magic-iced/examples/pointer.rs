mod _shared;

use iced::time::Instant;
use std::time::Duration;
use iced::widget::{column, container, row, text};
use iced::{Background, Border, Color, Element, Length, Size, Task};
use magic_iced::{PointerAction, PointerProps, PointerState, pointer};

pub fn main() -> iced::Result {
    iced::application(PointerExample::new, PointerExample::update, PointerExample::view)
        .title("Pointer")
        .window(iced::window::Settings {
            size: Size::new(900.0, 620.0),
            ..Default::default()
        })
        .subscription(PointerExample::subscription)
        
    .run()
}

struct PointerExample {
    theme_shell: _shared::ThemeShell,
    states: [PointerState; 4],
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    Tick(Instant),
    AnchorsMeasured(magic_iced::AnchorMeasurements),
    Pointer(usize, PointerAction),
}

impl PointerExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                states: std::array::from_fn(|_| PointerState::new()),
                now: Instant::now(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleTheme => {
                return _shared::ThemeShell::measure_toggle_anchor()
                    .map(Message::AnchorsMeasured);
            }
            Message::AnchorsMeasured(measurements) => {
                self.theme_shell.apply_anchor_and_toggle(&measurements);
            }
            Message::Tick(now) => {
                self.theme_shell.tick(now);
            }
            other => {
                let _ = other;
            }
        }
        Task::none()
    }

    fn preview_area() -> Element<'static, Message> {
        container(text("Hover me").size(16))
            .width(Length::Fill)
            .height(Length::Fixed(160.0))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(Color::from_rgba(0.08, 0.08, 0.1, 0.35))),
                border: Border {
                    radius: 8.0.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            })
            .into()
    }

    fn card_shell(body: Element<'static, Message>) -> Element<'static, Message> {
        container(body)
            .width(Length::Fixed(360.0))
            .style(|_| container::Style {
                border: Border {
                    width: 1.0,
                    radius: 12.0.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            })
            .into()
    }


    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let grid = column![
            row![
                Self::card_shell(column![
                    text("Animated Pointer").size(15),
                    text("Custom pink heart pointer").size(12),
                    pointer(
                        Self::preview_area(),
                        &self.states[0],
                        PointerProps {
                            color: Some(Color::from_rgb8(0xEC, 0x48, 0x99)),
                            ..PointerProps::new()
                        },
                        |action| Message::Pointer(0, action),
                        self.theme_shell.theme(),
                    ),
                ]
                .spacing(8)
                .padding(16)
                .into()),
                Self::card_shell(column![
                    text("Colored Pointer").size(15),
                    text("Blue arrow pointer").size(12),
                    pointer(
                        Self::preview_area(),
                        &self.states[1],
                        PointerProps {
                            color: Some(Color::from_rgb8(0x3B, 0x82, 0xF6)),
                            ..PointerProps::new()
                        },
                        |action| Message::Pointer(1, action),
                        self.theme_shell.theme(),
                    ),
                ]
                .spacing(8)
                .padding(16)
                .into()),
            ]
            .spacing(16),
            row![
                Self::card_shell(column![
                    text("Custom Shape").size(15),
                    text("Purple concentric pointer").size(12),
                    pointer(
                        Self::preview_area(),
                        &self.states[2],
                        PointerProps {
                            color: Some(Color::from_rgb8(0x8B, 0x5C, 0xF6)),
                            size: 28.0,
                            ..PointerProps::new()
                        },
                        |action| Message::Pointer(2, action),
                        self.theme_shell.theme(),
                    ),
                ]
                .spacing(8)
                .padding(16)
                .into()),
                Self::card_shell(column![
                    text("Emoji Pointer").size(15),
                    text("Using emoji as pointer").size(12),
                    pointer(
                        Self::preview_area(),
                        &self.states[3],
                        PointerProps {
                            pointer: Some(text("👆").size(24).into()),
                            ..PointerProps::new()
                        },
                        |action| Message::Pointer(3, action),
                        self.theme_shell.theme(),
                    ),
                ]
                .spacing(8)
                .padding(16)
                .into()),
            ]
            .spacing(16),
        ]
        .spacing(16);

        _shared::themed_shell("Pointer", column![grid].spacing(24), &self.theme_shell, Message::ToggleTheme)
    }
}
