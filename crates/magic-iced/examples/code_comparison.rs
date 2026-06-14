mod _shared;

use iced::widget::{column, text};
use iced::time::Instant;
use std::time::Duration;
use iced::{Element, Size, Subscription, Task};
use magic_iced::{CodeComparisonProps, code_comparison};

pub fn main() -> iced::Result {
    iced::application(
        CodeComparisonExample::new,
        CodeComparisonExample::update,
        CodeComparisonExample::view,
    )
    .title("Code Comparison")
    .window(iced::window::Settings {
        size: Size::new(1200.0, 820.0),
        ..Default::default()
    })
    .subscription(CodeComparisonExample::subscription)
        
    .run()
}

struct CodeComparisonExample {
    theme_shell: _shared::ThemeShell,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    Tick(Instant),
    AnchorsMeasured(magic_iced::AnchorMeasurements),
}

impl CodeComparisonExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }


    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let before = r#"fn load_data() {
    println!("loading...");
}"#;
        let after = r#"fn load_data() -> Result<Data, Error> {
    tracing::info!("loading data");
    Ok(fetch_data()?)
}"#;

        _shared::themed_shell("Code Comparison", column![
                text("Highlight the diff between the old and new code paths."),
                code_comparison(before, after, CodeComparisonProps::new(), self.theme_shell.theme()),
            ]
            .spacing(20),
            &self.theme_shell, Message::ToggleTheme)
    }
}
