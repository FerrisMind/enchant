use super::*;

impl WasmDemoApp {
    pub(crate) fn preview_panel(&self, slug: &str) -> Element<'_, Message> {
        let preview = match preview_slug(slug) {
            Some(ComponentSlug::Marquee) => self.preview_marquee(),
            Some(ComponentSlug::Terminal) => self.preview_terminal(),
            Some(ComponentSlug::TweetCard) => self.preview_tweet_card(),
            Some(ComponentSlug::WordRotate) => self.preview_word_rotate(),
            Some(ComponentSlug::SparklesText) => self.preview_sparkles_text(),
            Some(ComponentSlug::AnimatedGradientText) => self.preview_gradient_text(),
            Some(ComponentSlug::NumberTicker) => self.preview_number_ticker(),
            Some(ComponentSlug::TextReveal) => self.preview_text_reveal(),
            Some(ComponentSlug::ShimmerButton) => self.preview_shimmer_button(),
            Some(ComponentSlug::MagicCard) => self.preview_magic_card(),
            Some(ComponentSlug::Dock) => self.preview_dock(),
            Some(ComponentSlug::ThemeToggler) => self.preview_theme_toggler(),
            Some(ComponentSlug::Globe) => self.preview_globe(),
            Some(ComponentSlug::IconCloud) => self.preview_icon_cloud(),
            Some(ComponentSlug::VideoText) => self.preview_video_text(),
            None => unsupported_preview(slug, &self.theme),
        };

