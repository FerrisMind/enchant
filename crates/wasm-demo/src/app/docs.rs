use super::*;

impl WasmDemoApp {
    pub(crate) fn components_page(&self) -> Element<'_, Message> {
        let Some(doc) = reference_doc("index") else {
            return self.not_found_page("/components");
        };

        self.docs_shell(
            &doc.title,
            &doc.description,
            iced::widget::column![
                self.docs_breadcrumb("Docs / Components"),
                self.catalog_grid(),
            ]
            .spacing(18)
            .into(),
        )
    }

    pub(crate) fn component_detail(&self, slug: &str) -> Element<'_, Message> {
        if !is_supported_doc_slug(slug) {
            return self.not_found_page(&format!("/docs/components/{slug}"));
        }
        let Some(doc) = reference_doc(slug) else {
            return self.not_found_page(&format!("/docs/components/{slug}"));
        };
        let mut tabs = row![
            tab_button(
                &self.theme,
                "Preview",
                self.preview_tab == PreviewTab::Preview,
                Message::SetPreviewTab(PreviewTab::Preview),
            ),
            tab_button(
                &self.theme,
                "Code",
                self.preview_tab == PreviewTab::Code,
                Message::SetPreviewTab(PreviewTab::Code),
            ),
            Space::new().width(Length::Fill),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        if preview_slug(slug).is_some() {
            tabs = tabs.push(button(
                "Reset Demo",
                Some(Message::ResetDetailDemo),
                ButtonProps::new()
                    .variant(ButtonVariant::Outline)
                    .size(ButtonSize::Size1),
                &self.theme,
            ));
        }

        let panel = if self.preview_tab == PreviewTab::Preview {
            self.preview_panel(slug)
        } else {
            self.code_panel(doc)
        };

        self.docs_shell(
            &doc.title,
            &doc.description,
            iced::widget::column![
                self.docs_breadcrumb(&format!("Docs / Components / {}", doc.title)),
                tabs,
                panel,
                self.render_doc_body(doc),
                self.docs_navigation(slug),
            ]
            .spacing(18)
            .into(),
        )
    }

    pub(crate) fn not_found_page(&self, path: &str) -> Element<'_, Message> {
        self.docs_shell(
            "404",
            "The requested docs route is not part of the first native demo slice.",
            iced::widget::column![
                text(format!("Unknown route: {path}"))
                    .font(body_font())
                    .size(16)
                    .style(text_color(self.theme.palette.muted_foreground)),
                button(
                    "Back to Home",
                    Some(Message::Navigate(Route::Home)),
                    ButtonProps::new()
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Size2),
                    &self.theme,
                ),
            ]
            .spacing(16)
            .into(),
        )
    }

    pub(crate) fn docs_shell<'a>(
        &'a self,
        title: &'a str,
        description: &'a str,
        content: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let sidebar = self.docs_sidebar();
        let head = iced::widget::column![
            text(title)
                .font(display_font())
                .size(if self.is_lt_md() { 34 } else { 44 })
                .style(text_color(self.theme.palette.foreground)),
            text(description)
                .font(body_font())
                .size(17)
                .style(text_color(self.theme.palette.muted_foreground)),
        ]
        .spacing(8);

        let main = iced::widget::column![head, content]
            .spacing(20)
            .width(Length::Fill)
            .max_width(reference::data().layout.docs_content_max_width);

        let layout: Element<'_, Message> = if self.is_lt_md() {
            iced::widget::column![sidebar, main].spacing(20).into()
        } else {
            let right_rail: Element<'_, Message> = match &self.route {
                Route::Component(slug) => reference_doc(slug)
                    .map(|doc| self.render_right_rail(doc))
                    .unwrap_or_else(|| Space::new().width(Length::Shrink).into()),
                _ => self.docs_toc(),
            };
            row![
                container(sidebar).width(Length::Fixed(reference::data().layout.docs_sidebar_width)),
                container(main).width(Length::Fill),
                right_rail,
            ]
            .spacing(28)
            .into()
        };

        container(layout)
            .max_width(reference::data().layout.container_max_width)
            .center_x(Length::Fill)
            .width(Length::Fill)
            .padding([
                if self.is_lt_md() { 20.0 } else { 24.0 },
                reference::data().layout.container_padding,
            ])
            .into()
    }

    pub(crate) fn docs_breadcrumb(&self, text_value: impl Into<String>) -> Element<'_, Message> {
        text(text_value.into())
            .font(body_font())
            .size(14)
            .style(text_color(self.theme.palette.muted_foreground))
            .into()
    }

    pub(crate) fn docs_toc(&self) -> Element<'_, Message> {
        let Some(doc) = (match &self.route {
            Route::Component(slug) => reference_doc(slug),
            _ => None,
        }) else {
            return Space::new().width(Length::Shrink).into();
        };

        if self.window_width < 1280.0 || doc.toc.is_empty() {
            return Space::new().width(Length::Shrink).into();
        }

        let items = doc
            .toc
            .iter()
            .map(|item| {
                text(item.title.as_str())
                    .font(body_font())
                    .size(if item.level > 2 { 13 } else { 14 })
                    .style(text_color(self.theme.palette.muted_foreground))
                    .into()
            })
            .collect::<Vec<Element<'_, Message>>>();

        container(
            iced::widget::column![
                Space::new().height(Length::Fixed(reference::data().layout.docs_top_spacing)),
                text("On This Page")
                    .font(body_font())
                    .size(12)
                    .style(text_color(self.theme.palette.muted_foreground)),
                column(items).spacing(10),
            ]
            .spacing(14),
        )
        .width(Length::Fixed(reference::data().layout.docs_toc_width))
        .into()
    }

    pub(crate) fn docs_navigation(&self, slug: &str) -> Element<'_, Message> {
        let (previous, next) = supported_doc_neighbors(slug);
        if previous.is_none() && next.is_none() {
            return Space::new().height(Length::Shrink).into();
        }

        let previous_button: Element<'_, Message> = if let Some(previous) = previous {
            button(
                format!("← {}", previous.title),
                Some(Message::Navigate(Route::Component(previous.slug.clone()))),
                ButtonProps::new()
                    .variant(ButtonVariant::Secondary)
                    .size(ButtonSize::Size1),
                &self.theme,
            )
            .into()
        } else {
            Space::new().width(Length::Shrink).into()
        };

        let next_button: Element<'_, Message> = if let Some(next) = next {
            button(
                format!("{} →", next.title),
                Some(Message::Navigate(Route::Component(next.slug.clone()))),
                ButtonProps::new()
                    .variant(ButtonVariant::Secondary)
                    .size(ButtonSize::Size1),
                &self.theme,
            )
            .into()
        } else {
            Space::new().width(Length::Shrink).into()
        };

        row![previous_button, Space::new().width(Length::Fill), next_button,]
            .align_y(Alignment::Center)
            .into()
    }

    pub(crate) fn docs_sidebar(&self) -> Element<'_, Message> {
        let ref_data = reference::data();
        let sidebar = clone_sidebar_sections(match &self.route {
            Route::Component(slug) => reference_doc(slug)
                .map(|page| page.sidebar.as_slice())
                .unwrap_or(ref_data.home.sidebar.as_slice()),
            _ => ref_data.home.sidebar.as_slice(),
        });
        let search_results = ref_data
            .pages
            .iter()
            .filter(|doc| {
                let needle = self.search.trim().to_lowercase();
                needle.is_empty()
                    || doc.title.to_lowercase().contains(&needle)
                    || doc.description.to_lowercase().contains(&needle)
            })
            .map(|doc| doc.slug.clone())
            .collect::<std::collections::BTreeSet<_>>();
        let mut content = iced::widget::column![
            input(
                &self.search,
                &ref_data.header.search_placeholder,
                Some(Message::SearchChanged),
                InputProps::new()
                    .size(InputSize::Size2)
                    .variant(InputVariant::Surface),
                &self.theme,
            ),
            button(
                "Catalog",
                Some(Message::Navigate(Route::Components)),
                ButtonProps::new()
                    .variant(if matches!(self.route, Route::Components) {
                        ButtonVariant::Secondary
                    } else {
                        ButtonVariant::Ghost
                    })
                    .size(ButtonSize::Size1),
                &self.theme,
            ),
        ]
        .spacing(10);

        for group in &sidebar {
            let mut group_items = iced::widget::column![];
            let mut has_visible_items = false;
            for item in &group.items {
                let visible = self.search.trim().is_empty()
                    || item.title.to_lowercase().contains(&self.search.trim().to_lowercase())
                    || ref_data
                        .pages
                        .iter()
                        .find(|doc| Some(doc.slug.as_str()) == component_slug_from_href(&item.href))
                        .map(|doc| search_results.contains(&doc.slug))
                        .unwrap_or(false);
                if !visible {
                    continue;
                }

                let active = matches!(&self.route, Route::Component(slug) if Some(slug.as_str()) == component_slug_from_href(&item.href));
                has_visible_items = true;
                group_items = group_items.push(button(
                    item.title.clone(),
                    Some(message_for_href(&item.href)),
                    ButtonProps::new()
                        .variant(if active {
                            ButtonVariant::Secondary
                        } else {
                            ButtonVariant::Ghost
                        })
                        .size(ButtonSize::Size1),
                    &self.theme,
                ));
            }

            if !has_visible_items {
                continue;
            }
            content = content.push(
                container(
                    iced::widget::column![
                        text(group.title.clone())
                            .font(body_font())
                            .size(12)
                            .style(text_color(self.theme.palette.muted_foreground)),
                        group_items.spacing(6),
                    ]
                    .spacing(8),
                )
                .padding([8.0, 0.0]),
            );
        }

        container(content.spacing(12))
            .width(Length::Fill)
            .into()
    }

    pub(crate) fn catalog_grid(&self) -> Element<'_, Message> {
        let needle = self.search.trim().to_lowercase();
        let pages = catalog_sidebar_items()
            .into_iter()
            .filter(|item| {
                if needle.is_empty() {
                    return true;
                }

                item.title.to_lowercase().contains(&needle)
                    || component_slug_from_href(&item.href)
                        .and_then(reference_doc)
                        .map(|doc| doc.description.to_lowercase().contains(&needle))
                        .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        if pages.is_empty() {
            return text("No matching components.")
                .font(body_font())
                .size(15)
                .style(text_color(self.theme.palette.muted_foreground))
                .into();
        }

        let columns = if self.is_lt_md() { 1 } else { 3 };
        let mut rows_view = iced::widget::column![];
        for chunk in pages.chunks(columns) {
            let cols = chunk
                .iter()
                .map(|item| {
                    button_content(
                        text(item.title.as_str())
                            .font(body_font())
                            .size(16)
                            .style(text_color(self.theme.palette.foreground)),
                        Some(message_for_href(&item.href)),
                        ButtonProps::new()
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::Size1),
                        &self.theme,
                    )
                    .width(Length::Fill)
                    .into()
                })
                .collect::<Vec<Element<'_, Message>>>();

            rows_view = rows_view.push(
                row(cols)
                    .spacing(24)
                    .width(Length::Fill)
                    .align_y(Alignment::Start),
            );
        }

        rows_view.spacing(12).into()
    }

    fn render_doc_body<'a>(&'a self, doc: &'a RefDocPage) -> Element<'a, Message> {
        let blocks = doc
            .blocks
            .iter()
            .map(|block| self.render_body_block(block))
            .collect::<Vec<_>>();

        let _ = doc;
        column(blocks).spacing(18).into()
    }

    fn render_right_rail(&self, doc: &RefDocPage) -> Element<'_, Message> {
        let toc = doc
            .toc
            .iter()
            .map(|item| {
                let indent = if item.level > 2 { "    " } else { "" };
                text(format!("{indent}{}", item.title))
                    .font(body_font())
                    .size(14)
                    .style(text_color(self.theme.palette.muted_foreground))
                    .into()
            })
            .collect::<Vec<Element<'_, Message>>>();

        let contribute = iced::widget::column![
            text("Contribute")
                .font(display_font())
                .size(18)
                .style(text_color(self.theme.palette.foreground)),
            text("⚐ Report an issue").font(body_font()).size(14),
            text("✦ Request a feature").font(body_font()).size(14),
            text("✎ Edit this page").font(body_font()).size(14),
        ]
        .spacing(10);

        let promo = magic_card(
            iced::widget::column![
                text("Ship Faster with Magic UI Pro")
                    .font(display_font())
                    .size(20)
                    .style(text_color(self.theme.palette.foreground)),
                text("Stop building from scratch. Get premium templates and blocks that match the design language of the docs.")
                    .font(body_font())
                    .size(14)
                    .style(text_color(self.theme.palette.muted_foreground)),
            ]
            .spacing(12),
            MagicCardProps::new()
                .width(Length::Fill)
                .height(Length::Fixed(220.0)),
            &self.theme,
        );

        iced::widget::column![
            container(
                iced::widget::column![
                    text("On This Page")
                        .font(display_font())
                        .size(18)
                        .style(text_color(self.theme.palette.foreground)),
                    column(toc).spacing(10),
                ]
                .spacing(12),
            )
            .padding([16.0, 16.0])
            .style(card_shell_style(
                self.theme.palette.card,
                self.theme.palette.border,
                self.theme.radius.lg,
            )),
            container(contribute)
                .padding([16.0, 16.0])
                .style(card_shell_style(
                    self.theme.palette.card,
                    self.theme.palette.border,
                    self.theme.radius.lg,
                )),
            promo,
        ]
        .spacing(18)
        .into()
    }

    fn render_body_block<'a>(&'a self, block: &'a BodyBlock) -> Element<'a, Message> {
        match block {
            BodyBlock::Heading {
                level,
                text: heading_text,
                ..
            } => {
                let size = if *level == 2 { 34 } else { 24 };
                text(heading_text.as_str())
                    .font(display_font())
                    .size(size)
                    .style(text_color(self.theme.palette.foreground))
                    .into()
            }
            BodyBlock::Paragraph { text: body } => text(body.as_str())
                .font(body_font())
                .size(16)
                .style(text_color(self.theme.palette.primary))
                .into(),
            BodyBlock::CodeFence {
                language,
                title,
                code,
            } => self.render_code_fence(language.as_deref(), title.as_deref(), code),
            BodyBlock::PropsTable { headers, rows } => self.render_props_table(headers, rows),
            BodyBlock::PreviewRef { name } => self.render_preview_ref(name),
            BodyBlock::SourceRef { title, code, .. } => self.render_code_fence(
                Some("tsx"),
                title.as_deref(),
                code.as_deref().unwrap_or_default(),
            ),
            BodyBlock::Tabs {
                default_value,
                items,
            } => self.render_tabs_block(default_value.as_deref(), items),
            BodyBlock::Steps { items } => self.render_steps_block(items),
            BodyBlock::ComponentsList => self.catalog_grid(),
            BodyBlock::List { items } => {
                let nodes = items
                    .iter()
                    .map(|item| {
                        row![
                            text("•").size(16),
                            text(item.as_str())
                                .font(body_font())
                                .size(15)
                                .style(text_color(self.theme.palette.primary)),
                        ]
                        .spacing(10)
                        .into()
                    })
                    .collect::<Vec<Element<'_, Message>>>();
                column(nodes).spacing(10).into()
            }
            BodyBlock::Note { text: note } => container(
                text(note.as_str())
                    .font(body_font())
                    .size(15)
                    .style(text_color(self.theme.palette.foreground)),
            )
            .padding([14.0, 16.0])
            .style(card_shell_style(
                self.theme.palette.secondary,
                self.theme.palette.border,
                self.theme.radius.md,
            ))
            .into(),
        }
    }

    fn render_preview_ref(&self, name: &str) -> Element<'_, Message> {
        let (preview, prewrapped) = match name {
            "dock-demo" => (self.preview_dock_variant(DockDirection::Middle), false),
            "dock-demo-2" => (self.preview_dock_variant(DockDirection::Top), false),
            "dock-demo-3" => (self.preview_dock_variant(DockDirection::Bottom), false),
            "marquee-demo" => (self.preview_marquee(), false),
            "marquee-demo-vertical" => (self.preview_marquee_vertical(), false),
            "terminal-demo" => (self.preview_terminal(), false),
            "terminal-demo-2" => (self.preview_terminal_alt(), false),
            "tweet-card-demo" => (self.preview_tweet_card(), false),
            "tweet-card-images" => (self.preview_tweet_card_images(), false),
            "tweet-card-meta-preview" => (self.preview_tweet_card_meta(), false),
            "video-text-demo" => (self.preview_video_text_demo(), false),
            _ => {
                let slug = preview_slug_from_name(name).unwrap_or(name);
                (self.preview_panel(slug), true)
            }
        };
        let preview: Element<'_, Message> = if prewrapped {
            preview
        } else {
            container(preview)
                .width(Length::Fill)
                .padding([20.0, 20.0])
                .style(card_shell_style(
                    self.theme.palette.card,
                    self.theme.palette.border,
                    self.theme.radius.lg,
                ))
                .into()
        };
        iced::widget::column![
            row![
                text("Preview")
                    .font(display_font())
                    .size(18)
                    .style(text_color(self.theme.palette.foreground)),
                Space::new().width(Length::Fill),
                button(
                    "Reset",
                    Some(Message::ResetDetailDemo),
                    ButtonProps::new()
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::Size1),
                    &self.theme,
                ),
            ]
            .align_y(Alignment::Center),
            preview,
        ]
        .spacing(12)
        .into()
    }

    fn render_code_fence<'a>(
        &self,
        language: Option<&'a str>,
        title: Option<&'a str>,
        code: &'a str,
    ) -> Element<'a, Message> {
        let header = row![
            text(title.unwrap_or(language.unwrap_or("code")))
                .font(body_font())
                .size(13)
                .style(text_color(self.theme.palette.muted_foreground)),
        ];

        container(
            iced::widget::column![
                header,
                text(code)
                    .font(mono_font())
                    .size(14)
                    .style(text_color(self.theme.palette.foreground)),
            ]
            .spacing(12),
        )
        .padding([16.0, 16.0])
        .style(card_shell_style(
            self.theme.palette.card,
            self.theme.palette.border,
            self.theme.radius.lg,
        ))
        .into()
    }

    fn render_props_table<'a>(
        &self,
        headers: &'a [String],
        rows: &'a [Vec<String>],
    ) -> Element<'a, Message> {
        let mut items = iced::widget::column![];
        if !headers.is_empty() {
            items = items.push(
                row(
                    headers
                        .iter()
                        .map(|header| {
                            text(header.as_str())
                                .font(display_font())
                                .size(14)
                                .style(text_color(self.theme.palette.foreground))
                                .width(Length::FillPortion(1))
                                .into()
                        })
                        .collect::<Vec<Element<'_, Message>>>(),
                )
                .spacing(12),
            );
        }
        for row_values in rows {
            items = items.push(
                row(
                    row_values
                        .iter()
                        .map(|value| {
                            text(value.as_str())
                                .font(body_font())
                                .size(14)
                                .style(text_color(self.theme.palette.primary))
                                .width(Length::FillPortion(1))
                                .into()
                        })
                        .collect::<Vec<Element<'_, Message>>>(),
                )
                .spacing(12),
            );
        }

        container(items.spacing(10))
            .padding([16.0, 16.0])
            .style(card_shell_style(
                self.theme.palette.card,
                self.theme.palette.border,
                self.theme.radius.lg,
            ))
            .into()
    }

    fn render_tabs_block<'a>(
        &'a self,
        default_value: Option<&'a str>,
        items: &'a [crate::reference::TabItem],
    ) -> Element<'a, Message> {
        let active = default_value
            .and_then(|value| items.iter().find(|item| item.value == value))
            .unwrap_or_else(|| items.first().expect("tabs must contain at least one item"));

        let labels = row(
            items.iter()
                .map(|item| {
                    text(item.label.as_str())
                        .font(display_font())
                        .size(16)
                        .style(text_color(if item.value == active.value {
                            self.theme.palette.foreground
                        } else {
                            self.theme.palette.muted_foreground
                        }))
                        .into()
                })
                .collect::<Vec<Element<'_, Message>>>(),
        )
        .spacing(20);

        let blocks = active
            .blocks
            .iter()
            .map(|block| self.render_body_block(block))
            .collect::<Vec<_>>();

        iced::widget::column![labels, column(blocks).spacing(16)].spacing(14).into()
    }

    fn render_steps_block<'a>(
        &'a self,
        items: &'a [crate::reference::StepItem],
    ) -> Element<'a, Message> {
        let steps = items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let blocks = item
                    .blocks
                    .iter()
                    .map(|block| self.render_body_block(block))
                    .collect::<Vec<_>>();
                container(
                    iced::widget::column![
                        text(format!("{}. {}", index + 1, item.title))
                            .font(display_font())
                            .size(18)
                            .style(text_color(self.theme.palette.foreground)),
                        column(blocks).spacing(12),
                    ]
                    .spacing(10),
                )
                .padding([14.0, 16.0])
                .style(card_shell_style(
                    self.theme.palette.card,
                    self.theme.palette.border,
                    self.theme.radius.md,
                ))
                .into()
            })
            .collect::<Vec<Element<'_, Message>>>();

        column(steps).spacing(14).into()
    }
}

