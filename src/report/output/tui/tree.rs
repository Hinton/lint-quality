//! Directory tree data structures: build a hierarchical tree from flat
//! directory/file counts, then flatten into visible rows based on expand state.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

/// A single visible row in the flattened directory tree.
#[derive(Clone)]
pub struct DirTreeRow {
    pub depth: usize,
    pub name: String,
    pub full_path: String,
    pub count: usize,
    pub has_children: bool,
    pub expanded: bool,
    pub is_file: bool,
}

/// Intermediate tree node used during construction.
struct DirNode {
    segment: String,
    full_path: String,
    count: usize,
    children: BTreeMap<String, DirNode>,
    files: Vec<(String, String, usize)>,
}

impl DirNode {
    fn new(segment: String, full_path: String, count: usize) -> Self {
        Self {
            segment,
            full_path,
            count,
            children: BTreeMap::new(),
            files: Vec::new(),
        }
    }
}

/// Build a tree from directory counts + file counts, then flatten visible rows.
pub fn build_dir_tree(
    dir_counts: &HashMap<String, usize>,
    file_entries: &[(String, usize)],
    expanded: &HashSet<String>,
) -> Vec<DirTreeRow> {
    if dir_counts.is_empty() && file_entries.is_empty() {
        return Vec::new();
    }

    let mut roots: BTreeMap<String, DirNode> = BTreeMap::new();

    for (path, &count) in dir_counts {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.is_empty() {
            continue;
        }

        let root_key = parts[0].to_string();
        let root_full = root_key.clone();
        let root = roots
            .entry(root_key.clone())
            .or_insert_with(|| DirNode::new(root_key, root_full, 0));

        if parts.len() == 1 {
            root.count = count;
        } else {
            let mut current = root;
            for i in 1..parts.len() {
                let seg = parts[i].to_string();
                let full = parts[..=i].join("/");
                let child_count = if i == parts.len() - 1 { count } else { 0 };
                current = current
                    .children
                    .entry(seg.clone())
                    .or_insert_with(|| DirNode::new(seg, full, child_count));
                if i == parts.len() - 1 {
                    current.count = count;
                }
            }
        }
    }

    // Insert files into their parent directory nodes
    for (file_path, count) in file_entries {
        let path = Path::new(file_path);
        let filename = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| file_path.clone());
        let parent = path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        if parent.is_empty() {
            continue;
        }

        let parts: Vec<&str> = parent.split('/').collect();
        let root_key = parts[0].to_string();
        if let Some(root) = roots.get_mut(&root_key) {
            let mut current = root;
            for &seg in &parts[1..] {
                if let Some(child) = current.children.get_mut(seg) {
                    current = child;
                } else {
                    break;
                }
            }
            current.files.push((filename, file_path.clone(), *count));
        }
    }

    sort_files_recursive(roots.values_mut());

    let mut rows = Vec::new();
    for node in roots.values() {
        flatten_node(node, 0, expanded, &mut rows);
    }
    rows
}

fn sort_files_recursive<'a>(nodes: impl Iterator<Item = &'a mut DirNode>) {
    for node in nodes {
        node.files.sort_by(|a, b| b.2.cmp(&a.2));
        sort_files_recursive(node.children.values_mut());
    }
}

