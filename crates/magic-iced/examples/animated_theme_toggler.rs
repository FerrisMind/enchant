mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::column;
use iced::{Element, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{
    AnimatedThemeTogglerController, AnimatedThemeTogglerProps,
    animated_theme_toggler_with_controller,
};

pub fn main() -> iced::Result {
    iced::application(
        AnimatedThemeTogglerExample::new,
        AnimatedThemeTogglerExample::update,
        AnimatedThemeTogglerExample::view,
    )
    .title("Animated Theme Toggler")
    .window(iced::window::Settings {
        size: Size::new(900.0, 320.0),
        ..Default::default()
    })
    .subscription(AnimatedThemeTogglerExample::subscription)
    .run()
}

struct AnimatedThemeTogglerExample {
    theme: Theme,
    dark_mode: bool,
    props: AnimatedThemeTogglerProps,
    controller: AnimatedThemeTogglerController,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    Tick(Instant),
}

impl AnimatedThemeTogglerExample {
    fn new() -> (Self, Task<Message>) {
        let props = AnimatedThemeTogglerProps::new();
        let controller = AnimatedThemeTogglerController::new(true, &props);
        (
            Self {
                theme: Theme::default(),
                dark_mode: true,
                props,
                controller,
                now: Instant::now(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleTheme => {
                self.controller.toggle(self.now, &self.props);
                self.dark_mode = self.controller.dark_mode();
            }
            Message::Tick(now) => {
                self.now = now;
                self.controller.tick(now, &self.props);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let body = animated_theme_toggler_with_controller(
            &self.controller,
            self.now,
            Message::ToggleTheme,
            self.props,
            &self.theme,
        );
        _shared::shell("Animated Theme Toggler", column![body].spacing(24))
    }
}
