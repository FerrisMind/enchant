mod _shared;

use iced::time::Instant;
use iced::widget::column;
use iced::{Element, Length, Size, Subscription, Task};
use enchant_iced::{MorphingTextProps, morphing_text};
use std::time::Duration;

const MORPHING_TEXTS: &[&str] = &[
    "Hello",
    "Morphing",
    "Text",
    "Animation",
    "React",
    "Component",
    "Smooth",
    "Transition",
    "Engaging",
];

pub fn main() -> iced::Result {
    iced::application(
        MorphingTextExample::new,
        MorphingTextExample::update,
        MorphingTextExample::view,
    )
    .title("Morphing Text")
    .window(iced::window::Settings {
        size: Size::new(900.0, 420.0),
        ..Default::default()
    })
    .subscription(MorphingTextExample::subscription)
    .run()
}

struct MorphingTextExample {
    theme_shell: _shared::ThemeShell,
    start: Instant,
    elapsed: Duration,
    props: MorphingTextProps,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Tick(Instant),
}

impl MorphingTextExample {
    fn new() -> (Self, Task<Message>) {
        let start = Instant::now();
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                start,
                elapsed: Duration::ZERO,
                props: MorphingTextProps {
                    width: Length::Shrink,
                    ..MorphingTextProps::new(MORPHING_TEXTS.iter().copied())
                },
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

                self.elapsed = now.duration_since(self.start);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let props = MorphingTextProps {
            color: Some(self.theme_shell.blended_foreground(self.theme_shell.now)),
            ..self.props.clone()
        };
        let body = morphing_text(self.elapsed, &props, self.theme_shell.blended_theme());
        _shared::themed_shell(
            "Morphing Text",
            column![body].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
