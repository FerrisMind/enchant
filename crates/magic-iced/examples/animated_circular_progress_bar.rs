mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::column;
use iced::{Color, Element, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{
    AnimatedCircularProgressBarProps, AnimatedCircularProgressBarState,
    animated_circular_progress_bar,
};

pub fn main() -> iced::Result {
    iced::application(
        AnimatedCircularProgressBarExample::new,
        AnimatedCircularProgressBarExample::update,
        AnimatedCircularProgressBarExample::view,
    )
    .title("Animated Circular Progress Bar")
    .window(iced::window::Settings {
        size: Size::new(900.0, 420.0),
        ..Default::default()
    })
    .subscription(AnimatedCircularProgressBarExample::subscription)
    .run()
}

struct AnimatedCircularProgressBarExample {
    theme: Theme,
    value: f32,
    last_step: Instant,
    props: AnimatedCircularProgressBarProps,
    state: AnimatedCircularProgressBarState,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl AnimatedCircularProgressBarExample {
    fn new() -> (Self, Task<Message>) {
        let props = AnimatedCircularProgressBarProps {
            fill_color: Some(Color::from_rgb8(0x4F, 0x46, 0xE5)),
            ..AnimatedCircularProgressBarProps::new(0.0)
        };
        let state = AnimatedCircularProgressBarState::new(&props);
        (
            Self {
                theme: Theme::default(),
                value: 0.0,
                last_step: Instant::now(),
                props,
                state,
                now: Instant::now(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        if let Message::Tick(now) = message {
            self.now = now;
            if now.duration_since(self.last_step) >= Duration::from_secs(2) {
                self.value = if self.value >= 100.0 {
                    0.0
                } else {
                    self.value + 10.0
                };
                self.last_step = now;
                self.props = AnimatedCircularProgressBarProps {
                    value: self.value,
                    fill_color: Some(Color::from_rgb8(0x4F, 0x46, 0xE5)),
                    ..AnimatedCircularProgressBarProps::new(self.value)
                };
            }
            self.state.tick(now, &self.props);
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let body = animated_circular_progress_bar(&self.state, self.now, &self.props, &self.theme);
        _shared::shell("Animated Circular Progress Bar", column![body].spacing(24))
    }
}
