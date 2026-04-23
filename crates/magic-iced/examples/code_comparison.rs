mod _shared;

use iced::widget::{column, text};
use iced::{Element, Size, Task};
use iced_shadcn::Theme;
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
    .run()
}

struct CodeComparisonExample {
    theme: Theme,
}

#[derive(Debug, Clone)]
enum Message {}

impl CodeComparisonExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme: Theme::default(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let before = r#"fn load_data() {
    println!("loading...");
}"#;
        let after = r#"fn load_data() -> Result<Data, Error> {
    tracing::info!("loading data");
    Ok(fetch_data()?)
}"#;

        _shared::shell(
            "Code Comparison",
            column![
                text("Highlight the diff between the old and new code paths."),
                code_comparison(before, after, CodeComparisonProps::new(), &self.theme),
            ]
            .spacing(20),
        )
    }
}
