mod _shared;

use iced::time::Instant;

use enchant_iced::{SafariMode, SafariProps, safari};
use iced::widget::{column, container};
use iced::{Element, Length, Size, Subscription, Task};

pub fn main() -> iced::Result {
    iced::application(
        SafariExample::new,
        SafariExample::update,
        SafariExample::view,
    )
    .title("Safari Example")
    .window(iced::window::Settings {
        size: Size::new(1200.0, 900.0),
        ..Default::default()
    })
    .subscription(SafariExample::subscription)
    .run()
}

struct SafariExample {
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

impl SafariExample {
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

            Message::Select(index) => self.active = index.min(3),
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
        let labels = [
            "Safari Demo",
            "Safari Demo 2",
            "Safari Demo 3",
            "Safari Demo 4",
        ];
        let content = match self.active {
            0 => safari(
                SafariProps::new()
                    .width(Length::Fixed(680.0))
                    .chrome_label("magicui.design")
                    .screen_media(false),
                self.theme_shell.theme(),
            ),
            1 => safari(
                SafariProps::new()
                    .width(Length::Fixed(680.0))
                    .chrome_label("magicui.design")
                    .screen_media(true)
                    .media(include_bytes!("assets/hero-light.png"), "image/png"),
                self.theme_shell.theme(),
            ),
            2 => safari(
                SafariProps::new()
                    .width(Length::Fixed(680.0))
                    .chrome_label("magicui.design")
                    .screen_media(true)
                    .media(include_bytes!("assets/hero-dark.png"), "image/png"),
                self.theme_shell.theme(),
            ),
            _ => safari(
                SafariProps::new()
                    .width(Length::Fixed(680.0))
                    .chrome_label("magicui.design")
                    .mode(SafariMode::Simple)
                    .screen_media(true)
                    .media(include_bytes!("assets/hero-light.png"), "image/png"),
                self.theme_shell.theme(),
            ),
        };

        _shared::themed_shell(
            "Safari",
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
