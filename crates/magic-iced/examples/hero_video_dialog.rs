mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::{button, column, container, image, text};
use iced::{Background, Border, Element, Length, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{HeroVideoDialogProps, HeroVideoDialogState, hero_video_dialog};

fn thumbnail_handle() -> image::Handle {
    let mut pixels = vec![0_u8; 320 * 180 * 4];
    for y in 0..180 {
        for x in 0..320 {
            let i = (y * 320 + x) * 4;
            pixels[i] = 30 + (x / 4) as u8;
            pixels[i + 1] = 40 + (y / 3) as u8;
            pixels[i + 2] = 120 + ((x + y) / 6) as u8;
            pixels[i + 3] = 255;
        }
    }
    image::Handle::from_rgba(320, 180, pixels)
}

pub fn main() -> iced::Result {
    iced::application(
        HeroVideoDialogExample::new,
        HeroVideoDialogExample::update,
        HeroVideoDialogExample::view,
    )
    .title("Hero Video Dialog")
    .window(iced::window::Settings {
        size: Size::new(900.0, 560.0),
        ..Default::default()
    })
    .subscription(HeroVideoDialogExample::subscription)
    .run()
}

struct HeroVideoDialogExample {
    theme: Theme,
    state: HeroVideoDialogState,
    props: HeroVideoDialogProps,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    Open,
    Close,
    Tick(Instant),
}

impl HeroVideoDialogExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme: Theme::default(),
                state: HeroVideoDialogState::new(),
                props: HeroVideoDialogProps::new(),
                now: Instant::now(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Open => self.state.set_open(true, self.now, &self.props),
            Message::Close => self.state.set_open(false, self.now, &self.props),
            Message::Tick(now) => {
                self.now = now;
                self.state.tick(now);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn thumbnail(&self) -> Element<'_, Message> {
        button(
            container(image(thumbnail_handle()).width(Length::Fill).height(Length::Fill))
                .width(Length::Fixed(480.0))
                .height(Length::Fixed(270.0))
                .style(|_| container::Style {
                    border: Border {
                        radius: 20.0.into(),
                        ..Border::default()
                    },
                    ..container::Style::default()
                }),
        )
        .on_press(Message::Open)
        .into()
    }

    fn video_content(&self) -> Element<'_, Message> {
        container(
            column![
                text("Hero Video").size(24),
                text("Embedded product walkthrough").size(14),
                button(text("Close")).on_press(Message::Close),
            ]
            .spacing(12)
            .padding(24),
        )
        .width(Length::Fill)
        .height(Length::Fixed(320.0))
        .style({
            let card = self.theme.palette.card;
            move |_| container::Style {
                background: Some(Background::Color(card)),
                border: Border {
                    radius: 16.0.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            }
        })
        .into()
    }

    fn view(&self) -> Element<'_, Message> {
        let body = hero_video_dialog(
            &self.state,
            self.now,
            self.thumbnail(),
            self.video_content(),
            Message::Open,
            Message::Close,
            self.props,
            &self.theme,
        );

        _shared::shell("Hero Video Dialog", column![body].spacing(24))
    }
}
