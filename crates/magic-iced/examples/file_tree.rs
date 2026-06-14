mod _shared;

use iced::time::Instant;
use std::time::Duration;

use iced::widget::{column, container};
use iced::{Alignment, Border, Element, Length, Size, Task};
use magic_iced::{
    FileTreeAction, FileTreeElement, FileTreeProps, FileTreeState, file_tree,
};

fn reference_tree() -> Vec<FileTreeElement> {
    vec![FileTreeElement::folder(
        "src",
        "src",
        [
            FileTreeElement::folder("lib", "lib", [FileTreeElement::file("utils", "utils.ts")]),
            FileTreeElement::folder(
                "app",
                "app",
                [
                    FileTreeElement::file("page", "page.tsx"),
                    FileTreeElement::file("layout", "layout.tsx"),
                ],
            ),
            FileTreeElement::folder(
                "components",
                "components",
                [
                    FileTreeElement::file("header", "header.tsx"),
                    FileTreeElement::folder(
                        "ui",
                        "ui",
                        [FileTreeElement::file("button", "button.tsx")],
                    ),
                    FileTreeElement::file("footer", "footer.tsx"),
                ],
            ),
        ],
    )]
}

pub fn main() -> iced::Result {
    iced::application(
        FileTreeExample::new,
        FileTreeExample::update,
        FileTreeExample::view,
    )
    .title("File Tree")
    .window(iced::window::Settings {
        size: Size::new(900.0, 480.0),
        ..Default::default()
    })
    .subscription(FileTreeExample::subscription)
        
    .run()
}

struct FileTreeExample {
    theme_shell: _shared::ThemeShell,
    elements: Vec<FileTreeElement>,
    state: FileTreeState,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    Tick(Instant),
    AnchorsMeasured(magic_iced::AnchorMeasurements),
    Tree(FileTreeAction),
}

impl FileTreeExample {
    fn new() -> (Self, Task<Message>) {
        let elements = reference_tree();
        let state = FileTreeState::new()
            .with_selected_id("button")
            .with_expanded_ids(["src", "app", "components", "ui", "lib"]);
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                elements,
                state,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleTheme => {
                return _shared::ThemeShell::measure_toggle_anchor()
                    .map(Message::AnchorsMeasured);
            }
            Message::AnchorsMeasured(measurements) => {
                self.theme_shell.apply_anchor_and_toggle(&measurements);
            }
            Message::Tick(now) => {
                self.theme_shell.tick(now);
            }
            other => {
                let _ = other;
            }
        }
        Task::none()
    }


    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let stage = container(file_tree(
            &self.elements,
            &self.state,
            Message::Tree,
            FileTreeProps::new().width(Length::Fixed(360.0)).height(Length::Fixed(300.0)),
            self.theme_shell.theme(),
        ))
        .width(Length::Fixed(400.0))
        .height(Length::Fixed(320.0))
        .padding(12)
        .style(|_| container::Style {
            border: Border {
                width: 1.0,
                radius: 12.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        });

        _shared::themed_shell("File Tree", column![stage].spacing(24).align_x(Alignment::Center), &self.theme_shell, Message::ToggleTheme)
    }
}
