mod _shared;

use iced::widget::{column, container};
use iced::{Alignment, Border, Element, Length, Size, Task};
use iced_shadcn::Theme;
use magic_iced::{GridCellIndex, GridPatternProps, grid_pattern};

const REFERENCE_SQUARES: &[(i32, i32)] = &[
    (4, 4),
    (5, 1),
    (8, 2),
    (5, 3),
    (5, 5),
    (10, 10),
    (12, 15),
    (15, 10),
    (10, 15),
];

pub fn main() -> iced::Result {
    iced::application(
        GridPatternExample::new,
        GridPatternExample::update,
        GridPatternExample::view,
    )
    .title("Grid Pattern")
    .window(iced::window::Settings {
        size: Size::new(900.0, 620.0),
        ..Default::default()
    })
    .run()
}

struct GridPatternExample {
    theme: Theme,
}

#[derive(Debug, Clone)]
enum Message {}

impl GridPatternExample {
    fn new() -> (Self, Task<Message>) {
        (Self { theme: Theme::default() }, Task::none())
    }

    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let squares = REFERENCE_SQUARES
            .iter()
            .map(|&(x, y)| GridCellIndex { x, y })
            .collect();

        let stage = container(grid_pattern(
            GridPatternProps {
                width: Length::Fill,
                height: Length::Fill,
                squares,
                ..GridPatternProps::new()
            },
            &self.theme,
        ))
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

        _shared::shell("Grid Pattern", column![stage].spacing(24).align_x(Alignment::Center))
    }
}
