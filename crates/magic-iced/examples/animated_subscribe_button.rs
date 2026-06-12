mod _shared;

use iced::time::Instant;
use iced::widget::{button, column, text};
use iced::{Element, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{
    AnimatedSubscribeButtonProps, AnimatedSubscribeButtonState, animated_subscribe_button,
};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        AnimatedSubscribeButtonExample::new,
        AnimatedSubscribeButtonExample::update,
        AnimatedSubscribeButtonExample::view,
    )
    .title("Animated Subscribe Button")
    .window(iced::window::Settings {
        size: Size::new(1000.0, 760.0),
        ..Default::default()
    })
    .subscription(AnimatedSubscribeButtonExample::subscription)
    .run()
}

struct AnimatedSubscribeButtonExample {
    theme: Theme,
    uncontrolled: AnimatedSubscribeButtonState,
    controlled: AnimatedSubscribeButtonState,
    controlled_subscribed: bool,
    active_tab: usize,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
    PressUncontrolled,
    ToggleControlled,
    SelectTab(usize),
}

impl AnimatedSubscribeButtonExample {
    fn new() -> (Self, Task<Message>) {
        let now = Instant::now();
        let uncontrolled_props = AnimatedSubscribeButtonProps::new("Subscribe", "Subscribed");
        let controlled_props =
            AnimatedSubscribeButtonProps::new("Subscribe", "Subscribed").subscribe_status(false);
        (
            Self {
                theme: Theme::default(),
                uncontrolled: AnimatedSubscribeButtonState::new(&uncontrolled_props),
                controlled: AnimatedSubscribeButtonState::new(&controlled_props),
                controlled_subscribed: false,
                active_tab: 0,
                now,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick(now) => {
                self.now = now;
                let uncontrolled_props = AnimatedSubscribeButtonProps::new("Subscribe", "Subscribed");
                self.uncontrolled.tick(now, &uncontrolled_props);
                let controlled_props = AnimatedSubscribeButtonProps::new("Subscribe", "Subscribed")
                    .subscribe_status(self.controlled_subscribed);
                self.controlled.tick(now, &controlled_props);
            }
            Message::PressUncontrolled => {
                let props = AnimatedSubscribeButtonProps::new("Subscribe", "Subscribed");
                self.uncontrolled.press(self.now, &props);
            }
            Message::ToggleControlled => {
                self.controlled_subscribed = !self.controlled_subscribed;
            }
            Message::SelectTab(index) => {
                self.active_tab = index;
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let tabs = _shared::tabs(
            &["Uncontrolled", "Controlled"],
            self.active_tab,
            Message::SelectTab,
        );

        let body = if self.active_tab == 0 {
            let props = AnimatedSubscribeButtonProps::new("Subscribe", "Subscribed");
            column![
                animated_subscribe_button(
                    &self.uncontrolled,
                    self.now,
                    &props,
                    Some(Message::PressUncontrolled),
                    &self.theme,
                ),
                text("Tap the button to toggle between Subscribe and Subscribed."),
            ]
            .spacing(20)
        } else {
            let props = AnimatedSubscribeButtonProps::new("Subscribe", "Subscribed")
                .subscribe_status(self.controlled_subscribed);
            column![
                animated_subscribe_button(&self.controlled, self.now, &props, None, &self.theme),
                button(text("Toggle subscribe_status")).on_press(Message::ToggleControlled),
                text(format!(
                    "subscribe_status = {}",
                    if self.controlled_subscribed {
                        "true"
                    } else {
                        "false"
                    }
                )),
            ]
            .spacing(20)
        };

        _shared::shell(
            "Animated Subscribe Button",
            column![tabs, body].spacing(24),
        )
    }
}
