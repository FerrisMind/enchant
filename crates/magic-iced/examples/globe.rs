mod _shared;

use iced::time::Instant;
use iced::widget::{column, container, stack, text};
use iced::{Alignment, Border, Element, Length, Size, Subscription, Task};
use magic_iced::{GlobeAction, GlobeMarker, GlobeProps, GlobeState, globe};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(GlobeExample::new, GlobeExample::update, GlobeExample::view)
        .title("Globe")
        .window(iced::window::Settings {
            size: Size::new(900.0, 620.0),
            ..Default::default()
        })
        .subscription(GlobeExample::subscription)
        .run()
}

struct GlobeExample {
    theme_shell: _shared::ThemeShell,
    state: GlobeState,
    props: GlobeProps,
    start: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(magic_iced::AnchorMeasurements),
    Globe(GlobeAction),
    Tick(Instant),
}

impl GlobeExample {
    fn new() -> (Self, Task<Message>) {
        let props = GlobeProps {
            width: Length::Fill,
            height: Length::Fill,
            markers: vec![
                GlobeMarker::new(40.7128, -74.006, 1.0),
                GlobeMarker::new(51.5074, -0.1278, 1.0),
                GlobeMarker::new(35.6762, 139.6503, 1.0),
            ],
            ..GlobeProps::new()
        };
        let state = GlobeState::new(&props);
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
               state,
                props,
                start: Instant::now(),
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

            Message::Globe(action) => self.state.apply_action(&action, &self.props),
            Message::Tick(now) => self.state.tick(now, &self.props),
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let stage = container(stack(vec![
            container(globe(&self.state, &self.props, Message::Globe))
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
            container(text("Globe").size(64))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding([120.0, 0.0])
                .center_x(Length::Fill)
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

        _shared::themed_shell("Globe", column![stage].spacing(24).align_x(Alignment::Center), &self.theme_shell, Message::ToggleTheme)
    }
}
