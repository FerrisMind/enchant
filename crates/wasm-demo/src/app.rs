use crate::catalog::ComponentSlug;
use crate::reference::{self, BodyBlock, DocPage as RefDocPage, ShowcaseCard as RefShowcaseCard};
use crate::route::{self, Route};
use iced::border::Radius as BorderRadius;
use iced::time::{Duration, Instant};
use iced::widget::image;
use iced::widget::{Space, column, container, float, row, scrollable, stack, text};
use iced::{
    Alignment, Background, Border, Color, Element, Font, Length, Radians, Subscription, Task,
    Theme as IcedTheme, Vector, gradient,
};
use iced_shadcn::{
    ButtonProps, ButtonSize, ButtonVariant, InputProps, InputSize, InputVariant, Palette, Radius,
    Spacing, Theme as ShadTheme, button, button_content, input,
};
use lucide_icons::Icon as LucideIcon;
use magic_iced::{
    AnimatedGradientTextProps, AnimatedThemeTogglerController, AnimatedThemeTogglerProps,
    DockAction, DockDirection, DockProps, DockState, FrameDriver, FrameSequence, GlobeAction,
    GlobeProps, GlobeState, HeroVideoDialogProps, IconCloudAction, IconCloudItem, IconCloudProps,
    IconCloudState, MagicCardProps, MagicTheme, MarqueeItem, MarqueeOrientation, MarqueeProps,
    NumberTickerProps, NumberTickerState, ShimmerButtonProps, SparklesTextProps, TerminalLine,
    TerminalProps, TerminalSegment, TerminalState, TextRevealProps, TweetAsset, TweetAuthor,
    TweetCardData, TweetCardProps, TweetMedia, VideoTextProps, VideoTextSource, VideoTextState,
    WordRotateProps, WordRotateState, animated_gradient_text,
    animated_theme_toggler_with_controller, dock, dock_icon, globe_demo, hero_video_dialog,
    icon_cloud, magic_card, marquee, number_ticker, shimmer_button, sparkles_text, terminal, text_reveal,
    theme_transition_overlay, tweet_card, video_text, word_rotate,
};
use previews::*;
pub(crate) use shared::*;
use std::sync::OnceLock;
use twill::prelude::{Easing, TransitionDuration};

mod docs;
mod home;
mod previews;
mod shared;
mod shell;

