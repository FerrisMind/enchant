mod _shared;

use iced::widget::{column, container};
use iced::{Alignment, Border, Element, Length, Size, Task};
use iced_shadcn::Theme;
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
    .run()
}

struct FileTreeExample {
    theme: Theme,
    elements: Vec<FileTreeElement>,
    state: FileTreeState,
}

#[derive(Debug, Clone)]
enum Message {
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
                theme: Theme::default(),
                elements,
                state,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        if let Message::Tree(action) = message {
            self.state.apply_action(&action);
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let stage = container(file_tree(
            &self.elements,
            &self.state,
            Message::Tree,
            FileTreeProps::new().width(Length::Fixed(360.0)).height(Length::Fixed(300.0)),
            &self.theme,
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

        _shared::shell("File Tree", column![stage].spacing(24).align_x(Alignment::Center))
    }
}
