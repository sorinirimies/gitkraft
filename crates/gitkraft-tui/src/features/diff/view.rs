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
    // ── Commit range diff takes priority ──────────────────────────────────
    if !app.tab().commit_range_diffs.is_empty() {
        render_commit_range_diff(app, frame, area);
        return;
    }

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
    let is_active_file_list = app.active_pane == crate::app::ActivePane::DiffView
        && tab.diff_sub_pane == crate::app::DiffSubPane::FileList;

    // Pre-sort selected indices for stable 1-based rank badges.
    let mut sorted_selected: Vec<usize> = tab.selected_file_indices.iter().copied().collect();
    sorted_selected.sort_unstable();
    let multi = sorted_selected.len() >= 2;

    let title = if multi {
        format!(
            " Files ({}) — {} selected [J/K shrink · e open all] ",
            tab.commit_files.len(),
            sorted_selected.len()
        )
    } else if is_active_file_list {
        format!(" Files ({}) [J/K select · e open] ", tab.commit_files.len())
    } else {
        format!(" Files ({}) ", tab.commit_files.len())
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.bg));

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
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_secondary)
            };

            // Rank badge: number for range selection, ● for single toggle, blank otherwise.
            let badge = if let Some(pos) = sorted_selected.iter().position(|&s| s == i) {
                if multi {
                    format!("{:<2}", pos + 1)
                } else {
                    "● ".to_string()
                }
            } else {
                "  ".to_string()
            };

            let line = Line::from(vec![
                Span::styled(
                    badge,
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} ", status_char),
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(file_name.to_string(), name_style),
            ]);

            ListItem::new(line)
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

/// Render the combined range diff for multiple selected commits.
fn render_commit_range_diff(app: &mut App, frame: &mut Frame, area: Rect) {
    let theme = app.theme();
    let is_active = app.active_pane == ActivePane::DiffView;
    let border_color = if is_active {
        theme.border_active
    } else {
        theme.border_inactive
    };

    let count = app.tab().selected_commits.len();
    let title = format!(" Combined diff ({} commits) ", count);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.bg));

    let diffs = app.tab().commit_range_diffs.clone();
    let mut lines: Vec<Line> = Vec::new();

    for diff in &diffs {
        // File header
        lines.push(Line::from(Span::styled(
            format!("══ {} ══", diff.display_path()),
            Style::default()
                .fg(theme.diff_hunk)
                .add_modifier(Modifier::BOLD),
        )));

        for hunk in &diff.hunks {
            for line in &hunk.lines {
                lines.push(styled_diff_line(line, &theme));
            }
        }
        lines.push(Line::default()); // blank separator
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

/// Render the file-history overlay in the diff column.
pub fn render_file_history(app: &mut App, frame: &mut Frame, area: Rect) {
    let theme = app.theme();
    let is_active = app.active_pane == ActivePane::DiffView;
    let border_color = if is_active {
        theme.border_active
    } else {
        theme.border_inactive
    };

    let path = app.tab().file_history_path.clone().unwrap_or_default();
    let file_name = path.rsplit('/').next().unwrap_or(&path).to_string();

    let title = format!(" File History: {file_name}  Esc close  Enter select ");

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.bg));

    let cursor = app.tab().file_history_cursor;
    let commits = app.tab().file_history_commits.clone();

    if commits.is_empty() {
        let p = Paragraph::new(Line::from(Span::styled(
            "Loading… (or no commits touch this file)",
            Style::default().fg(theme.text_muted),
        )))
        .block(block);
        frame.render_widget(p, area);
        return;
    }

    let items: Vec<ListItem> = commits
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let is_sel = i == cursor;
            let style = if is_sel {
                Style::default()
                    .fg(theme.text_primary)
                    .bg(theme.sel_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_primary)
            };
            let rel = c.relative_time();
            let summary = gitkraft_core::truncate_str(&c.summary, 48);
            let line = Line::from(vec![
                Span::styled(
                    format!("{} ", c.short_oid),
                    Style::default().fg(theme.warning),
                ),
                Span::styled(summary, style),
                Span::styled(
                    format!(" ({}, {})", c.author_name, rel),
                    Style::default().fg(theme.text_muted),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(theme.sel_bg)
                .add_modifier(Modifier::REVERSED),
        )
        .highlight_symbol("▶ ");

    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(Some(cursor));
    frame.render_stateful_widget(list, area, &mut list_state);
}

/// Render the blame overlay in the diff column.
pub fn render_blame(app: &mut App, frame: &mut Frame, area: Rect) {
    let theme = app.theme();
    let is_active = app.active_pane == ActivePane::DiffView;
    let border_color = if is_active {
        theme.border_active
    } else {
        theme.border_inactive
    };

    let path = app.tab().blame_path.clone().unwrap_or_default();
    let file_name = path.rsplit('/').next().unwrap_or(&path).to_string();
    let title = format!(" Blame: {file_name}  Esc close  j/k scroll ");

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.bg));

    let lines_data = app.tab().blame_lines.clone();

    if lines_data.is_empty() {
        let p = Paragraph::new(Line::from(Span::styled(
            "Loading blame…",
            Style::default().fg(theme.text_muted),
        )))
        .block(block);
        frame.render_widget(p, area);
        return;
    }

    let lines: Vec<Line> = lines_data
        .iter()
        .map(|bl| {
            let rel = bl.relative_time();
            let author = gitkraft_core::truncate_str(&bl.author_name, 12);
            Line::from(vec![
                Span::styled(
                    format!("{} ", bl.short_oid),
                    Style::default().fg(theme.warning),
                ),
                Span::styled(
                    format!("{:<12} ", author),
                    Style::default().fg(theme.accent),
                ),
                Span::styled(
                    format!("{:<8} ", rel),
                    Style::default().fg(theme.text_muted),
                ),
                Span::styled(
                    format!("{:>4}  ", bl.line_number),
                    Style::default().fg(theme.text_muted),
                ),
                Span::styled(bl.content.clone(), Style::default().fg(theme.text_primary)),
            ])
        })
        .collect();

    // Clamp scroll
    let content_height = lines.len() as u16;
    let visible_height = area.height.saturating_sub(2);
    {
        let tab = app.tab_mut();
        if content_height > visible_height {
            if tab.blame_scroll > content_height.saturating_sub(visible_height) {
                tab.blame_scroll = content_height.saturating_sub(visible_height);
            }
        } else {
            tab.blame_scroll = 0;
        }
    }

    let scroll = app.tab().blame_scroll;
    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    frame.render_widget(paragraph, area);
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
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(theme.bg));

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
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.bg));

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
