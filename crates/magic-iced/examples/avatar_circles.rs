mod _shared;

use iced::widget::column;
use iced::{Color, Element, Size, Task};
use iced_shadcn::{AvatarVariant, Theme};
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
    .run()
}

struct AvatarCirclesExample {
    theme: Theme,
    props: AvatarCirclesProps<'static>,
}

#[derive(Debug, Clone)]
enum Message {}

impl AvatarCirclesExample {
    fn new() -> (Self, Task<Message>) {
        let props = AvatarCirclesProps::new([
            AvatarCircleEntry::new("DV").color(Color::from_rgb8(0x3B, 0x82, 0xF6)),
            AvatarCircleEntry::new("TF").color(Color::from_rgb8(0x10, 0xB9, 0x81)),
            AvatarCircleEntry::new("BK").color(Color::from_rgb8(0xF5, 0x9E, 0x0B)),
            AvatarCircleEntry::new("SC").color(Color::from_rgb8(0xEF, 0x44, 0x44)),
            AvatarCircleEntry::new("SM").color(Color::from_rgb8(0x8B, 0x5C, 0xF6)),
            AvatarCircleEntry::new("AD")
                .color(Color::from_rgb8(0x06, 0xB6, 0xD4))
                .variant(AvatarVariant::Solid),
        ])
        .extra_count(99);

        (
            Self {
                theme: Theme::default(),
                props,
            },
            Task::none(),
        )
    }

    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let body = avatar_circles(&self.props, &self.theme);
        _shared::shell("Avatar Circles", column![body].spacing(24))
    }
}
