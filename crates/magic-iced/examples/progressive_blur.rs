mod _shared;

use iced::widget::{column, container, scrollable, stack, text};
use iced::{Alignment, Border, Element, Length, Size, Task};
use iced_shadcn::Theme;
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
    .run()
}

struct ProgressiveBlurExample {
    theme: Theme,
}

#[derive(Debug, Clone)]
enum Message {}

impl ProgressiveBlurExample {
    fn new() -> (Self, Task<Message>) {
        (Self { theme: Theme::default() }, Task::none())
    }

    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
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
                &self.theme,
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

        _shared::shell(
            "Progressive Blur",
            column![stage].spacing(24).align_x(Alignment::Center),
        )
    }
}
