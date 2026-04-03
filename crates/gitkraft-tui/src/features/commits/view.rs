use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

use crate::app::{ActivePane, App};

fn graph_color(color_index: usize, graph_colors: &[Color; 8]) -> Color {
    graph_colors[color_index % graph_colors.len()]
}

/// Build the graph column spans for a single row.
///
/// Each column occupies 2 characters.  The node column gets `● `, active
/// pass-through lanes get `│ `, and edges that cross lanes get a simple
/// horizontal/diagonal representation.
fn build_graph_spans(
    row: &gitkraft_core::GraphRow,
    graph_colors: &[Color; 8],
) -> Vec<Span<'static>> {
    // We build a 2-char cell for every column in [0..width).
    // First, figure out what goes in each column.

    let width = row.width;
    if width == 0 {
        return vec![Span::styled(
            "● ",
            Style::default().fg(graph_color(row.node_color, graph_colors)),
        )];
    }

    // For each column, decide: is it the node? Is there a straight-through
    // edge (from_column == to_column == col)?  Is there a crossing edge that
    // starts or ends here?
    //
    // We keep it simple: first pass fills each cell with the "default" content,
    // then we handle the node column specially.

    // Collect pass-through edges (from == to) per column, and crossing edges.
    let mut column_passthrough: Vec<Option<usize>> = vec![None; width]; // color_index
    let mut has_left_cross = false; // edge going from node to a column left of node
    let mut has_right_cross = false; // edge going from node to a column right of node
    let mut left_cross_color: usize = 0;
    let mut right_cross_color: usize = 0;
    // Track the range of crossing for horizontal lines
    let mut cross_left_col: usize = row.node_column;
    let mut cross_right_col: usize = row.node_column;

    for edge in &row.edges {
        if edge.from_column == edge.to_column {
            // Straight-through or first-parent-continuation
            column_passthrough[edge.to_column] = Some(edge.color_index);
        } else {
            // Crossing edge — from node_column to some other column
            let target = edge.to_column;
            if target < row.node_column {
                has_left_cross = true;
                left_cross_color = edge.color_index;
                if target < cross_left_col {
                    cross_left_col = target;
                }
            } else if target > row.node_column {
                has_right_cross = true;
                right_cross_color = edge.color_index;
                if target > cross_right_col {
                    cross_right_col = target;
                }
            }
        }
    }

    let mut spans: Vec<Span<'static>> = Vec::with_capacity(width + 1);

    for col in 0..width {
        if col == row.node_column {
            // The commit node
            spans.push(Span::styled(
                "● ".to_string(),
                Style::default()
                    .fg(graph_color(row.node_color, graph_colors))
                    .add_modifier(Modifier::BOLD),
            ));
        } else if let Some(ci) = column_passthrough.get(col).copied().flatten() {
            // There is a straight-through lane here.
            // Check if a horizontal crossing line also passes through this column.
            let in_left_range = has_left_cross && col >= cross_left_col && col < row.node_column;
            let in_right_range = has_right_cross && col > row.node_column && col <= cross_right_col;

            if in_left_range || in_right_range {
                // Vertical lane intersects a horizontal crossing — draw a crossing
                let cross_ci = if in_left_range {
                    left_cross_color
                } else {
                    right_cross_color
                };
                // Use the crossing edge's color for the combined glyph
                spans.push(Span::styled(
                    "├─".to_string(),
                    Style::default().fg(graph_color(cross_ci, graph_colors)),
                ));
            } else {
                spans.push(Span::styled(
                    "│ ".to_string(),
                    Style::default().fg(graph_color(ci, graph_colors)),
                ));
            }
        } else {
            // Empty column — but might have a horizontal crossing line passing through.
            let in_left_range = has_left_cross && col >= cross_left_col && col < row.node_column;
            let in_right_range = has_right_cross && col > row.node_column && col <= cross_right_col;

            if in_left_range {
                if col == cross_left_col {
                    // The target column of the left-crossing edge
                    spans.push(Span::styled(
                        "╭─".to_string(),
                        Style::default().fg(graph_color(left_cross_color, graph_colors)),
                    ));
                } else {
                    spans.push(Span::styled(
                        "──".to_string(),
                        Style::default().fg(graph_color(left_cross_color, graph_colors)),
                    ));
                }
            } else if in_right_range {
                if col == cross_right_col {
                    // The target column of the right-crossing edge
                    spans.push(Span::styled(
                        "─╮".to_string(),
                        Style::default().fg(graph_color(right_cross_color, graph_colors)),
                    ));
                } else {
                    spans.push(Span::styled(
                        "──".to_string(),
                        Style::default().fg(graph_color(right_cross_color, graph_colors)),
                    ));
                }
            } else {
                spans.push(Span::styled(
                    "  ".to_string(),
                    Style::default().fg(Color::DarkGray),
                ));
            }
        }
    }

    // Add a separator after the graph portion
    spans.push(Span::styled(" ", Style::default()));
    spans
}

/// Render the commit log list inside the given `area`.
pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let theme = app.theme();
    let is_active = app.active_pane == ActivePane::CommitLog;

    let border_color = if is_active {
        theme.border_active
    } else {
        theme.border_inactive
    };

    let block = Block::default()
        .title(" Commit Log ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    if app.commits.is_empty() {
        let items: Vec<ListItem> = vec![ListItem::new(Line::from(vec![Span::styled(
            "  No commits yet",
            Style::default().fg(theme.text_muted),
        )]))];

        let list = List::new(items).block(block);
        frame.render_widget(list, area);
        return;
    }

    let items: Vec<ListItem> = app
        .commits
        .iter()
        .enumerate()
        .map(|(idx, commit)| {
            let summary = if commit.summary.len() > 50 {
                format!("{}…", &commit.summary[..49])
            } else {
                commit.summary.clone()
            };

            let relative = gitkraft_core::utils::relative_time(commit.time);

            // Build graph prefix spans for this row
            let mut spans = if let Some(row) = app.graph_rows.get(idx) {
                build_graph_spans(row, &theme.graph_colors)
            } else {
                vec![Span::raw("  ")]
            };

            // Append the commit info spans
            spans.push(Span::styled(
                format!("{} ", commit.short_oid),
                Style::default().fg(theme.warning),
            ));
            spans.push(Span::styled(
                summary,
                Style::default().fg(theme.text_primary),
            ));
            spans.push(Span::styled(
                format!(" ({}", commit.author_name),
                Style::default().fg(theme.accent),
            ));
            spans.push(Span::styled(
                format!(", {})", relative),
                Style::default().fg(theme.text_muted),
            ));

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(theme.sel_bg)
                .fg(theme.text_primary)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.commit_list_state);
}
