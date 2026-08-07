mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::column;
use iced::{Color, Element, Size, Subscription, Task};
use enchant_iced::{
    Text3DFlipAction, Text3DFlipDirection, Text3DFlipProps, Text3DFlipStaggerFrom, Text3DFlipState,
    text_3d_flip,
};

pub fn main() -> iced::Result {
    iced::application(
        Text3dFlipExample::new,
        Text3dFlipExample::update,
        Text3dFlipExample::view,
    )
    .title("Text 3D Flip")
    .window(iced::window::Settings {
        size: Size::new(900.0, 360.0),
        ..Default::default()
    })
    .subscription(Text3dFlipExample::subscription)
    .run()
}

struct Text3dFlipExample {
    theme_shell: _shared::ThemeShell,
    state: Text3DFlipState,
    props: Text3DFlipProps,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Flip(Text3DFlipAction),
    Tick(Instant),
}

impl Text3dFlipExample {
    fn new() -> (Self, Task<Message>) {
        let props = Text3DFlipProps {
            rotate_direction: Text3DFlipDirection::Top,
            stagger_from: Text3DFlipStaggerFrom::First,
            stagger_duration_secs: 0.03,
            font_size: 48.0,
            color: Color::WHITE,
            ..Text3DFlipProps::new()
        };
        let mut state = Text3DFlipState::new();
        let now = Instant::now();
        state.apply_action(&Text3DFlipAction::Trigger, now);
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                state,
                props,
                now,
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

            Message::Flip(action) => self.state.apply_action(&action, self.now),
            Message::Tick(now) => {
                self.theme_shell.tick(now);
                self.now = now;
                self.state
                    .tick(now, "Stay hungry, stay foolish", &self.props);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let body = text_3d_flip(
            "Stay hungry, stay foolish",
            &self.state,
            self.now,
            self.props,
            Message::Flip,
        );
        _shared::themed_shell(
            "Text 3D Flip",
            column![body].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
