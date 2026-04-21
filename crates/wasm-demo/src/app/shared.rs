use super::*;

pub(crate) fn centered_heading<'a>(
    title: &'a str,
    subtitle: &'a str,
    theme: &'a ShadTheme,
) -> Element<'a, Message> {
    let mut content = iced::widget::column![
        text(title)
            .font(display_font())
            .size(if subtitle.is_empty() { 44 } else { 48 })
            .style(text_color(theme.palette.foreground)),
    ]
    .align_x(Alignment::Center);

    if !subtitle.is_empty() {
        content = content.push(
            text(subtitle)
                .font(body_font())
                .size(18)
                .style(text_color(theme.palette.muted_foreground)),
        );
    }

    container(content.spacing(if subtitle.is_empty() { 0.0 } else { 8.0 }))
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into()
}

pub(crate) fn tech_stack<'a>(items: &'a [String]) -> Element<'a, Message> {
    let icon_for = |label: &str| match label {
        "react" => LucideIcon::Boxes,
        "typescript" => LucideIcon::Braces,
        "tailwindcss" => LucideIcon::SwatchBook,
        "motion" => LucideIcon::Sparkles,
        "shadcn" => LucideIcon::Shapes,
        _ => LucideIcon::Box,
    };

    let mut row_view = row![];
    for label in items {
        row_view = row_view.push(
            container(lucide(icon_for(label), 20, rgba([24, 24, 27, 255]))).padding([2.0, 2.0]),
        );
    }

    row_view.spacing(12).align_y(Alignment::Center).into()
}

pub(crate) fn showcase_track<'a>(
    cards: &'a [RefShowcaseCard],
    _elapsed: Duration,
    mobile: bool,
    theme: &'a ShadTheme,
) -> Element<'a, Message> {
    if cards.is_empty() {
        return Space::new().height(Length::Fixed(0.0)).into();
    }

    let card_width = if mobile { 224.0 } else { 332.0 };
    let preview_height = if mobile { 144.0 } else { 200.0 };
    let gap = if mobile { 16.0 } else { 20.0 };
    let _speed_px_per_sec = if mobile { 28.0 } else { 34.0 };
    let _track_extent = cards.len() as f32 * (card_width + gap);
    let offset = 0.0;

    let repeated = (0..3)
        .flat_map(|_| {
            cards.iter().map(|card| {
                showcase_reference_card(card, mobile, card_width, preview_height, theme)
            })
        })
        .collect::<Vec<Element<'_, Message>>>();
    let track = float(row(repeated).spacing(gap).align_y(Alignment::Center))
        .translate(move |_bounds, _viewport| Vector::new(offset, 0.0));
    let frame = container(track)
        .width(Length::Fill)
        .height(Length::Fixed(preview_height + if mobile { 84.0 } else { 92.0 }))
        .clip(true);

    stack(vec![frame.into(), showcase_edge_fades(mobile, theme)]).into()
}

pub(crate) fn showcase_reference_card<'a>(
    card_data: &'a RefShowcaseCard,
    mobile: bool,
    card_width: f32,
    preview_height: f32,
    theme: &'a ShadTheme,
) -> Element<'a, Message> {
    let preview = container(
        image(reference_image_handle(&card_data.image))
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fixed(card_width))
    .height(Length::Fixed(preview_height))
    .clip(true)
    .style(card_shell_style(
        theme.palette.card,
        theme.palette.border,
        theme.radius.lg,
    ));

    let content = container(
        iced::widget::column![
            preview,
            row![
                text(card_data.title.as_str())
                    .font(display_font())
                    .size(if mobile { 16 } else { 18 })
                    .style(text_color(theme.palette.foreground)),
                lucide(LucideIcon::ChevronRight, 16, theme.palette.muted_foreground),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
            text(card_data.affiliation.as_deref().unwrap_or_default())
                .font(body_font())
                .size(14)
                .style(text_color(theme.palette.muted_foreground)),
        ]
        .spacing(8),
    )
    .width(Length::Fixed(card_width));

    button_content(
        content,
        Some(Message::OpenUrl(card_data.href.clone())),
        ButtonProps::new()
            .variant(ButtonVariant::Ghost)
            .size(ButtonSize::Size1),
        theme,
    )
    .into()
}

