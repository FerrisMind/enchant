mod _shared;

use iced::time::Instant;
use std::time::Duration;

use iced::widget::{column, container, stack, text, text_input};
use iced::{Element, Length, Size, Task};
use iced_shadcn::{ButtonProps, ButtonSize, ButtonVariant, Theme, button_content};
use magic_iced::{NoiseTextureProps, noise_texture, noise_texture_layer};

pub fn main() -> iced::Result {
    iced::application(
        NoiseTextureExample::new,
        NoiseTextureExample::update,
        NoiseTextureExample::view,
    )
    .title("Noise Texture")
    .window(iced::window::Settings {
        size: Size::new(1100.0, 760.0),
        ..Default::default()
    })
    .subscription(NoiseTextureExample::subscription)
        
    .run()
}

struct NoiseTextureExample {
    theme_shell: _shared::ThemeShell,
    active_tab: usize,
    search: String,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    Tick(Instant),
    AnchorsMeasured(magic_iced::AnchorMeasurements),
    SelectTab(usize),
    SearchChanged(String),
}

impl NoiseTextureExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                active_tab: 0,
                search: String::new(),
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

            Message::SelectTab(index) => self.active_tab = index,
            Message::SearchChanged(value) => self.search = value,
            Message::Tick(now) => {
                self.theme_shell.tick(now);
            }
        }
        Task::none()
    }


    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let tabs = _shared::tabs(
            &["Demo", "Demo 2", "Demo 3", "Demo 4"],
            self.active_tab,
            Message::SelectTab,
        );

        let body = match self.active_tab {
            0 => self.radial_panel(),
            1 => self.newsletter_card(),
            2 => self.subscribe_button(),
            _ => self.search_field(),
        };

        _shared::themed_shell("Noise Texture", column![tabs, _shared::stage(body)].spacing(24),
            &self.theme_shell, Message::ToggleTheme)
    }

    fn radial_panel(&self) -> Element<'_, Message> {
        let noise = NoiseTextureProps::new()
            .height(Length::Fixed(500.0))
            .radial_mask_radius(420.0);

        container(stack(vec![
            container(text(""))
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
            noise_texture(noise),
        ]))
        .width(Length::Fill)
        .height(Length::Fixed(500.0))
        .into()
    }

    fn newsletter_card(&self) -> Element<'_, Message> {
        let noise = NoiseTextureProps::new().noise_opacity(0.45);

        container(noise_texture_layer(
            column![
                text("The weekly digest").size(22),
                text(
                    "One email on Sundays—new components, tips, and changelog highlights. \
                     No spam, unsubscribe anytime."
                )
                .size(14),
                text("Email").size(13),
                text_input("you@company.com", "")
                    .padding(10)
                    .width(Length::Fill),
                button_content(
                    text("Subscribe"),
                    None::<Message>,
                    ButtonProps::new()
                        .variant(ButtonVariant::Default)
                        .size(ButtonSize::Size2),
                    self.theme_shell.theme(),
                ),
            ]
            .spacing(12)
            .padding(20),
            noise,
        ))
        .width(Length::Fixed(420.0))
        .padding(1)
        .into()
    }

    fn subscribe_button(&self) -> Element<'_, Message> {
        let noise = NoiseTextureProps::new().noise_opacity(0.45);

        container(noise_texture_layer(
            container(text("Subscribe").size(16))
                .padding([12, 28])
                .center_x(Length::Shrink),
            noise,
        ))
        .padding(1)
        .into()
    }

    fn search_field(&self) -> Element<'_, Message> {
        let noise = NoiseTextureProps::new().noise_opacity(0.45);

        column![
            text("Search with texture").size(13),
            container(noise_texture_layer(
                text_input("Try typing…", &self.search)
                    .on_input(Message::SearchChanged)
                    .padding(12)
                    .width(Length::Fill),
                noise,
            ))
            .width(Length::Fill)
            .padding(1),
        ]
        .spacing(10)
        .width(Length::Fixed(360.0))
        .into()
    }
}
