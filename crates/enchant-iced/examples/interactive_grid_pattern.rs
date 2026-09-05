mod _shared;

use iced::time::Instant;

use enchant_iced::{
    InteractiveGridPatternAction, InteractiveGridPatternProps, InteractiveGridPatternState,
    interactive_grid_pattern,
};
use iced::widget::{column, container};
use iced::{Alignment, Border, Element, Length, Size, Task};

pub fn main() -> iced::Result {
    iced::application(
        InteractiveGridPatternExample::new,
        InteractiveGridPatternExample::update,
        InteractiveGridPatternExample::view,
    )
    .title("Interactive Grid Pattern")
    .window(iced::window::Settings {
        size: Size::new(900.0, 620.0),
        ..Default::default()
    })
    .subscription(InteractiveGridPatternExample::subscription)
    .run()
}

struct InteractiveGridPatternExample {
    theme_shell: _shared::ThemeShell,
    state: InteractiveGridPatternState,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    Tick(Instant),
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Grid(InteractiveGridPatternAction),
}

impl InteractiveGridPatternExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                state: InteractiveGridPatternState::new(),
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
            Message::Grid(action) => {
                self.state.apply_action(&action);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let stage = container(interactive_grid_pattern(
            &self.state,
            InteractiveGridPatternProps {
                width: Length::Fill,
                height: Length::Fill,
                ..InteractiveGridPatternProps::new()
            },
            self.theme_shell.theme(),
            Message::Grid,
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
            "Interactive Grid Pattern",
            column![stage].spacing(24).align_x(Alignment::Center),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
