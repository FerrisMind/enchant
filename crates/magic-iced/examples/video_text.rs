mod _shared;

use iced::time::Instant;
use iced::widget::{column, image, text};
use iced::{Element, Length, Size, Subscription, Task};
use magic_iced::{FrameSequence, VideoTextProps, VideoTextSource, VideoTextState, video_text};
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        VideoTextExample::new,
        VideoTextExample::update,
        VideoTextExample::view,
    )
    .title("VideoText Example")
    .window(iced::window::Settings {
        size: Size::new(1200.0, 800.0),
        ..Default::default()
    })
    .subscription(VideoTextExample::subscription)
    .run()
}

struct VideoTextExample {
    theme_shell: _shared::ThemeShell,
    active: usize,
    now: Instant,
    state: VideoTextState,
    props: VideoTextProps,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(magic_iced::AnchorMeasurements),
    Select(usize),
    Tick(Instant),
}

impl VideoTextExample {
    fn new() -> (Self, Task<Message>) {
        let now = Instant::now();
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
               active: 0,
                now,
                state: VideoTextState::new(),
                props: Self::props_for(0),
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

            Message::Select(index) => {
                self.active = index.min(1);
                self.now = Instant::now();
                self.state = VideoTextState::new();
                self.props = Self::props_for(self.active);
            }
            Message::Tick(now) => {
                self.theme_shell.tick(now);
                self.now = now;
                self.state.tick(now);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let labels = ["Frames", "Image"];

        _shared::themed_shell("Video Text", column![
                _shared::tabs(&labels, self.active, Message::Select),
                video_text(&self.state, self.now, &self.props),
                text("The top label and background source can be swapped without losing frame parity."),
            ]
            .spacing(20),
            &self.theme_shell, Message::ToggleTheme)
    }
}

impl VideoTextExample {
    fn props_for(active: usize) -> VideoTextProps {
        match active {
            0 => VideoTextProps {
                width: Length::Fill,
                height: Length::Fixed(420.0),
                source: VideoTextSource::Frames(FrameSequence::new(
                    vec![
                        frame([0x13, 0x1C, 0x2E, 0xFF]),
                        frame([0x2F, 0x4B, 0x7C, 0xFF]),
                        frame([0x67, 0xA4, 0xFF, 0xFF]),
                    ],
                    140,
                )),
                text: "MAGIC".into(),
                font_size: 150.0,
                font_weight: 700.0,
                font_family: "Sans".into(),
                text_color: iced::Color::WHITE,
            },
            _ => VideoTextProps {
                width: Length::Fill,
                height: Length::Fixed(420.0),
                source: VideoTextSource::Image(image::Handle::from_rgba(
                    4,
                    4,
                    vec![
                        0x1C, 0x26, 0x3D, 0xFF, 0x1C, 0x26, 0x3D, 0xFF, 0x4D, 0x74, 0xA8, 0xFF,
                        0x4D, 0x74, 0xA8, 0xFF, 0x1C, 0x26, 0x3D, 0xFF, 0x1C, 0x26, 0x3D, 0xFF,
                        0x4D, 0x74, 0xA8, 0xFF, 0x4D, 0x74, 0xA8, 0xFF, 0x1C, 0x26, 0x3D, 0xFF,
                        0x1C, 0x26, 0x3D, 0xFF, 0x4D, 0x74, 0xA8, 0xFF, 0x4D, 0x74, 0xA8, 0xFF,
                        0x1C, 0x26, 0x3D, 0xFF, 0x1C, 0x26, 0x3D, 0xFF, 0x4D, 0x74, 0xA8, 0xFF,
                        0x4D, 0x74, 0xA8, 0xFF,
                    ],
                )),
                text: "ICED".into(),
                font_size: 150.0,
                font_weight: 700.0,
                font_family: "Sans".into(),
                text_color: iced::Color::WHITE,
            },
        }
    }
}

fn frame(color: [u8; 4]) -> image::Handle {
    image::Handle::from_rgba(
        4,
        4,
        vec![
            color[0], color[1], color[2], color[3], color[0], color[1], color[2], color[3],
            color[0], color[1], color[2], color[3], color[0], color[1], color[2], color[3],
            color[0], color[1], color[2], color[3], color[0], color[1], color[2], color[3],
            color[0], color[1], color[2], color[3], color[0], color[1], color[2], color[3],
            color[0], color[1], color[2], color[3], color[0], color[1], color[2], color[3],
            color[0], color[1], color[2], color[3], color[0], color[1], color[2], color[3],
            color[0], color[1], color[2], color[3], color[0], color[1], color[2], color[3],
            color[0], color[1], color[2], color[3], color[0], color[1], color[2], color[3],
            color[0], color[1], color[2], color[3], color[0], color[1], color[2], color[3],
            color[0], color[1], color[2], color[3], color[0], color[1], color[2], color[3],
            color[0], color[1], color[2], color[3], color[0], color[1], color[2], color[3],
            color[0], color[1], color[2], color[3], color[0], color[1], color[2], color[3],
            color[0], color[1], color[2], color[3], color[0], color[1], color[2], color[3],
            color[0], color[1], color[2], color[3], color[0], color[1], color[2], color[3],
            color[0], color[1], color[2], color[3], color[0], color[1], color[2], color[3],
        ],
    )
}
