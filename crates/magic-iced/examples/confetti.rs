mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::{column, container, stack, text};
use iced::{Alignment, Border, Element, Length, Size, Subscription, Task};
use magic_iced::{ConfettiAction, ConfettiProps, ConfettiState, confetti, confetti_button};

pub fn main() -> iced::Result {
    iced::application(
        ConfettiExample::new,
        ConfettiExample::update,
        ConfettiExample::view,
    )
    .title("Confetti")
    .window(iced::window::Settings {
        size: Size::new(900.0, 620.0),
        ..Default::default()
    })
    .subscription(ConfettiExample::subscription)
    .run()
}

struct ConfettiExample {
    theme_shell: _shared::ThemeShell,
    state: ConfettiState,
    props: ConfettiProps,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(magic_iced::AnchorMeasurements),
    Fire(ConfettiAction),
    Tick(Instant),
}

impl ConfettiExample {
    fn new() -> (Self, Task<Message>) {
        let props = ConfettiProps::new();
        let now = Instant::now();
        let mut state = ConfettiState::new();
        state.apply_action(&ConfettiAction::Fire(None), now, &props);
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                state,
                props,
                now,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleTheme => {
                return _shared::ThemeShell::measure_toggle_anchor()
                    .map(Message::AnchorsMeasured);
            }
            Message::AnchorsMeasured(measurements) => {
                self.theme_shell.apply_anchor_and_toggle(&measurements);
            }

            Message::Fire(action) => {
                self.state
                    .apply_action(&action, self.now, &self.props);
            }
            Message::Tick(now) => {
                self.theme_shell.tick(now);
                self.now = now;
                self.state.tick(now, &self.props);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let stage = container(stack(vec![
            container(confetti(&self.state, self.now, &self.props))
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
            container(
                column![
                    text("Confetti").size(72),
                    confetti_button(
                        text("Hover to celebrate").size(14),
                        &self.state,
                        self.now,
                        &self.props,
                        Message::Fire,
                    ),
                ]
                .spacing(16)
                .align_x(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
        ]))
        .width(Length::Fixed(720.0))
        .height(Length::Fixed(500.0))
        .style(|_| container::Style {
            border: Border {
                width: 1.0,
                radius: 12.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        });

        _shared::themed_shell("Confetti", column![stage].spacing(24).align_x(Alignment::Center), &self.theme_shell, Message::ToggleTheme)
    }
}
