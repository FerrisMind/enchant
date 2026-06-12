mod _shared;

use iced::time::Instant;
use iced::widget::{column, text};
use iced::{Color, Element, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{RippleButtonAction, RippleButtonProps, RippleButtonState, ripple_button};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        RippleButtonExample::new,
        RippleButtonExample::update,
        RippleButtonExample::view,
    )
    .title("Ripple Button")
    .window(iced::window::Settings {
        size: Size::new(900.0, 420.0),
        ..Default::default()
    })
    .subscription(RippleButtonExample::subscription)
    .run()
}

struct RippleButtonExample {
    theme: Theme,
    state: RippleButtonState,
    props: RippleButtonProps,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
    Action(RippleButtonAction),
}

impl RippleButtonExample {
    fn new() -> (Self, Task<Message>) {
        let now = Instant::now();
        let props = RippleButtonProps {
            ripple_color: Some(Color::from_rgb8(0xAD, 0xD8, 0xE6)),
            ..RippleButtonProps::new()
        };
        (
            Self {
                theme: Theme::default(),
                state: RippleButtonState::new(),
                props,
                now,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick(now) => {
                self.now = now;
                self.state.tick(now, &self.props);
            }
            Message::Action(action) => {
                self.state.apply_action(&action, self.now, &self.props);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let body = ripple_button(
            text("Click me"),
            &self.state,
            self.now,
            self.props,
            Message::Action,
            &self.theme,
        );

        _shared::shell("Ripple Button", column![body].spacing(24))
    }
}