pub(crate) fn showcase_edge_fades<'a>(mobile: bool, theme: &'a ShadTheme) -> Element<'a, Message> {
    let fade_size = if mobile { 48.0 } else { 88.0 };
    let background = theme.palette.background;
    let left = container(Space::new())
        .width(Length::Fixed(fade_size))
        .height(Length::Fill)
        .style(move |_| {
            gradient::Linear::new(Radians(0.0))
                .add_stop(0.0, background)
                .add_stop(
                    1.0,
                    Color {
                        a: 0.0,
                        ..background
                    },
                )
                .into()
        });
    let right = container(Space::new())
        .width(Length::Fixed(fade_size))
        .height(Length::Fill)
        .style(move |_| {
            gradient::Linear::new(Radians(std::f32::consts::PI))
                .add_stop(0.0, background)
                .add_stop(
                    1.0,
                    Color {
                        a: 0.0,
                        ..background
                    },
                )
                .into()
        });

    row![left, Space::new().width(Length::Fill), right]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

pub(crate) fn video_thumbnail_tile<'a>(
    embed_url: &'a str,
    height: f32,
    _large: bool,
    theme: &'a ShadTheme,
) -> Element<'a, Message> {
    let state = magic_iced::HeroVideoDialogState::new();
    let props = HeroVideoDialogProps {
        thumbnail_radius: 12.0,
        show_close_button: false,
        ..HeroVideoDialogProps::new()
    };
    let thumbnail = container(
        image(reference_image_handle(&youtube_thumbnail_url(embed_url)))
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fixed(height))
    .clip(true);

    hero_video_dialog(
        &state,
        Instant::now(),
        thumbnail,
        Space::new().width(Length::Shrink),
        Message::OpenUrl(youtube_watch_url(embed_url)),
        Message::Noop,
        props,
        theme,
    )
}

pub(crate) fn badge<'a>(
    label: &'a str,
    background: Color,
    foreground: Color,
) -> Element<'a, Message> {
    container(text(label).font(body_font()).size(12))
        .padding([4.0, 8.0])
        .style(badge_style(background, foreground))
        .into()
}

pub(crate) fn tab_button(
    theme: &ShadTheme,
    label: &'static str,
    active: bool,
    message: Message,
) -> Element<'static, Message> {
    button(
        label,
        Some(message),
        ButtonProps::new()
            .variant(if active {
                ButtonVariant::Secondary
            } else {
                ButtonVariant::Ghost
            })
            .size(ButtonSize::Size1),
        theme,
    )
    .into()
}

pub(crate) fn centered<'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

pub(crate) fn badge_style(
    background: Color,
    foreground: Color,
) -> impl Fn(&IcedTheme) -> iced::widget::container::Style + Clone {
    move |_| iced::widget::container::Style {
        background: Some(Background::Color(background)),
        text_color: Some(foreground),
        border: Border {
            color: Color {
                a: 0.12,
                ..foreground
            },
            width: 1.0,
            radius: BorderRadius::from(999.0),
        },
        ..iced::widget::container::Style::default()
    }
}

pub(crate) fn card_shell_style(
    background: Color,
    border: Color,
    radius: f32,
) -> impl Fn(&IcedTheme) -> iced::widget::container::Style + Clone {
    move |_| iced::widget::container::Style {
        background: Some(Background::Color(background)),
        border: Border {
            color: border,
            width: 1.0,
            radius: BorderRadius::from(radius),
        },
        ..iced::widget::container::Style::default()
    }
}

pub(crate) fn text_color(color: Color) -> impl Fn(&IcedTheme) -> iced::widget::text::Style + Clone {
    move |_| iced::widget::text::Style { color: Some(color) }
}

pub(crate) fn display_font() -> Font {
    Font::with_name("Geist")
}

pub(crate) fn body_font() -> Font {
    Font::with_name("Geist")
}

pub(crate) fn mono_font() -> Font {
    Font::with_name("Geist Mono")
}

pub(crate) fn lucide<'a>(icon: LucideIcon, size: u16, color: Color) -> Element<'a, Message> {
    text(char::from(icon).to_string())
        .font(Font::with_name("lucide"))
        .size(u32::from(size))
        .style(text_color(color))
        .into()
}

pub(crate) fn reference_doc(slug: &str) -> Option<&'static RefDocPage> {
    reference::data().pages.iter().find(|page| page.slug == slug)
}

