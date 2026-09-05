use enchant_iced::{
    AnimatedThemeTogglerController, AnimatedThemeTogglerProps, anchor,
    animated_theme_toggler_with_controller, measure_anchors, theme_transition_overlay,
};
use iced::time::{Duration, Instant};
use iced::widget::Id;
use iced::widget::{container, stack};
use iced::{Background, Element, Length, Size, Subscription, Task};

const TOGGLER_ANCHOR: &str = "theme-toggler";

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
    props: AnimatedThemeTogglerProps,
    controller: AnimatedThemeTogglerController,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Tick(Instant),
}

impl AnimatedThemeTogglerExample {
    fn new() -> (Self, Task<Message>) {
        let props = AnimatedThemeTogglerProps::new();
        let controller = AnimatedThemeTogglerController::new(false, &props);
        (
            Self {
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
                return measure_anchors([Id::new(TOGGLER_ANCHOR)]).map(Message::AnchorsMeasured);
            }
            Message::AnchorsMeasured(measurements) => {
                if let Some(bounds) = measurements.get(0) {
                    self.controller.set_origin(bounds.center);
                }
                self.controller.toggle(self.now, &self.props);
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
        let display_theme = self.controller.display_theme(self.now);

        let toggler = anchor(
            TOGGLER_ANCHOR,
            animated_theme_toggler_with_controller(
                &self.controller,
                self.now,
                Message::ToggleTheme,
                self.props,
                &display_theme,
            ),
        );

        // Background uses the old theme during the reveal; toggler stays above the overlay.
        let background = container(iced::widget::Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(display_theme.palette.background)),
                ..iced::widget::container::Style::default()
            });

        let toggler_layer = container(container(toggler).padding(24))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill);

        let mut layers = vec![background.into()];
        if let Some(overlay) = theme_transition_overlay(&self.controller, self.now, &self.props) {
            layers.push(overlay);
        }
        layers.push(toggler_layer.into());

        container(stack(layers).width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
