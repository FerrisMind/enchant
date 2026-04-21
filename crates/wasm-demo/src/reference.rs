use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Debug, Clone, Deserialize)]
pub struct ReferenceData {
    pub layout: LayoutState,
    pub theme: ThemeState,
    pub banner: BannerState,
    pub header: HeaderState,
    pub footer: FooterState,
    pub home: HomeState,
    pub pages: Vec<DocPage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutState {
    pub header_height: f32,
    pub footer_height: f32,
    pub footer_height_xl: f32,
    pub container_max_width: f32,
    pub container_padding: f32,
    pub container_padding_lg: f32,
    pub docs_sidebar_width: f32,
    pub docs_toc_width: f32,
    pub docs_content_max_width: f32,
    pub docs_top_spacing: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ThemeState {
    pub radius: RadiusState,
    pub spacing_unit: f32,
    pub light: ThemePalette,
    pub dark: ThemePalette,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RadiusState {
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ThemePalette {
    pub background: [u8; 4],
    pub foreground: [u8; 4],
    pub card: [u8; 4],
    pub card_foreground: [u8; 4],
    pub popover: [u8; 4],
    pub popover_foreground: [u8; 4],
    pub primary: [u8; 4],
    pub primary_foreground: [u8; 4],
    pub secondary: [u8; 4],
    pub secondary_foreground: [u8; 4],
    pub muted: [u8; 4],
    pub muted_foreground: [u8; 4],
    pub accent: [u8; 4],
    pub accent_foreground: [u8; 4],
    pub destructive: [u8; 4],
    pub destructive_foreground: [u8; 4],
    pub border: [u8; 4],
    pub input: [u8; 4],
    pub ring: [u8; 4],
    pub chart_1: [u8; 4],
    pub chart_2: [u8; 4],
    pub chart_3: [u8; 4],
    pub chart_4: [u8; 4],
    pub chart_5: [u8; 4],
    pub sidebar: [u8; 4],
    pub sidebar_foreground: [u8; 4],
    pub sidebar_primary: [u8; 4],
    pub sidebar_primary_foreground: [u8; 4],
    pub sidebar_accent: [u8; 4],
    pub sidebar_accent_foreground: [u8; 4],
    pub sidebar_border: [u8; 4],
    pub sidebar_ring: [u8; 4],
}

#[derive(Debug, Clone, Deserialize)]
pub struct BannerState {
    pub text: String,
    pub href: String,
    pub background: [u8; 4],
    pub border: [u8; 4],
}

#[derive(Debug, Clone, Deserialize)]
pub struct HeaderState {
    pub nav_items: Vec<NavItem>,
    pub search_placeholder: String,
    pub github_stars: String,
    pub discord_online: String,
    pub github_href: String,
    pub discord_href: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FooterState {
    pub author_href: String,
    pub github_href: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NavItem {
    pub title: String,
    pub href: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SidebarSection {
    pub title: String,
    pub items: Vec<SidebarItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SidebarItem {
    pub title: String,
    pub href: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HomeState {
    pub hero: HeroState,
    pub showcase_title: String,
    pub showcase_description: String,
    pub showcase_cards: Vec<ShowcaseCard>,
    pub testimonials_title: String,
    pub video_title: String,
    pub tweet_ids: Vec<String>,
    pub video_embeds: Vec<String>,
    pub sidebar: Vec<SidebarSection>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HeroState {
    pub announcement: String,
    pub announcement_href: String,
    pub title: String,
    pub description: String,
    pub primary_cta: String,
    pub primary_cta_href: String,
    pub secondary_cta: String,
    pub secondary_cta_href: String,
    pub tech_stack: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShowcaseCard {
    pub title: String,
    pub image: String,
    pub href: String,
    pub affiliation: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DocPage {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub sidebar: Vec<SidebarSection>,
    pub toc: Vec<TocItem>,
    pub blocks: Vec<BodyBlock>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TocItem {
    pub level: u8,
    pub title: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BodyBlock {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph {
        text: String,
    },
    CodeFence {
        language: Option<String>,
        title: Option<String>,
        code: String,
    },
    PropsTable {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    PreviewRef {
        name: String,
    },
    SourceRef {
        title: Option<String>,
        code: Option<String>,
    },
    Tabs {
        default_value: Option<String>,
        items: Vec<TabItem>,
    },
    Steps {
        items: Vec<StepItem>,
    },
    ComponentsList,
    List {
        items: Vec<String>,
    },
    Note {
        text: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct TabItem {
    pub value: String,
    pub label: String,
    pub blocks: Vec<BodyBlock>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StepItem {
    pub title: String,
    pub blocks: Vec<BodyBlock>,
}

pub fn data() -> &'static ReferenceData {
    static DATA: OnceLock<ReferenceData> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!(concat!(env!("OUT_DIR"), "/reference-data.json")))
            .expect("generated reference data must be valid")
    })
}
