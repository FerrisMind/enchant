mod _shared;

use enchant_iced::CoolMode;
use iced::time::Instant;
use iced::widget::{column, container, text};
use iced::{Element, Length, Size, Subscription, Task};
use iced_shadcn::{ButtonProps, ButtonSize, ButtonVariant, button_content};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        CoolModeExample::new,
        CoolModeExample::update,
        CoolModeExample::view,
    )
    .title("Cool Mode")
    .window(iced::window::Settings {
        size: Size::new(900.0, 520.0),
        ..Default::default()
    })
    .subscription(CoolModeExample::subscription)
    .run()
}

struct CoolModeExample {
    theme_shell: _shared::ThemeShell,
    start: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Tick(Instant),
}

impl CoolModeExample {
    fn new() -> (Self, Task<Message>) {
        let start = Instant::now();
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                start,
                elapsed: Duration::ZERO,
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
                self.elapsed = now.duration_since(self.start);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let trigger = button_content(
            text("Click Me!"),
            None::<Message>,
            ButtonProps::new()
                .variant(ButtonVariant::Default)
                .size(ButtonSize::Size2),
            self.theme_shell.theme(),
        );

        let content = container(trigger)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill);

        // CoolMode fills so particles travel across the stage (not clipped to the button).
        let stage = container(
            CoolMode::new(content).into_element(self.elapsed),
        )
        .width(Length::Fill)
        .height(Length::Fixed(420.0));

        _shared::themed_shell(
            "Cool Mode",
            column![stage].spacing(24).width(Length::Fill),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