        container(preview)
            .width(Length::Fill)
            .padding([20.0, 20.0])
            .style(card_shell_style(
                self.theme.palette.card,
                self.theme.palette.border,
                self.theme.radius.lg,
            ))
            .into()
    }

    pub(crate) fn code_panel<'a>(&self, doc: &'a RefDocPage) -> Element<'a, Message> {
        container(
            text(doc_primary_source(doc).unwrap_or_default())
                .font(mono_font())
                .size(14)
                .style(text_color(self.theme.palette.foreground)),
        )
        .width(Length::Fill)
        .padding([20.0, 20.0])
        .style(card_shell_style(
            self.theme.palette.card,
            self.theme.palette.border,
            self.theme.radius.lg,
        ))
        .into()
    }

    pub(crate) fn preview_marquee(&self) -> Element<'_, Message> {
        let render_item = |item: &MarqueeItem| {
            container(
                iced::widget::column![
                    text(item.title.clone())
                        .font(display_font())
                        .size(16)
                        .style(text_color(self.theme.palette.foreground)),
                    text(item.subtitle.clone())
                        .font(body_font())
                        .size(12)
                        .style(text_color(self.theme.palette.muted_foreground)),
                ]
                .spacing(4),
            )
            .width(Length::Fixed(260.0))
            .height(Length::Fixed(112.0))
            .padding([14.0, 16.0])
            .style(card_shell_style(
                self.theme.palette.card,
                self.theme.palette.border,
                self.theme.radius.lg,
            ))
            .into()
        };

        marquee(
            &self.showcase_items,
            render_item,
            self.elapsed,
            MarqueeProps {
                item_extent: 260.0,
                item_cross: 120.0,
                ..MarqueeProps::new()
            },
            &self.theme,
            |_| Message::Noop,
        )
    }

    pub(crate) fn preview_marquee_vertical(&self) -> Element<'_, Message> {
        let render_item = |item: &MarqueeItem| {
            container(
                iced::widget::column![
                    text(item.title.clone())
                        .font(display_font())
                        .size(16)
                        .style(text_color(self.theme.palette.foreground)),
                    text(item.subtitle.clone())
                        .font(body_font())
                        .size(12)
                        .style(text_color(self.theme.palette.muted_foreground)),
                ]
                .spacing(4),
            )
            .width(Length::Fixed(260.0))
            .height(Length::Fixed(112.0))
            .padding([14.0, 16.0])
            .style(card_shell_style(
                self.theme.palette.card,
                self.theme.palette.border,
                self.theme.radius.lg,
            ))
            .into()
        };

        marquee(
            &self.showcase_items,
            render_item,
            self.elapsed,
            MarqueeProps {
                orientation: MarqueeOrientation::Vertical,
                item_extent: 120.0,
                item_cross: 260.0,
                gap: 12.0,
                ..MarqueeProps::new()
            },
            &self.theme,
            |_| Message::Noop,
        )
    }

    pub(crate) fn preview_terminal(&self) -> Element<'_, Message> {
        terminal(
            &self.terminal_state,
            &self.terminal_lines,
            &self.terminal_props,
            &self.theme,
        )
    }

    pub(crate) fn preview_tweet_card(&self) -> Element<'_, Message> {
        tweet_card(&self.sample_tweets[0], &TweetCardProps::new(), &self.theme)
    }

    pub(crate) fn preview_tweet_card_images(&self) -> Element<'_, Message> {
        let mut data = self.sample_tweets[0].clone();
        data.media = Some(TweetMedia::Images(vec![
            TweetAsset {
                alt: "Showcase image one".into(),
                handle: Some(reference_image_handle(
                    &reference::data().home.showcase_cards[0].image,
                )),
                source_url: Some(reference::data().home.showcase_cards[0].href.clone()),
            },
            TweetAsset {
                alt: "Showcase image two".into(),
                handle: Some(reference_image_handle(
                    &reference::data().home.showcase_cards[1].image,
                )),
                source_url: Some(reference::data().home.showcase_cards[1].href.clone()),
            },
        ]));
        tweet_card(&data, &TweetCardProps::new(), &self.theme)
    }

    pub(crate) fn preview_tweet_card_meta(&self) -> Element<'_, Message> {
        let mut data = self.sample_tweets[1].clone();
        data.media = Some(TweetMedia::LinkPreview {
            title: "Magic UI".into(),
            site: "magicui.design".into(),
            thumbnail: Some(TweetAsset {
                alt: "Preview image".into(),
                handle: Some(reference_image_handle(
                    &reference::data().home.showcase_cards[2].image,
                )),
                source_url: Some(reference::data().home.showcase_cards[2].href.clone()),
            }),
            url: "https://magicui.design".into(),
        });
        tweet_card(&data, &TweetCardProps::new(), &self.theme)
    }

    pub(crate) fn preview_word_rotate(&self) -> Element<'_, Message> {
        centered(word_rotate(
            &self.word_rotate_state,
            Instant::now(),
            &self.word_rotate_props,
            &self.theme,
        ))
    }

    pub(crate) fn preview_sparkles_text(&self) -> Element<'_, Message> {
        centered(sparkles_text(
            "Magic UI",
            self.elapsed,
            SparklesTextProps::new(),
            &self.theme,
        ))
    }

    pub(crate) fn preview_gradient_text(&self) -> Element<'_, Message> {
        centered(animated_gradient_text(
            "premium motion",
            self.elapsed,
            AnimatedGradientTextProps::new().font_size(28),
        ))
    }

    pub(crate) fn preview_number_ticker(&self) -> Element<'_, Message> {
        centered(number_ticker(
            &self.number_ticker_state,
            Instant::now(),
            &self.number_ticker_props,
            &self.theme,
        ))
    }

    pub(crate) fn preview_text_reveal(&self) -> Element<'_, Message> {
        centered(text_reveal(
            "Beautifully designed landing page components.",
            TextRevealProps::new()
                .progress((self.elapsed.as_secs_f32() * 0.22).sin() * 0.5 + 0.5)
                .font_size(22),
            &self.theme,
        ))
    }

    pub(crate) fn preview_shimmer_button(&self) -> Element<'_, Message> {
        centered(shimmer_button(
            text("Browse Components").font(body_font()).size(16),
            Some(Message::Noop),
            self.elapsed,
            ShimmerButtonProps::new().button(
                ButtonProps::new()
                    .variant(ButtonVariant::Default)
                    .size(ButtonSize::Size3),
            ),
            &self.theme,
        ))
    }

    pub(crate) fn preview_magic_card(&self) -> Element<'_, Message> {
        magic_card(
            iced::widget::column![
                text("Native exactness")
                    .font(display_font())
                    .size(24)
                    .style(text_color(self.theme.palette.foreground)),
                text("Rainbow borders, controlled glow, and card hierarchy that mirrors the reference shell.")
                    .font(body_font())
                    .size(14)
                    .style(text_color(self.theme.palette.muted_foreground)),
            ]
            .spacing(8),
            MagicCardProps::new()
                .width(Length::Fill)
                .height(Length::Fixed(180.0)),
            &self.theme,
        )
    }

    pub(crate) fn preview_dock(&self) -> Element<'_, Message> {
        self.preview_dock_variant(DockDirection::Middle)
    }

    pub(crate) fn preview_dock_variant(&self, direction: DockDirection) -> Element<'_, Message> {
        let now = self.origin + self.elapsed;
        let theme = self.theme_toggler.blended_theme(now);
        let foreground = theme.palette.foreground;
        let items = vec![
            dock_icon(lucide(LucideIcon::House, 18, foreground)),
            dock_icon(lucide(LucideIcon::Code, 18, foreground)),
            dock_icon(lucide(LucideIcon::Box, 18, foreground)),
            dock_icon(lucide(LucideIcon::Sparkles, 18, foreground)),
        ];

        let props = DockProps {
            direction,
            ..DockProps::new()
        };
        centered(dock(
            items,
            &self.dock_state,
            props,
            Message::Dock,
            &theme,
        ))
    }

    pub(crate) fn preview_theme_toggler(&self) -> Element<'_, Message> {
        let props = theme_toggler_props();
        let now = Instant::now();
        let display_theme = self.theme_toggler.display_theme(now);
        let icon_theme = self.theme_toggler.icon_theme();
        let toggler = animated_theme_toggler_with_controller(
            &self.theme_toggler,
            now,
            Message::ToggleTheme,
            props,
            &icon_theme,
        );

        let background = container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(display_theme.palette.background)),
                ..iced::widget::container::Style::default()
            });

        let mut layers = vec![background.into()];
        if let Some(overlay) = theme_transition_overlay(&self.theme_toggler, now, &props) {
            layers.push(overlay);
        }
        layers.push(centered(toggler).into());

        container(
            stack(layers)
                .width(Length::Fill)
                .height(Length::Fixed(160.0)),
        )
        .width(Length::Fill)
        .height(Length::Fixed(160.0))
        .into()
    }

    pub(crate) fn preview_globe(&self) -> Element<'_, Message> {
        let now = Instant::now();
        let colors = blended_globe_demo_colors(&self.theme_toggler, now);
        globe_demo(
            &self.globe_state,
            &self.globe_props,
            colors,
            Message::Globe,
        )
    }

    pub(crate) fn preview_icon_cloud(&self) -> Element<'_, Message> {
        icon_cloud(
            &self.icon_cloud_state,
            &self.icon_cloud_props,
            Message::IconCloud,
        )
    }

    pub(crate) fn preview_video_text(&self) -> Element<'_, Message> {
        video_text(
            &self.video_text_state,
            Instant::now(),
            &self.video_text_props,
        )
    }

    pub(crate) fn preview_video_text_demo(&self) -> Element<'_, Message> {
        video_text(
            &self.video_text_state,
            Instant::now(),
            video_text_demo_props(),
        )
    }

    pub(crate) fn preview_terminal_alt(&self) -> Element<'_, Message> {
        terminal(
            &self.terminal_state,
            terminal_lines_alt(),
            terminal_props_alt(),
            &self.theme,
        )
    }
}

