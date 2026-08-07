mod _shared;

use iced::time::Instant;

use iced::widget::{column, container};
use iced::{Alignment, Border, Element, Length, Size, Task};
use enchant_iced::{HexagonPatternProps, hexagon_pattern};

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
    .subscription(HexagonPatternExample::subscription)
    .run()
}

struct HexagonPatternExample {
    theme_shell: _shared::ThemeShell,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    Tick(Instant),
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
}

impl HexagonPatternExample {
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
        let stage = container(hexagon_pattern(
            HexagonPatternProps {
                width: Length::Fill,
                height: Length::Fill,
                radius: 40.0,
                hexagons: HIGHLIGHTS.to_vec(),
                ..HexagonPatternProps::new()
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
            "Hexagon Pattern",
            column![stage].spacing(24).align_x(Alignment::Center),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
