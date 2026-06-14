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
        AnimatedBeamExample::new,
        AnimatedBeamExample::update,
        AnimatedBeamExample::view,
    )
    .title("AnimatedBeam Example")
    .window(iced::window::Settings {
        size: Size::new(1200.0, 820.0),
        ..Default::default()
    })
    .subscription(AnimatedBeamExample::subscription)
    .run()
}

struct AnimatedBeamExample {
    theme_shell: _shared::ThemeShell,
    active: usize,
    started_at: Instant,
    elapsed: Duration,
    measurements: Option<AnchorMeasurements>,
    remeasure: AnchorRemeasure,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(magic_iced::AnchorMeasurements),
    Select(usize),
    Tick(Instant),
    Measured(AnchorMeasurements),
}

impl AnimatedBeamExample {
    fn new() -> (Self, Task<Message>) {
        let started_at = Instant::now();
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                active: 0,
                started_at,
                elapsed: Duration::ZERO,
                measurements: None,
                remeasure: AnchorRemeasure::new(),
            },
            measure_scene(0),
        )
    }

    fn scene_kind(active: usize) -> BeamSceneKind {
        match active {
            0 => BeamSceneKind::Demo,
            1 => BeamSceneKind::Unidirectional,
            2 => BeamSceneKind::Bidirectional,
            _ => BeamSceneKind::MultipleInputs,
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleTheme => {
                return _shared::ThemeShell::measure_toggle_anchor().map(Message::AnchorsMeasured);
            }
            Message::AnchorsMeasured(measurements) => {
                self.theme_shell.apply_anchor_and_toggle(&measurements);
            }
            Message::Select(index) => {
                self.active = index.min(3);
                self.started_at = Instant::now();
                self.elapsed = Duration::ZERO;
                self.measurements = None;
                self.remeasure.schedule();
                return measure_scene(self.active);
            }
            Message::Tick(now) => {
                self.theme_shell.tick(now);
                self.elapsed = now.duration_since(self.started_at);
                return if self.remeasure.take() {
                    measure_scene(self.active)
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
        let labels = ["Demo", "Unidirectional", "Bidirectional", "Multiple Inputs"];
        let measurements = self.measurements.as_ref();
        let scene = match self.active {
            0 => beam_demo::demo_scene(self.elapsed, measurements),
            1 => beam_demo::unidirectional_scene(self.elapsed, measurements),
            2 => beam_demo::bidirectional_scene(self.elapsed, measurements),
            _ => beam_demo::multiple_inputs_scene(self.elapsed, measurements),
        };

        _shared::themed_shell(
            "AnimatedBeam",
            column![
                _shared::tabs(&labels, self.active, Message::Select),
                container(scene).width(Length::Fill).center_x(Length::Fill),
            ]
            .spacing(20),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}

fn measure_scene(active: usize) -> Task<Message> {
    measure_anchors(beam_demo::measure_ids(AnimatedBeamExample::scene_kind(
        active,
    )))
    .map(Message::Measured)
}
