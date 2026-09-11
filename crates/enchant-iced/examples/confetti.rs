mod _shared;

use enchant_iced::{
    AnchorMeasurements, AnchorRemeasure, Confetti, ConfettiButton, ConfettiOptions, ConfettiShape,
    ConfettiState, anchor, measure_anchors,
};
use iced::time::{Duration, Instant};
use iced::widget::{Id, button, column, container, mouse_area, stack, text};
use iced::{Background, Border, Color, Element, Length, Point, Size, Subscription, Task};

const STAGE_ID: &str = "confetti-stage";
const BUTTON_ID: &str = "confetti-fire-button";
const STAGE_HEIGHT: f32 = 500.0;

const TAB_LABELS: &[&str] = &[
    "Demo",
    "Basic",
    "Random",
    "Fireworks",
    "Side Cannons",
    "Stars",
    "Shapes",
    "Emoji",
];

const SIDE_CANNON_COLORS: [Color; 4] = [
    Color::from_rgb8(0xA7, 0x86, 0xFF),
    Color::from_rgb8(0xFD, 0x8B, 0xBC),
    Color::from_rgb8(0xEC, 0xA1, 0x84),
    Color::from_rgb8(0xF8, 0xDE, 0xB1),
];

const STAR_COLORS: [Color; 5] = [
    Color::from_rgb8(0xFF, 0xE4, 0x00),
    Color::from_rgb8(0xFF, 0xBD, 0x00),
    Color::from_rgb8(0xE8, 0x94, 0x00),
    Color::from_rgb8(0xFF, 0xCA, 0x6C),
    Color::from_rgb8(0xFD, 0xFF, 0xB8),
];

const PATH_TRIANGLE: &str = "M0 10 L5 0 L10 10z";
const PATH_SQUARE: &str = "M0 0 L10 0 L10 10 L0 10 Z";
const PATH_COIN: &str = "M5 0 A5 5 0 1 0 5 10 A5 5 0 1 0 5 0 Z";
const PATH_TREE: &str = "M5 0 L10 10 L0 10 Z";

pub fn main() -> iced::Result {
    iced::application(
        ConfettiExample::new,
        ConfettiExample::update,
        ConfettiExample::view,
    )
    .title("Confetti")
    .window(iced::window::Settings {
        size: Size::new(1000.0, 720.0),
        ..Default::default()
    })
    .subscription(ConfettiExample::subscription)
    .run()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DemoTab {
    Demo,
    Basic,
    Random,
    Fireworks,
    SideCannons,
    Stars,
    Shapes,
    Emoji,
}

impl DemoTab {
    fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Demo,
            1 => Self::Basic,
            2 => Self::Random,
            3 => Self::Fireworks,
            4 => Self::SideCannons,
            5 => Self::Stars,
            6 => Self::Shapes,
            _ => Self::Emoji,
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Demo => 0,
            Self::Basic => 1,
            Self::Random => 2,
            Self::Fireworks => 3,
            Self::SideCannons => 4,
            Self::Stars => 5,
            Self::Shapes => 6,
            Self::Emoji => 7,
        }
    }

    fn needs_button_origin(self) -> bool {
        matches!(self, Self::Basic | Self::Random)
    }
}

enum ActiveSequence {
    Fireworks {
        end_at: Instant,
        next_at: Instant,
        duration: Duration,
    },
    SideCannons {
        end_at: Instant,
    },
    Staggered {
        started_at: Instant,
        shots: Vec<(Duration, ConfettiOptions)>,
        next_index: usize,
    },
}

struct ConfettiExample {
    theme_shell: _shared::ThemeShell,
    active_tab: DemoTab,
    state: ConfettiState,
    now: Instant,
    stage_size: Option<Size>,
    button_origin: Option<Point>,
    pending_button_fire: Option<ConfettiOptions>,
    pending_stage_fire: Option<ConfettiOptions>,
    sequence: Option<ActiveSequence>,
    remeasure: AnchorRemeasure,
    rng: u64,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    ThemeAnchors(enchant_iced::AnchorMeasurements),
    LayoutMeasured(AnchorMeasurements),
    SelectTab(usize),
    DemoEnter,
    FireBasic,
    FireRandom,
    TriggerFireworks,
    TriggerSideCannons,
    TriggerStars,
    TriggerShapes,
    TriggerEmoji,
    WindowResized,
    Tick(Instant),
}

