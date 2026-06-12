mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::{button, column, container, text};
use iced::{Background, Border, Element, Length, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{GlareHoverAction, GlareHoverProps, GlareHoverState, glare_hover};

pub fn main() -> iced::Result {
    iced::application(
        GlareHoverExample::new,
        GlareHoverExample::update,
        GlareHoverExample::view,
    )
    .title("Glare Hover")
    .window(iced::window::Settings {
        size: Size::new(900.0, 520.0),
        ..Default::default()
    })
    .subscription(GlareHoverExample::subscription)
    .run()
}

struct GlareHoverExample {
    theme: Theme,
    state: GlareHoverState,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    Glare(GlareHoverAction),
    Tick(Instant),
}

impl GlareHoverExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme: Theme::default(),
                state: GlareHoverState::new(),
                now: Instant::now(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Glare(action) => self.state.apply_action(&action, self.now),
            Message::Tick(now) => self.now = now,
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn pricing_card(&self) -> Element<'_, Message> {
        container(
            column![
                column![
                    text("Pro").size(22),
                    text("Popular").size(12),
                ]
                .spacing(4),
                text("For teams that need more.").size(14),
                column![
                    text("$49 /mo").size(32),
                    text("Unlimited projects").size(14),
                    text("Team collaboration").size(14),
                    text("Advanced analytics").size(14),
                ]
                .spacing(6),
                button(text("Get started")).width(Length::Fill),
            ]
            .spacing(14)
            .padding(24),
        )
        .width(Length::Fixed(340.0))
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
        let card = glare_hover(
            self.pricing_card(),
            &self.state,
            self.now,
            GlareHoverProps {
                duration_ms: 600,
                ..GlareHoverProps::new()
            },
            Message::Glare,
            &self.theme,
        );

        _shared::shell("Glare Hover", column![card].spacing(24))
    }
}
