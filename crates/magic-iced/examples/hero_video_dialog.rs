mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::{column, container, image};
use iced::{Element, Length, Size, Subscription, Task};
use iced_shadcn::{ButtonProps, ButtonSize, ButtonVariant, Theme, button_content};
use iced::widget::text;
use magic_iced::{HeroVideoDialogProps, HeroVideoDialogState, hero_video_dialog};

fn hero_light_handle() -> image::Handle {
    image::Handle::from_bytes(include_bytes!("assets/hero-light.png").as_slice())
}

fn hero_dark_handle() -> image::Handle {
    image::Handle::from_bytes(include_bytes!("assets/hero-dark.png").as_slice())
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

    fn thumbnail(&self, handle: image::Handle) -> Element<'_, Message> {
        button_content(
            container(image(handle).width(Length::Fill).height(Length::Fill))
                .width(Length::Fixed(480.0))
                .height(Length::Fixed(270.0)),
            Some(Message::Open),
            ButtonProps::new()
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Size2),
            &self.theme,
        )
        .into()
    }

    fn video_content(&self) -> Element<'_, Message> {
        column![
            text("Hero Video").size(24),
            text("Embedded product walkthrough").size(14),
            button_content(
                text("Close"),
                Some(Message::Close),
                ButtonProps::new()
                    .variant(ButtonVariant::Outline)
                    .size(ButtonSize::Size2),
                &self.theme,
            ),
        ]
        .spacing(12)
        .padding(24)
        .into()
    }

    fn view(&self) -> Element<'_, Message> {
        let light = hero_video_dialog(
            &self.state,
            self.now,
            self.thumbnail(hero_light_handle()),
            self.video_content(),
            Message::Open,
            Message::Close,
            self.props,
            &self.theme,
        );

        let dark = hero_video_dialog(
            &self.state,
            self.now,
            self.thumbnail(hero_dark_handle()),
            self.video_content(),
            Message::Open,
            Message::Close,
            self.props,
            &self.theme,
        );

        _shared::shell(
            "Hero Video Dialog",
            column![light, dark].spacing(16),
        )
    }
}
