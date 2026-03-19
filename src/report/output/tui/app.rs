//! TUI application state: enums, filtering, navigation, and data rebuild logic.

use ratatui::widgets::TableState;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::report::Report;

use super::tree::{DirTreeRow, build_dir_tree};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FilterDimension {
    Pattern,
    Category,
    Rule,
    Owner,
}

pub const DIMENSIONS: &[FilterDimension] = &[
    FilterDimension::Pattern,
    FilterDimension::Category,
    FilterDimension::Rule,
    FilterDimension::Owner,
];

impl FilterDimension {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pattern => "Pattern",
            Self::Category => "Category",
            Self::Rule => "Rule",
            Self::Owner => "Owner",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DataView {
    Files,
    Rules,
    Patterns,
    Categories,
    Owners,
    Directories,
}

pub const DATA_VIEWS: &[DataView] = &[
    DataView::Files,
    DataView::Rules,
    DataView::Patterns,
    DataView::Categories,
    DataView::Owners,
    DataView::Directories,
];

pub const DATA_VIEW_COUNT: usize = 6;

impl DataView {
    pub fn label(self) -> &'static str {
        match self {
            Self::Files => "Files",
            Self::Rules => "Rules",
            Self::Patterns => "Patterns",
            Self::Categories => "Categories",
            Self::Owners => "Owners",
            Self::Directories => "Dirs",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Filters,
    Data,
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

pub struct App<'a> {
    pub report: &'a Report,
    pub focus: Focus,

    // Left panel
    pub dimension: usize,
    pub filter_list_state: TableState,
    pub dimension_values: Vec<Vec<(String, usize)>>,
    pub active_filters: Vec<HashSet<usize>>,

    // Right panel
    pub data_view: usize,
    pub data_states: [TableState; DATA_VIEW_COUNT],

    // Cached filtered data
    pub filtered_files: Vec<(String, String, usize)>,
    pub filtered_rules: Vec<(String, usize)>,
    pub filtered_patterns: Vec<(String, usize)>,
    pub filtered_categories: Vec<(String, usize)>,
    pub filtered_owners: Vec<(String, usize)>,
    pub filtered_dir_counts: HashMap<String, usize>,
    pub filtered_violation_count: usize,

    // Directory tree state
    pub expanded_dirs: HashSet<String>,
    pub dir_tree_rows: Vec<DirTreeRow>,
}

impl<'a> App<'a> {
    pub fn new(report: &'a Report) -> Self {
        let s = &report.summary;

        let mut patterns: Vec<_> = s.by_pattern.iter().map(|(k, v)| (k.clone(), *v)).collect();
        patterns.sort_by(|a, b| b.1.cmp(&a.1));

        let mut categories: Vec<_> = s.by_category.iter().map(|(k, v)| (k.clone(), *v)).collect();
        categories.sort_by(|a, b| b.1.cmp(&a.1));

        let mut rules: Vec<_> = s.by_rule.iter().map(|(k, v)| (k.clone(), *v)).collect();
        rules.sort_by(|a, b| b.1.cmp(&a.1));

        let mut owners: Vec<_> = s.by_owner.iter().map(|(k, v)| (k.clone(), *v)).collect();
        owners.sort_by(|a, b| b.1.cmp(&a.1));

        let dimension_values = vec![patterns, categories, rules, owners];
        let active_filters = vec![
            HashSet::new(),
            HashSet::new(),
            HashSet::new(),
            HashSet::new(),
        ];

        let mut filter_list_state = TableState::default();
        if !dimension_values[0].is_empty() {
            filter_list_state.select(Some(0));
        }

        let mut expanded_dirs = HashSet::new();
        for key in s.by_directory.keys() {
            if !key.contains('/') {
                expanded_dirs.insert(key.clone());
            }
        }

        let mut app = Self {
            report,
            focus: Focus::Filters,
            dimension: 0,
            filter_list_state,
            dimension_values,
            active_filters,
            data_view: 0,
            data_states: Default::default(),
            filtered_files: Vec::new(),
            filtered_rules: Vec::new(),
            filtered_patterns: Vec::new(),
            filtered_categories: Vec::new(),
            filtered_owners: Vec::new(),
            filtered_dir_counts: HashMap::new(),
            filtered_violation_count: 0,
            expanded_dirs,
            dir_tree_rows: Vec::new(),
        };
        app.rebuild_filtered_data();
        app
    }

