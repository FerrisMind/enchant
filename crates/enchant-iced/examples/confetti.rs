mod _shared;

use enchant_iced::{
    AnchorMeasurements, AnchorRemeasure, Confetti, ConfettiButton, ConfettiState, anchor,
    measure_anchors,
};
use iced::time::{Duration, Instant};
use iced::widget::{Id, button, column, container, stack, text};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Point, Size, Subscription, Task,
};

const STAGE_ID: &str = "confetti-stage";
const FIRE_BUTTON_ID: &str = "confetti-fire-button";
const BUTTON_STAGE_ID: &str = "confetti-button-stage";
const CONFETTI_BUTTON_ID: &str = "confetti-button";

pub fn main() -> iced::Result {
    iced::application(
        ConfettiExample::new,
        ConfettiExample::update,
        ConfettiExample::view,
    )
    .title("Confetti")
    .window(iced::window::Settings {
        size: Size::new(900.0, 920.0),
        ..Default::default()
    })
    .subscription(ConfettiExample::subscription)
    .run()
}

struct ConfettiExample {
    theme_shell: _shared::ThemeShell,
    preview_state: ConfettiState,
    preview_confetti: Confetti,
    button_state: ConfettiState,
    button_confetti: Confetti,
    now: Instant,
    preview_origin: Option<Point>,
    button_origin: Option<Point>,
    auto_fired_preview: bool,
    pending_fire_preview: bool,
    pending_fire_button: bool,
    remeasure: AnchorRemeasure,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    ThemeAnchors(enchant_iced::AnchorMeasurements),
    LayoutMeasured(AnchorMeasurements),
    FirePreview,
    FireButton,
    WindowResized,
    Tick(Instant),
}

impl ConfettiExample {
    fn new() -> (Self, Task<Message>) {
        let now = Instant::now();
        let preview_confetti = Confetti::new();
        let button_confetti = ConfettiButton::<()>::new(text("")).confetti();

        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                preview_state: ConfettiState::new(),
                preview_confetti,
                button_state: ConfettiState::new(),
                button_confetti,
                now,
                preview_origin: None,
                button_origin: None,
                auto_fired_preview: false,
                pending_fire_preview: false,
                pending_fire_button: false,
                remeasure: AnchorRemeasure::new(),
            },
            measure_demo_layout(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleTheme => {
                return _shared::ThemeShell::measure_toggle_anchor().map(Message::ThemeAnchors);
            }
            Message::ThemeAnchors(measurements) => {
                self.theme_shell.apply_anchor_and_toggle(&measurements);
            }
            Message::LayoutMeasured(measurements) => {
                self.apply_layout(&measurements);
                self.flush_pending_fires();

                if !self.auto_fired_preview {
                    if let Some(origin) = self.preview_origin {
                        self.preview_state
                            .fire_at(self.now, &self.preview_confetti, origin);
                        self.auto_fired_preview = true;
                    } else {
                        self.remeasure.schedule();
                    }
                }

                if (self.pending_fire_preview && self.preview_origin.is_none())
                    || (self.pending_fire_button && self.button_origin.is_none())
                {
                    self.remeasure.schedule();
                }
            }
            Message::FirePreview => {
                if let Some(origin) = self.preview_origin {
                    self.fire_preview(origin);
                } else {
                    self.pending_fire_preview = true;
                    self.remeasure.schedule();
                    return measure_demo_layout();
                }
            }
            Message::FireButton => {
                if let Some(origin) = self.button_origin {
                    self.fire_button(origin);
                } else {
                    self.pending_fire_button = true;
                    self.remeasure.schedule();
                    return measure_demo_layout();
                }
            }
            Message::WindowResized => {
                // Layout may still be settling; remeasure on the next tick.
                self.remeasure.schedule();
            }
            Message::Tick(now) => {
                self.theme_shell.tick(now);
                self.now = now;
                self.preview_state.tick(now, &self.preview_confetti);
                self.button_state.tick(now, &self.button_confetti);
                if self.remeasure.take() {
                    return measure_demo_layout();
                }
            }
        }
        Task::none()
    }

    fn fire_preview(&mut self, origin: Point) {
        self.preview_state.clear();
        self.preview_state
            .fire_at(self.now, &self.preview_confetti, origin);
        self.pending_fire_preview = false;
    }

    fn fire_button(&mut self, origin: Point) {
        self.button_state.clear();
        self.button_state
            .fire_at(self.now, &self.button_confetti, origin);
        self.pending_fire_button = false;
    }

    fn flush_pending_fires(&mut self) {
        if self.pending_fire_preview
            && let Some(origin) = self.preview_origin
        {
            self.fire_preview(origin);
            // User click already covered the cold-start auto-fire.
            self.auto_fired_preview = true;
        }
        if self.pending_fire_button
            && let Some(origin) = self.button_origin
        {
            self.fire_button(origin);
        }
    }

    fn apply_layout(&mut self, measurements: &AnchorMeasurements) {
        // Indices match measure_demo_layout() order (API is index-only; no named lookup).
        if let (Some(stage), Some(button)) = (measurements.get(0), measurements.get(1)) {
            self.preview_origin = Some(content_top_center(stage.bounds, button.bounds));
        }
        if let (Some(stage), Some(button)) = (measurements.get(2), measurements.get(3)) {
            self.button_origin = Some(content_top_center(stage.bounds, button.bounds));
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::time::every(Duration::from_millis(16)).map(Message::Tick),
            iced::window::resize_events().map(|(_id, _size)| Message::WindowResized),
        ])
    }

    fn view(&self) -> Element<'_, Message> {
        let fire_button = button(text("Fire Confetti").size(14).color(Color::WHITE))
            .padding([10.0, 16.0])
            .style(fire_button_style)
            .on_press(Message::FirePreview);

        let preview_overlay = self
            .preview_confetti
            .width(Length::Fill)
            .height(Length::Fill)
            .view(&self.preview_state, self.now);

        let preview_stage = bordered_stage(
            STAGE_ID,
            FIRE_BUTTON_ID,
            fire_button.into(),
            preview_overlay,
        );

        let confetti_button = ConfettiButton::new(text("Click Me!").size(14).color(Color::WHITE))
            .on_press(Message::FireButton)
            .view();

        let button_overlay = self
            .button_confetti
            .width(Length::Fill)
            .height(Length::Fill)
            .view(&self.button_state, self.now);

        let confetti_button_stage = bordered_stage(
            BUTTON_STAGE_ID,
            CONFETTI_BUTTON_ID,
            confetti_button,
            button_overlay,
        );

        _shared::themed_shell(
            "Confetti",
            column![
                section_label("Confetti"),
                preview_stage,
                section_label("ConfettiButton"),
                confetti_button_stage,
            ]
            .spacing(16)
            .align_x(Alignment::Center),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}

