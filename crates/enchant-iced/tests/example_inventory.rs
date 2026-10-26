use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

const INTERNAL_HELPER_EXAMPLES: &[&str] = &[
    "animated_subscribe_button",
    "border_track",
    "gooey",
    "metal_fx",
    "motion_grid",
    "thinking_orb",
    "voice_glow",
];

const HELPER_EXPORTS: &[&str] = &[
    "animated_subscribe_button",
    "animated_theme_toggler_with_controller",
    "bento_card",
    "blend_globe_demo_colors",
    "confetti_button",
    "dia_text_reveal_companion",
    "dock_entry_icon_count",
    "dock_icon",
    "dock_separator",
    "globe_demo",
    "globe_demo_colors_blended",
    "globe_demo_colors_for_controller",
    "globe_demo_colors_for_mode",
    "globe_demo_colors_from_theme",
    "line_shadow_color_for_mode",
    "marquee_indexed",
    "noise_texture_layer",
    "rasterize_text_mask",
    "scroll_velocity_container",
    "scroll_velocity_row",
    "shiny_text_color_for_mode",
    "text_inner_shadow_mask",
    "theme_transition_overlay",
];

#[derive(Clone, Debug)]
struct OfficialScenario {
    name: String,
    path: String,
    components: BTreeSet<String>,
}