    pub fn current_dim_values(&self) -> &[(String, usize)] {
        &self.dimension_values[self.dimension]
    }

    pub fn current_dim_len(&self) -> usize {
        self.dimension_values[self.dimension].len()
    }

    pub fn has_any_active_filter(&self) -> bool {
        self.active_filters.iter().any(|s| !s.is_empty())
    }

    pub fn active_filter_values(&self, dim: usize) -> HashSet<&str> {
        self.active_filters[dim]
            .iter()
            .filter_map(|&idx| self.dimension_values[dim].get(idx).map(|(s, _)| s.as_str()))
            .collect()
    }

    pub fn current_data_len(&self) -> usize {
        match DATA_VIEWS[self.data_view] {
            DataView::Files => self.filtered_files.len(),
            DataView::Rules => self.filtered_rules.len(),
            DataView::Patterns => self.filtered_patterns.len(),
            DataView::Categories => self.filtered_categories.len(),
            DataView::Owners => self.filtered_owners.len(),
            DataView::Directories => self.dir_tree_rows.len(),
        }
    }

    pub fn rebuild_filtered_data(&mut self) {
        let pattern_filter = self.active_filter_values(0);
        let category_filter = self.active_filter_values(1);
        let rule_filter = self.active_filter_values(2);
        let owner_filter = self.active_filter_values(3);

        let mut file_counts: HashMap<usize, usize> = HashMap::new();
        let mut rules_map: HashMap<String, usize> = HashMap::new();
        let mut patterns_map: HashMap<String, usize> = HashMap::new();
        let mut categories_map: HashMap<String, usize> = HashMap::new();
        let mut owners_map: HashMap<String, usize> = HashMap::new();
        let mut directories_map: HashMap<String, usize> = HashMap::new();
        let mut total = 0usize;

        for (fi, file) in self.report.files.iter().enumerate() {
            let file_owner = file.owner.as_deref().unwrap_or("@unowned");
            if !owner_filter.is_empty() && !owner_filter.contains(file_owner) {
                continue;
            }

            for v in &file.violations {
                if !pattern_filter.is_empty() && !pattern_filter.contains(v.pattern.as_str()) {
                    continue;
                }
                if !category_filter.is_empty() && !category_filter.contains(v.category.as_str()) {
                    continue;
                }
                if !rule_filter.is_empty()
                    && !v.rules.iter().any(|r| rule_filter.contains(r.as_str()))
                {
                    continue;
                }

                total += 1;
                *file_counts.entry(fi).or_default() += 1;
                *patterns_map.entry(v.pattern.clone()).or_default() += 1;
                *categories_map.entry(v.category.clone()).or_default() += 1;
                *owners_map.entry(file_owner.to_string()).or_default() += 1;

                for rule in &v.rules {
                    *rules_map.entry(rule.clone()).or_default() += 1;
                }

                if let Some(parent) = Path::new(&file.path).parent() {
                    let mut dir = parent.to_path_buf();
                    loop {
                        let dir_str = dir.to_string_lossy().to_string();
                        if dir_str.is_empty() {
                            break;
                        }
                        *directories_map.entry(dir_str).or_default() += 1;
                        if !dir.pop() {
                            break;
                        }
                    }
                }
            }
        }

        let mut files: Vec<_> = file_counts
            .into_iter()
            .map(|(fi, count)| {
                let f = &self.report.files[fi];
                let owner = f.owner.as_deref().unwrap_or("-").to_string();
                (f.path.clone(), owner, count)
            })
            .collect();
        files.sort_by(|a, b| b.2.cmp(&a.2));

        self.filtered_files = files;
        self.filtered_rules = sorted_vec(rules_map);
        self.filtered_patterns = sorted_vec(patterns_map);
        self.filtered_categories = sorted_vec(categories_map);
        self.filtered_owners = sorted_vec(owners_map);
        self.filtered_dir_counts = directories_map;
        self.filtered_violation_count = total;

        self.rebuild_dir_tree();

        for (i, state) in self.data_states.iter_mut().enumerate() {
            let len = match DATA_VIEWS[i] {
                DataView::Files => self.filtered_files.len(),
                DataView::Rules => self.filtered_rules.len(),
                DataView::Patterns => self.filtered_patterns.len(),
                DataView::Categories => self.filtered_categories.len(),
                DataView::Owners => self.filtered_owners.len(),
                DataView::Directories => self.dir_tree_rows.len(),
            };
            if len == 0 {
                state.select(None);
            } else {
                state.select(Some(0));
            }
        }
    }

