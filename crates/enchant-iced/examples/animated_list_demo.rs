mod _shared;

use enchant_iced::{AnimatedListProps, AnimatedListState, animated_list};
use iced::time::Instant;
use iced::widget::scrollable::Scrollbar;
use iced::widget::{column, container, scrollable, stack};
use iced::{Alignment, Element, Length, Size, Subscription, Task};
use std::time::Duration;

use _shared::animated_list_notifications::{
    bottom_background_fade, notification_card, repeated_notifications,
};

const STAGE_HEIGHT: f32 = 500.0;
/// Room for rounded card shadows inside the clipped stage (`blur_radius` + offset).
const STAGE_PADDING: f32 = 24.0;
const FADE_HEIGHT: f32 = STAGE_HEIGHT / 4.0;
const VIEWPORT_HEIGHT: f32 = STAGE_HEIGHT;

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
    theme_shell: _shared::ThemeShell,
    props: AnimatedListProps,
    state: AnimatedListState,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Tick(Instant),
}

impl AnimatedListDemoExample {
    fn new() -> (Self, Task<Message>) {
        let props = AnimatedListProps::new();
        let now = Instant::now();
        (
            Self {
                theme_shell: _shared::ThemeShell::new_dark(),
                state: AnimatedListState::new(&props),
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
            Message::Tick(now) => {
                self.theme_shell.tick(now);
                self.now = now;
                self.state.tick(now, &self.props);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let items = repeated_notifications(10)
            .into_iter()
            .map(|item| notification_card(item, self.theme_shell.theme()))
            .collect();

        let list = scrollable(
            container(animated_list(items, &self.state, self.now, &self.props))
                .padding(STAGE_PADDING),
        )
        .width(Length::Fill)
        .height(Length::Fixed(VIEWPORT_HEIGHT))
        .direction(scrollable::Direction::Vertical(Scrollbar::hidden()));

        let fade = container(bottom_background_fade(
            FADE_HEIGHT,
            self.theme_shell.theme().palette.background,
        ))
        .width(Length::Fill)
        .height(Length::Fixed(FADE_HEIGHT));

        let stage = container(
            stack![
                container(list).width(Length::Fill).height(Length::Fill),
                container(fade)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::End),
            ]
            .width(Length::Fill)
            .height(Length::Fixed(STAGE_HEIGHT)),
        )
        .width(Length::Fill)
        .height(Length::Fixed(STAGE_HEIGHT))
        .clip(true);

        _shared::themed_shell(
            "Animated List Demo",
            column![stage].spacing(20).align_x(Alignment::Center),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
