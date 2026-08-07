mod _shared;

use iced::time::Instant;
use iced::widget::{column, container};
use iced::{Alignment, Background, Border, Color, Element, Length, Size, Subscription, Task};
use enchant_iced::{GlyphMatrixProps, glyph_matrix};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        GlyphMatrixExample::new,
        GlyphMatrixExample::update,
        GlyphMatrixExample::view,
    )
    .title("Glyph Matrix")
    .window(iced::window::Settings {
        size: Size::new(900.0, 620.0),
        ..Default::default()
    })
    .subscription(GlyphMatrixExample::subscription)
    .run()
}

struct GlyphMatrixExample {
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

impl GlyphMatrixExample {
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
            Message::Tick(now) => self.elapsed = now.duration_since(self.start),
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(90)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let theme = self.theme_shell.theme();
        let props = GlyphMatrixProps::new()
            .width(Length::Fill)
            .height(Length::Fill)
            .color(theme.palette.foreground);
        let stage = container(glyph_matrix(self.elapsed, props))
            .width(Length::Fixed(720.0))
            .height(Length::Fixed(500.0))
            .style(move |_| container::Style {
                background: Some(Background::Color(theme.palette.background)),
                border: Border {
                    width: 1.0,
                    color: Color {
                        a: 0.2,
                        ..theme.palette.foreground
                    },
                    radius: 12.0.into(),
                },
                ..container::Style::default()
            });

        _shared::themed_shell(
            "Glyph Matrix",
            column![stage].spacing(24).align_x(Alignment::Center),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