fn flatten_node(
    node: &DirNode,
    depth: usize,
    expanded: &HashSet<String>,
    rows: &mut Vec<DirTreeRow>,
) {
    let has_children = !node.children.is_empty() || !node.files.is_empty();
    let is_expanded = expanded.contains(&node.full_path);

    rows.push(DirTreeRow {
        depth,
        name: node.segment.clone(),
        full_path: node.full_path.clone(),
        count: node.count,
        has_children,
        expanded: is_expanded,
        is_file: false,
    });

    if is_expanded {
        let mut children: Vec<_> = node.children.values().collect();
        children.sort_by(|a, b| b.count.cmp(&a.count));
        for child in children {
            flatten_node(child, depth + 1, expanded, rows);
        }

        for (filename, full_path, count) in &node.files {
            rows.push(DirTreeRow {
                depth: depth + 1,
                name: filename.clone(),
                full_path: full_path.clone(),
                count: *count,
                has_children: false,
                expanded: false,
                is_file: true,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir_counts(entries: &[(&str, usize)]) -> HashMap<String, usize> {
        entries.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn empty_inputs() {
        let rows = build_dir_tree(&HashMap::new(), &[], &HashSet::new());
        assert!(rows.is_empty());
    }

    #[test]
    fn single_directory_collapsed() {
        let dirs = dir_counts(&[("src", 5)]);
        let rows = build_dir_tree(&dirs, &[], &HashSet::new());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "src");
        assert_eq!(rows[0].count, 5);
        assert_eq!(rows[0].depth, 0);
        assert!(!rows[0].expanded);
        assert!(!rows[0].is_file);
    }

    #[test]
    fn nested_dirs_expand_reveals_children() {
        let dirs = dir_counts(&[("src", 10), ("src/components", 6), ("src/utils", 4)]);
        let expanded: HashSet<String> = ["src".to_string()].into();
        let rows = build_dir_tree(&dirs, &[], &expanded);

        assert_eq!(rows.len(), 3);
        // Root
        assert_eq!(rows[0].name, "src");
        assert_eq!(rows[0].depth, 0);
        assert!(rows[0].expanded);
        // Children sorted by count desc
        assert_eq!(rows[1].name, "components");
        assert_eq!(rows[1].depth, 1);
        assert_eq!(rows[1].count, 6);
        assert_eq!(rows[2].name, "utils");
        assert_eq!(rows[2].depth, 1);
        assert_eq!(rows[2].count, 4);
    }

    #[test]
    fn collapsed_hides_children() {
        let dirs = dir_counts(&[("src", 10), ("src/components", 6)]);
        let rows = build_dir_tree(&dirs, &[], &HashSet::new());
        // Only root is visible
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "src");
        assert!(rows[0].has_children);
    }

    #[test]
    fn files_appear_under_expanded_dir() {
        let dirs = dir_counts(&[("src", 5)]);
        let files = vec![
            ("src/foo.ts".to_string(), 3usize),
            ("src/bar.ts".to_string(), 2),
        ];
        let expanded: HashSet<String> = ["src".to_string()].into();
        let rows = build_dir_tree(&dirs, &files, &expanded);

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].name, "src");
        assert!(!rows[0].is_file);
        // Files sorted by count desc
        assert_eq!(rows[1].name, "foo.ts");
        assert!(rows[1].is_file);
        assert_eq!(rows[1].count, 3);
        assert_eq!(rows[2].name, "bar.ts");
        assert!(rows[2].is_file);
        assert_eq!(rows[2].count, 2);
    }

    #[test]
    fn files_hidden_when_collapsed() {
        let dirs = dir_counts(&[("src", 5)]);
        let files = vec![("src/foo.ts".to_string(), 3)];
        let rows = build_dir_tree(&dirs, &files, &HashSet::new());
        assert_eq!(rows.len(), 1);
        assert!(rows[0].has_children);
    }

    #[test]
    fn subdirs_before_files() {
        let dirs = dir_counts(&[("src", 10), ("src/lib", 3)]);
        let files = vec![("src/main.rs".to_string(), 7)];
        let expanded: HashSet<String> = ["src".to_string()].into();
        let rows = build_dir_tree(&dirs, &files, &expanded);

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].name, "src");
        // Subdir first
        assert_eq!(rows[1].name, "lib");
        assert!(!rows[1].is_file);
        // Then file
        assert_eq!(rows[2].name, "main.rs");
        assert!(rows[2].is_file);
    }

    #[test]
    fn deeply_nested_expand() {
        let dirs = dir_counts(&[("a", 10), ("a/b", 8), ("a/b/c", 5)]);
        let expanded: HashSet<String> = ["a".to_string(), "a/b".to_string()].into();
        let rows = build_dir_tree(&dirs, &[], &expanded);

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].depth, 0);
        assert_eq!(rows[0].name, "a");
        assert_eq!(rows[1].depth, 1);
        assert_eq!(rows[1].name, "b");
        assert_eq!(rows[2].depth, 2);
        assert_eq!(rows[2].name, "c");
    }
}
