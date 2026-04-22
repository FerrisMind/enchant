#![allow(dead_code)]

#[path = "../build_support/mod.rs"]
mod build_support;

use build_support::assets::generate_reference_assets_module;
use build_support::css_theme::parse_theme_state;
use build_support::docs_source::parse_docs_config;
use build_support::mdx::parse_doc_page;
use build_support::model::BodyBlock;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use twill::prelude::ColorValue;

fn temp_dir(prefix: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for tests")
        .as_nanos();
    dir.push(format!("wasm-demo-{prefix}-{stamp}-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn parse_theme_state_uses_destructive_foreground() {
    let css = r#"
:root {
  --destructive: oklch(60% 0.20 30);
  --destructive-foreground: oklch(50% 0.10 20);
  --primary-foreground: oklch(70% 0.30 40);
}
.dark {
}
"#;

    let theme = parse_theme_state(css);
    let expected = {
        let (r, g, b, a) = ColorValue::from_oklch(0.5, 0.10, 20.0).to_rgba8();
        [r, g, b, a]
    };
    let primary_expected = {
        let (r, g, b, a) = ColorValue::from_oklch(0.7, 0.30, 40.0).to_rgba8();
        [r, g, b, a]
    };

    assert_eq!(theme.light.destructive_foreground, expected);
    assert_ne!(theme.light.destructive_foreground, primary_expected);
}

#[test]
fn parse_docs_config_extracts_navigation_and_sidebar() {
    let input = "mainNav: [\n    {\n      title: \"Docs\",\n      href: \"/docs\",\n    },\n  ],\nsidebarNav: [\n    {\n      title: \"Getting Started\",\n      items: [\n        {\n          title: \"Install\",\n          href: \"/docs/install\",\n        },\n      ],\n    },\n  ],\n";

    let (nav_items, sidebar_sections) = parse_docs_config(input);

    assert_eq!(nav_items.len(), 1);
    assert_eq!(nav_items[0].title, "Docs");
    assert_eq!(nav_items[0].href, "/docs");
    assert_eq!(sidebar_sections.len(), 1);
    assert_eq!(sidebar_sections[0].title, "Getting Started");
    assert_eq!(sidebar_sections[0].items.len(), 1);
    assert_eq!(sidebar_sections[0].items[0].title, "Install");
    assert_eq!(sidebar_sections[0].items[0].href, "/docs/install");
}

#[test]
fn parse_doc_page_captures_tabs_steps_and_sources() {
    let dir = temp_dir("mdx");
    let path = dir.join("sample.mdx");
    fs::write(
        &path,
        r#"---
title: Sample
description: Example
---

## Intro

Paragraph one.

<ComponentSource name="sample-source" title="Source title" />

<Tabs defaultValue="alpha">
<TabsTrigger value="alpha">Alpha</TabsTrigger>
<TabsContent value="alpha">
<ComponentPreview name="preview-one" />
</TabsContent>
</Tabs>

<Steps>
<Step>
First step text.
</Step>
</Steps>
"#,
    )
    .unwrap();

    let mut sources = BTreeMap::new();
    sources.insert("sample-source".to_owned(), "export const x = 1;".to_owned());

    let page = parse_doc_page(&path, &[], &sources);

    assert_eq!(page.title, "Sample");
    assert_eq!(page.description, "Example");
    assert_eq!(page.toc.len(), 1);
    assert!(matches!(
        page.blocks[0],
        BodyBlock::Heading { level: 2, .. }
    ));
    assert!(matches!(page.blocks[1], BodyBlock::Paragraph { .. }));
    assert!(matches!(page.blocks[2], BodyBlock::SourceRef { .. }));

    match &page.blocks[3] {
        BodyBlock::Tabs { items, .. } => {
            assert_eq!(items.len(), 1);
            assert!(matches!(items[0].blocks[0], BodyBlock::PreviewRef { .. }));
        }
        other => panic!("expected tabs block, got {other:?}"),
    }

    match &page.blocks[4] {
        BodyBlock::Steps { items } => {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].title, "First step text.");
            assert!(items[0].blocks.is_empty());
        }
        other => panic!("expected steps block, got {other:?}"),
    }
}

#[test]
fn generate_reference_assets_module_lists_existing_files() {
    let dir = temp_dir("assets");
    let asset_path = dir.join("example.png");
    fs::write(&asset_path, b"png").unwrap();

    let source = generate_reference_assets_module(&dir);

    assert!(source.contains("reference-assets/example.png"));
    assert!(source.contains("include_bytes!"));
}
