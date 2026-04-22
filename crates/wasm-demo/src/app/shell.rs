use super::*;

impl WasmDemoApp {
    pub(crate) fn site_banner(&self) -> Element<'_, Message> {
        let banner = &reference::data().banner;
        let banner_row = row![
            text("✨")
                .font(body_font())
                .size(if self.is_lt_md() { 13 } else { 14 })
                .style(text_color(Color::WHITE)),
            text(banner.text.as_str())
                .font(body_font())
                .size(if self.is_lt_md() { 12 } else { 14 })
                .style(text_color(Color::WHITE)),
            if self.is_lg_up() {
                lucide(LucideIcon::ChevronRight, 14, Color::WHITE)
            } else {
                Space::new().width(Length::Fixed(0.0)).into()
            }
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let bar = container(
            container(button_content(
                container(banner_row)
                    .width(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
                Some(Message::OpenUrl(banner.href.clone())),
                ButtonProps::new()
                    .variant(ButtonVariant::Ghost)
                    .size(ButtonSize::Size1),
                &self.theme,
            ))
            .max_width(reference::data().layout.container_max_width)
            .width(Length::Fill)
            .center_x(Length::Fill),
        )
        .height(Length::Fixed(if self.is_lt_md() { 44.0 } else { 48.0 }))
        .width(Length::Fill)
        .padding([0.0, self.shell_horizontal_padding()])
        .style({
            let background = rgba(reference::data().banner.background);
            move |_| iced::widget::container::Style {
                background: Some(Background::Color(background)),
                ..iced::widget::container::Style::default()
            }
        });

        iced::widget::column![
            bar,
            container(Space::new().height(Length::Fixed(1.0)))
                .height(Length::Fixed(1.0))
                .style({
                    let border = rgba(reference::data().banner.border);
                    move |_| iced::widget::container::Style {
                        background: Some(Background::Color(border)),
                        ..iced::widget::container::Style::default()
                    }
                }),
        ]
        .width(Length::Fill)
        .into()
    }

    pub(crate) fn site_header(&self) -> Element<'_, Message> {
        let ref_data = reference::data();
        let nav_buttons = ref_data
            .header
            .nav_items
            .iter()
            .map(|item| {
                let message = message_for_href(&item.href);
                let active = item.href == "/components"
                    && matches!(self.route, Route::Components | Route::Component(_));
                button(
                    item.title.as_str(),
                    Some(message),
                    ButtonProps::new()
                        .variant(if active {
                            ButtonVariant::Secondary
                        } else {
                            ButtonVariant::Ghost
                        })
                        .size(ButtonSize::Size1),
                    &self.theme,
                )
                .into()
            })
            .collect::<Vec<Element<'_, Message>>>();

        let social_links = row![
            button_content(
                row![
                    lucide(LucideIcon::Github, 16, self.theme.palette.foreground),
                    text(ref_data.header.github_stars.as_str())
                        .font(body_font())
                        .size(13)
                        .style(text_color(self.theme.palette.foreground)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                Some(Message::OpenUrl(ref_data.header.github_href.clone())),
                ButtonProps::new()
                    .variant(ButtonVariant::Ghost)
                    .size(ButtonSize::Size1),
                &self.theme,
            ),
            button_content(
                row![
                    lucide(
                        LucideIcon::MessageCircleMore,
                        16,
                        self.theme.palette.foreground,
                    ),
                    container(
                        Space::new()
                            .width(Length::Fixed(7.0))
                            .height(Length::Fixed(7.0))
                    )
                    .style(|_| iced::widget::container::Style {
                        background: Some(Background::Color(rgba([34, 197, 94, 255]))),
                        border: Border {
                            color: Color::TRANSPARENT,
                            width: 0.0,
                            radius: BorderRadius::from(999.0),
                        },
                        ..iced::widget::container::Style::default()
                    }),
                    text(ref_data.header.discord_online.as_str())
                        .font(body_font())
                        .size(13)
                        .style(text_color(self.theme.palette.foreground)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                Some(Message::OpenUrl(ref_data.header.discord_href.clone())),
                ButtonProps::new()
                    .variant(ButtonVariant::Ghost)
                    .size(ButtonSize::Size1),
                &self.theme,
            ),
            self.theme_toggle_button(),
        ]
        .spacing(2)
        .align_y(Alignment::Center);

        let content: Element<'_, Message> = if self.is_lt_md() {
            row![
                self.mobile_nav_button(),
                self.brand_lockup(false),
                Space::new().width(Length::Fill),
                button_content(
                    row![
                        lucide(LucideIcon::Github, 16, self.theme.palette.foreground),
                        text(ref_data.header.github_stars.as_str())
                            .font(body_font())
                            .size(13)
                            .style(text_color(self.theme.palette.foreground)),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                    Some(Message::OpenUrl(ref_data.header.github_href.clone())),
                    ButtonProps::new()
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::Size1),
                    &self.theme,
                ),
                button_content(
                    row![
                        lucide(
                            LucideIcon::MessageCircleMore,
                            16,
                            self.theme.palette.foreground,
                        ),
                        text(ref_data.header.discord_online.as_str())
                            .font(body_font())
                            .size(13)
                            .style(text_color(self.theme.palette.foreground)),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                    Some(Message::OpenUrl(ref_data.header.discord_href.clone())),
                    ButtonProps::new()
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::Size1),
                    &self.theme,
                ),
                self.theme_toggle_button(),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        } else if self.is_lt_lg() {
            row![
                self.mobile_nav_button(),
                self.brand_lockup(true),
                Space::new().width(Length::Fill),
                self.header_search(),
                self.theme_toggle_button(),
            ]
            .spacing(12)
            .align_y(Alignment::Center)
            .into()
        } else {
            row![
                self.brand_lockup(true),
                row(nav_buttons).spacing(8).align_y(Alignment::Center),
                Space::new().width(Length::Fill),
                self.header_search(),
                social_links,
            ]
            .spacing(16)
            .align_y(Alignment::Center)
            .into()
        };

        container(
            container(content)
                .max_width(reference::data().layout.container_max_width)
                .width(Length::Fill)
                .center_x(Length::Fill),
        )
        .height(Length::Fixed(reference::data().layout.header_height + 1.0))
        .width(Length::Fill)
        .padding([0.0, self.shell_horizontal_padding()])
        .style({
            let background = Color {
                a: 0.94,
                ..self.theme.palette.background
            };
            let border = Color {
                a: 0.12,
                ..self.theme.palette.border
            };
            move |_| iced::widget::container::Style {
                background: Some(Background::Color(background)),
                border: Border {
                    color: border,
                    width: 0.0,
                    radius: BorderRadius::from(0.0),
                },
                ..iced::widget::container::Style::default()
            }
        })
        .into()
    }

    pub(crate) fn header_search(&self) -> Element<'_, Message> {
        let label = if self.is_lg_up() {
            "Search documentation..."
        } else {
            "Search..."
        };
        let width = if self.is_lg_up() { 256.0 } else { 160.0 };

        button_content(
            container(
                row![
                    lucide(LucideIcon::Search, 16, self.theme.palette.muted_foreground),
                    text(label)
                        .font(body_font())
                        .size(13)
                        .style(text_color(self.theme.palette.muted_foreground)),
                    Space::new().width(Length::Fill),
                    badge(
                        "⌘K",
                        Color {
                            a: 0.08,
                            ..self.theme.palette.foreground
                        },
                        self.theme.palette.muted_foreground,
                    ),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .width(Length::Fixed(width))
            .padding([8.0, 12.0])
            .style(card_shell_style(
                self.theme.palette.card,
                self.theme.palette.border,
                self.theme.radius.md,
            )),
            Some(Message::Noop),
            ButtonProps::new()
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Size1),
            &self.theme,
        )
        .into()
    }

    pub(crate) fn site_footer(&self) -> Element<'_, Message> {
        let footer_height = if self.window_width >= 1280.0 {
            reference::data().layout.footer_height_xl
        } else {
            reference::data().layout.footer_height
        };

        let author_link: Element<'_, Message> = button(
            "dillion",
            Some(Message::OpenUrl(
                reference::data().footer.author_href.clone(),
            )),
            ButtonProps::new()
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Size1),
            &self.theme,
        )
        .into();
        let github_link: Element<'_, Message> = button(
            "GitHub",
            Some(Message::OpenUrl(
                reference::data().footer.github_href.clone(),
            )),
            ButtonProps::new()
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Size1),
            &self.theme,
        )
        .into();
        let content: Element<'_, Message> = if self.is_lt_md() {
            iced::widget::column![
                row![
                    text("Built by")
                        .font(body_font())
                        .size(12)
                        .style(text_color(self.theme.palette.muted_foreground)),
                    author_link,
                ]
                .spacing(4)
                .align_y(Alignment::Center),
                row![
                    text("The source code is available on")
                        .font(body_font())
                        .size(12)
                        .style(text_color(self.theme.palette.muted_foreground)),
                    github_link,
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            ]
            .spacing(2)
            .into()
        } else {
            row![
                text("Built by")
                    .font(body_font())
                    .size(13)
                    .style(text_color(self.theme.palette.muted_foreground)),
                author_link,
                text(". The source code is available on")
                    .font(body_font())
                    .size(13)
                    .style(text_color(self.theme.palette.muted_foreground)),
                github_link,
                text(".")
                    .font(body_font())
                    .size(13)
                    .style(text_color(self.theme.palette.muted_foreground)),
            ]
            .spacing(4)
            .align_y(Alignment::Center)
            .into()
        };