pub(crate) fn showcase_items() -> Vec<MarqueeItem> {
    vec![
        MarqueeItem::new("SaaS Hero").subtitle("Next.js landing"),
        MarqueeItem::new("Design System").subtitle("High-craft motion"),
        MarqueeItem::new("Launch Page").subtitle("Conversion-focused blocks"),
        MarqueeItem::new("AI Product").subtitle("Premium visual polish"),
        MarqueeItem::new("Portfolio").subtitle("Interactive storytelling"),
    ]
}

pub(crate) fn sample_tweets(ids: &[String]) -> Vec<TweetCardData> {
    const AUTHORS: &[(&str, &str)] = &[
        ("Steven Tey", "steventey"),
        ("Guillermo Rauch", "rauchg"),
        ("McKay Wrigley", "mckaywrigley"),
        ("Chronark", "chronark_"),
        ("Aiden Bai", "aidenybai"),
        ("Whop", "WhopIO"),
        ("Design Sumu", "DesignSumu"),
        ("Jordan Hughes", "jordanphughes"),
    ];
    const MESSAGES: &[&str] = &[
        "Magic UI keeps setting the bar for motion-heavy product sites. The shell, docs, and component rhythm all feel premium.",
        "The component docs are the killer feature: clear preview/code switching, strong hierarchy, and production-ready examples.",
        "Great design systems feel inevitable. Magic UI gets there with sharp spacing, convincing motion, and excellent landing-page polish.",
        "This is the kind of UI library that makes people ship prettier products faster without spending weeks rebuilding common patterns.",
        "Landing pages, launch screens, and docs all keep the same visual confidence. That's much harder than a few isolated components.",
        "If you care about interaction polish, this library is full of high-signal examples worth studying and reusing.",
    ];

    ids.iter()
        .take(18)
        .enumerate()
        .map(|(index, id)| {
            let (name, handle) = AUTHORS[index % AUTHORS.len()];
            let text = MESSAGES[index % MESSAGES.len()];
            let media = (index % 4 == 0).then(|| TweetMedia::LinkPreview {
                title: "Magic UI".into(),
                site: "magicui.design".into(),
                thumbnail: None,
                url: "https://magicui.design".into(),
            });

            TweetCardData {
                id: id.clone(),
                text: text.into(),
                author: TweetAuthor {
                    name: name.into(),
                    handle: handle.into(),
                    avatar: None,
                    verified: true,
                },
                url: Some(format!("https://x.com/i/status/{id}")),
                timestamp: Some(format!(
                    "{:02}:{} PM",
                    (index % 11) + 1,
                    10 + (index % 5) * 7
                )),
                media,
            }
        })
        .collect()
}

