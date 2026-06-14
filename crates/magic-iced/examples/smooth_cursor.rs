mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::{column, container, stack, text};
use iced::{Element, Length, Size, Subscription, Task};
use magic_iced::{SmoothCursorAction, SmoothCursorProps, SmoothCursorState, smooth_cursor};

pub fn main() -> iced::Result {
    iced::application(
        SmoothCursorExample::new,
        SmoothCursorExample::update,
        SmoothCursorExample::view,
    )
    .title("Smooth Cursor")
    .window(iced::window::Settings {
        size: Size::new(900.0, 420.0),
        ..Default::default()
    })
    .subscription(SmoothCursorExample::subscription)
    .run()
}

struct SmoothCursorExample {
    theme_shell: _shared::ThemeShell,
    state: SmoothCursorState,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(magic_iced::AnchorMeasurements),
    Cursor(SmoothCursorAction),
    Tick(Instant),
}

impl SmoothCursorExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                state: SmoothCursorState::new(),
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

            Message::Cursor(action) => {
                let props = SmoothCursorProps::<()>::new();
                self.state.apply_action(&action, self.now, &props);
            }
            Message::Tick(now) => {
                self.theme_shell.tick(now);
                self.now = now;
                let props = SmoothCursorProps::<()>::new();
                self.state.tick(now, &props);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let stage = container(
            column![
                text("Move your mouse around").size(22),
                text("SmoothCursor follows with spring physics").size(14),
            ]
            .spacing(8),
        )
        .width(Length::Fill)
        .height(Length::Fixed(280.0))
        .center_x(Length::Fill)
        .center_y(Length::Fill);

        let props = SmoothCursorProps::<Message>::new();
        let overlay = smooth_cursor(&self.state, self.now, props, Message::Cursor);
        let body = stack![stage, overlay];

        _shared::themed_shell(
            "Smooth Cursor",
            column![body].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
