mod build_support;

use build_support::{
    assets::generate_reference_assets_module,
    css_theme::{parse_banner_state, parse_layout_state, parse_theme_state},
    docs_source::{load_doc_pages, load_registry_sources, parse_docs_config, parse_home_state},
    model::{FooterState, HeaderState, ReferenceData},
    text::{capture_site_link, parse_search_placeholder},
};
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let ref_root = manifest_dir.join("../../../magic-iced_refs/magicui/apps/www");
    let assets_dir = manifest_dir.join("generated/reference-assets");
    println!("cargo:rerun-if-changed={}", ref_root.display());
    let _ = fs::create_dir_all(&assets_dir);

    let site_ts = fs::read_to_string(ref_root.join("config/site.ts")).unwrap();
    let docs_ts = fs::read_to_string(ref_root.join("config/docs.ts")).unwrap();
    let globals_css = fs::read_to_string(ref_root.join("styles/globals.css")).unwrap();
    let app_layout = fs::read_to_string(ref_root.join("app/layout.tsx")).unwrap();
    let banner_ts = fs::read_to_string(ref_root.join("components/site-banner.tsx")).unwrap();
    let (main_nav, sidebar) = parse_docs_config(&docs_ts);
    let layout = parse_layout_state(&globals_css, &app_layout);
    let theme = parse_theme_state(&globals_css);
    let banner = parse_banner_state(&banner_ts);

    let home = parse_home_state(&ref_root, &sidebar, &assets_dir);
    let sources = load_registry_sources(&ref_root.join("public/r"));
    let pages = load_doc_pages(
        &ref_root.join("content/docs/components"),
        &sidebar,
        &sources,
    );

    let data = ReferenceData {
        layout,
        theme,
        banner,
        header: HeaderState {
            nav_items: main_nav,
            search_placeholder: parse_search_placeholder(
                &fs::read_to_string(ref_root.join("components/command-menu.tsx")).unwrap(),
            ),
            github_stars: "20,692".to_owned(),
            discord_online: "244".to_owned(),
            github_href: capture_site_link(&site_ts, "github"),
            discord_href: capture_site_link(&site_ts, "discord"),
        },
        footer: FooterState {
            author_href: capture_site_link(&site_ts, "twitter"),
            github_href: capture_site_link(&site_ts, "github"),
        },
        home,
        pages,
    };

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    fs::write(
        out_dir.join("reference-assets.rs"),
        generate_reference_assets_module(&assets_dir),
    )
    .unwrap();
    fs::write(
        out_dir.join("reference-data.json"),
        serde_json::to_string_pretty(&data).unwrap(),
    )
    .unwrap();
}
