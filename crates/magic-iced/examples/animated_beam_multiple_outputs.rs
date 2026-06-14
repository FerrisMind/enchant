mod _shared;

use iced::time::Instant;
use iced::widget::{column, container};
use iced::{Element, Length, Size, Subscription, Task};
use magic_iced::{AnchorMeasurements, AnchorRemeasure, measure_anchors};
use std::time::Duration;

use _shared::beam_demo;
use _shared::beam_scene::BeamSceneKind;

pub fn main() -> iced::Result {
    iced::application(
        AnimatedBeamMultipleOutputsExample::new,
        AnimatedBeamMultipleOutputsExample::update,
        AnimatedBeamMultipleOutputsExample::view,
    )
    .title("Animated Beam Multiple Outputs")
    .window(iced::window::Settings {
        size: Size::new(1200.0, 820.0),
        ..Default::default()
    })
    .subscription(AnimatedBeamMultipleOutputsExample::subscription)
    .run()
}

struct AnimatedBeamMultipleOutputsExample {
    theme_shell: _shared::ThemeShell,
    started_at: Instant,
    elapsed: Duration,
    measurements: Option<AnchorMeasurements>,
    remeasure: AnchorRemeasure,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(magic_iced::AnchorMeasurements),
    Tick(Instant),
    Measured(AnchorMeasurements),
}

impl AnimatedBeamMultipleOutputsExample {
    fn new() -> (Self, Task<Message>) {
        let started_at = Instant::now();
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
               started_at,
                elapsed: Duration::ZERO,
                measurements: None,
                remeasure: AnchorRemeasure::new(),
            },
            measure_anchors(beam_demo::measure_ids(BeamSceneKind::MultipleOutputs))
                .map(Message::Measured),
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
            Message::Tick(now) => {
                self.theme_shell.tick(now);
                self.elapsed = now.duration_since(self.started_at);
                return if self.remeasure.take() {
                    measure_anchors(beam_demo::measure_ids(BeamSceneKind::MultipleOutputs))
                        .map(Message::Measured)
                } else {
                    Task::none()
                };
            }
            Message::Measured(measurements) => {
                self.measurements = Some(measurements);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let scene = beam_demo::multiple_outputs_scene(self.elapsed, self.measurements.as_ref());

        _shared::themed_shell(
            "Animated Beam Multiple Outputs",
            column![
                container(scene)
                    .width(Length::Fill)
                    .center_x(Length::Fill),
            ]
            .spacing(20),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
