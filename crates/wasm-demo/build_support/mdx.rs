use super::model::{BodyBlock, DocPage, SidebarSection, StepItem, TabItem, TocItem};
use super::text::{attr_value, frontmatter_value, inner_text, split_frontmatter};
use regex::Regex;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub fn parse_doc_page(
    path: &Path,
    sidebar: &[SidebarSection],
    sources: &BTreeMap<String, String>,
) -> DocPage {
    let raw = fs::read_to_string(path).unwrap();
    let (frontmatter, body) = split_frontmatter(&raw).unwrap_or(("", raw.as_str()));
    let title = frontmatter_value(frontmatter, "title").unwrap_or_else(|| {
        path.file_stem()
            .and_then(|value| value.to_str())
            .unwrap()
            .to_owned()
    });
    let description = frontmatter_value(frontmatter, "description").unwrap_or_default();
    let slug = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap()
        .to_owned();
    let mut toc = Vec::new();
    let lines = body.lines().map(ToOwned::to_owned).collect::<Vec<_>>();
    let mut index = 0;
    let blocks = parse_blocks(&lines, &mut index, &mut toc, sources);

    DocPage {
        slug,
        title,
        description,
        sidebar: sidebar.to_vec(),
        toc,
        blocks,
    }
}

fn parse_blocks(
    lines: &[String],
    index: &mut usize,
    toc: &mut Vec<TocItem>,
    sources: &BTreeMap<String, String>,
) -> Vec<BodyBlock> {
    let mut blocks = Vec::new();

    while *index < lines.len() {
        let line = lines[*index].trim();

        if line.is_empty() {
            *index += 1;
            continue;
        }
        if matches!(line, "</Tabs>" | "</TabsContent>" | "</Steps>" | "</Step>") {
            break;
        }
        if line.starts_with("```") {
            blocks.push(parse_code_fence(lines, index));
            continue;
        }
        if line.starts_with("## ") || line.starts_with("### ") || line.starts_with("#### ") {
            let (level, prefix_len) = if line.starts_with("#### ") {
                (4, 5)
            } else if line.starts_with("### ") {
                (3, 4)
            } else {
                (2, 3)
            };
            let text = line[prefix_len..].trim().to_owned();
            toc.push(TocItem {
                level,
                title: text.clone(),
            });
            blocks.push(BodyBlock::Heading { level, text });
            *index += 1;
            continue;
        }
        if line.starts_with("<ComponentPreview ") {
            let name = attr_value(line, "name").unwrap();
            blocks.push(BodyBlock::PreviewRef { name });
            *index += 1;
            continue;
        }
        if line.starts_with("<ComponentSource ") {
            let name = attr_value(line, "name").unwrap();
            let title = attr_value(line, "title");
            let code = sources.get(&name).cloned();
            blocks.push(BodyBlock::SourceRef { title, code });
            *index += 1;
            continue;
        }
        if line == "<ComponentsList />" {
            blocks.push(BodyBlock::ComponentsList);
            *index += 1;
            continue;
        }
        if line.starts_with("<Tabs ") {
            blocks.push(parse_tabs(lines, index, toc, sources));
            continue;
        }
        if line == "<Steps>" {
            blocks.push(parse_steps(lines, index, toc, sources));
            continue;
        }
        if line.starts_with('|') {
            blocks.push(parse_table(lines, index));
            continue;
        }
        if line.starts_with("- ") {
            blocks.push(parse_list(lines, index));
            continue;
        }
        if line.starts_with('>') {
            blocks.push(parse_note(lines, index));
            continue;
        }
        if line.starts_with("<div")
            || line == "</div>"
            || line.starts_with("<p>")
            || line == "</p>"
            || line == "<>"
            || line == "</>"
        {
            *index += 1;
            continue;
        }
        if line.starts_with('<') {
            panic!("Unsupported MDX block: {line}");
        }

        blocks.push(parse_paragraph(lines, index));
    }

    blocks
}

