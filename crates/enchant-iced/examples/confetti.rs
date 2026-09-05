mod _shared;

use enchant_iced::{Confetti, ConfettiButton, ConfettiState};
use iced::time::{Duration, Instant};
use iced::widget::{column, container, stack, text};
use iced::{Alignment, Border, Color, Element, Length, Size, Subscription, Task};

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
    confetti: Confetti,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Fire,
    Tick(Instant),
}

impl ConfettiExample {
    fn new() -> (Self, Task<Message>) {
        let now = Instant::now();
        let confetti = ConfettiButton::<Message>::new(text("")).confetti();
        let mut state = ConfettiState::new();
        state.fire(now, &confetti);

        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                state,
                confetti,
                now,
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
            Message::Fire => {
                self.state.clear();
                self.state.fire(self.now, &self.confetti);
            }
            Message::Tick(now) => {
                self.theme_shell.tick(now);
                self.now = now;
                self.state.tick(now, &self.confetti);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let fire_button = ConfettiButton::new(
            text("Fire Confetti")
                .size(14)
                .color(Color::WHITE),
        )
        .on_press(Message::Fire);

        let overlay = self
            .confetti
            .width(Length::Fill)
            .height(Length::Fill)
            .view(&self.state, self.now);

        let stage = container(
            stack![
                container(overlay)
                    .width(Length::Fill)
                    .height(Length::Fill),
                container(fire_button.into_element())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
            ]
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fixed(500.0))
        .style(|_| container::Style {
            border: Border {
                width: 1.0,
                radius: 12.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        });

        _shared::themed_shell(
            "Confetti",
            column![stage].spacing(24).align_x(Alignment::Center),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
