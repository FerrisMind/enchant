mod _shared;

use iced::time::Instant;
use iced::widget::{column, image};
use iced::{Element, Size, Task};

use magic_iced::{AvatarCircleEntry, AvatarCirclesProps, avatar_circles};

pub fn main() -> iced::Result {
    iced::application(
        AvatarCirclesExample::new,
        AvatarCirclesExample::update,
        AvatarCirclesExample::view,
    )
    .title("Avatar Circles")
    .window(iced::window::Settings {
        size: Size::new(900.0, 320.0),
        ..Default::default()
    })
    .subscription(AvatarCirclesExample::subscription)
    .run()
}

struct AvatarCirclesExample {
    theme_shell: _shared::ThemeShell,
    props: AvatarCirclesProps<'static>,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    Tick(Instant),
    AnchorsMeasured(magic_iced::AnchorMeasurements),
}

impl AvatarCirclesExample {
    fn new() -> (Self, Task<Message>) {
        let props = AvatarCirclesProps::new([
            AvatarCircleEntry::image(avatar_image("16860528")),
            AvatarCircleEntry::image(avatar_image("20110627")),
            AvatarCircleEntry::image(avatar_image("106103625")),
            AvatarCircleEntry::image(avatar_image("59228569")),
            AvatarCircleEntry::image(avatar_image("59442788")),
            AvatarCircleEntry::image(avatar_image("89768406")),
        ])
        .extra_count(99);

        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                props,
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
        let body = avatar_circles(&self.props, self.theme_shell.theme());
        _shared::themed_shell(
            "Avatar Circles",
            column![body].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}

fn avatar_image(id: &str) -> image::Handle {
    let bytes: &'static [u8] = match id {
        "16860528" => include_bytes!("assets/avatars/16860528.png"),
        "20110627" => include_bytes!("assets/avatars/20110627.png"),
        "106103625" => include_bytes!("assets/avatars/106103625.png"),
        "59228569" => include_bytes!("assets/avatars/59228569.png"),
        "59442788" => include_bytes!("assets/avatars/59442788.png"),
        "89768406" => include_bytes!("assets/avatars/89768406.png"),
        _ => panic!("unknown avatar id: {id}"),
    };
    image::Handle::from_bytes(bytes)
}