impl ConfettiExample {
    fn new() -> (Self, Task<Message>) {
        let now = Instant::now();
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                active_tab: DemoTab::Demo,
                state: ConfettiState::new(),
                now,
                stage_size: None,
                button_origin: None,
                pending_button_fire: None,
                pending_stage_fire: None,
                sequence: None,
                remeasure: AnchorRemeasure::new(),
                rng: 0xC0FFEE_u64.wrapping_mul(now.elapsed().as_nanos() as u64 | 1),
            },
            measure_demo_layout(false),
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
                if let Some(opts) = self.pending_button_fire.take() {
                    if let Some(origin) = self.button_origin {
                        self.state.fire_at(self.now, &opts, origin);
                    } else {
                        self.pending_button_fire = Some(opts);
                        self.remeasure.schedule();
                    }
                }
                if let Some(opts) = self.pending_stage_fire.take() {
                    if let Some(size) = self.stage_size {
                        self.state.fire(self.now, &opts, size);
                    } else {
                        self.pending_stage_fire = Some(opts);
                        self.remeasure.schedule();
                    }
                }
                // Catch up timed sequences that started before stage bounds were known.
                self.advance_sequence();
            }
            Message::SelectTab(index) => {
                self.active_tab = DemoTab::from_index(index);
                self.state.clear();
                self.sequence = None;
                self.pending_button_fire = None;
                self.pending_stage_fire = None;
                self.button_origin = None;
                self.remeasure.schedule();
                return measure_demo_layout(self.active_tab.needs_button_origin());
            }
            Message::DemoEnter => {
                if let Some(size) = self.stage_size {
                    self.state.fire(self.now, &ConfettiOptions::new(), size);
                } else {
                    self.pending_stage_fire = Some(ConfettiOptions::new());
                    self.remeasure.schedule();
                    return measure_demo_layout(false);
                }
            }
            Message::FireBasic => {
                return self.fire_from_button(ConfettiOptions::new());
            }
            Message::FireRandom => {
                let angle = self.next_f32() * 360.0;
                return self.fire_from_button(ConfettiOptions::new().with_angle(angle));
            }
            Message::TriggerFireworks => {
                let duration = Duration::from_secs(5);
                self.sequence = Some(ActiveSequence::Fireworks {
                    end_at: self.now + duration,
                    next_at: self.now,
                    duration,
                });
            }
            Message::TriggerSideCannons => {
                self.sequence = Some(ActiveSequence::SideCannons {
                    end_at: self.now + Duration::from_secs(3),
                });
            }
            Message::TriggerStars => {
                self.start_staggered(star_shots());
            }
            Message::TriggerShapes => {
                self.start_staggered(shape_shots());
            }
            Message::TriggerEmoji => {
                self.start_staggered(emoji_shots());
            }
            Message::WindowResized => {
                self.remeasure.schedule();
            }
            Message::Tick(now) => {
                self.theme_shell.tick(now);
                self.now = now;
                self.state.tick(now);
                self.advance_sequence();
                if self.remeasure.take() {
                    return measure_demo_layout(self.active_tab.needs_button_origin());
                }
            }
        }
        Task::none()
    }

    fn fire_from_button(&mut self, opts: ConfettiOptions) -> Task<Message> {
        if let Some(origin) = self.button_origin {
            self.state.fire_at(self.now, &opts, origin);
            Task::none()
        } else {
            self.pending_button_fire = Some(opts);
            self.remeasure.schedule();
            measure_demo_layout(true)
        }
    }

    fn start_staggered(&mut self, shots: Vec<(Duration, ConfettiOptions)>) {
        self.sequence = Some(ActiveSequence::Staggered {
            started_at: self.now,
            shots,
            next_index: 0,
        });
    }

    fn advance_sequence(&mut self) {
        let Some(size) = self.stage_size else {
            return;
        };
        let now = self.now;

        match self.sequence.take() {
            Some(ActiveSequence::Fireworks {
                end_at,
                mut next_at,
                duration,
            }) => {
                if now >= end_at {
                    return;
                }
                while now >= next_at {
                    let time_left = end_at.saturating_duration_since(now);
                    let particle_count =
                        ((50.0 * (time_left.as_secs_f32() / duration.as_secs_f32())) as usize)
                            .max(1);
                    let defaults = ConfettiOptions::new()
                        .with_start_velocity(30.0)
                        .with_spread(360.0)
                        .with_ticks(60)
                        .with_particle_count(particle_count);

                    let left = defaults
                        .with_origin_x(lerp(0.1, 0.3, self.next_f32()))
                        .with_origin_y(self.next_f32() - 0.2);
                    let right = defaults
                        .with_origin_x(lerp(0.7, 0.9, self.next_f32()))
                        .with_origin_y(self.next_f32() - 0.2);
                    self.state.fire(now, &left, size);
                    self.state.fire(now, &right, size);
                    next_at += Duration::from_millis(250);
                }
                self.sequence = Some(ActiveSequence::Fireworks {
                    end_at,
                    next_at,
                    duration,
                });
            }
            Some(ActiveSequence::SideCannons { end_at }) => {
                if now >= end_at {
                    return;
                }
                let left = ConfettiOptions::new()
                    .with_particle_count(2)
                    .with_angle(60.0)
                    .with_spread(55.0)
                    .with_start_velocity(60.0)
                    .with_origin_x(0.0)
                    .with_origin_y(0.5)
                    .with_colors(&SIDE_CANNON_COLORS);
                let right = ConfettiOptions::new()
                    .with_particle_count(2)
                    .with_angle(120.0)
                    .with_spread(55.0)
                    .with_start_velocity(60.0)
                    .with_origin_x(1.0)
                    .with_origin_y(0.5)
                    .with_colors(&SIDE_CANNON_COLORS);
                self.state.fire(now, &left, size);
                self.state.fire(now, &right, size);
                self.sequence = Some(ActiveSequence::SideCannons { end_at });
            }
            Some(ActiveSequence::Staggered {
                started_at,
                shots,
                mut next_index,
            }) => {
                while next_index < shots.len() {
                    let (delay, opts) = shots[next_index];
                    if now.saturating_duration_since(started_at) < delay {
                        break;
                    }
                    self.state.fire(now, &opts, size);
                    next_index += 1;
                }
                if next_index < shots.len() {
                    self.sequence = Some(ActiveSequence::Staggered {
                        started_at,
                        shots,
                        next_index,
                    });
                }
            }
            None => {}
        }
    }

    fn apply_layout(&mut self, measurements: &AnchorMeasurements) {
        if let Some(stage) = measurements.get(0) {
            self.stage_size = Some(Size::new(stage.bounds.width, stage.bounds.height));
        }
        if self.active_tab.needs_button_origin()
            && let (Some(stage), Some(button)) = (measurements.get(0), measurements.get(1))
        {
            self.button_origin = Some(content_center(stage.bounds, button.bounds));
        } else {
            self.button_origin = None;
        }
    }

    fn next_f32(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng as f64 / u64::MAX as f64) as f32
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::time::every(Duration::from_millis(16)).map(Message::Tick),
            iced::window::resize_events().map(|(_id, _size)| Message::WindowResized),
        ])
    }

    fn view(&self) -> Element<'_, Message> {
        let tabs = _shared::tabs(TAB_LABELS, self.active_tab.index(), Message::SelectTab);
        let body = match self.active_tab {
            DemoTab::Demo => self.demo_tab(),
            DemoTab::Basic => self.button_tab("Confetti 🎉", Message::FireBasic),
            DemoTab::Random => self.button_tab("Random Confetti 🎉", Message::FireRandom),
            DemoTab::Fireworks => self.trigger_tab("Trigger Fireworks", Message::TriggerFireworks),
            DemoTab::SideCannons => {
                self.trigger_tab("Trigger Side Cannons", Message::TriggerSideCannons)
            }
            DemoTab::Stars => self.trigger_tab("Trigger Stars", Message::TriggerStars),
            DemoTab::Shapes => self.trigger_tab("Trigger Shapes", Message::TriggerShapes),
            DemoTab::Emoji => self.trigger_tab("Trigger Emoji", Message::TriggerEmoji),
        };

        _shared::themed_shell(
            "Confetti",
            column![tabs, _shared::stage(body)].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }

    fn overlay(&self) -> Element<'_, Message> {
        Confetti::new()
            .width(Length::Fill)
            .height(Length::Fill)
            .view(&self.state, self.now)
    }

    fn demo_tab(&self) -> Element<'_, Message> {
        let title = text("Confetti")
            .size(96)
            .color(self.theme_shell.blended_foreground(self.theme_shell.now));

        let stage = container(
            mouse_area(
                stack![
                    container(self.overlay())
                        .width(Length::Fill)
                        .height(Length::Fill),
                    container(title)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center_x(Length::Fill)
                        .center_y(Length::Fill),
                ]
                .width(Length::Fill)
                .height(Length::Fill),
            )
            .on_enter(Message::DemoEnter),
        )
        .width(Length::Fill)
        .height(Length::Fixed(STAGE_HEIGHT))
        .style(stage_style);

        anchor(STAGE_ID, stage).into()
    }

    fn button_tab(&self, label: &'static str, on_press: Message) -> Element<'_, Message> {
        let confetti_button = ConfettiButton::new(text(label).size(14).color(Color::WHITE))
            .on_press(on_press)
            .view();

        bordered_stage(
            STAGE_ID,
            Some((BUTTON_ID, confetti_button)),
            self.overlay(),
        )
    }

    fn trigger_tab(&self, label: &'static str, on_press: Message) -> Element<'_, Message> {
        let trigger = button(text(label).size(14).color(Color::WHITE))
            .padding([12.0, 24.0])
            .style(trigger_button_style)
            .on_press(on_press);

        bordered_stage(STAGE_ID, Some((BUTTON_ID, trigger.into())), self.overlay())
    }
}

fn star_shots() -> Vec<(Duration, ConfettiOptions)> {
    let defaults = ConfettiOptions::new()
        .with_spread(360.0)
        .with_ticks(50)
        .with_gravity(0.0)
        .with_decay(0.94)
        .with_start_velocity(30.0)
        .with_colors(&STAR_COLORS);

    let mut shots = Vec::new();
    for delay_ms in [0_u64, 100, 200] {
        let delay = Duration::from_millis(delay_ms);
        shots.push((
            delay,
            defaults
                .with_particle_count(40)
                .with_scalar(1.2)
                .with_shapes(&[ConfettiShape::Star]),
        ));
        shots.push((
            delay,
            defaults
                .with_particle_count(10)
                .with_scalar(0.75)
                .with_shapes(&[ConfettiShape::Circle]),
        ));
    }
    shots
}

fn shape_shots() -> Vec<(Duration, ConfettiOptions)> {
    let scalar = 2.0;
    let path_shapes = [
        ConfettiShape::Path { data: PATH_TRIANGLE },
        ConfettiShape::Path { data: PATH_SQUARE },
        ConfettiShape::Path { data: PATH_COIN },
        ConfettiShape::Path { data: PATH_TREE },
    ];
    let defaults = ConfettiOptions::new()
        .with_spread(360.0)
        .with_ticks(60)
        .with_gravity(0.0)
        .with_decay(0.96)
        .with_start_velocity(20.0)
        .with_shapes(&path_shapes)
        .with_scalar(scalar);

    let mut shots = Vec::new();
    for delay_ms in [0_u64, 100, 200] {
        let delay = Duration::from_millis(delay_ms);
        shots.push((delay, defaults.with_particle_count(30)));
        shots.push((delay, defaults.with_particle_count(5)));
        shots.push((
            delay,
            defaults
                .with_particle_count(15)
                .with_scalar(scalar / 2.0)
                .with_shapes(&[ConfettiShape::Circle]),
        ));
    }
    shots
}

fn emoji_shots() -> Vec<(Duration, ConfettiOptions)> {
    let scalar = 2.0;
    let defaults = ConfettiOptions::new()
        .with_spread(360.0)
        .with_ticks(60)
        .with_gravity(0.0)
        .with_decay(0.96)
        .with_start_velocity(20.0)
        .with_shapes(&[ConfettiShape::Text { text: "🦄" }])
        .with_scalar(scalar);

    let mut shots = Vec::new();
    for delay_ms in [0_u64, 100, 200] {
        let delay = Duration::from_millis(delay_ms);
        shots.push((delay, defaults.with_particle_count(30)));
        shots.push((delay, defaults.with_particle_count(5)));
        shots.push((
            delay,
            defaults
                .with_particle_count(15)
                .with_scalar(scalar / 2.0)
                .with_shapes(&[ConfettiShape::Circle]),
        ));
    }
    shots
}

fn measure_demo_layout(include_button: bool) -> Task<Message> {
    if include_button {
        measure_anchors([Id::new(STAGE_ID), Id::new(BUTTON_ID)]).map(Message::LayoutMeasured)
    } else {
        measure_anchors([Id::new(STAGE_ID)]).map(Message::LayoutMeasured)
    }
}

fn content_center(stage: iced::Rectangle, button: iced::Rectangle) -> Point {
    Point::new(
        button.x - stage.x + button.width / 2.0,
        button.y - stage.y + button.height / 2.0,
    )
}

fn lerp(min: f32, max: f32, t: f32) -> f32 {
    min + (max - min) * t.clamp(0.0, 1.0)
}

fn bordered_stage<'a>(
    stage_id: &'static str,
    button: Option<(&'static str, Element<'a, Message>)>,
    overlay: Element<'a, Message>,
) -> Element<'a, Message> {
    let content: Element<'a, Message> = if let Some((button_id, button)) = button {
        // Magic UI ConfettiButton / body canvas: z-index 9999, pointer-events none.
        // Draw button first, confetti overlay on top. EffectCanvas reports
        // Interaction::None so clicks pass through to the button underneath.
        stack![
            container(anchor(button_id, button))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
            container(overlay)
                .width(Length::Fill)
                .height(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    } else {
        overlay
    };

    let stage = container(content)
        .width(Length::Fill)
        .height(Length::Fixed(STAGE_HEIGHT))
        .style(stage_style);

    anchor(stage_id, stage).into()
}

fn stage_style(_: &iced::Theme) -> container::Style {
    container::Style {
        border: Border {
            width: 1.0,
            radius: 12.0.into(),
            color: Color::from_rgb8(228, 228, 231),
        },
        ..container::Style::default()
    }
}

fn trigger_button_style(
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
