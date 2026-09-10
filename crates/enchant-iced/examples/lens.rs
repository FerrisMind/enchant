mod _shared;

use enchant_iced::{Lens, LensAction, LensState};
use iced::time::Instant;
use iced::widget::{column, image, row, text};
use iced::{Alignment, Element, Length, Point, Size, Task};
use iced_shadcn::{
    ButtonProps, ButtonSize, ButtonVariant, CardProps, CardSize, button_content, card,
};

/// Tailwind `max-w-md` = 28rem ≈ 448px.
const CARD_WIDTH: f32 = 448.0;
const IMAGE_HEIGHT: f32 = 280.0;
/// Magic UI docs demo lens point (Static / Default Position).
const DEMO_LENS_POINT: Point = Point::new(260.0, 150.0);

fn camp_image() -> image::Handle {
    use std::sync::OnceLock;

    static HANDLE: OnceLock<image::Handle> = OnceLock::new();
    HANDLE
        .get_or_init(|| {
            let rgba = ::image::load_from_memory(include_bytes!("assets/lens-camp.jpg"))
                .expect("decode assets/lens-camp.jpg")
                .into_rgba8();
            let (width, height) = rgba.dimensions();
            image::Handle::from_rgba(width, height, rgba.into_raw())
        })
        .clone()
}

/// Docs-parity base config (`zoom_factor=2`, `lens_size=150`) wrapping `content`.
fn camp_lens<'a>(content: impl Into<Element<'a, Message>>) -> Lens<'a, Message> {
    Lens::new(content)
        .zoom_factor(2.0)
        .lens_size(150.0)
        .width(Length::Fill)
        .height(Length::Fixed(IMAGE_HEIGHT))
}

fn static_lens<'a>(content: impl Into<Element<'a, Message>>) -> Lens<'a, Message> {
    camp_lens(content)
        .is_static(true)
        .position(DEMO_LENS_POINT)
}

fn default_position_lens<'a>(content: impl Into<Element<'a, Message>>) -> Lens<'a, Message> {
    camp_lens(content).default_position(DEMO_LENS_POINT)
}

pub fn main() -> iced::Result {
    iced::application(LensExample::new, LensExample::update, LensExample::view)
        .title("Lens")
        .window(iced::window::Settings {
            size: Size::new(980.0, 720.0),
            ..Default::default()
        })
        .subscription(LensExample::subscription)
        .run()
}

struct LensExample {
    theme_shell: _shared::ThemeShell,
    static_state: LensState,
    default_position_state: LensState,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    Tick(Instant),
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    LensStatic(LensAction),
    LensDefaultPosition(LensAction),
    LetsGo,
    AnotherTime,
}

impl LensExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                static_state: LensState::new(),
                default_position_state: LensState::new(),
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
                // Config-only lenses for animation timing (content unused by `tick`).
                self.static_state.tick(now, &static_lens(text("")));
                self.default_position_state
                    .tick(now, &default_position_lens(text("")));
            }
            Message::LensStatic(action) => {
                self.static_state.apply_action(&action);
                self.static_state
                    .tick(self.theme_shell.now, &static_lens(text("")));
            }
            Message::LensDefaultPosition(action) => {
                self.default_position_state.apply_action(&action);
                self.default_position_state
                    .tick(self.theme_shell.now, &default_position_lens(text("")));
            }
            Message::LetsGo | Message::AnotherTime => {}
        }
        Task::none()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(16)).map(Message::Tick)
    }

    fn camp_card<'a>(
        &'a self,
        title: &'a str,
        magnifier: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let theme = self.theme_shell.theme();
        let foreground = theme.palette.foreground;
        let muted = theme.palette.muted_foreground;

        let footer = row![
            button_content(
                text("Let's go"),
                Some(Message::LetsGo),
                ButtonProps::new()
                    .variant(ButtonVariant::Default)
                    .size(ButtonSize::Size2),
                theme,
            ),
            button_content(
                text("Another time"),
                Some(Message::AnotherTime),
                ButtonProps::new()
                    .variant(ButtonVariant::Secondary)
                    .size(ButtonSize::Size2),
                theme,
            ),
        ]
        .spacing(16)
        .align_y(Alignment::Center);

        let card_body = column![
            magnifier,
            text("Your next camp").size(24).color(foreground),
            text(
                "See our latest and best camp destinations all across the five continents of the globe."
            )
            .size(14)
            .color(muted),
            footer,
        ]
        .spacing(16)
        .padding(24)
        .width(Length::Fill);

        let preview = card(
            card_body,
            CardProps::new()
                .size(CardSize::Size2)
                .show_shadow(false)
                .padding(0.0)
                .radius(theme.radius.lg),
            theme,
        )
        .width(Length::Fixed(CARD_WIDTH));

        column![text(title).size(16).color(muted), preview]
            .spacing(8)
            .width(Length::Fixed(CARD_WIDTH))
            .into()
    }

    fn view(&self) -> Element<'_, Message> {
        let photo = || {
            image(camp_image())
                .width(Length::Fill)
                .height(Length::Fixed(IMAGE_HEIGHT))
                .content_fit(iced::ContentFit::Cover)
                .border_radius(12.0)
        };

        let static_card = self.camp_card(
            "Static Lens",
            static_lens(photo()).into_element(&self.static_state, Message::LensStatic),
        );
        let default_position_card = self.camp_card(
            "Default Position",
            default_position_lens(photo())
                .into_element(&self.default_position_state, Message::LensDefaultPosition),
        );

        _shared::themed_shell(
            "Lens",
            row![static_card, default_position_card]
                .spacing(24)
                .align_y(Alignment::Start),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
