use super::model::{BannerState, LayoutState, RadiusState, ThemePalette, ThemeState};
use super::text::{capture_first_href, capture_link_href, capture_tag_block, normalize_jsx_text};
use regex::Regex;
use std::collections::BTreeMap;
use twill::prelude::ColorValue;

pub fn parse_layout_state(globals_css: &str, app_layout: &str) -> LayoutState {
    let header_height =
        extract_calc_spacing_multiplier(app_layout, "--header-height").unwrap_or(14.0) * 4.0;
    let footer_height =
        extract_calc_spacing_multiplier(app_layout, "--footer-height").unwrap_or(14.0) * 4.0;
    let footer_height_xl = extract_breakpoint_spacing_multiplier(
        app_layout,
        "xl:[--footer-height:calc(var(--spacing)*",
    )
    .unwrap_or(24.0)
        * 4.0;

    let container_max_width =
        extract_bracket_px(globals_css, "@utility container", "max-w-[", "px]").unwrap_or(1400.0);

    LayoutState {
        header_height,
        footer_height,
        footer_height_xl,
        container_max_width,
        container_padding: 16.0,
        container_padding_lg: 32.0,
        docs_sidebar_width: 240.0,
        docs_toc_width: 336.0,
        docs_content_max_width: 768.0,
        docs_top_spacing: 16.0,
    }
}

pub fn parse_theme_state(globals_css: &str) -> ThemeState {
    let root_vars = css_var_map(
        extract_css_block(globals_css, ":root {")
            .as_deref()
            .unwrap_or(""),
    );
    let mut dark_vars = root_vars.clone();
    dark_vars.extend(css_var_map(
        extract_css_block(globals_css, ".dark {")
            .as_deref()
            .unwrap_or(""),
    ));

    ThemeState {
        radius: RadiusState {
            sm: 6.0,
            md: 8.0,
            lg: 10.0,
        },
        spacing_unit: 4.0,
        light: theme_palette_from_vars(&root_vars),
        dark: theme_palette_from_vars(&dark_vars),
    }
}

pub fn parse_banner_state(input: &str) -> BannerState {
    BannerState {
        text: normalize_jsx_text(
            capture_tag_block(input, "span", 0)
                .as_deref()
                .unwrap_or("Introducing Magic UI Pro"),
        ),
        href: capture_link_href(input, "Introducing Magic UI Pro")
            .or_else(|| capture_first_href(input))
            .unwrap_or_else(|| "https://pro.magicui.design".to_owned()),
        background: [79, 70, 229, 255],
        border: [229, 229, 229, 77],
    }
}

fn extract_calc_spacing_multiplier(input: &str, key: &str) -> Option<f32> {
    let pattern = format!(r#"{key}:calc\(var\(--spacing\)\*([0-9.]+)\)"#);
    Regex::new(&pattern)
        .unwrap()
        .captures(input)
        .and_then(|captures| captures[1].parse::<f32>().ok())
}

fn extract_breakpoint_spacing_multiplier(input: &str, prefix: &str) -> Option<f32> {
    let escaped = regex::escape(prefix);
    let pattern = format!(r#"{escaped}([0-9.]+)\)\]"#);
    Regex::new(&pattern)
        .unwrap()
        .captures(input)
        .and_then(|captures| captures[1].parse::<f32>().ok())
}

fn extract_bracket_px(input: &str, anchor: &str, prefix: &str, suffix: &str) -> Option<f32> {
    let block = extract_css_utility_block(input, anchor)?;
    let escaped_prefix = regex::escape(prefix);
    let escaped_suffix = regex::escape(suffix);
    let pattern = format!(r#"{escaped_prefix}([0-9.]+){escaped_suffix}"#);
    Regex::new(&pattern)
        .unwrap()
        .captures(&block)
        .and_then(|captures| captures[1].parse::<f32>().ok())
}

fn extract_css_block(input: &str, selector: &str) -> Option<String> {
    let start = input.find(selector)?;
    let tail = &input[start..];
    let open = tail.find('{')?;
    let mut depth = 0usize;
    for (offset, ch) in tail[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(tail[open + 1..open + offset].to_owned());
                }
            }
            _ => {}
        }
    }
    None
}

