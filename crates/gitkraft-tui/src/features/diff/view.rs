use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use gitkraft_core::DiffLine;

use crate::app::{ActivePane, App, DiffSubPane};

/// Render the diff pane — shows colored hunks when a diff is selected,
/// or a placeholder message when nothing is selected.
///
/// When the current commit has more than one changed file, the area is
/// split horizontally into a file-list sidebar on the left and the diff
/// content on the right, matching the GUI's "Files" panel.
pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let theme = app.theme();
    let is_active = app.active_pane == ActivePane::DiffView;
    let sub_pane = app.tab().diff_sub_pane.clone();

    let file_list_border = if is_active && sub_pane == DiffSubPane::FileList {
        theme.border_active
    } else {
        theme.border_inactive
    };
    let content_border = if is_active && sub_pane == DiffSubPane::Content {
        theme.border_active
    } else {
        theme.border_inactive
    };

    if !app.tab().commit_files.is_empty() {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(30), Constraint::Min(20)])
            .split(area);
        render_file_list(app, frame, chunks[0], file_list_border);
        render_diff_content(app, frame, chunks[1], content_border);
    } else {
        render_diff_content(app, frame, area, content_border);
    }
}

/// Render the file list sidebar for the current commit's changed files.
fn render_file_list(
    app: &mut App,
    frame: &mut Frame,
    area: Rect,
    border_color: ratatui::style::Color,
) {
    let theme = app.theme();
    let tab = app.tab();
    let commit_diff_file_index = tab.commit_diff_file_index;

    let block = Block::default()
        .title(format!(" Files ({}) ", tab.commit_files.len()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let items: Vec<ListItem> = tab
        .commit_files
        .iter()
        .enumerate()
        .map(|(i, diff)| {
            let is_current = i == commit_diff_file_index;
            let is_multi = tab.selected_file_indices.contains(&i);
            let file_name = diff.file_name();
            let status_char = format!("{}", diff.status);

            let status_color = match diff.status.color_category() {
                gitkraft_core::StatusColorCategory::Added => theme.success,
                gitkraft_core::StatusColorCategory::Modified => theme.warning,
                gitkraft_core::StatusColorCategory::Deleted => theme.error,
                gitkraft_core::StatusColorCategory::Renamed => theme.accent,
            };

            let name_style = if is_current {
                Style::default().fg(theme.text_primary)
            } else if is_multi {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.text_secondary)
            };

            // Apply a background for multi-selected items that aren't the cursor
            let item_style = if is_multi && !is_current {
                Style::default().bg(theme.sel_bg)
            } else {
                Style::default()
            };

            let line = Line::from(vec![
                Span::styled(
                    format!("{} ", status_char),
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(file_name.to_string(), name_style),
            ]);

            ListItem::new(line).style(item_style)
        })
        .collect();

    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(Some(commit_diff_file_index));

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(theme.sel_bg)
                .add_modifier(Modifier::REVERSED),
        )
        .highlight_symbol("▸ ");

    frame.render_stateful_widget(list, area, &mut list_state);
}

/// Convert a single `DiffLine` into a styled ratatui `Line`.
fn styled_diff_line(
    line: &DiffLine,
    theme: &crate::features::theme::palette::UiTheme,
) -> Line<'static> {
    match line {
        DiffLine::Addition(s) => Line::from(Span::styled(
            format!("+{}", s),
            Style::default().fg(theme.diff_add),
        )),
        DiffLine::Deletion(s) => Line::from(Span::styled(
            format!("-{}", s),
            Style::default().fg(theme.diff_del),
        )),
        DiffLine::Context(s) => Line::from(Span::styled(
            format!(" {}", s),
            Style::default().fg(theme.diff_context),
        )),
        DiffLine::HunkHeader(s) => Line::from(Span::styled(
            s.clone(),
            Style::default()
                .fg(theme.diff_hunk)
                .add_modifier(Modifier::BOLD),
        )),
    }
}

/// Render the diff content for the currently selected file (or all selected files).
fn render_diff_content(
    app: &mut App,
    frame: &mut Frame,
    area: Rect,
    border_color: ratatui::style::Color,
) {
    let theme = app.theme();
    let is_multi = app.tab().selected_file_indices.len() > 1;

    if is_multi {
        // ── Multi-file concatenated view ──────────────────────────────────
        let mut sorted_indices: Vec<usize> =
            app.tab().selected_file_indices.iter().copied().collect();
        sorted_indices.sort();

        let title = format!(" Diff ({} files) ", sorted_indices.len());
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let mut lines: Vec<Line> = Vec::new();
        for idx in &sorted_indices {
            let file_name = app
                .tab()
                .commit_files
                .get(*idx)
                .map(|f| f.display_path().to_string())
                .unwrap_or_else(|| format!("file {}", idx));

            // File header separator
            lines.push(Line::from(Span::styled(
                format!("══ {} ══", file_name),
                Style::default()
                    .fg(theme.diff_hunk)
                    .add_modifier(Modifier::BOLD),
            )));

            if let Some(diff) = app.tab().commit_diffs.get(idx).cloned() {
                for hunk in &diff.hunks {
                    for line in &hunk.lines {
                        lines.push(styled_diff_line(line, &theme));
                    }
                }
            } else {
                lines.push(Line::from(Span::styled(
                    "  Loading…",
                    Style::default().fg(theme.text_muted),
                )));
            }
            // Blank separator between files
            lines.push(Line::default());
        }

        // Clamp scroll
        let content_height = lines.len() as u16;
        let visible_height = area.height.saturating_sub(2);
        {
            let tab = app.tab_mut();
            if content_height > visible_height {
                if tab.diff_scroll > content_height.saturating_sub(visible_height) {
                    tab.diff_scroll = content_height.saturating_sub(visible_height);
                }
            } else {
                tab.diff_scroll = 0;
            }
        }

        let scroll = app.tab().diff_scroll;
        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));

        frame.render_widget(paragraph, area);
        return;
    }

    // ── Single-file view ──────────────────────────────────────────────────
    let tab = app.tab_mut();

    let title = match &tab.selected_diff {
        Some(diff) => {
            let name = diff.display_path();
            if name.is_empty() {
                " Diff ".to_string()
            } else {
                format!(" Diff: {} ", name)
            }
        }
        None => " Diff ".to_string(),
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    match &tab.selected_diff {
        None => {
            let placeholder = Paragraph::new(Line::from(vec![Span::styled(
                "Select a commit or file to view diff",
                Style::default().fg(theme.text_muted),
            )]))
            .block(block)
            .alignment(ratatui::layout::Alignment::Center);

            frame.render_widget(placeholder, area);
        }
        Some(diff) => {
            let mut lines: Vec<Line> = Vec::new();

            for hunk in &diff.hunks {
                for line in &hunk.lines {
                    lines.push(styled_diff_line(line, &theme));
                }
            }

            // Clamp scroll so it doesn't go past the content
            let content_height = lines.len() as u16;
            let visible_height = area.height.saturating_sub(2); // subtract border rows
            if content_height > visible_height {
                if tab.diff_scroll > content_height.saturating_sub(visible_height) {
                    tab.diff_scroll = content_height.saturating_sub(visible_height);
                }
            } else {
                tab.diff_scroll = 0;
            }

            let paragraph = Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false })
                .scroll((tab.diff_scroll, 0));

            frame.render_widget(paragraph, area);
        }
    }
}
