mod _shared;

use iced::time::Instant;
use iced::widget::{column, container, image, row, svg, text};
use iced::{Alignment, ContentFit, Element, Length, Size, Task};
use magic_iced::theme::MagicTheme;
use magic_iced::{BacklightProps, backlight};
use magic_iced::primitives::glow_from_svg_bytes;

const IMAGE_WIDTH: f32 = 480.0;
const IMAGE_HEIGHT: f32 = 320.0;
const SVG_ICON: f32 = 64.0;
const SVG_BLUR: f32 = 5.0;

fn hero_image() -> image::Handle {
    use std::sync::OnceLock;

    static HERO: OnceLock<image::Handle> = OnceLock::new();
    HERO.get_or_init(|| {
        image::Handle::from_bytes(include_bytes!("assets/backlight/hero.jpg").as_slice())
    })
    .clone()
}

fn prewarm_image_glow(source: &image::Handle, props: &BacklightProps, width: f32, height: f32) {
    let _ = magic_iced::primitives::glow_from_handle(
        source,
        props.blur,
        props.saturation,
        props.radius,
        width,
        height,
    );
}

fn prewarm_svg_glows() {
    for dark in [false, true] {
        let theme = theme_for_mode(dark);
        for icon in svg_icon_specs() {
            let tint = icon.mono.then(|| theme.palette.foreground);
            let _ = glow_from_svg_bytes(
                icon.bytes,
                SVG_BLUR,
                BacklightProps::new().saturation,
                0.0,
                SVG_ICON,
                SVG_ICON,
                tint,
            );
        }
    }
}

fn theme_for_mode(dark: bool) -> iced_shadcn::Theme {
    if dark {
        MagicTheme::dark()
    } else {
        MagicTheme::light()
    }
}

pub fn main() -> iced::Result {
    iced::application(
        BacklightExample::new,
        BacklightExample::update,
        BacklightExample::view,
    )
    .title("Backlight")
    .window(iced::window::Settings {
        size: Size::new(900.0, 720.0),
        ..Default::default()
    })
    .subscription(BacklightExample::subscription)
    .run()
}

struct BacklightExample {
    theme_shell: _shared::ThemeShell,
    image_props: BacklightProps,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    Tick(Instant),
    AnchorsMeasured(magic_iced::AnchorMeasurements),
}

impl BacklightExample {
    fn new() -> (Self, Task<Message>) {
        let image_props = BacklightProps::new().glow_source(hero_image()).radius(16.0);
        prewarm_image_glow(
            image_props.glow_source.as_ref().expect("hero"),
            &image_props,
            IMAGE_WIDTH,
            IMAGE_HEIGHT,
        );
        prewarm_svg_glows();

        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                image_props,
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
            Message::Tick(now) => {
                self.theme_shell.tick(now);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(16)).map(Message::Tick)
    }

    fn image_demo(&self) -> Element<'_, Message> {
        let image_card = container(
            image(
                self.image_props
                    .glow_source
                    .as_ref()
                    .expect("hero")
                    .clone(),
            )
            .width(Length::Fixed(IMAGE_WIDTH))
            .height(Length::Fixed(IMAGE_HEIGHT))
            .content_fit(ContentFit::Cover)
            .border_radius(16.0),
        )
        .width(Length::Fixed(IMAGE_WIDTH))
        .height(Length::Fixed(IMAGE_HEIGHT));

        backlight(
            image_card,
            self.image_props.clone(),
            self.theme_shell.theme(),
        )
    }

    fn svg_demo(&self) -> Element<'_, Message> {
        let icons = svg_icon_specs()
            .into_iter()
            .map(|icon| self.svg_icon(icon))
            .collect::<Vec<_>>();

        row(icons)
            .spacing(64)
            .align_y(Alignment::Center)
            .into()
    }

    fn svg_icon(&self, icon: SvgIconSpec) -> Element<'_, Message> {
        let foreground = self.theme_shell.theme().palette.foreground;
        let mut graphic = svg(icon.handle)
            .width(Length::Fixed(SVG_ICON))
            .height(Length::Fixed(SVG_ICON))
            .content_fit(ContentFit::Contain);

        if icon.mono {
            graphic = graphic.style(move |_, _| svg::Style {
                color: Some(foreground),
                ..svg::Style::default()
            });
        }

        let props = BacklightProps::new()
            .blur(SVG_BLUR)
            .glow_svg(icon.bytes, icon.mono)
            .radius(0.0);

        backlight(graphic, props, self.theme_shell.theme())
    }

    fn view(&self) -> Element<'_, Message> {
        let body = column![
            text("Image").size(16),
            self.image_demo(),
            text("SVG").size(16),
            self.svg_demo(),
        ]
        .spacing(20)
        .align_x(Alignment::Center);

        _shared::themed_shell("Backlight", body, &self.theme_shell, Message::ToggleTheme)
    }
}

#[derive(Clone)]
struct SvgIconSpec {
    bytes: &'static [u8],
    handle: svg::Handle,
    mono: bool,
}

fn svg_icon_specs() -> [SvgIconSpec; 3] {
    [
        SvgIconSpec {
            bytes: include_bytes!("assets/beam/openai.svg"),
            handle: _shared::beam_icons::openai(),
            mono: true,
        },
        SvgIconSpec {
            bytes: include_bytes!("assets/beam/google-drive.svg"),
            handle: _shared::beam_icons::google_drive(),
            mono: false,
        },
        SvgIconSpec {
            bytes: include_bytes!("assets/beam/whatsapp.svg"),
            handle: _shared::beam_icons::whatsapp(),
            mono: false,
        },
    ]
}
