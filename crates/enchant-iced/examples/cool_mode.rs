mod _shared;

use enchant_iced::{CoolMode, CoolParticleKind};
use iced::time::Instant;
use iced::widget::{column, container, image, text};
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
        size: Size::new(900.0, 720.0),
        ..Default::default()
    })
    .subscription(CoolModeExample::subscription)
    .run()
}

struct CoolModeExample {
    theme_shell: _shared::ThemeShell,
    start: Instant,
    elapsed: Duration,
    svelte_logo: image::Handle,
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
                svelte_logo: image::Handle::from_bytes(
                    include_bytes!("assets/cool-mode/svelte-logo.png").as_slice(),
                ),
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
        // Pass the button only — not a Fill-centering wrapper (that made the whole
        // stage the CoolMode content hit target). CoolMode fills the stage for
        // particle travel and centers Shrink content internally.
        let click_me = button_content(
            text("Click Me!"),
            None::<Message>,
            ButtonProps::new()
                .variant(ButtonVariant::Default)
                .size(ButtonSize::Size2),
            self.theme_shell.theme(),
        );

        let svelte_image = button_content(
            text("Svelte Image"),
            None::<Message>,
            ButtonProps::new()
                .variant(ButtonVariant::Default)
                .size(ButtonSize::Size2),
            self.theme_shell.theme(),
        );

        let preview_stage = container(CoolMode::new(click_me).into_element(self.elapsed))
            .width(Length::Fill)
            .height(Length::Fixed(300.0))
            .center_x(Length::Fill)
            .center_y(Length::Fill);

        let custom_image_stage = container(
            CoolMode::new(svelte_image)
                .particle(CoolParticleKind::Image(self.svelte_logo.clone()))
                .into_element(self.elapsed),
        )
        .width(Length::Fill)
        .height(Length::Fixed(300.0))
        .center_x(Length::Fill)
        .center_y(Length::Fill);

        _shared::themed_shell(
            "Cool Mode",
            column![preview_stage, custom_image_stage]
                .spacing(24)
                .width(Length::Fill),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
