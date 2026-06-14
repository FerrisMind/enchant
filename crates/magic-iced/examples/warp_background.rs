mod _shared;

use iced::time::Instant;
use iced::widget::{column, text};
use iced::{Alignment, Element, Length, Size, Subscription, Task};
use iced_shadcn::{CardProps, CardSize, card};
use magic_iced::{WarpBackgroundProps, warp_background};
use std::time::Duration;

/// Tailwind `w-80` = 20rem = 320px at the default 16px root size.
const CARD_WIDTH: f32 = 320.0;

pub fn main() -> iced::Result {
    iced::application(
        WarpBackgroundExample::new,
        WarpBackgroundExample::update,
        WarpBackgroundExample::view,
    )
    .title("Warp Background")
    .window(iced::window::Settings {
        size: Size::new(900.0, 620.0),
        ..Default::default()
    })
    .subscription(WarpBackgroundExample::subscription)
    .run()
}

struct WarpBackgroundExample {
    theme_shell: _shared::ThemeShell,
    start: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(magic_iced::AnchorMeasurements),
    Tick(Instant),
}

impl WarpBackgroundExample {
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
        let theme = self.theme_shell.theme();
        let border = self.theme_shell.blended_border(self.theme_shell.now);
        let card_content = column![
            text("Congratulations on Your Promotion!")
                .size(16)
                .color(theme.palette.foreground),
            text(
                "Your hard work and dedication have paid off. We're thrilled to see you take this next step in your career. Keep up the fantastic work!"
            )
            .size(14)
            .color(theme.palette.muted_foreground),
        ]
        .spacing(8)
        .width(Length::Fill);

        let card = card(
            card_content,
            CardProps::new()
                .size(CardSize::Size2)
                .padding(16.0)
                .radius(theme.radius.lg),
            theme,
        )
        .width(Length::Fixed(CARD_WIDTH));

        let body = warp_background(
            card,
            self.elapsed,
            WarpBackgroundProps {
                width: Length::Shrink,
                height: Length::Shrink,
                radius: theme.radius.md,
                grid_color: Some(border),
                ..WarpBackgroundProps::new()
            },
            theme,
        );

        _shared::themed_shell(
            "Warp Background",
            column![body].spacing(24).align_x(Alignment::Center),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