fn parse_code_fence(lines: &[String], index: &mut usize) -> BodyBlock {
    let header = lines[*index].trim().trim_start_matches("```").trim();
    let mut language = None;
    let mut title = None;

    if !header.is_empty() {
        let parts = header.split_whitespace().collect::<Vec<_>>();
        language = parts.first().map(|value| value.to_string());

        let title_re = Regex::new(r#"title="([^"]+)""#).unwrap();
        if let Some(captures) = title_re.captures(header) {
            title = Some(captures[1].to_owned());
        }
    }

    *index += 1;
    let mut code = Vec::new();
    while *index < lines.len() {
        if lines[*index].trim().starts_with("```") {
            *index += 1;
            break;
        }
        code.push(lines[*index].clone());
        *index += 1;
    }

    BodyBlock::CodeFence {
        language,
        title,
        code: code.join("\n"),
    }
}

fn parse_tabs(
    lines: &[String],
    index: &mut usize,
    toc: &mut Vec<TocItem>,
    sources: &BTreeMap<String, String>,
) -> BodyBlock {
    let default_value = attr_value(lines[*index].trim(), "defaultValue");
    *index += 1;

    let mut labels = BTreeMap::new();
    let mut items = Vec::new();

    while *index < lines.len() {
        let line = lines[*index].trim();
        if line == "</Tabs>" {
            *index += 1;
            break;
        }
        if line.starts_with("<TabsTrigger ") {
            let value = attr_value(line, "value").unwrap_or_default();
            let label = inner_text(line).unwrap_or_else(|| value.clone());
            labels.insert(value, label);
            *index += 1;
            continue;
        }
        if line.starts_with("<TabsContent ") {
            let value = attr_value(line, "value").unwrap_or_default();
            *index += 1;
            let blocks = parse_blocks(lines, index, toc, sources);
            if *index < lines.len() && lines[*index].trim() == "</TabsContent>" {
                *index += 1;
            }
            let label = labels.get(&value).cloned().unwrap_or_else(|| value.clone());
            items.push(TabItem {
                value,
                label,
                blocks,
            });
            continue;
        }
        *index += 1;
    }

    BodyBlock::Tabs {
        default_value,
        items,
    }
}

fn parse_steps(
    lines: &[String],
    index: &mut usize,
    toc: &mut Vec<TocItem>,
    sources: &BTreeMap<String, String>,
) -> BodyBlock {
    *index += 1;
    let mut steps = Vec::<StepItem>::new();

    while *index < lines.len() {
        let line = lines[*index].trim();
        if line == "</Steps>" {
            *index += 1;
            break;
        }
        if line.is_empty() {
            *index += 1;
            continue;
        }
        if line.starts_with("<Step>") && line.ends_with("</Step>") {
            let title = line
                .trim_start_matches("<Step>")
                .trim_end_matches("</Step>")
                .trim()
                .to_owned();
            steps.push(StepItem {
                title,
                blocks: Vec::new(),
            });
            *index += 1;
            continue;
        }
        if line == "<Step>" {
            *index += 1;
            let blocks = parse_blocks(lines, index, toc, sources);
            if *index < lines.len() && lines[*index].trim() == "</Step>" {
                *index += 1;
            }
            let title = blocks
                .iter()
                .find_map(|block| match block {
                    BodyBlock::Paragraph { text } => Some(text.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| "Step".to_owned());
            let mut blocks = blocks;
            if matches!(blocks.first(), Some(BodyBlock::Paragraph { .. })) {
                blocks.remove(0);
            }
            steps.push(StepItem { title, blocks });
            continue;
        }

        let block = parse_single_block(lines, index, toc, sources);
        if let Some(step) = steps.last_mut() {
            step.blocks.push(block);
        }
    }

    BodyBlock::Steps { items: steps }
}

fn parse_single_block(
    lines: &[String],
    index: &mut usize,
    toc: &mut Vec<TocItem>,
    sources: &BTreeMap<String, String>,
) -> BodyBlock {
    let line = lines[*index].trim();
    if line.is_empty() {
        *index += 1;
        return parse_single_block(lines, index, toc, sources);
    }
    if line.starts_with("<div")
        || line == "</div>"
        || line.starts_with("<p>")
        || line == "</p>"
        || line == "<>"
        || line == "</>"
    {
        *index += 1;
        return parse_single_block(lines, index, toc, sources);
    }
    if line.starts_with("```") {
        return parse_code_fence(lines, index);
    }
    if line.starts_with("## ") || line.starts_with("### ") || line.starts_with("#### ") {
        let (level, prefix_len) = if line.starts_with("#### ") {
            (4, 5)
        } else if line.starts_with("### ") {
            (3, 4)
        } else {
            (2, 3)
        };
        let text = line[prefix_len..].trim().to_owned();
        toc.push(TocItem {
            level,
            title: text.clone(),
        });
        *index += 1;
        return BodyBlock::Heading { level, text };
    }
    if line.starts_with("<ComponentPreview ") {
        let name = attr_value(line, "name").unwrap();
        *index += 1;
        return BodyBlock::PreviewRef { name };
    }
    if line.starts_with("<ComponentSource ") {
        let title = attr_value(line, "title");
        let code = sources.get(&attr_value(line, "name").unwrap()).cloned();
        *index += 1;
        return BodyBlock::SourceRef { title, code };
    }
    if line.starts_with('|') {
        return parse_table(lines, index);
    }
    if line.starts_with("- ") {
        return parse_list(lines, index);
    }
    if line.starts_with('>') {
        return parse_note(lines, index);
    }
    if line.starts_with('<') {
        panic!("Unsupported MDX block inside <Steps>: {line}");
    }
    parse_paragraph(lines, index)
}

fn parse_table(lines: &[String], index: &mut usize) -> BodyBlock {
    let mut rows = Vec::new();
    while *index < lines.len() {
        let line = lines[*index].trim();
        if !line.starts_with('|') {
            break;
        }
        rows.push(
            line.trim_matches('|')
                .split('|')
                .map(|value| value.trim().to_owned())
                .collect::<Vec<_>>(),
        );
        *index += 1;
    }

    let headers = rows.first().cloned().unwrap_or_default();
    let body_rows = rows.into_iter().skip(2).collect::<Vec<_>>();

    BodyBlock::PropsTable {
        headers,
        rows: body_rows,
    }
}

fn parse_list(lines: &[String], index: &mut usize) -> BodyBlock {
    let mut items = Vec::new();
    while *index < lines.len() {
        let line = lines[*index].trim();
        if let Some(stripped) = line.strip_prefix("- ") {
            items.push(stripped.trim().to_owned());
            *index += 1;
        } else {
            break;
        }
    }
    BodyBlock::List { items }
}

fn parse_note(lines: &[String], index: &mut usize) -> BodyBlock {
    let mut text = Vec::new();
    while *index < lines.len() {
        let line = lines[*index].trim();
        if let Some(stripped) = line.strip_prefix('>') {
            text.push(stripped.trim().to_owned());
            *index += 1;
        } else {
            break;
        }
    }
    BodyBlock::Note {
        text: text.join(" "),
    }
}

fn parse_paragraph(lines: &[String], index: &mut usize) -> BodyBlock {
    let mut parts = Vec::new();
    let start = *index;
    while *index < lines.len() {
        let line = lines[*index].trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with("```")
            || line.starts_with('<')
            || line.starts_with('|')
            || line.starts_with("- ")
            || line.starts_with('>')
        {
            break;
        }
        parts.push(line.to_owned());
        *index += 1;
    }
    if *index == start {
        panic!("Parser stalled on paragraph line: {}", lines[*index].trim());
    }
    BodyBlock::Paragraph {
        text: parts.join(" "),
    }
}
