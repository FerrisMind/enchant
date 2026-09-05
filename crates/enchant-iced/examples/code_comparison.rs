mod _shared;

use enchant_iced::{CodeComparisonProps, code_comparison};
use iced::time::Instant;
use iced::widget::{column, text};
use iced::{Element, Size, Task};

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
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
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
        let before = r#"fn load_data() {
    println!("loading...");
}"#;
        let after = r#"fn load_data() -> Result<Data, Error> {
    tracing::info!("loading data");
    Ok(fetch_data()?)
}"#;

        _shared::themed_shell(
            "Code Comparison",
            column![
                text("Highlight the diff between the old and new code paths."),
                code_comparison(
                    before,
                    after,
                    CodeComparisonProps::new(),
                    self.theme_shell.theme()
                ),
            ]
            .spacing(20),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
