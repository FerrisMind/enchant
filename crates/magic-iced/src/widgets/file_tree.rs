use iced::widget::container;
use iced::{Element, Length};
use iced_shadcn::{
    Theme, TreeNode, TreeScrollbarVisibility, TreeViewAction, TreeViewProps, TreeViewState,
    tree_view,
};
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FileTreeKind {
    #[default]
    File,
    Folder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FileTreeSort {
    None,
    #[default]
    FoldersFirstNatural,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileTreeElement {
    pub id: String,
    pub name: String,
    pub kind: FileTreeKind,
    pub children: Vec<FileTreeElement>,
}

impl FileTreeElement {
    pub fn file(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind: FileTreeKind::File,
            children: Vec::new(),
        }
    }

    pub fn folder<I>(id: impl Into<String>, name: impl Into<String>, children: I) -> Self
    where
        I: IntoIterator<Item = FileTreeElement>,
    {
        Self {
            id: id.into(),
            name: name.into(),
            kind: FileTreeKind::Folder,
            children: children.into_iter().collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FileTreeProps {
    pub width: Length,
    pub height: Length,
    pub indent: f32,
    pub icon_size: f32,
    pub font_size: f32,
    pub row_height: f32,
    pub max_label_chars: usize,
    pub content_offset: f32,
    pub sort: FileTreeSort,
    pub scrollbar_visibility: TreeScrollbarVisibility,
}

impl Default for FileTreeProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            height: Length::Shrink,
            indent: 16.0,
            icon_size: 16.0,
            font_size: 13.0,
            row_height: 28.0,
            max_label_chars: 30,
            content_offset: 0.0,
            sort: FileTreeSort::FoldersFirstNatural,
            scrollbar_visibility: TreeScrollbarVisibility::Auto,
        }
    }
}

impl FileTreeProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn sort(mut self, sort: FileTreeSort) -> Self {
        self.sort = sort;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FileTreeState {
    pub selected_id: Option<String>,
    pub expanded_ids: Vec<String>,
}

impl FileTreeState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_selected_id(mut self, selected_id: impl Into<String>) -> Self {
        self.selected_id = Some(selected_id.into());
        self
    }

    pub fn with_expanded_ids<I, S>(mut self, expanded_ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.expanded_ids = expanded_ids.into_iter().map(Into::into).collect();
        self
    }

    pub fn expand_all(mut self, elements: &[FileTreeElement]) -> Self {
        self.expanded_ids = collect_folder_ids(elements);
        self
    }

    pub fn collapse_all(mut self) -> Self {
        self.expanded_ids.clear();
        self
    }

    pub fn apply_action(&mut self, action: &FileTreeAction) {
        match action {
            FileTreeAction::ToggleFolder(id) => {
                if let Some(index) = self.expanded_ids.iter().position(|item| item == id) {
                    self.expanded_ids.remove(index);
                } else {
                    self.expanded_ids.push(id.clone());
                }
            }
            FileTreeAction::SelectFile(id) => {
                self.selected_id = Some(id.clone());
            }
            FileTreeAction::LoadFolder(id) => {
                if !self.expanded_ids.iter().any(|item| item == id) {
                    self.expanded_ids.push(id.clone());
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileTreeAction {
    ToggleFolder(String),
    SelectFile(String),
    LoadFolder(String),
}

pub fn file_tree<'a, Message: Clone + 'static>(
    elements: &[FileTreeElement],
    state: &FileTreeState,
    on_action: impl Fn(FileTreeAction) -> Message + Clone + 'static,
    props: FileTreeProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let sorted = sort_elements(elements, props.sort);
    let mut name_path_to_id = HashMap::new();
    let mut id_to_name_path = HashMap::new();
    let nodes = to_tree_nodes(&sorted, "", &mut name_path_to_id, &mut id_to_name_path);

    let selected = state
        .selected_id
        .as_ref()
        .and_then(|id| id_to_name_path.get(id))
        .cloned();
    let open_folders = state
        .expanded_ids
        .iter()
        .filter_map(|id| id_to_name_path.get(id).cloned())
        .collect();

    let tree_state = TreeViewState {
        open_folders,
        selected,
    };
    let tree_props = TreeViewProps::new()
        .indent(props.indent)
        .icon_size(props.icon_size)
        .font_size(props.font_size)
        .row_height(props.row_height)
        .max_label_chars(props.max_label_chars)
        .content_offset(props.content_offset)
        .scrollbar_visibility(props.scrollbar_visibility);

    let name_path_to_id = name_path_to_id.clone();

    container(tree_view(
        nodes,
        tree_state,
        move |action| on_action(map_tree_action(action, &name_path_to_id)),
        tree_props,
        theme,
    ))
    .width(props.width)
    .height(props.height)
    .into()
}

fn map_tree_action(
    action: TreeViewAction,
    name_path_to_id: &HashMap<String, String>,
) -> FileTreeAction {
    match action {
        TreeViewAction::ToggleFolder(path) => {
            FileTreeAction::ToggleFolder(name_path_to_id.get(&path).cloned().unwrap_or(path))
        }
        TreeViewAction::SelectFile(path) => {
            FileTreeAction::SelectFile(name_path_to_id.get(&path).cloned().unwrap_or(path))
        }
        TreeViewAction::LoadFolder(path) => {
            FileTreeAction::LoadFolder(name_path_to_id.get(&path).cloned().unwrap_or(path))
        }
    }
}

fn sort_elements(elements: &[FileTreeElement], sort: FileTreeSort) -> Vec<FileTreeElement> {
    let mut sorted = elements
        .iter()
        .map(|element| FileTreeElement {
            id: element.id.clone(),
            name: element.name.clone(),
            kind: element.kind,
            children: sort_elements(&element.children, sort),
        })
        .collect::<Vec<_>>();

    if matches!(sort, FileTreeSort::FoldersFirstNatural) {
        sorted.sort_by(file_tree_cmp);
    }

    sorted
}

fn file_tree_cmp(left: &FileTreeElement, right: &FileTreeElement) -> Ordering {
    match (left.kind, right.kind) {
        (FileTreeKind::Folder, FileTreeKind::File) => Ordering::Less,
        (FileTreeKind::File, FileTreeKind::Folder) => Ordering::Greater,
        _ => natural_cmp(&left.name, &right.name),
    }
}

fn natural_cmp(left: &str, right: &str) -> Ordering {
    let left_segments = tokenize_natural(left);
    let right_segments = tokenize_natural(right);

    for (left_segment, right_segment) in left_segments.iter().zip(right_segments.iter()) {
        let ordering = match (left_segment, right_segment) {
            (NaturalSegment::Number(left, _), NaturalSegment::Number(right, _)) => left
                .cmp(right)
                .then_with(|| left_segment.as_str().cmp(right_segment.as_str())),
            _ => left_segment.as_str().cmp(right_segment.as_str()),
        };

        if ordering != Ordering::Equal {
            return ordering;
        }
    }

    left_segments.len().cmp(&right_segments.len())
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum NaturalSegment {
    Text(String),
    Number(u64, String),
}

impl NaturalSegment {
    fn as_str(&self) -> &str {
        match self {
            Self::Text(value) => value.as_str(),
            Self::Number(_, raw) => raw.as_str(),
        }
    }
}

fn tokenize_natural(value: &str) -> Vec<NaturalSegment> {
    let mut segments = Vec::new();
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.peek().copied() {
        if ch.is_ascii_digit() {
            let mut raw = String::new();
            while let Some(next) = chars.peek().copied() {
                if !next.is_ascii_digit() {
                    break;
                }
                raw.push(next);
                let _ = chars.next();
            }

            let parsed = raw.parse::<u64>().unwrap_or(u64::MAX);
            segments.push(NaturalSegment::Number(parsed, raw));
        } else {
            let mut raw = String::new();
            while let Some(next) = chars.peek().copied() {
                if next.is_ascii_digit() {
                    break;
                }
                raw.push(next.to_ascii_lowercase());
                let _ = chars.next();
            }

            segments.push(NaturalSegment::Text(raw));
        }
    }

    segments
}

fn to_tree_nodes(
    elements: &[FileTreeElement],
    prefix: &str,
    name_path_to_id: &mut HashMap<String, String>,
    id_to_name_path: &mut HashMap<String, String>,
) -> Vec<TreeNode> {
    elements
        .iter()
        .map(|element| {
            let path = if prefix.is_empty() {
                element.name.clone()
            } else {
                format!("{prefix}/{}", element.name)
            };
            name_path_to_id.insert(path.clone(), element.id.clone());
            id_to_name_path.insert(element.id.clone(), path.clone());

            match element.kind {
                FileTreeKind::Folder => TreeNode::folder(
                    element.name.clone(),
                    to_tree_nodes(&element.children, &path, name_path_to_id, id_to_name_path),
                ),
                FileTreeKind::File => TreeNode::file(element.name.clone()),
            }
        })
        .collect()
}

fn collect_folder_ids(elements: &[FileTreeElement]) -> Vec<String> {
    let mut ids = Vec::new();

    for element in elements {
        if matches!(element.kind, FileTreeKind::Folder) {
            ids.push(element.id.clone());
            ids.extend(collect_folder_ids(&element.children));
        }
    }

    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_sort_orders_folders_first_and_numeric_suffixes_naturally() {
        let sorted = sort_elements(
            &[
                FileTreeElement::file("file-10", "file10.ts"),
                FileTreeElement::folder("src", "src", []),
                FileTreeElement::file("file-2", "file2.ts"),
            ],
            FileTreeSort::FoldersFirstNatural,
        );

        assert_eq!(sorted[0].id, "src");
        assert_eq!(sorted[1].id, "file-2");
        assert_eq!(sorted[2].id, "file-10");
    }

    #[test]
    fn expand_all_collects_folder_ids_only() {
        let ids = collect_folder_ids(&[
            FileTreeElement::folder(
                "src",
                "src",
                [FileTreeElement::folder("nested", "nested", [])],
            ),
            FileTreeElement::file("readme", "README.md"),
        ]);

        assert_eq!(ids, vec!["src".to_string(), "nested".to_string()]);
    }

    #[test]
    fn tree_action_mapping_uses_element_ids() {
        let sorted = sort_elements(
            &[FileTreeElement::folder(
                "src-id",
                "src",
                [FileTreeElement::file("button-id", "button.tsx")],
            )],
            FileTreeSort::FoldersFirstNatural,
        );
        let mut name_path_to_id = HashMap::new();
        let mut id_to_name_path = HashMap::new();
        let _ = to_tree_nodes(&sorted, "", &mut name_path_to_id, &mut id_to_name_path);

        let mapped = map_tree_action(
            TreeViewAction::SelectFile("src/button.tsx".to_string()),
            &name_path_to_id,
        );

        assert_eq!(mapped, FileTreeAction::SelectFile("button-id".to_string()));
    }
}
