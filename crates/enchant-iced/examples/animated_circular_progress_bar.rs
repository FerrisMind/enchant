mod _shared;

use enchant_iced::{
    AnimatedCircularProgressBarProps, AnimatedCircularProgressBarState,
    animated_circular_progress_bar,
};
use iced::time::{Duration, Instant};
use iced::widget::column;
use iced::{Color, Element, Size, Subscription, Task};

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
    theme_shell: _shared::ThemeShell,
    value: f32,
    last_step: Instant,
    props: AnimatedCircularProgressBarProps,
    state: AnimatedCircularProgressBarState,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
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
                theme_shell: _shared::ThemeShell::new(),
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
        match message {
            Message::ToggleTheme => {
                return _shared::ThemeShell::measure_toggle_anchor().map(Message::AnchorsMeasured);
            }
            Message::AnchorsMeasured(measurements) => {
                self.theme_shell.apply_anchor_and_toggle(&measurements);
            }
            Message::Tick(now) => {
                self.theme_shell.tick(now);
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
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let foreground = self.theme_shell.blended_foreground(self.now);
        let body = animated_circular_progress_bar(
            &self.state,
            self.now,
            &self.props,
            self.theme_shell.theme(),
            Some(foreground),
        );
        _shared::themed_shell(
            "Animated Circular Progress Bar",
            column![body].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
