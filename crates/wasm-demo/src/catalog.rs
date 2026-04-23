#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ComponentSlug {
    Marquee,
    Terminal,
    TweetCard,
    WordRotate,
    SparklesText,
    AnimatedGradientText,
    NumberTicker,
    TextReveal,
    ShimmerButton,
    MagicCard,
    Dock,
    ThemeToggler,
    Globe,
    IconCloud,
    VideoText,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComponentDoc {
    pub slug: ComponentSlug,
    pub title: &'static str,
    pub description: &'static str,
    pub group: &'static str,
    pub resettable: bool,
    pub source: &'static str,
}

pub const DOCS: &[ComponentDoc] = &[
    ComponentDoc {
        slug: ComponentSlug::Marquee,
        title: "Marquee",
        description: "An infinite scrolling strip for featured companies and polished landing-page proof points.",
        group: "Featured",
        resettable: false,
        source: r#"use magic_iced::{marquee, MarqueeItem, MarqueeProps};

let items = vec![
    MarqueeItem::new("Next.js landing").subtitle("featured client"),
    MarqueeItem::new("Design engineering").subtitle("motion systems"),
];

marquee(
    &items,
    |item| {
        /* render the item */
        iced::widget::text(&item.title).into()
    },
    elapsed,
    MarqueeProps::new(),
    theme,
    |_| Message::Noop,
)"#,
    },
    ComponentDoc {
        slug: ComponentSlug::Terminal,
        title: "Terminal",
        description: "MacOS-style terminal sequencing that reveals commands and status lines without DOM timing hooks.",
        group: "Featured",
        resettable: true,
        source: r#"use magic_iced::{
    terminal, TerminalLine, TerminalProps, TerminalSegment, TerminalState
};

let lines = vec![
    TerminalLine::typing(vec![TerminalSegment::new("pnpm dlx shadcn@latest init")]),
    TerminalLine::instant(vec![TerminalSegment::new("✔ Preflight checks.")]),
];

terminal(&state, &lines, &TerminalProps::new(), theme)"#,
    },
    ComponentDoc {
        slug: ComponentSlug::TweetCard,
        title: "Tweet Card",
        description: "A native social proof card with author metadata, media blocks, and visual weight similar to the original site.",
        group: "Featured",
        resettable: false,
        source: r#"use magic_iced::{tweet_card, TweetCardData, TweetCardProps};

let data = TweetCardData { /* sample data */ };

tweet_card(&data, &TweetCardProps::new(), theme)"#,
    },
    ComponentDoc {
        slug: ComponentSlug::WordRotate,
        title: "Word Rotate",
        description: "Animated headline swapping with typed state and explicit motion tokens.",
        group: "Text Motion",
        resettable: true,
        source: r#"use magic_iced::{word_rotate, WordRotateProps, WordRotateState};

let props = WordRotateProps::new(["faster", "clearer", "native"]);
word_rotate(&state, now, &props, theme)"#,
    },
    ComponentDoc {
        slug: ComponentSlug::SparklesText,
        title: "Sparkles Text",
        description: "Canvas sparkles over headline typography using deterministic noise instead of browser particles.",
        group: "Text Motion",
        resettable: false,
        source: r#"use magic_iced::{sparkles_text, SparklesTextProps};

sparkles_text("Magic UI for iced", elapsed, SparklesTextProps::new(), theme)"#,
    },
    ComponentDoc {
        slug: ComponentSlug::AnimatedGradientText,
        title: "Animated Gradient Text",
        description: "Moving color band typography with grapheme-safe segmentation.",
        group: "Text Motion",
        resettable: false,
        source: r#"use magic_iced::{animated_gradient_text, AnimatedGradientTextProps};

animated_gradient_text("premium motion", elapsed, AnimatedGradientTextProps::new())"#,
    },
    ComponentDoc {
        slug: ComponentSlug::NumberTicker,
        title: "Number Ticker",
        description: "Value transitions with explicit start, direction, and timing controls.",
        group: "Text Motion",
        resettable: true,
        source: r#"use magic_iced::{number_ticker, NumberTickerProps, NumberTickerState};

let props = NumberTickerProps::new(150.0).suffix("+");
number_ticker(&state, now, &props, theme)"#,
    },
    ComponentDoc {
        slug: ComponentSlug::TextReveal,
        title: "Text Reveal",
        description: "Word-by-word reveal driven by progress rather than browser intersection observers.",
        group: "Text Motion",
        resettable: false,
        source: r#"use magic_iced::{text_reveal, TextRevealProps};

text_reveal(
    "Beautifully designed landing page components.",
    TextRevealProps::new().progress(0.72),
    theme,
)"#,
    },
    ComponentDoc {
        slug: ComponentSlug::ShimmerButton,
        title: "Shimmer Button",
        description: "Primary CTA surface with a travelling highlight rendered natively in iced.",
        group: "Interaction",
        resettable: false,
        source: r#"use magic_iced::{shimmer_button, ShimmerButtonProps};

shimmer_button(
    "Browse Components",
    Some(Message::Noop),
    elapsed,
    ShimmerButtonProps::new(),
    theme,
)"#,
    },
    ComponentDoc {
        slug: ComponentSlug::MagicCard,
        title: "Magic Card",
        description: "Gradient-border premium surface with glow, overlay, and composable inner content.",
        group: "Interaction",
        resettable: false,
        source: r#"use magic_iced::{magic_card, MagicCardProps};

magic_card(content, MagicCardProps::new().width(420.0).height(260.0), theme)"#,
    },
    ComponentDoc {
        slug: ComponentSlug::Dock,
        title: "Dock",
        description: "Pointer-reactive icon dock that scales items with distance-based magnification.",
        group: "Interaction",
        resettable: true,
        source: r#"use magic_iced::{dock, dock_icon, DockProps, DockState};

dock(items, &state, DockProps::new(), Message::Dock, theme)"#,
    },
    ComponentDoc {
        slug: ComponentSlug::ThemeToggler,
        title: "Animated Theme Toggler",
        description: "A controlled light switch that drives the same theme palette used by the demo shell.",
        group: "Interaction",
        resettable: true,
        source: r#"use magic_iced::{
    animated_theme_toggler_with_controller,
    AnimatedThemeTogglerController,
    AnimatedThemeTogglerProps,
};