mod generated_reference_assets {
    include!(concat!(env!("OUT_DIR"), "/reference-assets.rs"));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewTab {
    Preview,
    Code,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ThemeMode {
    Light,
    Dark,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick(Instant),
    Navigate(Route),
    OpenUrl(String),
    ToggleMobileNav,
    ToggleTheme,
    SetPreviewTab(PreviewTab),
    ResetDetailDemo,
    SearchChanged(String),
    Dock(DockAction),
    Globe(GlobeAction),
    IconCloud(IconCloudAction),
    Noop,
}

pub struct WasmDemoApp {
    route: Route,
    route_revision: u64,
    preview_tab: PreviewTab,
    theme_mode: ThemeMode,
    theme: ShadTheme,
    driver: FrameDriver,
    origin: Instant,
    elapsed: Duration,
    window_width: f32,
    search: String,
    mobile_nav_open: bool,
    showcase_items: Vec<MarqueeItem>,
    sample_tweets: Vec<TweetCardData>,
    terminal_lines: Vec<TerminalLine>,
    terminal_props: TerminalProps,
    word_rotate_props: WordRotateProps,
    number_ticker_props: NumberTickerProps,
    globe_props: GlobeProps,
    icon_cloud_props: IconCloudProps,
    video_text_props: VideoTextProps,
    terminal_state: TerminalState,
    word_rotate_state: WordRotateState,
    number_ticker_state: NumberTickerState,
    dock_state: DockState,
    globe_state: GlobeState,
    icon_cloud_state: IconCloudState,
    theme_toggler: AnimatedThemeTogglerController,
    video_text_state: VideoTextState,
}

impl WasmDemoApp {
    pub fn boot() -> (Self, Task<Message>) {
        let route = route::read_initial_route();
        let theme_mode = initial_theme_mode();
        let theme = theme_for_mode(theme_mode);
        let toggler_props = theme_toggler_props();
        let showcase_items = showcase_items();
        let sample_tweets = sample_tweets(&reference::data().home.tweet_ids);
        let terminal_lines = terminal_lines();
        let terminal_props = terminal_props();
        let word_rotate_props = word_rotate_props();
        let number_ticker_props = number_ticker_props();
        let globe_props = globe_props();
        let icon_cloud_props = icon_cloud_props();
        let video_text_props = video_text_props();
        let terminal_state = TerminalState::new(&terminal_lines, &terminal_props);
        let word_rotate_state = WordRotateState::new(&word_rotate_props);
        let number_ticker_state = NumberTickerState::new(&number_ticker_props);
        let globe_state = GlobeState::new(&globe_props);
        let icon_cloud_state = IconCloudState::new(&icon_cloud_props);
        let app = Self {
            route,
            route_revision: 0,
            preview_tab: initial_preview_tab(),
            theme_mode,
            theme,
            driver: FrameDriver::new(),
            origin: Instant::now(),
            elapsed: Duration::ZERO,
            window_width: route::current_window_width().unwrap_or(1440.0),
            search: String::new(),
            mobile_nav_open: false,
            showcase_items,
            sample_tweets,
            terminal_lines,
            terminal_props,
            word_rotate_props,
            number_ticker_props,
            globe_props,
            icon_cloud_props,
            video_text_props,
            terminal_state,
            word_rotate_state,
            number_ticker_state,
            dock_state: DockState::new(),
            globe_state,
            icon_cloud_state,
            theme_toggler: AnimatedThemeTogglerController::new(true, &toggler_props),
            video_text_state: VideoTextState::new(),
        };
        route::sync_url(&app.route);
        scroll_window_to_top();
        (app, Task::none())
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick(now) => {
                self.elapsed = now.saturating_duration_since(self.origin);
                if let Some(width) = route::current_window_width() {
                    self.window_width = width;
                }
                self.terminal_state
                    .tick(now, &self.terminal_lines, &self.terminal_props);
                self.word_rotate_state.tick(now, &self.word_rotate_props);
                self.number_ticker_state
                    .tick(now, &self.number_ticker_props);
                self.theme_toggler.tick(now, &theme_toggler_props());
                self.theme = self.theme_toggler.display_theme(now);
                self.globe_state.tick(now, &self.globe_props);
                self.icon_cloud_state.tick(now, &self.icon_cloud_props);
                self.video_text_state.tick(now);
            }
            Message::Navigate(route) => {
                self.route = route;
                self.route_revision = self.route_revision.wrapping_add(1);
                self.preview_tab = PreviewTab::Preview;
                self.mobile_nav_open = false;
                route::sync_url(&self.route);
                scroll_window_to_top();
            }
            Message::OpenUrl(_url) => {
                self.mobile_nav_open = false;
                #[cfg(target_arch = "wasm32")]
                {
                    if let Some(window) = web_sys::window() {
                        let _ = window.open_with_url(&_url);
                    }
                }
            }
            Message::ToggleMobileNav => self.mobile_nav_open = !self.mobile_nav_open,
            Message::ToggleTheme => self.toggle_theme(),
            Message::SetPreviewTab(tab) => self.preview_tab = tab,
            Message::ResetDetailDemo => self.reset_demos(),
            Message::SearchChanged(value) => self.search = value,
            Message::Dock(action) => self.dock_state.apply_action(&action),
            Message::Globe(action) => self.globe_state.apply_action(&action, &self.globe_props),
            Message::IconCloud(action) => self
                .icon_cloud_state
                .apply_action(&action, &self.icon_cloud_props),
            Message::Noop => {}
        }

        Task::none()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        self.driver.subscription(Message::Tick)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content = match &self.route {
            Route::Home => self.home_page(),
            Route::Components => self.components_page(),
            Route::Component(slug) => self.component_detail(slug),
            Route::NotFound(path) => self.not_found_page(path),
        };

        let shell = column![
            self.site_banner(),
            self.site_header(),
            scrollable(content)
                .id(iced::widget::Id::new(Box::leak(
                    format!("app-scroll-{}-{}", self.route.path(), self.route_revision)
                        .into_boxed_str(),
                )))
                .width(Length::Fill)
                .height(Length::Fill),
            self.site_footer(),
        ]
        .width(Length::Fill)
        .height(Length::Fill);

        let shell: Element<'_, Message> = if self.mobile_nav_open && self.is_lt_lg() {
            stack(vec![shell.into(), self.mobile_nav_overlay()]).into()
        } else {
            shell.into()
        };

        container(shell)
            .width(Length::Fill)
            .height(Length::Fill)
            .style({
                let background = self.theme.palette.background;
                let foreground = self.theme.palette.foreground;
                move |_| iced::widget::container::Style {
                    background: Some(Background::Color(background)),
                    text_color: Some(foreground),
                    ..iced::widget::container::Style::default()
                }
            })
            .into()
    }

