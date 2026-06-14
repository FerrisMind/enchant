mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::column;
use iced::{Element, Length, Size, Subscription, Task};
use magic_iced::{IconCloudAction, IconCloudItem, IconCloudProps, IconCloudState, icon_cloud};

pub fn main() -> iced::Result {
    iced::application(
        IconCloudExample::new,
        IconCloudExample::update,
        IconCloudExample::view,
    )
    .title("Icon Cloud")
    .window(iced::window::Settings {
        size: Size::new(900.0, 520.0),
        ..Default::default()
    })
    .subscription(IconCloudExample::subscription)
    .run()
}

struct IconCloudExample {
    theme_shell: _shared::ThemeShell,
    state: IconCloudState,
    props: IconCloudProps,
    start: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(magic_iced::AnchorMeasurements),
    IconCloud(IconCloudAction),
    Tick(Instant),
}

impl IconCloudExample {
    fn new() -> (Self, Task<Message>) {
        let mut props = IconCloudProps::new();
        props.width = Length::Fixed(420.0);
        props.height = Length::Fixed(320.0);
        props.items = vec![
            IconCloudItem::Text("React".into()),
            IconCloudItem::Text("Rust".into()),
            IconCloudItem::Text("Iced".into()),
            IconCloudItem::Text("Svelte".into()),
            IconCloudItem::Text("Next.js".into()),
            IconCloudItem::Text("Tailwind".into()),
            IconCloudItem::Text("Magic UI".into()),
            IconCloudItem::Text("WASM".into()),
        ];
        let state = IconCloudState::new(&props);
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                state,
                props,
                start: Instant::now(),
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

            Message::IconCloud(action) => self.state.apply_action(&action, &self.props),
            Message::Tick(now) => {
                self.theme_shell.tick(now);
                let _ = now.duration_since(self.start);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let body = icon_cloud(&self.state, &self.props, Message::IconCloud);
        _shared::themed_shell(
            "Icon Cloud",
            column![body].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
