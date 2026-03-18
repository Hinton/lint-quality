//! TUI rendering: all draw_* functions for header, filter panel, data panel,
//! stats tables, file table, directory tree, and footer.

use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Padding, Paragraph, Row, Table},
};

use super::app::{App, DATA_VIEWS, DIMENSIONS, DataView, Focus};

pub fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let s = &app.report.summary;

    let count_info = if app.has_any_active_filter() {
        format!(
            "{} / {} violations in {} / {} files (filtered)",
            fmt_num(app.filtered_violation_count),
            fmt_num(s.total_violations),
            fmt_num(app.filtered_files.len()),
            fmt_num(s.total_files_with_violations),
        )
    } else {
        format!(
            "{} violations in {} files",
            fmt_num(s.total_violations),
            fmt_num(s.total_files_with_violations),
        )
    };

    let lines = vec![
        Line::from(vec![
            Span::styled(
                " lint-quality ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("— interactive report"),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled(count_info, Style::default().fg(Color::Yellow)),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::BOTTOM)
        .padding(Padding::new(0, 0, 0, 0));
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(Paragraph::new(lines), inner);
}

pub fn draw_filter_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.focus == Focus::Filters;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let dim_titles: String = DIMENSIONS
        .iter()
        .enumerate()
        .map(|(i, d)| {
            if i == app.dimension {
                format!("[{}]", d.label())
            } else {
                d.label().to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    let active_count: usize = app.active_filters.iter().map(|s| s.len()).sum();
    let title = if active_count > 0 {
        format!(" Filters ({active_count} active) — {dim_titles} ")
    } else {
        format!(" Filters — {dim_titles} ")
    };

    let items: Vec<_> = app.current_dim_values().to_vec();
    let active_set = app.active_filters[app.dimension].clone();

    let rows: Vec<Row> = items
        .iter()
        .enumerate()
        .map(|(idx, (name, count))| {
            let marker = if active_set.contains(&idx) {
                "●"
            } else {
                "○"
            };
            let style = if active_set.contains(&idx) {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(marker).style(style),
                Cell::from(name.clone()).style(style),
                Cell::from(fmt_num(*count)).style(Style::default().fg(Color::Yellow)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Min(10),
            Constraint::Length(8),
        ],
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title),
    )
    .row_highlight_style(if is_focused {
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    });

    f.render_stateful_widget(table, area, &mut app.filter_list_state);
}

pub fn draw_data_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.focus == Focus::Data;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let view_tabs: String = DATA_VIEWS
        .iter()
        .enumerate()
        .map(|(i, v)| {
            if i == app.data_view {
                format!("[{}]", v.label())
            } else {
                v.label().to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    let current_view = DATA_VIEWS[app.data_view];
    let item_count = app.current_data_len();
    let title = format!(" {view_tabs} ({item_count}) ");

    let highlight_style = if is_focused {
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title);

    match current_view {
        DataView::Files => draw_files_table(f, app, area, block, highlight_style),
        DataView::Rules => {
            let items: Vec<_> = app.filtered_rules.clone();
            draw_stats_table(f, app, area, block, highlight_style, "Rule", &items);
        }
        DataView::Patterns => {
            let items: Vec<_> = app.filtered_patterns.clone();
            draw_stats_table(f, app, area, block, highlight_style, "Pattern", &items);
        }
        DataView::Categories => {
            let items: Vec<_> = app.filtered_categories.clone();
            draw_stats_table(f, app, area, block, highlight_style, "Category", &items);
        }
        DataView::Owners => {
            let items: Vec<_> = app.filtered_owners.clone();
            draw_stats_table(f, app, area, block, highlight_style, "Owner", &items);
        }
        DataView::Directories => draw_dir_tree(f, app, area, block, highlight_style),
    }
}

fn draw_files_table(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    block: Block,
    highlight_style: Style,
) {
    let header = Row::new(vec![
        Cell::from("File").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Owner").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Count").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("").style(Style::default().add_modifier(Modifier::BOLD)),
    ]);

    let max_count = app.filtered_files.first().map_or(1, |(_, _, c)| *c);

    let rows: Vec<Row> = app
        .filtered_files
        .iter()
        .map(|(path, owner, count)| {
            let bar = bar_string(*count, max_count, 20);
            Row::new(vec![
                Cell::from(path.as_str()),
                Cell::from(owner.as_str()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(fmt_num(*count)).style(Style::default().fg(Color::Yellow)),
                Cell::from(bar).style(Style::default().fg(Color::Cyan)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Min(20),
            Constraint::Length(15),
            Constraint::Length(8),
            Constraint::Length(22),
        ],
    )
    .header(header)
    .block(block)
    .row_highlight_style(highlight_style);

    f.render_stateful_widget(table, area, &mut app.data_states[0]);
}

fn draw_stats_table(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    block: Block,
    highlight_style: Style,
    col_name: &str,
    items: &[(String, usize)],
) {
    let total = app.filtered_violation_count;

    let header = Row::new(vec![
        Cell::from(col_name).style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Count").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("%").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("").style(Style::default().add_modifier(Modifier::BOLD)),
    ]);

    let max_count = items.first().map_or(1, |(_, c)| *c);

    let rows: Vec<Row> = items
        .iter()
        .map(|(name, count)| {
            let pct = if total > 0 {
                *count as f64 / total as f64 * 100.0
            } else {
                0.0
            };
            let bar = bar_string(*count, max_count, 20);

            Row::new(vec![
                Cell::from(name.as_str()),
                Cell::from(fmt_num(*count)).style(Style::default().fg(Color::Yellow)),
                Cell::from(format!("{:.1}%", pct)).style(Style::default().fg(Color::DarkGray)),
                Cell::from(bar).style(Style::default().fg(Color::Cyan)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Min(20),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(22),
        ],
    )
    .header(header)
    .block(block)
    .row_highlight_style(highlight_style);

    f.render_stateful_widget(table, area, &mut app.data_states[app.data_view]);
}

fn draw_dir_tree(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    block: Block,
    highlight_style: Style,
) {
    let header = Row::new(vec![
        Cell::from("Directory").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Count").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("").style(Style::default().add_modifier(Modifier::BOLD)),
    ]);

    let max_count = app.dir_tree_rows.first().map_or(1, |r| r.count);
    let tree_rows: Vec<_> = app.dir_tree_rows.clone();

    let rows: Vec<Row> = tree_rows
        .iter()
        .map(|row| {
            let indent = "  ".repeat(row.depth);
            let (marker, name_style) = if row.is_file {
                ("  ", Style::default().fg(Color::DarkGray))
            } else if row.has_children {
                if row.expanded {
                    ("▼ ", Style::default())
                } else {
                    ("▶ ", Style::default())
                }
            } else {
                ("  ", Style::default())
            };
            let label = format!("{indent}{marker}{}", row.name);
            let bar = bar_string(row.count, max_count, 20);

            Row::new(vec![
                Cell::from(label).style(name_style),
                Cell::from(fmt_num(row.count)).style(Style::default().fg(Color::Yellow)),
                Cell::from(bar).style(Style::default().fg(Color::Cyan)),
            ])
        })
        .collect();

    let view_idx = DATA_VIEWS
        .iter()
        .position(|v| *v == DataView::Directories)
        .unwrap();

    let table = Table::new(
        rows,
        [
            Constraint::Min(30),
            Constraint::Length(10),
            Constraint::Length(22),
        ],
    )
    .header(header)
    .block(block)
    .row_highlight_style(highlight_style);

    f.render_stateful_widget(table, area, &mut app.data_states[view_idx]);
}

pub fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let key_style = Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Color::DarkGray);

    let is_dir_view =
        app.focus == Focus::Data && DATA_VIEWS[app.data_view] == DataView::Directories;

    let mut spans = vec![
        Span::styled(" Tab ", key_style),
        Span::styled("panel  ", desc_style),
        Span::styled("←→ ", key_style),
        Span::styled("switch view  ", desc_style),
        Span::styled("↑↓ ", key_style),
        Span::styled("scroll  ", desc_style),
    ];

    if app.focus == Focus::Filters {
        spans.push(Span::styled("Space ", key_style));
        spans.push(Span::styled("toggle filter  ", desc_style));
    } else if is_dir_view {
        spans.push(Span::styled("Enter ", key_style));
        spans.push(Span::styled("expand/collapse  ", desc_style));
    }

    spans.push(Span::styled("c ", key_style));
    spans.push(Span::styled("clear  ", desc_style));
    spans.push(Span::styled("q ", key_style));
    spans.push(Span::styled("quit", desc_style));

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn fmt_num(n: usize) -> String {
    if n < 1_000 {
        return n.to_string();
    }
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

fn bar_string(count: usize, max: usize, width: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let w = (count as f64 / max as f64 * width as f64) as usize;
    "█".repeat(w)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_num_small() {
        assert_eq!(fmt_num(0), "0");
        assert_eq!(fmt_num(1), "1");
        assert_eq!(fmt_num(999), "999");
    }

    #[test]
    fn fmt_num_thousands() {
        assert_eq!(fmt_num(1_000), "1,000");
        assert_eq!(fmt_num(1_234), "1,234");
        assert_eq!(fmt_num(12_345), "12,345");
        assert_eq!(fmt_num(123_456), "123,456");
        assert_eq!(fmt_num(1_234_567), "1,234,567");
    }

    #[test]
    fn bar_string_full_width() {
        assert_eq!(bar_string(100, 100, 20), "█".repeat(20));
    }

    #[test]
    fn bar_string_half() {
        assert_eq!(bar_string(50, 100, 20), "█".repeat(10));
    }

    #[test]
    fn bar_string_zero() {
        assert_eq!(bar_string(0, 100, 20), "");
    }

    #[test]
    fn bar_string_zero_max() {
        assert_eq!(bar_string(50, 0, 20), "");
    }
}
