mod _shared;

use iced::time::Instant;
use iced::widget::{column, container, stack, Space};
use iced::{Alignment, Element, Length, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{AnimatedListProps, AnimatedListState, animated_list};
use std::time::Duration;
use twill::prelude::{SemanticThemeVars, ThemeVariant};

use _shared::animated_list_notifications::{
    bottom_background_fade, notification_card, repeated_notifications,
};

const STAGE_HEIGHT: f32 = 500.0;
const FADE_HEIGHT: f32 = STAGE_HEIGHT / 4.0;

pub fn main() -> iced::Result {
    iced::application(
        AnimatedListDemoExample::new,
        AnimatedListDemoExample::update,
        AnimatedListDemoExample::view,
    )
    .title("Animated List Demo")
    .window(iced::window::Settings {
        size: Size::new(900.0, 620.0),
        ..Default::default()
    })
    .subscription(AnimatedListDemoExample::subscription)
    .run()
}

struct AnimatedListDemoExample {
    theme: Theme,
    props: AnimatedListProps,
    state: AnimatedListState,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl AnimatedListDemoExample {
    fn new() -> (Self, Task<Message>) {
        let props = AnimatedListProps::new();
        let now = Instant::now();
        (
            Self {
                theme: Theme::from_semantic_theme(
                    SemanticThemeVars::shadcn_neutral(),
                    ThemeVariant::Dark,
                ),
                state: AnimatedListState::new(&props),
                props,
                now,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        if let Message::Tick(now) = message {
            self.now = now;
            self.state.tick(now, &self.props);
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let items = repeated_notifications(10)
            .into_iter()
            .map(|item| notification_card(item, &self.theme))
            .collect();

        let list = container(animated_list(items, &self.state, self.now, &self.props))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(8)
            .align_x(Alignment::Center)
            .align_y(Alignment::Start);

        let fade = container(
            column![
                Space::new().height(Length::Fill),
                bottom_background_fade(FADE_HEIGHT, self.theme.palette.background),
            ]
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill);

        let stage = container(
            stack![list, fade]
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fixed(STAGE_HEIGHT))
        .clip(true);

        _shared::shell("Animated List Demo", column![stage].align_x(Alignment::Center))
    }
}
