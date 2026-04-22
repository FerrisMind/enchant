use super::*;

impl WasmDemoApp {
    pub(crate) fn home_page(&self) -> Element<'_, Message> {
        let content = iced::widget::column![
            self.hero_section(),
            self.showcase_section(),
            self.testimonials_section(),
            self.video_testimonials_section(),
        ]
        .spacing(if self.is_lt_md() { 36.0 } else { 56.0 });

        container(content)
            .max_width(reference::data().layout.container_max_width)
            .width(Length::Fill)
            .padding([
                if self.is_lt_md() { 20.0 } else { 28.0 },
                self.shell_horizontal_padding(),
            ])
            .center_x(Length::Fill)
            .into()
    }

    pub(crate) fn hero_section(&self) -> Element<'_, Message> {
        let hero = &reference::data().home.hero;
        let align = if self.is_lt_md() {
            Alignment::Start
        } else {
            Alignment::Center
        };
        let ctas: Element<'_, Message> = if self.is_lt_md() {
            container(
                iced::widget::column![
                    container(hero_primary_cta(&self.theme, self.elapsed)).width(Length::Fill),
                    container(hero_secondary_cta(&self.theme)).width(Length::Fill),
                ]
                .spacing(12),
            )
            .width(Length::Fill)
            .into()
        } else {
            container(
                row![
                    hero_primary_cta(&self.theme, self.elapsed),
                    hero_secondary_cta(&self.theme)
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            )
            .width(Length::Fixed(320.0))
            .center_x(Length::Shrink)
            .into()
        };

        let text_side = iced::widget::column![
            container(button_content(
                row![
                    text("🎉")
                        .font(body_font())
                        .size(13)
                        .style(text_color(self.theme.palette.foreground)),
                    text(hero.announcement.as_str())
                        .font(body_font())
                        .size(13)
                        .style(text_color(self.theme.palette.foreground)),
                    lucide(
                        LucideIcon::ChevronRight,
                        14,
                        self.theme.palette.muted_foreground
                    ),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                Some(message_for_href(&hero.announcement_href)),
                ButtonProps::new()
                    .variant(ButtonVariant::Outline)
                    .size(ButtonSize::Size1),
                &self.theme,
            ))
            .center_x(if self.is_lt_md() {
                Length::Shrink
            } else {
                Length::Fill
            }),
            container(
                text(hero.title.as_str())
                    .font(display_font())
                    .size(if self.is_lt_md() { 48 } else { 72 })
                    .style(text_color(self.theme.palette.foreground)),
            )
            .max_width(696.0)
            .width(if self.is_lt_md() {
                Length::Fill
            } else {
                Length::Shrink
            })
            .center_x(if self.is_lt_md() {
                Length::Shrink
            } else {
                Length::Fill
            }),
            container(
                text(hero.description.as_str())
                    .font(body_font())
                    .size(if self.is_lt_md() { 16 } else { 18 })
                    .style(text_color(self.theme.palette.muted_foreground)),
            )
            .max_width(576.0)
            .width(if self.is_lt_md() {
                Length::Fill
            } else {
                Length::Shrink
            })
            .center_x(if self.is_lt_md() {
                Length::Shrink
            } else {
                Length::Fill
            }),
            ctas,
            container(tech_stack(&hero.tech_stack))
                .width(Length::Fixed(224.0))
                .center_x(if self.is_lt_md() {
                    Length::Shrink
                } else {
                    Length::Fill
                }),
        ]
        .spacing(if self.is_lt_md() { 18 } else { 24 })
        .width(Length::Fill)
        .align_x(align);

        container(
            container(text_side)
                .max_width(if self.is_3xl_up() { 720.0 } else { 696.0 })
                .width(Length::Fill)
                .center_x(Length::Fill),
        )
        .width(Length::Fill)
        .padding([
            if self.is_lt_md() { 20.0 } else { 56.0 },
            if self.is_lt_md() { 28.0 } else { 40.0 },
        ])
        .into()
    }

    pub(crate) fn showcase_section(&self) -> Element<'_, Message> {
        let home = &reference::data().home;
        iced::widget::column![
            centered_heading(
                home.showcase_title.as_str(),
                home.showcase_description.as_str(),
                &self.theme,
            ),
            showcase_track(
                &home.showcase_cards,
                self.elapsed,
                self.is_lt_md(),
                &self.theme
            ),
        ]
        .spacing(24)
        .into()
    }

    pub(crate) fn testimonials_section(&self) -> Element<'_, Message> {
        if self.sample_tweets.is_empty() {
            return iced::widget::column![
                centered_heading(
                    reference::data().home.testimonials_title.as_str(),
                    "",
                    &self.theme,
                ),
                text("Testimonials are unavailable in this build.")
                    .font(body_font())
                    .size(15)
                    .style(text_color(self.theme.palette.muted_foreground)),
            ]
            .spacing(24)
            .into();
        }

        let visible = self
            .sample_tweets
            .iter()
            .take(if self.is_lt_md() { 6 } else { 12 })
            .cloned()
            .collect::<Vec<_>>();
        let columns = if self.is_lt_md() { 1 } else { 4 };
        let mut distributed = vec![Vec::new(); columns];
        for (index, tweet) in visible.iter().enumerate() {
            distributed[index % columns].push(tweet.clone());
        }
        let mut rows = row![];
        for tweets in distributed {
            let mut column_view = iced::widget::column![];
            for tweet in tweets {
                column_view =
                    column_view.push(tweet_card(&tweet, &TweetCardProps::new(), &self.theme));
            }
            rows = rows.push(column_view.spacing(16).width(Length::FillPortion(1)));
        }

        let body: Element<'_, Message> = if self.is_lt_md() {
            column(
                visible
                    .iter()
                    .map(|tweet| tweet_card(tweet, &TweetCardProps::new(), &self.theme))
                    .collect::<Vec<_>>(),
            )
            .spacing(16)
            .into()
        } else {
            rows.spacing(16).into()
        };

        iced::widget::column![
            centered_heading(
                reference::data().home.testimonials_title.as_str(),
                "",
                &self.theme,
            ),
            body,
        ]
        .spacing(24)
        .align_x(Alignment::Center)
        .into()
    }

    pub(crate) fn video_testimonials_section(&self) -> Element<'_, Message> {
        let body: Element<'_, Message> = if self.is_lt_md() {
            column(
                reference::data()
                    .home
                    .video_embeds
                    .iter()
                    .map(|embed_url| video_thumbnail_tile(embed_url, 220.0, false, &self.theme))
                    .collect::<Vec<_>>(),
            )
            .spacing(16)
            .into()
        } else {
            let embeds = &reference::data().home.video_embeds;
            let hero = video_thumbnail_tile(&embeds[0], 360.0, true, &self.theme);
            let top_right = iced::widget::column![
                video_thumbnail_tile(&embeds[1], 172.0, false, &self.theme),
                video_thumbnail_tile(&embeds[2], 172.0, false, &self.theme),
            ]
            .spacing(16)
            .width(Length::FillPortion(1));
            let bottom = row![
                container(video_thumbnail_tile(&embeds[3], 196.0, false, &self.theme))
                    .width(Length::FillPortion(1)),
                container(video_thumbnail_tile(&embeds[4], 196.0, false, &self.theme))
                    .width(Length::FillPortion(1)),
            ]
            .spacing(16);

            iced::widget::column![
                row![container(hero).width(Length::FillPortion(2)), top_right].spacing(16),
                bottom,
            ]
            .spacing(16)
            .into()
        };

        iced::widget::column![
            centered_heading(reference::data().home.video_title.as_str(), "", &self.theme),
            body,
        ]
        .spacing(24)
        .into()
    }
}

fn hero_primary_cta(theme: &ShadTheme, elapsed: Duration) -> Element<'_, Message> {
    shimmer_button(
        text(reference::data().home.hero.primary_cta.as_str())
            .font(body_font())
            .size(16),
        Some(message_for_href(
            &reference::data().home.hero.primary_cta_href,
        )),
        elapsed,
        ShimmerButtonProps::new().button(
            ButtonProps::new()
                .variant(ButtonVariant::Default)
                .size(ButtonSize::Size3),
        ),
        theme,
    )
}

fn hero_secondary_cta(theme: &ShadTheme) -> Element<'_, Message> {
    button(
        reference::data().home.hero.secondary_cta.as_str(),
        Some(message_for_href(
            &reference::data().home.hero.secondary_cta_href,
        )),
        ButtonProps::new()
            .variant(ButtonVariant::Outline)
            .size(ButtonSize::Size3),
        theme,
    )
    .into()
}