animated_theme_toggler_with_controller(&controller, now, Message::ToggleTheme, props, theme)"#,
    },
    ComponentDoc {
        slug: ComponentSlug::Globe,
        title: "Globe",
        description: "Canvas globe projection with markers, drag control, and auto rotation.",
        group: "Media",
        resettable: true,
        source: r#"use magic_iced::{globe, GlobeProps, GlobeState};

globe(&state, &GlobeProps::new(), Message::Globe)"#,
    },
    ComponentDoc {
        slug: ComponentSlug::IconCloud,
        title: "Icon Cloud",
        description: "Projected icon sphere with focus rotation and Lucide-based content.",
        group: "Media",
        resettable: true,
        source: r#"use magic_iced::{icon_cloud, IconCloudItem, IconCloudProps, IconCloudState};

icon_cloud(&state, &props, Message::IconCloud)"#,
    },
    ComponentDoc {
        slug: ComponentSlug::VideoText,
        title: "Video Text",
        description: "Poster or frame-sequence text masking with deterministic native timing.",
        group: "Media",
        resettable: true,
        source: r#"use magic_iced::{video_text, VideoTextProps, VideoTextSource, VideoTextState};

let props = VideoTextProps::new(VideoTextSource::Frames(frames), "MAGIC");
video_text(&state, now, &props)"#,
    },
];

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
impl ComponentSlug {
    pub fn all() -> &'static [Self] {
        &[
            Self::Marquee,
            Self::Terminal,
            Self::TweetCard,
            Self::WordRotate,
            Self::SparklesText,
            Self::AnimatedGradientText,
            Self::NumberTicker,
            Self::TextReveal,
            Self::ShimmerButton,
            Self::MagicCard,
            Self::Dock,
            Self::ThemeToggler,
            Self::Globe,
            Self::IconCloud,
            Self::VideoText,
        ]
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::Marquee => "marquee",
            Self::Terminal => "terminal",
            Self::TweetCard => "tweet-card",
            Self::WordRotate => "word-rotate",
            Self::SparklesText => "sparkles-text",
            Self::AnimatedGradientText => "animated-gradient-text",
            Self::NumberTicker => "number-ticker",
            Self::TextReveal => "text-reveal",
            Self::ShimmerButton => "shimmer-button",
            Self::MagicCard => "magic-card",
            Self::Dock => "dock",
            Self::ThemeToggler => "animated-theme-toggler",
            Self::Globe => "globe",
            Self::IconCloud => "icon-cloud",
            Self::VideoText => "video-text",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::all().iter().copied().find(|item| item.slug() == slug)
    }

    pub fn doc(self) -> &'static ComponentDoc {
        DOCS.iter()
            .find(|doc| doc.slug == self)
            .expect("component doc definition must exist")
    }
}
