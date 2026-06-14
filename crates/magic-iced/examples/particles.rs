mod _shared;

use iced::time::Instant;
use iced::widget::{column, container, stack, text};
use iced::{Alignment, Border, Color, Element, Length, Size, Subscription, Task};
use magic_iced::{ParticlesAction, ParticlesProps, ParticlesState, particles};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        ParticlesExample::new,
        ParticlesExample::update,
        ParticlesExample::view,
    )
    .title("Particles")
    .window(iced::window::Settings {
        size: Size::new(900.0, 620.0),
        ..Default::default()
    })
    .subscription(ParticlesExample::subscription)
    .run()
}

struct ParticlesExample {
    theme_shell: _shared::ThemeShell,
    state: ParticlesState,
    start: Instant,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(magic_iced::AnchorMeasurements),
    Particles(ParticlesAction),
    Tick(Instant),
}

impl ParticlesExample {
    fn new() -> (Self, Task<Message>) {
        let start = Instant::now();
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                state: ParticlesState::new(),
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

            Message::Particles(action) => self.state.apply_action(&action),
            Message::Tick(now) => self.elapsed = now.duration_since(self.start),
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let props = ParticlesProps {
            width: Length::Fill,
            height: Length::Fill,
            quantity: 100,
            ease: 80.0,
            color: Color::from_rgb8(0xE5, 0xE7, 0xEB),
            ..ParticlesProps::new()
        };

        let stage = container(stack(vec![
            container(particles(
                &self.state,
                self.elapsed,
                props,
                Message::Particles,
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
            container(text("Particles").size(72))
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

        _shared::themed_shell(
            "Particles",
            column![stage].spacing(24).align_x(Alignment::Center),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