pub(crate) fn is_supported_doc_slug(slug: &str) -> bool {
    preview_slug(slug).is_some() || slug == "index"
}

pub(crate) fn component_slug_from_href(href: &str) -> Option<&str> {
    href.strip_prefix("/docs/components/")
}

pub(crate) fn message_for_href(href: &str) -> Message {
    if href == "/" || href == "/docs" {
        Message::Navigate(Route::Home)
    } else if href == "/components" {
        Message::Navigate(Route::Components)
    } else if let Some(slug) = component_slug_from_href(href) {
        if is_supported_doc_slug(slug) && reference_doc(slug).is_some() {
            Message::Navigate(Route::Component(slug.to_owned()))
        } else {
            Message::OpenUrl(format!("https://magicui.design{href}"))
        }
    } else if href.starts_with("http") {
        Message::OpenUrl(href.to_owned())
    } else {
        Message::OpenUrl(format!("https://magicui.design{href}"))
    }
}

pub(crate) fn doc_primary_source(doc: &RefDocPage) -> Option<&str> {
    doc.blocks.iter().find_map(|block| match block {
        BodyBlock::SourceRef { code: Some(code), .. } => Some(code.as_str()),
        BodyBlock::Tabs { items, .. } => items.iter().find_map(|item| {
            item.blocks.iter().find_map(|block| match block {
                BodyBlock::SourceRef { code: Some(code), .. } => Some(code.as_str()),
                _ => None,
            })
        }),
        _ => None,
    })
}

pub(crate) fn preview_slug(slug: &str) -> Option<ComponentSlug> {
    match slug {
        "marquee" => Some(ComponentSlug::Marquee),
        "terminal" => Some(ComponentSlug::Terminal),
        "tweet-card" => Some(ComponentSlug::TweetCard),
        "word-rotate" => Some(ComponentSlug::WordRotate),
        "sparkles-text" => Some(ComponentSlug::SparklesText),
        "animated-gradient-text" => Some(ComponentSlug::AnimatedGradientText),
        "number-ticker" => Some(ComponentSlug::NumberTicker),
        "text-reveal" => Some(ComponentSlug::TextReveal),
        "shimmer-button" => Some(ComponentSlug::ShimmerButton),
        "magic-card" => Some(ComponentSlug::MagicCard),
        "dock" => Some(ComponentSlug::Dock),
        "animated-theme-toggler" => Some(ComponentSlug::ThemeToggler),
        "globe" => Some(ComponentSlug::Globe),
        "icon-cloud" => Some(ComponentSlug::IconCloud),
        "video-text" => Some(ComponentSlug::VideoText),
        _ => None,
    }
}

pub(crate) fn preview_slug_from_name(name: &str) -> Option<&str> {
    const PREFIXES: &[(&str, &str)] = &[
        ("marquee", "marquee"),
        ("terminal", "terminal"),
        ("tweet-card", "tweet-card"),
        ("word-rotate", "word-rotate"),
        ("sparkles-text", "sparkles-text"),
        ("animated-gradient-text", "animated-gradient-text"),
        ("number-ticker", "number-ticker"),
        ("text-reveal", "text-reveal"),
        ("shimmer-button", "shimmer-button"),
        ("magic-card", "magic-card"),
        ("dock", "dock"),
        ("animated-theme-toggler", "animated-theme-toggler"),
        ("globe", "globe"),
        ("icon-cloud", "icon-cloud"),
        ("video-text", "video-text"),
        ("bento", "magic-card"),
        ("hero-video-dialog", "video-text"),
    ];

    PREFIXES
        .iter()
        .find_map(|(prefix, slug)| name.starts_with(prefix).then_some(*slug))
}

pub(crate) fn unsupported_preview<'a>(slug: &str, theme: &'a ShadTheme) -> Element<'a, Message> {
    container(
        iced::widget::column![
            text("Native preview coverage is still being expanded.")
                .font(display_font())
                .size(18)
                .style(text_color(theme.palette.foreground)),
            text(format!("Reference slug: {slug}"))
                .font(body_font())
                .size(14)
                .style(text_color(theme.palette.muted_foreground)),
        ]
        .spacing(10),
    )
    .padding([20.0, 20.0])
    .style(card_shell_style(
        theme.palette.card,
        theme.palette.border,
        theme.radius.lg,
    ))
    .into()
}

