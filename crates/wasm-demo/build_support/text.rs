use regex::Regex;

pub fn frontmatter_value(frontmatter: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    frontmatter.lines().find_map(|line| {
        line.trim()
            .strip_prefix(&prefix)
            .map(|value| clean_ts_value(value.trim()))
    })
}

pub fn split_frontmatter(input: &str) -> Option<(&str, &str)> {
    if !input.starts_with("---") {
        return None;
    }
    let mut parts = input.splitn(3, "---");
    let _ = parts.next()?;
    let frontmatter = parts.next()?.trim();
    let body = parts.next()?.trim_start();
    Some((frontmatter, body))
}

pub fn parse_key_value(line: &str) -> Option<(String, String)> {
    let (key, value) = line.split_once(':')?;
    Some((
        key.trim().to_owned(),
        clean_ts_value(value.trim().trim_end_matches(',')),
    ))
}

fn clean_ts_value(value: &str) -> String {
    value.trim_matches('"').trim_matches('`').trim().to_owned()
}

pub fn capture_site_link(site_ts: &str, key: &str) -> String {
    let pattern = format!(r#"{key}:\s*"([^"]+)""#);
    Regex::new(&pattern)
        .unwrap()
        .captures(site_ts)
        .map(|captures| captures[1].to_owned())
        .unwrap_or_default()
}

pub fn parse_search_placeholder(input: &str) -> String {
    Regex::new(r#">Search documentation\.\.\.<"#)
        .unwrap()
        .find(input)
        .map(|_| "Search documentation...".to_owned())
        .unwrap_or_else(|| "Search documentation...".to_owned())
}

pub fn capture_jsx_text(input: &str, tag: &str) -> Option<String> {
    capture_nth_jsx_text(input, tag, 0)
}

pub fn capture_tag_block(input: &str, tag: &str, index: usize) -> Option<String> {
    let pattern = format!(r#"<{tag}[^>]*>(?s:(.*?))</{tag}>"#);
    let re = Regex::new(&pattern).unwrap();
    re.captures_iter(input)
        .nth(index)
        .map(|captures| captures[1].to_owned())
}

pub fn capture_nth_jsx_text(input: &str, tag: &str, index: usize) -> Option<String> {
    let pattern = format!(r#"<{tag}[^>]*>(?s:(.*?))</{tag}>"#);
    let re = Regex::new(&pattern).unwrap();
    re.captures_iter(input)
        .nth(index)
        .map(|captures| normalize_jsx_text(&captures[1]))
}

pub fn capture_first_href(input: &str) -> Option<String> {
    Regex::new(r#"href="([^"]+)""#)
        .unwrap()
        .captures(input)
        .map(|captures| captures[1].to_owned())
}

pub fn capture_link_href(input: &str, label: &str) -> Option<String> {
    let link_re = Regex::new(r#"(?s)<Link[^>]*href="([^"]+)"[^>]*>(.*?)</Link>"#).unwrap();
    link_re.captures_iter(input).find_map(|captures| {
        let text = normalize_jsx_text(&captures[2]);
        text.contains(label).then(|| captures[1].to_owned())
    })
}

pub fn normalize_jsx_text(input: &str) -> String {
    let tag_re = Regex::new(r#"<[^>]+>"#).unwrap();
    let expr_re = Regex::new(r#"\{[^}]+\}"#).unwrap();
    let without_tags = tag_re.replace_all(input, " ");
    let without_expr = expr_re.replace_all(&without_tags, " ");
    without_expr
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn capture_tech_stack(input: &str) -> Vec<String> {
    let re = Regex::new(r#""([a-z0-9-]+)""#).unwrap();
    if let Some(start) = input.find("technologies={[") {
        let tail = &input[start..];
        if let Some(end) = tail.find("]}") {
            return re
                .captures_iter(&tail[..end])
                .map(|captures| captures[1].to_owned())
                .collect();
        }
    }
    vec![
        "react".to_owned(),
        "typescript".to_owned(),
        "tailwindcss".to_owned(),
        "motion".to_owned(),
        "shadcn".to_owned(),
    ]
}

pub fn parse_tweet_ids(input: &str) -> Vec<String> {
    let re = Regex::new(r#"status/(\d+)"#).unwrap();
    re.captures_iter(input)
        .map(|captures| captures[1].to_owned())
        .collect()
}

pub fn parse_video_embeds(input: &str) -> Vec<String> {
    let re = Regex::new(r#""(https://www\.youtube\.com/embed/[^"]+)""#).unwrap();
    re.captures_iter(input)
        .map(|captures| captures[1].to_owned())
        .collect()
}

pub fn attr_value(input: &str, key: &str) -> Option<String> {
    let pattern = format!(r#"{key}="([^"]+)""#);
    Regex::new(&pattern)
        .unwrap()
        .captures(input)
        .map(|captures| captures[1].to_owned())
}

pub fn inner_text(input: &str) -> Option<String> {
    Regex::new(r#">([^<]+)<"#)
        .unwrap()
        .captures(input)
        .map(|captures| captures[1].trim().to_owned())
}

pub fn slugify(input: &str) -> String {
    input
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
