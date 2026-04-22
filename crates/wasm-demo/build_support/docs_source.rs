use super::assets::{cache_showcase_image, cache_video_thumbnails};
use super::mdx::parse_doc_page;
use super::model::{
    DocPage, HeroState, HomeState, NavItem, ShowcaseCard, SidebarItem, SidebarSection,
};
use super::text::{
    capture_jsx_text, capture_link_href, capture_nth_jsx_text, capture_tech_stack,
    frontmatter_value, parse_key_value, parse_tweet_ids, parse_video_embeds, split_frontmatter,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub fn parse_docs_config(input: &str) -> (Vec<NavItem>, Vec<SidebarSection>) {
    let lines = input.lines().collect::<Vec<_>>();

    let mut nav_items = Vec::new();
    let mut sidebar_sections = Vec::new();

    let mut in_main = false;
    let mut in_sidebar = false;
    let mut current_nav: BTreeMap<String, String> = BTreeMap::new();
    let mut current_section: Option<SidebarSection> = None;
    let mut current_item: BTreeMap<String, String> = BTreeMap::new();
    let mut in_items = false;

    for raw_line in lines {
        let line = raw_line.trim_end();
        let trimmed = line.trim();
        let indent = line.len() - line.trim_start().len();

        if trimmed == "mainNav: [" {
            in_main = true;
            continue;
        }
        if trimmed == "sidebarNav: [" {
            in_sidebar = true;
            continue;
        }

        if in_main {
            if indent == 2 && trimmed == "]," {
                in_main = false;
                continue;
            }
            if indent == 4 && trimmed == "{" {
                current_nav.clear();
                continue;
            }
            if indent == 6 {
                if let Some((key, value)) = parse_key_value(trimmed) {
                    current_nav.insert(key, value);
                }
                continue;
            }
            if indent == 4 && (trimmed == "}," || trimmed == "}") {
                if let (Some(title), Some(href)) =
                    (current_nav.get("title"), current_nav.get("href"))
                {
                    nav_items.push(NavItem {
                        title: title.clone(),
                        href: href.clone(),
                    });
                }
                current_nav.clear();
                continue;
            }
        }

        if in_sidebar {
            if indent == 2 && trimmed == "]," {
                if let Some(section) = current_section.take() {
                    sidebar_sections.push(section);
                }
                in_sidebar = false;
                continue;
            }
            if indent == 4 && trimmed == "{" {
                if !in_items {
                    current_section = Some(SidebarSection {
                        title: String::new(),
                        items: Vec::new(),
                    });
                }
                continue;
            }
            if indent == 6 && trimmed == "items: [" {
                in_items = true;
                continue;
            }
            if !in_items && indent == 6 {
                if let Some((key, value)) = parse_key_value(trimmed) {
                    if key == "title" {
                        if let Some(section) = current_section.as_mut() {
                            section.title = value;
                        }
                    }
                }
                continue;
            }
            if in_items && indent == 8 && trimmed == "{" {
                current_item.clear();
                continue;
            }
            if in_items && indent == 10 {
                if let Some((key, value)) = parse_key_value(trimmed) {
                    current_item.insert(key, value);
                }
                continue;
            }
            if in_items && indent == 8 && (trimmed == "}," || trimmed == "}") {
                if let (Some(title), Some(href)) =
                    (current_item.get("title"), current_item.get("href"))
                {
                    if let Some(section) = current_section.as_mut() {
                        section.items.push(SidebarItem {
                            title: title.clone(),
                            href: href.clone(),
                        });
                    }
                }
                current_item.clear();
                continue;
            }
            if in_items && indent == 6 && trimmed == "]," {
                in_items = false;
                continue;
            }
            if !in_items && indent == 4 && (trimmed == "}," || trimmed == "}") {
                if let Some(section) = current_section.take() {
                    sidebar_sections.push(section);
                }
            }
        }
    }

    (nav_items, sidebar_sections)
}

pub fn parse_home_state(
    ref_root: &Path,
    sidebar: &[SidebarSection],
    assets_dir: &Path,
) -> HomeState {
    let hero = fs::read_to_string(ref_root.join("components/sections/hero.tsx")).unwrap();
    let showcase = fs::read_to_string(ref_root.join("components/sections/showcase.tsx")).unwrap();
    let testimonials =
        fs::read_to_string(ref_root.join("components/sections/testimonials.tsx")).unwrap();
    let video_testimonials =
        fs::read_to_string(ref_root.join("components/sections/video-testimonials.tsx")).unwrap();
    let video_embeds = parse_video_embeds(&video_testimonials);
    cache_video_thumbnails(&video_embeds, assets_dir);

    HomeState {
        hero: HeroState {
            announcement: format!("Introducing {}", latest_docs_title(ref_root)),
            announcement_href: latest_docs_href(ref_root),
            title: capture_jsx_text(&hero, "h1")
                .unwrap_or_else(|| "UI library for Design Engineers".to_owned()),
            description: capture_nth_jsx_text(&hero, "p", 0).unwrap_or_else(|| {
                "150+ free and open-source animated components and effects built with React, Typescript, Tailwind CSS, and Motion. Perfect companion for shadcn/ui."
                    .to_owned()
            }),
            primary_cta: "Browse Components".to_owned(),
            primary_cta_href: capture_link_href(&hero, "Browse Components")
                .unwrap_or_else(|| "/components".to_owned()),
            secondary_cta: "Browse Templates".to_owned(),
            secondary_cta_href: capture_link_href(&hero, "Browse Templates")
                .unwrap_or_else(|| "https://pro.magicui.design".to_owned()),
            tech_stack: capture_tech_stack(&hero),
        },
        showcase_title: capture_nth_jsx_text(&showcase, "h2", 0)
            .unwrap_or_else(|| "Showcase".to_owned()),
        showcase_description: capture_nth_jsx_text(&showcase, "h3", 0).unwrap_or_else(|| {
            "Companies choose Magic UI to build their landing pages.".to_owned()
        }),
        showcase_cards: load_showcase_cards(ref_root, assets_dir),
        testimonials_title: capture_nth_jsx_text(&testimonials, "h2", 0)
            .unwrap_or_else(|| "What People Are Saying on Twitter".to_owned()),
        video_title: capture_nth_jsx_text(&video_testimonials, "h2", 0)
            .unwrap_or_else(|| "Featured on YouTube".to_owned()),
        tweet_ids: parse_tweet_ids(&testimonials),
        video_embeds,
        sidebar: sidebar.to_vec(),
    }
}

fn load_showcase_cards(ref_root: &Path, assets_dir: &Path) -> Vec<ShowcaseCard> {
    let showcase_root = ref_root.join("content/showcase");
    let mut paths = WalkDir::new(showcase_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("mdx"))
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    paths.sort();

    paths
        .into_iter()
        .filter_map(|path| {
            let content = fs::read_to_string(&path).ok()?;
            let (frontmatter, _) = split_frontmatter(&content)?;
            if frontmatter_value(frontmatter, "featured").as_deref() != Some("true") {
                return None;
            }

            let title = frontmatter_value(frontmatter, "title")?;
            let image = frontmatter_value(frontmatter, "image")?;
            let href = frontmatter_value(frontmatter, "href")
                .unwrap_or_else(|| "https://magicui.design/showcase".to_owned());

            let cached_image = cache_showcase_image(&title, &image, assets_dir);

            Some(ShowcaseCard {
                title,
                image: cached_image,
                href,
                affiliation: frontmatter_value(frontmatter, "affiliation"),
            })
        })
        .collect()
}

fn latest_docs_title(ref_root: &Path) -> String {
    latest_docs_entry(ref_root)
        .map(|(_, title, _)| title)
        .unwrap_or_else(|| "Noise Texture".to_owned())
}

fn latest_docs_href(ref_root: &Path) -> String {
    latest_docs_entry(ref_root)
        .map(|(_, _, href)| href)
        .unwrap_or_else(|| "/docs/components/noise-texture".to_owned())
}

fn latest_docs_entry(ref_root: &Path) -> Option<(String, String, String)> {
    let docs_root = ref_root.join("content/docs");
    let mut latest: Option<(String, String, String)> = None;

    for entry in WalkDir::new(docs_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("mdx"))
    {
        let content = fs::read_to_string(entry.path()).unwrap();
        let frontmatter = split_frontmatter(&content).map(|(frontmatter, _)| frontmatter);
        if let Some(frontmatter) = frontmatter {
            let title = frontmatter_value(frontmatter, "title").unwrap_or_default();
            let date = frontmatter_value(frontmatter, "date").unwrap_or_default();
            if !title.is_empty() && !date.is_empty() {
                let href = href_for_doc(entry.path(), ref_root.join("content/docs").as_path());
                if latest
                    .as_ref()
                    .map(|(old, _, _)| old < &date)
                    .unwrap_or(true)
                {
                    latest = Some((date, title, href));
                }
            }
        }
    }

    latest
}

fn href_for_doc(path: &Path, docs_root: &Path) -> String {
    let relative = path.strip_prefix(docs_root).unwrap_or(path);
    let without_ext = relative.with_extension("");
    let segments = without_ext
        .components()
        .map(|component| component.as_os_str().to_string_lossy().replace('\\', "/"))
        .collect::<Vec<_>>();

    format!("/docs/{}", segments.join("/"))
}

pub fn load_registry_sources(root: &Path) -> BTreeMap<String, String> {
    let mut sources = BTreeMap::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
    {
        let raw = fs::read_to_string(entry.path()).unwrap();
        let value: Value = serde_json::from_str(&raw).unwrap();
        let stem = entry
            .path()
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap()
            .to_owned();

        if let Some(content) = value
            .get("files")
            .and_then(|files| files.as_array())
            .and_then(|files| files.first())
            .and_then(|file| file.get("content"))
            .and_then(|content| content.as_str())
        {
            sources.insert(stem.clone(), content.to_owned());
        }

        if let Some(name) = value.get("name").and_then(|name| name.as_str()) {
            if let Some(content) = value
                .get("files")
                .and_then(|files| files.as_array())
                .and_then(|files| files.first())
                .and_then(|file| file.get("content"))
                .and_then(|content| content.as_str())
            {
                sources.insert(name.to_owned(), content.to_owned());
            }
        }
    }

    sources
}

pub fn load_doc_pages(
    root: &Path,
    sidebar: &[SidebarSection],
    sources: &BTreeMap<String, String>,
) -> Vec<DocPage> {
    let mut pages = WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("mdx"))
        .map(|entry| parse_doc_page(entry.path(), sidebar, sources))
        .collect::<Vec<_>>();

    pages.sort_by(|left, right| left.title.cmp(&right.title));
    pages
}
