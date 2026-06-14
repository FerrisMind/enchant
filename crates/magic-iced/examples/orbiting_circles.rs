mod _shared;

use iced::time::Instant;
use iced::widget::{column, container, stack};
use iced::{Alignment, Element, Length, Size, Subscription, Task};
use magic_iced::{OrbitingCirclesProps, orbiting_circles};
use std::time::Duration;

const OUTER_ICON_SIZE: f32 = 30.0;
const INNER_ICON_SIZE: f32 = 40.0;
const AVATAR_PAD_RATIO: f32 = 12.0 / 48.0;

pub fn main() -> iced::Result {
    iced::application(
        OrbitingCirclesExample::new,
        OrbitingCirclesExample::update,
        OrbitingCirclesExample::view,
    )
    .title("Orbiting Circles")
    .window(iced::window::Settings {
        size: Size::new(900.0, 620.0),
        ..Default::default()
    })
    .subscription(OrbitingCirclesExample::subscription)
    .run()
}

struct OrbitingCirclesExample {
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

impl OrbitingCirclesExample {
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
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn orbit_avatar(&self, icon: iced::widget::svg::Handle, size: f32) -> Element<'_, Message> {
        _shared::beam_demo::beam_avatar(icon, size, size * AVATAR_PAD_RATIO)
    }

    fn view(&self) -> Element<'_, Message> {
        let outer: Vec<Element<'_, Message>> = [
            _shared::beam_icons::whatsapp(),
            _shared::beam_icons::notion(),
            _shared::beam_icons::openai(),
            _shared::beam_icons::google_drive(),
        ]
        .into_iter()
        .map(|icon| self.orbit_avatar(icon, OUTER_ICON_SIZE))
        .collect();

        let inner: Vec<Element<'_, Message>> = [
            _shared::beam_icons::whatsapp(),
            _shared::beam_icons::notion(),
            _shared::beam_icons::openai(),
            _shared::beam_icons::google_drive(),
            _shared::beam_icons::whatsapp(),
        ]
        .into_iter()
        .map(|icon| self.orbit_avatar(icon, INNER_ICON_SIZE))
        .collect();

        let stage = container(stack(vec![
            orbiting_circles(
                inner,
                self.elapsed,
                OrbitingCirclesProps {
                    width: Length::Fill,
                    height: Length::Fill,
                    icon_size: INNER_ICON_SIZE,
                    radius: 160.0,
                    ..OrbitingCirclesProps::new()
                },
            ),
            orbiting_circles(
                outer,
                self.elapsed,
                OrbitingCirclesProps {
                    width: Length::Fill,
                    height: Length::Fill,
                    icon_size: OUTER_ICON_SIZE,
                    radius: 100.0,
                    reverse: true,
                    speed: 2.0,
                    ..OrbitingCirclesProps::new()
                },
            ),
        ]))
        .width(Length::Fixed(720.0))
        .height(Length::Fixed(500.0));

        _shared::themed_shell(
            "Orbiting Circles",
            column![stage].spacing(24).align_x(Alignment::Center),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
