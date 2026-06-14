mod _shared;

use iced::time::Instant;
use std::time::Duration;

use iced::widget::{column, container};
use iced::{Alignment, Border, Element, Length, Size, Task};
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
    .subscription(GridPatternExample::subscription)
    .run()
}

struct GridPatternExample {
    theme_shell: _shared::ThemeShell,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    Tick(Instant),
    AnchorsMeasured(magic_iced::AnchorMeasurements),
}

impl GridPatternExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
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
        }
        Task::none()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(16)).map(Message::Tick)
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
            self.theme_shell.theme(),
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

        _shared::themed_shell(
            "Grid Pattern",
            column![stage].spacing(24).align_x(Alignment::Center),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