pub(crate) fn rgba(color: [u8; 4]) -> Color {
    Color::from_rgba8(color[0], color[1], color[2], f32::from(color[3]) / 255.0)
}

pub(crate) fn reference_image_handle(path: &str) -> image::Handle {
    generated_reference_assets::handle(path)
        .unwrap_or_else(|| image::Handle::from_path(path.to_owned()))
}

pub(crate) fn clone_sidebar_sections(
    sections: &[reference::SidebarSection],
) -> Vec<reference::SidebarSection> {
    sections.to_vec()
}

pub(crate) fn catalog_sidebar_items() -> Vec<&'static reference::SidebarItem> {
    let Some(index) = reference_doc("index") else {
        return Vec::new();
    };

    index
        .sidebar
        .iter()
        .flat_map(|section| section.items.iter())
        .filter(|item| component_slug_from_href(&item.href).is_some())
        .collect()
}

pub(crate) fn youtube_video_id(embed_url: &str) -> &str {
    embed_url
        .trim_end_matches('/')
        .split('/')
        .next_back()
        .unwrap_or_default()
}

pub(crate) fn youtube_thumbnail_url(embed_url: &str) -> String {
    format!(
        "reference-assets/youtube-{}.jpg",
        youtube_video_id(embed_url)
    )
}

pub(crate) fn youtube_watch_url(embed_url: &str) -> String {
    format!(
        "https://www.youtube.com/watch?v={}",
        youtube_video_id(embed_url)
    )
}

pub(crate) fn supported_reference_pages() -> Vec<&'static RefDocPage> {
    reference::data()
        .pages
        .iter()
        .filter(|page| preview_slug(&page.slug).is_some() || page.slug == "index")
        .collect()
}

pub(crate) fn supported_doc_neighbors(
    slug: &str,
) -> (Option<&'static RefDocPage>, Option<&'static RefDocPage>) {
    let pages = supported_reference_pages();
    let Some(index) = pages.iter().position(|page| page.slug == slug) else {
        return (None, None);
    };

    (
        index.checked_sub(1).and_then(|idx| pages.get(idx).copied()),
        pages.get(index + 1).copied(),
    )
}

pub(crate) fn initial_theme_mode() -> ThemeMode {
    match initial_query_param("theme").as_deref() {
        Some("light") => ThemeMode::Light,
        Some("dark") => ThemeMode::Dark,
        _ => ThemeMode::Dark,
    }
}

pub(crate) fn initial_preview_tab() -> PreviewTab {
    match initial_query_param("tab").as_deref() {
        Some("code") => PreviewTab::Code,
        _ => PreviewTab::Preview,
    }
}

pub(crate) fn initial_query_param(key: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window()?;
        let search = window.location().search().ok()?;
        let params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
        params.get(key)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = key;
        None
    }
}

pub(crate) fn scroll_window_to_top() {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            window.scroll_to_with_x_and_y(0.0, 0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_for_href_maps_internal_links() {
        assert!(matches!(message_for_href("/"), Message::Navigate(Route::Home)));
        assert!(matches!(
            message_for_href("/components"),
            Message::Navigate(Route::Components)
        ));
        assert!(matches!(
            message_for_href("/docs/components/marquee"),
            Message::Navigate(Route::Component(slug)) if slug == "marquee"
        ));
    }

    #[test]
    fn message_for_href_leaves_external_links_alone() {
        assert!(matches!(
            message_for_href("https://example.com"),
            Message::OpenUrl(url) if url == "https://example.com"
        ));
    }

    #[test]
    fn preview_slug_from_name_handles_aliases() {
        assert_eq!(preview_slug_from_name("hero-video-dialog"), Some("video-text"));
        assert_eq!(preview_slug_from_name("bento-card"), Some("magic-card"));
    }

    #[test]
    fn youtube_helpers_use_last_path_segment() {
        let embed = "https://www.youtube.com/embed/dQw4w9WgXcQ";
        assert_eq!(youtube_video_id(embed), "dQw4w9WgXcQ");
        assert_eq!(
            youtube_thumbnail_url(embed),
            "reference-assets/youtube-dQw4w9WgXcQ.jpg"
        );
        assert_eq!(
            youtube_watch_url(embed),
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        );
    }
}