#[test]
fn example_inventory_matches_official_registry() {
    let official = official_inventory();
    let public_components = local_public_components();
    let example_binaries = local_example_binaries();

    let total_official_scenarios = official.values().map(Vec::len).sum::<usize>();
    eprintln!("official components: {}", official.len());
    eprintln!("official scenarios: {}", total_official_scenarios);
    eprintln!("local public components: {}", public_components.len());
    eprintln!("example binaries: {}", example_binaries.len());

    let missing_official_binaries = official
        .keys()
        .filter(|component| !example_covers_component(component, &example_binaries))
        .cloned()
        .collect::<Vec<_>>();

    let local_only_public_components = public_components
        .difference(&official.keys().cloned().collect())
        .cloned()
        .collect::<Vec<_>>();

    let internal_examples = example_binaries
        .difference(&public_components)
        .filter(|name| !official.contains_key(example_owned_component(name)))
        .cloned()
        .collect::<Vec<_>>();

    let mut problems = Vec::new();

    if !missing_official_binaries.is_empty() {
        problems.push(format!(
            "official components missing example binaries:\n{}",
            missing_official_binaries
                .iter()
                .map(|component| {
                    let scenarios = official.get(component.as_str()).unwrap();
                    format!(
                        "  {component}: {}",
                        scenarios
                            .iter()
                            .map(|scenario| format!("{} ({})", scenario.name, scenario.path))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }

    if !local_only_public_components.is_empty() {
        problems.push(format!(
            "local public components not covered by official registry inventory:\n  {}",
            local_only_public_components.join("\n  ")
        ));
    }

    let unexpected_internal_examples = internal_examples
        .iter()
        .filter(|name| !INTERNAL_HELPER_EXAMPLES.contains(&name.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    if !unexpected_internal_examples.is_empty() {
        problems.push(format!(
            "unexpected example binaries outside public inventory:\n  {}",
            unexpected_internal_examples.join("\n  ")
        ));
    } else {
        for name in internal_examples {
            eprintln!("internal helper example kept out of public inventory: {name}");
        }
    }

    assert!(
        problems.is_empty(),
        "example inventory mismatch:\n{}",
        problems.join("\n\n")
    );
}

fn official_inventory() -> BTreeMap<String, Vec<OfficialScenario>> {
    let registry_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/registry-examples.ts");
    let registry = fs::read_to_string(&registry_path)
        .unwrap_or_else(|err| panic!("read {}: {}", registry_path.display(), err));

    let mut inventory = BTreeMap::new();
    for scenario in parse_official_scenarios(&registry) {
        for component in &scenario.components {
            inventory
                .entry(component.clone())
                .or_insert_with(Vec::new)
                .push(scenario.clone());
        }
    }

    for scenarios in inventory.values_mut() {
        scenarios
            .sort_by(|left, right| left.name.cmp(&right.name).then(left.path.cmp(&right.path)));
    }

    inventory
}

fn parse_official_scenarios(source: &str) -> Vec<OfficialScenario> {
    let mut scenarios = Vec::new();
    let mut block = Vec::new();
    let mut in_block = false;

    for line in source.lines() {
        if line.starts_with("  {") {
            in_block = true;
            block.clear();
        }

        if in_block {
            block.push(line);
        }

        if in_block && line.starts_with("  },") {
            let block = block.join("\n");
            scenarios.push(OfficialScenario {
                name: capture_quoted(&block, "name: \"")
                    .unwrap_or_else(|| panic!("missing scenario name in:\n{}", block)),
                path: capture_quoted(&block, "path: \"")
                    .unwrap_or_else(|| panic!("missing scenario path in:\n{}", block)),
                components: capture_magicui_components(&block),
            });
            in_block = false;
        }
    }

    scenarios
}

fn capture_quoted(source: &str, marker: &str) -> Option<String> {
    let start = source.find(marker)? + marker.len();
    let tail = &source[start..];
    let end = tail.find('"')?;
    Some(tail[..end].to_string())
}

fn capture_magicui_components(source: &str) -> BTreeSet<String> {
    let mut components = BTreeSet::new();
    let mut tail = source;

    while let Some(offset) = tail.find("@magicui/") {
        let after = &tail[offset + "@magicui/".len()..];
        let end = after
            .find(|ch: char| !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-'))
            .unwrap_or(after.len());
        components.insert(after[..end].replace('-', "_"));
        tail = &after[end..];
    }

    components
}

fn local_public_components() -> BTreeSet<String> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib_path = manifest_dir.join("src/lib.rs");
    let lib = fs::read_to_string(&lib_path)
        .unwrap_or_else(|err| panic!("read {}: {}", lib_path.display(), err));

    let mut components = lowercase_tokens(extract_block(&lib, "pub use widgets::{"));
    components.extend(lowercase_tokens(extract_block(&lib, "pub use sections::{")));
    for helper in HELPER_EXPORTS {
        components.remove(*helper);
    }

    components
}

fn extract_block<'a>(source: &'a str, marker: &str) -> &'a str {
    let start = source
        .find(marker)
        .unwrap_or_else(|| panic!("missing block marker: {marker}"))
        + marker.len();
    let tail = &source[start..];
    let end = tail
        .find("};")
        .unwrap_or_else(|| panic!("missing block terminator after: {marker}"));
    &tail[..end]
}

fn lowercase_tokens(source: &str) -> BTreeSet<String> {
    source
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .filter(|token| !token.is_empty())
        .filter(|token| {
            token
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_lowercase())
        })
        .map(str::to_string)
        .collect()
}

fn local_example_binaries() -> BTreeSet<String> {
    let examples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");
    fs::read_dir(&examples_dir)
        .unwrap_or_else(|err| panic!("read {}: {}", examples_dir.display(), err))
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            let is_rs = path.extension().and_then(|ext| ext.to_str()) == Some("rs");
            if !is_rs {
                return None;
            }
            path.file_stem()
                .and_then(|stem| stem.to_str())
                // Local `_foo.rs` debug probes stay out of inventory (and gitignore).
                .filter(|stem| !stem.starts_with('_'))
                .map(|stem| stem.to_string())
        })
        .collect()
}

fn example_covers_component(component: &str, example_binaries: &BTreeSet<String>) -> bool {
    example_binaries.contains(component) || example_binaries.contains(&format!("{component}_demo"))
}

fn example_owned_component(name: &str) -> &str {
    name.strip_suffix("_demo")
        .or_else(|| name.strip_suffix("_vertical"))
        .unwrap_or(name)
}