    pub fn rebuild_dir_tree(&mut self) {
        let file_entries: Vec<(String, usize)> = self
            .filtered_files
            .iter()
            .map(|(path, _, count)| (path.clone(), *count))
            .collect();
        self.dir_tree_rows =
            build_dir_tree(&self.filtered_dir_counts, &file_entries, &self.expanded_dirs);
    }

    pub fn toggle_dir_expand(&mut self) {
        let view_idx = DATA_VIEWS
            .iter()
            .position(|v| *v == DataView::Directories)
            .unwrap();
        if let Some(selected) = self.data_states[view_idx].selected() {
            if let Some(row) = self.dir_tree_rows.get(selected) {
                if row.has_children {
                    let path = row.full_path.clone();
                    if self.expanded_dirs.contains(&path) {
                        self.expanded_dirs.remove(&path);
                    } else {
                        self.expanded_dirs.insert(path);
                    }
                    self.rebuild_dir_tree();
                    let len = self.dir_tree_rows.len();
                    if selected >= len {
                        self.data_states[view_idx].select(if len > 0 {
                            Some(len - 1)
                        } else {
                            None
                        });
                    }
                }
            }
        }
    }

    pub fn next_dimension(&mut self) {
        self.dimension = (self.dimension + 1) % DIMENSIONS.len();
        self.reset_filter_selection();
    }

    pub fn prev_dimension(&mut self) {
        self.dimension = (self.dimension + DIMENSIONS.len() - 1) % DIMENSIONS.len();
        self.reset_filter_selection();
    }

    pub fn reset_filter_selection(&mut self) {
        if self.current_dim_len() == 0 {
            self.filter_list_state.select(None);
        } else {
            self.filter_list_state.select(Some(0));
        }
    }

    pub fn next_data_view(&mut self) {
        self.data_view = (self.data_view + 1) % DATA_VIEWS.len();
    }

    pub fn prev_data_view(&mut self) {
        self.data_view = (self.data_view + DATA_VIEWS.len() - 1) % DATA_VIEWS.len();
    }

    pub fn toggle_filter(&mut self) {
        if let Some(idx) = self.filter_list_state.selected() {
            let set = &mut self.active_filters[self.dimension];
            if set.contains(&idx) {
                set.remove(&idx);
            } else {
                set.insert(idx);
            }
            self.rebuild_filtered_data();
        }
    }

    pub fn clear_filters(&mut self) {
        for set in &mut self.active_filters {
            set.clear();
        }
        self.rebuild_filtered_data();
    }

    pub fn scroll_down_filter(&mut self) {
        let len = self.current_dim_len();
        if len == 0 {
            return;
        }
        let i = self.filter_list_state.selected().map_or(0, |i| {
            if i >= len - 1 { 0 } else { i + 1 }
        });
        self.filter_list_state.select(Some(i));
    }

    pub fn scroll_up_filter(&mut self) {
        let len = self.current_dim_len();
        if len == 0 {
            return;
        }
        let i = self.filter_list_state.selected().map_or(0, |i| {
            if i == 0 { len - 1 } else { i - 1 }
        });
        self.filter_list_state.select(Some(i));
    }

    pub fn scroll_down_data(&mut self) {
        let len = self.current_data_len();
        if len == 0 {
            return;
        }
        let state = &mut self.data_states[self.data_view];
        let i = state.selected().map_or(0, |i| {
            if i >= len - 1 { 0 } else { i + 1 }
        });
        state.select(Some(i));
    }

