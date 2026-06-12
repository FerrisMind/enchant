mod _shared;

use iced::widget::{column, container};
use iced::{Alignment, Border, Element, Length, Size, Task};
use iced_shadcn::Theme;
use magic_iced::{
    InteractiveGridPatternAction, InteractiveGridPatternProps, InteractiveGridPatternState,
    interactive_grid_pattern,
};

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
    .run()
}

struct InteractiveGridPatternExample {
    theme: Theme,
    state: InteractiveGridPatternState,
}

#[derive(Debug, Clone)]
enum Message {
    Grid(InteractiveGridPatternAction),
}

impl InteractiveGridPatternExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme: Theme::default(),
                state: InteractiveGridPatternState::new(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        if let Message::Grid(action) = message {
            self.state.apply_action(&action);
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let stage = container(interactive_grid_pattern(
            &self.state,
            InteractiveGridPatternProps {
                width: Length::Fill,
                height: Length::Fill,
                ..InteractiveGridPatternProps::new()
            },
            &self.theme,
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

        _shared::shell(
            "Interactive Grid Pattern",
            column![stage].spacing(24).align_x(Alignment::Center),
        )
    }
}
