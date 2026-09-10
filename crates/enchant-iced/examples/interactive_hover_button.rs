mod _shared;

use enchant_iced::{InteractiveHoverButton, InteractiveHoverIcon};
use iced::time::Instant;
use iced::widget::{column, text};
use iced::{Element, Font, Size, Subscription, Task};
use lucide_icons::{Icon as LucideIcon, LUCIDE_FONT_BYTES};
use std::time::Duration;

/// Lucide default SVG size measured on the svelte docs preview.
const DOCS_ARROW_SIZE: f32 = 24.0;

pub fn main() -> iced::Result {
    iced::application(
        InteractiveHoverButtonExample::new,
        InteractiveHoverButtonExample::update,
        InteractiveHoverButtonExample::view,
    )
    .title("Interactive Hover Button")
    .font(LUCIDE_FONT_BYTES)
    .window(iced::window::Settings {
        size: Size::new(1000.0, 700.0),
        ..Default::default()
    })
    .subscription(InteractiveHoverButtonExample::subscription)
    .run()
}

struct InteractiveHoverButtonExample {
    theme_shell: _shared::ThemeShell,
    start: Instant,
    elapsed: Duration,
    presses: u32,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Tick(Instant),
    Pressed,
}

impl InteractiveHoverButtonExample {
    fn new() -> (Self, Task<Message>) {
        let start = Instant::now();
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                start,
                elapsed: Duration::ZERO,
                presses: 0,
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
            Message::Pressed => {
                self.presses = self.presses.saturating_add(1);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let now = self.theme_shell.now;
        let palette = self.theme_shell.blended_theme().palette;

        // Docs demo: optional Lucide ArrowRight at 24px — not baked into the widget.
        let docs_arrow = InteractiveHoverIcon::new(
            char::from(LucideIcon::ArrowRight),
            Font::with_name("lucide"),
            DOCS_ARROW_SIZE,
        );

        let body = InteractiveHoverButton::new("Hover Me")
            .hover_icon(docs_arrow)
            .on_press(Message::Pressed)
            .background(self.theme_shell.blended_shiny_background(now))
            .primary(self.theme_shell.blended_primary(now))
            .idle_foreground(self.theme_shell.blended_primary(now))
            .hover_foreground(palette.primary_foreground)
            .border_color(self.theme_shell.blended_shiny_border(now))
            .into_element(self.elapsed);

        let feedback = text(format!("clicks: {}", self.presses))
            .size(14)
            .color(self.theme_shell.blended_foreground(now));

        _shared::themed_shell(
            "Interactive Hover Button",
            column![body, feedback]
                .spacing(16)
                .align_x(iced::Alignment::Center),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