    pub fn scroll_up_data(&mut self) {
        let len = self.current_data_len();
        if len == 0 {
            return;
        }
        let state = &mut self.data_states[self.data_view];
        let i = state.selected().map_or(0, |i| {
            if i == 0 { len - 1 } else { i - 1 }
        });
        state.select(Some(i));
    }
}

fn sorted_vec(map: HashMap<String, usize>) -> Vec<(String, usize)> {
    let mut v: Vec<_> = map.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::analysis::build_summary;
    use crate::scan::{FileReport, Violation};
    use chrono::Utc;
    use crate::report::{Report, ReportMetadata};

    fn make_violation(pattern: &str, category: &str, rules: &[&str]) -> Violation {
        Violation {
            line: 1,
            pattern: pattern.to_string(),
            category: category.to_string(),
            rules: rules.iter().map(|r| r.to_string()).collect(),
            raw_text: String::new(),
        }
    }

    fn make_report(files: Vec<FileReport>) -> Report {
        let summary = build_summary(&files);
        Report {
            metadata: ReportMetadata {
                timestamp: Utc::now(),
                tool_version: "test".to_string(),
                scanned_paths: vec![".".to_string()],
                config_path: None,
                files_scanned: 10,
                scan_duration_ms: 42,
            },
            files,
            summary,
        }
    }

    fn test_report() -> Report {
        make_report(vec![
            FileReport {
                path: "src/foo.ts".to_string(),
                owner: Some("@team-a".to_string()),
                violations: vec![
                    make_violation("eslint-disable-next-line", "eslint", &["no-any"]),
                    make_violation("ts-ignore", "typescript", &["*"]),
                ],
            },
            FileReport {
                path: "src/bar.ts".to_string(),
                owner: Some("@team-b".to_string()),
                violations: vec![
                    make_violation("eslint-disable-next-line", "eslint", &["no-unused-vars"]),
                ],
            },
            FileReport {
                path: "lib/utils.js".to_string(),
                owner: Some("@team-a".to_string()),
                violations: vec![
                    make_violation("eslint-disable", "eslint", &["*"]),
                    make_violation("eslint-disable", "eslint", &["no-any"]),
                ],
            },
        ])
    }

    #[test]
    fn initial_state_no_filters() {
        let report = test_report();
        let app = App::new(&report);

        assert_eq!(app.filtered_violation_count, 5);
        assert_eq!(app.filtered_files.len(), 3);
        assert!(!app.has_any_active_filter());
    }

    #[test]
    fn filter_by_pattern() {
        let report = test_report();
        let mut app = App::new(&report);

        // Find "eslint-disable-next-line" in the pattern dimension (index 0)
        let idx = app.dimension_values[0]
            .iter()
            .position(|(name, _)| name == "eslint-disable-next-line")
            .unwrap();
        app.active_filters[0].insert(idx);
        app.rebuild_filtered_data();

        assert_eq!(app.filtered_violation_count, 2);
        assert_eq!(app.filtered_files.len(), 2);
        assert!(app.has_any_active_filter());
    }

    #[test]
    fn filter_by_owner() {
        let report = test_report();
        let mut app = App::new(&report);

        // Owner dimension is index 3
        let idx = app.dimension_values[3]
            .iter()
            .position(|(name, _)| name == "@team-b")
            .unwrap();
        app.active_filters[3].insert(idx);
        app.rebuild_filtered_data();

        assert_eq!(app.filtered_violation_count, 1);
        assert_eq!(app.filtered_files.len(), 1);
        assert_eq!(app.filtered_files[0].0, "src/bar.ts");
    }

    #[test]
    fn filter_by_category() {
        let report = test_report();
        let mut app = App::new(&report);

        // Category dimension is index 1
        let idx = app.dimension_values[1]
            .iter()
            .position(|(name, _)| name == "typescript")
            .unwrap();
        app.active_filters[1].insert(idx);
        app.rebuild_filtered_data();

        assert_eq!(app.filtered_violation_count, 1);
        assert_eq!(app.filtered_files.len(), 1);
    }

    #[test]
    fn filter_by_rule() {
        let report = test_report();
        let mut app = App::new(&report);

        // Rule dimension is index 2
        let idx = app.dimension_values[2]
            .iter()
            .position(|(name, _)| name == "no-any")
            .unwrap();
        app.active_filters[2].insert(idx);
        app.rebuild_filtered_data();

        // "no-any" appears in foo.ts and utils.js
        assert_eq!(app.filtered_violation_count, 2);
    }

    #[test]
    fn cross_dimension_filters_are_anded() {
        let report = test_report();
        let mut app = App::new(&report);

        // Filter: owner=@team-a AND category=typescript
        let owner_idx = app.dimension_values[3]
            .iter()
            .position(|(name, _)| name == "@team-a")
            .unwrap();
        let cat_idx = app.dimension_values[1]
            .iter()
            .position(|(name, _)| name == "typescript")
            .unwrap();
        app.active_filters[3].insert(owner_idx);
        app.active_filters[1].insert(cat_idx);
        app.rebuild_filtered_data();

        // Only the ts-ignore in foo.ts (owned by @team-a)
        assert_eq!(app.filtered_violation_count, 1);
        assert_eq!(app.filtered_files[0].0, "src/foo.ts");
    }

    #[test]
    fn clear_filters_restores_all() {
        let report = test_report();
        let mut app = App::new(&report);

        let idx = app.dimension_values[3]
            .iter()
            .position(|(name, _)| name == "@team-b")
            .unwrap();
        app.active_filters[3].insert(idx);
        app.rebuild_filtered_data();
        assert_eq!(app.filtered_violation_count, 1);

        app.clear_filters();
        assert_eq!(app.filtered_violation_count, 5);
        assert!(!app.has_any_active_filter());
    }

    #[test]
    fn filtered_aggregations_are_correct() {
        let report = test_report();
        let mut app = App::new(&report);

        // Filter to @team-a only
        let idx = app.dimension_values[3]
            .iter()
            .position(|(name, _)| name == "@team-a")
            .unwrap();
        app.active_filters[3].insert(idx);
        app.rebuild_filtered_data();

        assert_eq!(app.filtered_violation_count, 4);
        // Check that filtered_owners only has @team-a
        assert_eq!(app.filtered_owners.len(), 1);
        assert_eq!(app.filtered_owners[0].0, "@team-a");
        // Check patterns breakdown
        let pattern_names: Vec<&str> = app.filtered_patterns.iter().map(|(n, _)| n.as_str()).collect();
        assert!(pattern_names.contains(&"eslint-disable-next-line"));
        assert!(pattern_names.contains(&"eslint-disable"));
        assert!(pattern_names.contains(&"ts-ignore"));
    }

    #[test]
    fn dimension_cycling() {
        let report = test_report();
        let mut app = App::new(&report);

        assert_eq!(app.dimension, 0);
        app.next_dimension();
        assert_eq!(app.dimension, 1);
        app.next_dimension();
        assert_eq!(app.dimension, 2);
        app.next_dimension();
        assert_eq!(app.dimension, 3);
        app.next_dimension();
        assert_eq!(app.dimension, 0); // wraps

        app.prev_dimension();
        assert_eq!(app.dimension, 3); // wraps back
    }

    #[test]
    fn data_view_cycling() {
        let report = test_report();
        let mut app = App::new(&report);

        assert_eq!(app.data_view, 0);
        for _ in 0..DATA_VIEW_COUNT {
            app.next_data_view();
        }
        assert_eq!(app.data_view, 0); // full cycle

        app.prev_data_view();
        assert_eq!(app.data_view, DATA_VIEW_COUNT - 1);
    }

    #[test]
    fn files_sorted_by_count_descending() {
        let report = test_report();
        let app = App::new(&report);

        let counts: Vec<usize> = app.filtered_files.iter().map(|(_, _, c)| *c).collect();
        for w in counts.windows(2) {
            assert!(w[0] >= w[1], "files not sorted descending: {:?}", counts);
        }
    }
}
