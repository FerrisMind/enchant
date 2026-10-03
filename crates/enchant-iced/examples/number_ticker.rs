mod _shared;

use enchant_iced::NumberTicker;
use iced::font::Weight;
use iced::time::Instant;
use iced::widget::{button, column, text};
use iced::{Alignment, Background, Border, Color, Element, Font, Size, Subscription, Task};
use std::time::Duration;

const TICK_MS: u64 = 16;

pub fn main() -> iced::Result {
    iced::application(
        NumberTickerExample::new,
        NumberTickerExample::update,
        NumberTickerExample::view,
    )
    .title("Number Ticker")
    .window(iced::window::Settings {
        size: Size::new(900.0, 420.0),
        ..Default::default()
    })
    .subscription(NumberTickerExample::subscription)
    .run()
}

struct NumberTickerExample {
    theme_shell: _shared::ThemeShell,
    play_start: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Tick(Instant),
    Restart,
}

impl NumberTickerExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                play_start: Instant::now(),
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
            Message::Restart => {
                self.play_start = self.theme_shell.now;
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(TICK_MS)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let now = self.theme_shell.now;
        let elapsed = now.duration_since(self.play_start);
        let foreground = self.theme_shell.blended_foreground(now);

        let ticker = NumberTicker::new(100.0)
            .font_size(96.0)
            .letter_spacing(-0.05 * 96.0)
            .font(Font {
                weight: Weight::Medium,
                ..Font::DEFAULT
            })
            .color(foreground)
            .once(false)
            .into_element(elapsed);

        let primary = self.theme_shell.blended_primary(now);
        let primary_fg = self.theme_shell.blended_theme().palette.primary_foreground;
        let restart = button(text("Restart").size(14))
            .padding([10.0, 14.0])
            .on_press(Message::Restart)
            .style(move |_theme, status| primary_button_style(primary, primary_fg, status));

        let body = column![ticker, restart]
            .spacing(24)
            .align_x(Alignment::Center);

        _shared::themed_shell(
            "Number Ticker",
            body,
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}

fn primary_button_style(
    primary: Color,
    primary_fg: Color,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let mut style = iced::widget::button::Style {
        background: Some(Background::Color(primary)),
        text_color: primary_fg,
        border: Border {
            radius: 0.0.into(),
            ..Border::default()
        },
        ..iced::widget::button::Style::default()
    };

    if matches!(status, iced::widget::button::Status::Pressed) {
        style.text_color = Color {
            a: 0.92,
            ..primary_fg
        };
    }

    style
}