fn measure_demo_layout() -> Task<Message> {
    measure_anchors([
        Id::new(STAGE_ID),
        Id::new(FIRE_BUTTON_ID),
        Id::new(BUTTON_STAGE_ID),
        Id::new(CONFETTI_BUTTON_ID),
    ])
    .map(Message::LayoutMeasured)
}

fn content_top_center(stage: iced::Rectangle, button: iced::Rectangle) -> Point {
    Point::new(
        button.x - stage.x + button.width / 2.0,
        button.y - stage.y,
    )
}

fn section_label(label: &str) -> Element<'_, Message> {
    text(label)
        .size(18)
        .color(Color::from_rgb8(24, 24, 27))
        .into()
}

fn bordered_stage<'a>(
    stage_id: &'static str,
    button_id: &'static str,
    button: Element<'a, Message>,
    overlay: Element<'a, Message>,
) -> Element<'a, Message> {
    let stage = container(
        stack![
            container(overlay)
                .width(Length::Fill)
                .height(Length::Fill),
            container(anchor(button_id, button))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fixed(500.0))
    .style(|_| container::Style {
        border: Border {
            width: 1.0,
            radius: 12.0.into(),
            color: Color::from_rgb8(228, 228, 231),
        },
        ..container::Style::default()
    });

    anchor(stage_id, stage).into()
}

fn fire_button_style(
    _theme: &iced::Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let background = match status {
        iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed => {
            Color::from_rgb8(38, 38, 38)
        }
        iced::widget::button::Status::Disabled => Color::from_rgb8(23, 23, 23).scale_alpha(0.5),
        iced::widget::button::Status::Active => Color::from_rgb8(23, 23, 23),
    };

    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        text_color: Color::WHITE,
        border: Border {
            radius: 8.0.into(),
            ..Border::default()
        },
        ..iced::widget::button::Style::default()
    }
}