pub(crate) fn terminal_props() -> TerminalProps {
    TerminalProps::new()
}

pub(crate) fn terminal_props_alt() -> &'static TerminalProps {
    static PROPS: OnceLock<TerminalProps> = OnceLock::new();
    PROPS.get_or_init(terminal_props)
}

pub(crate) fn terminal_lines() -> Vec<TerminalLine> {
    vec![
        TerminalLine::typing(vec![TerminalSegment::new("cargo run -p wasm-demo")]),
        TerminalLine::instant(vec![TerminalSegment::new("✔ Marketing shell loaded.")]),
        TerminalLine::instant(vec![TerminalSegment::new("✔ Docs routes mounted.")]),
        TerminalLine::typing(vec![TerminalSegment::new("Success! Native preview ready.")]),
    ]
}

pub(crate) fn terminal_lines_alt() -> &'static [TerminalLine] {
    static LINES: OnceLock<Vec<TerminalLine>> = OnceLock::new();
    LINES.get_or_init(|| {
        vec![
            TerminalLine::instant(vec![TerminalSegment::new("$ pnpm dlx shadcn@latest add")]),
            TerminalLine::instant(vec![
                TerminalSegment::new("✔ "),
                TerminalSegment::new("@magicui/terminal").color(Color::from_rgb8(59, 130, 246)),
            ]),
            TerminalLine::instant(vec![
                TerminalSegment::new("Success. "),
                TerminalSegment::new("1 file updated").color(Color::from_rgb8(113, 113, 122)),
            ]),
        ]
    })
}

pub(crate) fn word_rotate_props() -> WordRotateProps {
    WordRotateProps::new(["faster", "clearer", "native"]).font_size(34)
}

pub(crate) fn number_ticker_props() -> NumberTickerProps {
    NumberTickerProps::new(150.0)
        .start_value(0.0)
        .suffix("+")
        .font_size(44)
        .motion(
            magic_iced::MotionSpec::new()
                .duration(TransitionDuration::Ms1000)
                .easing(Easing::Out),
        )
}

pub(crate) fn globe_props() -> GlobeProps {
    GlobeProps::new()
}

pub(crate) fn icon_cloud_props() -> IconCloudProps {
    let mut props = IconCloudProps::new();
    props.items = vec![
        IconCloudItem::Text("React".into()),
        IconCloudItem::Text("Rust".into()),
        IconCloudItem::Text("Iced".into()),
        IconCloudItem::Text("Twill".into()),
        IconCloudItem::Text("Lucide".into()),
        IconCloudItem::Text("WASM".into()),
        IconCloudItem::Text("Docs".into()),
        IconCloudItem::Text("Magic".into()),
    ];
    props
}

pub(crate) fn theme_toggler_props() -> AnimatedThemeTogglerProps {
    AnimatedThemeTogglerProps::new()
}

pub(crate) fn video_text_props() -> VideoTextProps {
    let frames = FrameSequence::new(
        vec![
            solid_rgba([30, 41, 59, 255]),
            solid_rgba([37, 99, 235, 255]),
            solid_rgba([99, 102, 241, 255]),
            solid_rgba([249, 115, 22, 255]),
        ],
        220,
    );

    VideoTextProps::new(VideoTextSource::Frames(frames), "MAGIC")
}

pub(crate) fn video_text_demo_props() -> &'static VideoTextProps {
    static PROPS: OnceLock<VideoTextProps> = OnceLock::new();
    PROPS.get_or_init(|| {
        VideoTextProps::new(
            VideoTextSource::Image(reference_image_handle(
                &reference::data().home.showcase_cards[0].image,
            )),
            "MAGIC",
        )
    })
}

pub(crate) fn solid_rgba(color: [u8; 4]) -> image::Handle {
    image::Handle::from_rgba(1, 1, color.to_vec())
}