    fn toggle_theme(&mut self) {
        let now = Instant::now();
        self.theme_toggler.toggle(now, &theme_toggler_props());
        self.theme_mode = if self.theme_toggler.dark_mode() {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        };
    }

    fn reset_demos(&mut self) {
        self.origin = Instant::now();
        self.elapsed = Duration::ZERO;
        self.terminal_state = TerminalState::new(&self.terminal_lines, &self.terminal_props);
        self.word_rotate_state = WordRotateState::new(&self.word_rotate_props);
        self.number_ticker_state = NumberTickerState::new(&self.number_ticker_props);
        self.dock_state = DockState::new();
        self.globe_state = GlobeState::new(&self.globe_props);
        self.icon_cloud_state = IconCloudState::new(&self.icon_cloud_props);
        self.theme_toggler = AnimatedThemeTogglerController::new(
            matches!(self.theme_mode, ThemeMode::Dark),
            &theme_toggler_props(),
        );
        self.video_text_state = VideoTextState::new();
    }

    fn is_lt_md(&self) -> bool {
        self.window_width < 768.0
    }

    fn is_lt_lg(&self) -> bool {
        self.window_width < 1024.0
    }

    fn is_md_up(&self) -> bool {
        self.window_width >= 768.0
    }

    fn is_lg_up(&self) -> bool {
        self.window_width >= 1024.0
    }

    fn is_3xl_up(&self) -> bool {
        self.window_width >= 1600.0
    }

    fn shell_horizontal_padding(&self) -> f32 {
        if self.is_lg_up() {
            reference::data().layout.container_padding_lg
        } else {
            reference::data().layout.container_padding
        }
    }
}

fn theme_for_mode(mode: ThemeMode) -> ShadTheme {
    let reference_theme = &reference::data().theme;
    let palette_ref = match mode {
        ThemeMode::Light => &reference_theme.light,
        ThemeMode::Dark => &reference_theme.dark,
    };

    let mut theme = match mode {
        ThemeMode::Light => MagicTheme::light(),
        ThemeMode::Dark => MagicTheme::dark(),
    };

    theme.palette = Palette {
        background: rgba(palette_ref.background),
        foreground: rgba(palette_ref.foreground),
        card: rgba(palette_ref.card),
        card_foreground: rgba(palette_ref.card_foreground),
        popover: rgba(palette_ref.popover),
        popover_foreground: rgba(palette_ref.popover_foreground),
        border: rgba(palette_ref.border),
        input: rgba(palette_ref.input),
        ring: rgba(palette_ref.ring),
        primary: rgba(palette_ref.primary),
        primary_foreground: rgba(palette_ref.primary_foreground),
        secondary: rgba(palette_ref.secondary),
        secondary_foreground: rgba(palette_ref.secondary_foreground),
        accent: rgba(palette_ref.accent),
        accent_foreground: rgba(palette_ref.accent_foreground),
        muted: rgba(palette_ref.muted),
        muted_foreground: rgba(palette_ref.muted_foreground),
        destructive: rgba(palette_ref.destructive),
        destructive_foreground: rgba(palette_ref.destructive_foreground),
        chart_1: rgba(palette_ref.chart_1),
        chart_2: rgba(palette_ref.chart_2),
        chart_3: rgba(palette_ref.chart_3),
        chart_4: rgba(palette_ref.chart_4),
        chart_5: rgba(palette_ref.chart_5),
        sidebar: rgba(palette_ref.sidebar),
        sidebar_foreground: rgba(palette_ref.sidebar_foreground),
        sidebar_primary: rgba(palette_ref.sidebar_primary),
        sidebar_primary_foreground: rgba(palette_ref.sidebar_primary_foreground),
        sidebar_accent: rgba(palette_ref.sidebar_accent),
        sidebar_accent_foreground: rgba(palette_ref.sidebar_accent_foreground),
        sidebar_border: rgba(palette_ref.sidebar_border),
        sidebar_ring: rgba(palette_ref.sidebar_ring),
    };
    theme.radius = Radius {
        sm: reference_theme.radius.sm,
        md: reference_theme.radius.md,
        lg: reference_theme.radius.lg,
    };
    theme.spacing = Spacing {
        xs: reference_theme.spacing_unit,
        sm: reference_theme.spacing_unit * 2.0,
        md: reference_theme.spacing_unit * 3.0,
        lg: reference_theme.spacing_unit * 4.0,
    };

    theme
}