fn extract_css_utility_block(input: &str, utility: &str) -> Option<String> {
    let start = input.find(utility)?;
    let tail = &input[start..];
    let open = tail.find('{')?;
    let close = tail[open..].find('}')?;
    Some(tail[open + 1..open + close].to_owned())
}

fn css_var_map(block: &str) -> BTreeMap<String, [u8; 4]> {
    block
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if !trimmed.starts_with("--") {
                return None;
            }
            let (key, value) = trimmed.split_once(':')?;
            let normalized = key.trim().trim_start_matches("--").to_owned();
            let value = value.trim().trim_end_matches(';');
            parse_oklch(value).map(|rgba| (normalized, rgba))
        })
        .collect()
}

fn theme_palette_from_vars(vars: &BTreeMap<String, [u8; 4]>) -> ThemePalette {
    ThemePalette {
        background: color_var(vars, "background"),
        foreground: color_var(vars, "foreground"),
        card: color_var(vars, "card"),
        card_foreground: color_var(vars, "card-foreground"),
        popover: color_var(vars, "popover"),
        popover_foreground: color_var(vars, "popover-foreground"),
        primary: color_var(vars, "primary"),
        primary_foreground: color_var(vars, "primary-foreground"),
        secondary: color_var(vars, "secondary"),
        secondary_foreground: color_var(vars, "secondary-foreground"),
        muted: color_var(vars, "muted"),
        muted_foreground: color_var(vars, "muted-foreground"),
        accent: color_var(vars, "accent"),
        accent_foreground: color_var(vars, "accent-foreground"),
        destructive: color_var(vars, "destructive"),
        destructive_foreground: color_var(vars, "destructive-foreground"),
        border: color_var(vars, "border"),
        input: color_var(vars, "input"),
        ring: color_var(vars, "ring"),
        chart_1: color_var(vars, "chart-1"),
        chart_2: color_var(vars, "chart-2"),
        chart_3: color_var(vars, "chart-3"),
        chart_4: color_var(vars, "chart-4"),
        chart_5: color_var(vars, "chart-5"),
        sidebar: color_var(vars, "sidebar"),
        sidebar_foreground: color_var(vars, "sidebar-foreground"),
        sidebar_primary: color_var(vars, "sidebar-primary"),
        sidebar_primary_foreground: color_var(vars, "sidebar-primary-foreground"),
        sidebar_accent: color_var(vars, "sidebar-accent"),
        sidebar_accent_foreground: color_var(vars, "sidebar-accent-foreground"),
        sidebar_border: color_var(vars, "sidebar-border"),
        sidebar_ring: color_var(vars, "sidebar-ring"),
    }
}

fn color_var(vars: &BTreeMap<String, [u8; 4]>, key: &str) -> [u8; 4] {
    vars.get(key).copied().unwrap_or([0, 0, 0, 255])
}

fn parse_oklch(input: &str) -> Option<[u8; 4]> {
    let oklch = input.trim().strip_prefix("oklch(")?.strip_suffix(')')?;
    let (color_part, alpha_part) = if let Some((left, right)) = oklch.split_once('/') {
        (left.trim(), Some(right.trim()))
    } else {
        (oklch.trim(), None)
    };

    let mut components = color_part.split_whitespace();
    let l = components
        .next()?
        .trim_end_matches('%')
        .parse::<f32>()
        .ok()?;
    let c = components.next()?.parse::<f32>().ok()?;
    let h = components.next()?.parse::<f32>().ok()?;
    let alpha = alpha_part.and_then(parse_alpha).unwrap_or(1.0);

    let mut value = ColorValue::from_oklch(if l > 1.0 { l / 100.0 } else { l }, c, h);
    value = value.with_alpha(alpha);
    let (r, g, b, a) = value.to_rgba8();
    Some([r, g, b, a])
}

fn parse_alpha(input: &str) -> Option<f32> {
    let trimmed = input.trim();
    if let Some(percent) = trimmed.strip_suffix('%') {
        percent.parse::<f32>().ok().map(|value| value / 100.0)
    } else {
        trimmed.parse::<f32>().ok()
    }
}
