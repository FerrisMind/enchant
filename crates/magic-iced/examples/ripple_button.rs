mod _shared;

use iced::time::Instant;
use iced::widget::{column, text};
use iced::{Element, Size, Subscription, Task};
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
        size: Size::new(1000.0, 760.0),
        ..Default::default()
    })
    .subscription(RippleButtonExample::subscription)
    .run()
}

struct RippleButtonExample {
    theme: Theme,
    state: RippleButtonState,
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
        (
            Self {
                theme: Theme::default(),
                state: RippleButtonState::new(),
                now,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick(now) => {
                self.now = now;
                self.state.tick(now, &RippleButtonProps::new());
            }
            Message::Action(action) => {
                self.state.apply_action(&action, self.now, &RippleButtonProps::new());
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        _shared::shell(
            "Ripple Button",
            column![
                ripple_button(
                    text("Tap me"),
                    &self.state,
                    self.now,
                    RippleButtonProps::new(),
                    Message::Action,
                    &self.theme,
                ),
                text("Press, move, and leave the button to see ripple bursts."),
            ]
            .spacing(20),
        )
    }
}
