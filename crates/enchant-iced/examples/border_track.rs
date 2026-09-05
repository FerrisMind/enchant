mod _shared;

use enchant_iced::BorderBeam;
use iced::time::Instant;
use iced::widget::{column, container, stack, text};
use iced::{Alignment, Border, Element, Length, Size, Subscription, Task};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        BorderTrackExample::new,
        BorderTrackExample::update,
        BorderTrackExample::view,
    )
    .title("Border Track")
    .window(iced::window::Settings {
        size: Size::new(900.0, 520.0),
        ..Default::default()
    })
    .subscription(BorderTrackExample::subscription)
    .run()
}

struct BorderTrackExample {
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

impl BorderTrackExample {
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

    fn tracked_card(&self, label: &'static str, radius: f32) -> Element<'static, Message> {
        let now = self.theme_shell.now;
        let transition = self.theme_shell.controller.is_transition_active(now);
        let border_color = self.theme_shell.blended_border(now);
        let label_color = self.theme_shell.blended_foreground(now);
        let border_radius = radius;
        container(stack(vec![
            container(text(label).size(16).style({
                let color = label_color;
                move |_| iced::widget::text::Style { color: Some(color) }
            }))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
            container(
                BorderBeam::new()
                    .radius(radius)
                    .size(80.0)
                    .duration_secs(6.0)
                    .beam_enabled(!transition)
                    .into_element(self.elapsed),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
        ]))
        .width(Length::Fixed(220.0))
        .height(Length::Fixed(120.0))
        .style(move |_| container::Style {
            border: Border {
                width: 1.0,
                radius: border_radius.into(),
                color: border_color,
            },
            ..container::Style::default()
        })
        .into()
    }

    fn view(&self) -> Element<'_, Message> {
        let muted = self.theme_shell.blended_theme().palette.muted_foreground;
        let body = column![
            text("Rounded-rect perimeter sampling powers Border Beam and Shine Border.")
                .size(14)
                .style({
                    let color = muted;
                    move |_| iced::widget::text::Style { color: Some(color) }
                }),
            column![
                self.tracked_card("radius 12", 12.0),
                self.tracked_card("radius 24", 24.0),
                self.tracked_card("radius 36", 36.0),
            ]
            .spacing(16)
            .align_x(Alignment::Center),
        ]
        .spacing(20)
        .align_x(Alignment::Center);

        _shared::themed_shell(
            "Border Track",
            body,
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
