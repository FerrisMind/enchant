mod _shared;

use iced::widget::{column, container};
use iced::{Alignment, Border, Element, Length, Size, Task};
use iced_shadcn::Theme;
use magic_iced::{HexagonPatternProps, hexagon_pattern};

const HIGHLIGHTS: &[(i32, i32)] = &[
    (1, 1),
    (4, 4),
    (2, 2),
    (3, 4),
    (5, 4),
    (8, 2),
    (6, 3),
    (8, 5),
    (10, 10),
];

pub fn main() -> iced::Result {
    iced::application(
        HexagonPatternExample::new,
        HexagonPatternExample::update,
        HexagonPatternExample::view,
    )
    .title("Hexagon Pattern")
    .window(iced::window::Settings {
        size: Size::new(900.0, 620.0),
        ..Default::default()
    })
    .run()
}

struct HexagonPatternExample {
    theme: Theme,
}

#[derive(Debug, Clone)]
enum Message {}

impl HexagonPatternExample {
    fn new() -> (Self, Task<Message>) {
        (Self { theme: Theme::default() }, Task::none())
    }

    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let stage = container(hexagon_pattern(
            HexagonPatternProps {
                width: Length::Fill,
                height: Length::Fill,
                radius: 40.0,
                hexagons: HIGHLIGHTS.to_vec(),
                ..HexagonPatternProps::new()
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

        _shared::shell(
            "Hexagon Pattern",
            column![stage].spacing(24).align_x(Alignment::Center),
        )
    }
}
