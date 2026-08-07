mod _shared;

use iced::time::Instant;

use iced::widget::{column, container};
use iced::{Element, Length, Size, Subscription, Task};
use enchant_iced::{AndroidProps, android};

pub fn main() -> iced::Result {
    iced::application(
        AndroidExample::new,
        AndroidExample::update,
        AndroidExample::view,
    )
    .title("Android Example")
    .window(iced::window::Settings {
        size: Size::new(1200.0, 900.0),
        ..Default::default()
    })
    .subscription(AndroidExample::subscription)
    .run()
}

struct AndroidExample {
    active: usize,
    theme_shell: _shared::ThemeShell,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Select(usize),
}

impl AndroidExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                active: 0,
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

            Message::Select(index) => self.active = index.min(2),
            Message::Tick(now) => {
                self.theme_shell.tick(now);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let labels = ["Android Demo", "Android Demo 2", "Android Demo 3"];
        let content = match self.active {
            0 => android(
                AndroidProps::new()
                    .width(Length::Fixed(320.0))
                    .screen_media(false),
                self.theme_shell.theme(),
            ),
            1 => android(
                AndroidProps::new()
                    .width(Length::Fixed(320.0))
                    .screen_media(true)
                    .media(include_bytes!("assets/android-demo.jpg"), "image/jpeg"),
                self.theme_shell.theme(),
            ),
            _ => android(
                AndroidProps::new()
                    .width(Length::Fixed(320.0))
                    .screen_media(true)
                    .media(
                        include_bytes!("assets/android-demo-video.jpg"),
                        "image/jpeg",
                    ),
                self.theme_shell.theme(),
            ),
        };

        _shared::themed_shell(
            "Android",
            column![
                _shared::tabs(&labels, self.active, Message::Select),
                container(content).center_x(Length::Fill)
            ]
            .spacing(20),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