        container(
            container(content)
                .max_width(reference::data().layout.container_max_width)
                .width(Length::Fill)
                .center_x(Length::Fill),
        )
        .height(Length::Fixed(footer_height))
        .width(Length::Fill)
        .padding([0.0, self.shell_horizontal_padding()])
        .style({
            let border = Color {
                a: 0.14,
                ..self.theme.palette.border
            };
            move |_| iced::widget::container::Style {
                border: Border {
                    color: border,
                    width: 1.0,
                    radius: BorderRadius::from(0.0),
                },
                ..iced::widget::container::Style::default()
            }
        })
        .into()
    }

    pub(crate) fn brand_lockup(&self, show_wordmark: bool) -> Element<'_, Message> {
        let icon = container(
            text("m")
                .font(display_font())
                .size(18)
                .style(text_color(Color::WHITE)),
        )
        .width(Length::Fixed(28.0))
        .height(Length::Fixed(28.0))
        .center_x(Length::Fixed(28.0))
        .center_y(Length::Fixed(28.0))
        .style({
            let fill = rgba([246, 134, 162, 255]);
            move |_| iced::widget::container::Style {
                background: Some(Background::Color(fill)),
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: BorderRadius::from(999.0),
                },
                ..iced::widget::container::Style::default()
            }
        });

        let mut content = row![icon].spacing(10).align_y(Alignment::Center);
        if show_wordmark {
            content = content.push(
                text("Magic UI")
                    .font(display_font())
                    .size(22)
                    .style(text_color(self.theme.palette.foreground)),
            );
        }

        button_content(
            content,
            Some(Message::Navigate(Route::Home)),
            ButtonProps::new()
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Size1),
            &self.theme,
        )
        .into()
    }

    pub(crate) fn mobile_nav_button(&self) -> Element<'_, Message> {
        let top = container(
            Space::new()
                .width(Length::Fixed(16.0))
                .height(Length::Fixed(2.0)),
        )
        .style({
            let color = self.theme.palette.foreground;
            move |_| iced::widget::container::Style {
                background: Some(Background::Color(color)),
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: BorderRadius::from(999.0),
                },
                ..iced::widget::container::Style::default()
            }
        });
        let bottom = container(
            Space::new()
                .width(Length::Fixed(16.0))
                .height(Length::Fixed(2.0)),
        )
        .style({
            let color = self.theme.palette.foreground;
            move |_| iced::widget::container::Style {
                background: Some(Background::Color(color)),
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: BorderRadius::from(999.0),
                },
                ..iced::widget::container::Style::default()
            }
        });
        let glyph: Element<'_, Message> = if self.mobile_nav_open {
            text("✕")
                .font(body_font())
                .size(18)
                .style(text_color(self.theme.palette.foreground))
                .into()
        } else {
            iced::widget::column![top, bottom]
                .spacing(5)
                .align_x(Alignment::Center)
                .into()
        };

        button_content(
            container(glyph)
                .width(Length::Fixed(32.0))
                .height(Length::Fixed(32.0))
                .center_x(Length::Fixed(32.0))
                .center_y(Length::Fixed(32.0)),
            Some(Message::ToggleMobileNav),
            ButtonProps::new()
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Size1),
            &self.theme,
        )
        .into()
    }

    pub(crate) fn theme_toggle_button(&self) -> Element<'_, Message> {
        let icon = if matches!(self.theme_mode, ThemeMode::Dark) {
            LucideIcon::Moon
        } else {
            LucideIcon::Sun
        };

        button_content(
            container(lucide(icon, 16, self.theme.palette.foreground))
                .width(Length::Fixed(32.0))
                .height(Length::Fixed(32.0))
                .center_x(Length::Fixed(32.0))
                .center_y(Length::Fixed(32.0)),
            Some(Message::ToggleTheme),
            ButtonProps::new()
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Size1),
            &self.theme,
        )
        .into()
    }

    pub(crate) fn mobile_nav_overlay(&self) -> Element<'_, Message> {
        let nav_items = reference::data()
            .header
            .nav_items
            .iter()
            .map(|item| {
                button_content(
                    text(item.title.as_str())
                        .font(display_font())
                        .size(28)
                        .style(text_color(self.theme.palette.foreground)),
                    Some(message_for_href(&item.href)),
                    ButtonProps::new()
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::Size1),
                    &self.theme,
                )
                .into()
            })
            .collect::<Vec<Element<'_, Message>>>();

        container(
            container(
                iced::widget::column![
                    if self.is_md_up() {
                        self.header_search()
                    } else {
                        Space::new().height(Length::Fixed(0.0)).into()
                    },
                    column(nav_items).spacing(12),
                    Space::new().height(Length::Fill),
                    row![
                        button(
                            "GitHub",
                            Some(Message::OpenUrl(
                                reference::data().header.github_href.clone()
                            )),
                            ButtonProps::new()
                                .variant(ButtonVariant::Ghost)
                                .size(ButtonSize::Size1),
                            &self.theme,
                        ),
                        button(
                            "Discord",
                            Some(Message::OpenUrl(
                                reference::data().header.discord_href.clone()
                            )),
                            ButtonProps::new()
                                .variant(ButtonVariant::Ghost)
                                .size(ButtonSize::Size1),
                            &self.theme,
                        ),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                ]
                .spacing(20)
                .width(Length::Fill),
            )
            .max_width(reference::data().layout.container_max_width)
            .width(Length::Fill)
            .center_x(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([
            reference::data().layout.header_height + 24.0,
            self.shell_horizontal_padding(),
        ])
        .style({
            let background = Color {
                a: 0.88,
                ..self.theme.palette.background
            };
            move |_| iced::widget::container::Style {
                background: Some(Background::Color(background)),
                ..iced::widget::container::Style::default()
            }
        })
        .into()
    }
}
