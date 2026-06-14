mod _shared;

use iced::time::Instant;
use std::time::Duration;

use iced::widget::{column, container, scrollable, stack, text};
use iced::{Alignment, Border, Element, Length, Size, Task};
use magic_iced::{ProgressiveBlurPosition, ProgressiveBlurProps, progressive_blur};

pub fn main() -> iced::Result {
    iced::application(
        ProgressiveBlurExample::new,
        ProgressiveBlurExample::update,
        ProgressiveBlurExample::view,
    )
    .title("Progressive Blur")
    .window(iced::window::Settings {
        size: Size::new(900.0, 620.0),
        ..Default::default()
    })
    .subscription(ProgressiveBlurExample::subscription)
        
    .run()
}

struct ProgressiveBlurExample {
    theme_shell: _shared::ThemeShell,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    Tick(Instant),
    AnchorsMeasured(magic_iced::AnchorMeasurements),
}

impl ProgressiveBlurExample {
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
        let rows: Vec<Element<'_, Message>> = (0..20)
            .map(|index| {
                container(text(format!("{index}")).size(18))
                    .width(Length::Fill)
                    .height(Length::Fixed(80.0))
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .into()
            })
            .collect();

        let list = container(scrollable(column(rows).spacing(8).padding(16)))
            .width(Length::Fill)
            .height(Length::Fill);

        let stage = container(stack![
            list,
            container(progressive_blur(
                ProgressiveBlurProps {
                    width: Length::Fill,
                    height: Length::Fill,
                    position: ProgressiveBlurPosition::Bottom,
                    ..ProgressiveBlurProps::new()
                },
                self.theme_shell.theme(),
            ))
            .width(Length::Fill)
            .height(Length::Fill),
        ])
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

        _shared::themed_shell("Progressive Blur", column![stage].spacing(24).align_x(Alignment::Center), &self.theme_shell, Message::ToggleTheme,
        )
    }
}
