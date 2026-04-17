use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use gitkraft_core::DiffLine;

use crate::app::{ActivePane, App};

/// Render the diff pane — shows colored hunks when a diff is selected,
/// or a placeholder message when nothing is selected.
///
/// When the current commit has more than one changed file, the area is
/// split horizontally into a file-list sidebar on the left and the diff
/// content on the right, matching the GUI's "Files" panel.
pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let theme = app.theme();
    let is_active = app.active_pane == ActivePane::DiffView;
    let border_color = if is_active {
        theme.border_active
    } else {
        theme.border_inactive
    };

    // Split into file list + diff content when there are multiple files
    if app.commit_diffs.len() > 1 {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(30), Constraint::Min(20)])
            .split(area);

        render_file_list(app, frame, chunks[0], border_color);
        render_diff_content(app, frame, chunks[1], border_color);
    } else {
        render_diff_content(app, frame, area, border_color);
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

    let block = Block::default()
        .title(format!(" Files ({}) ", app.commit_diffs.len()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let items: Vec<ListItem> = app
        .commit_diffs
        .iter()
        .enumerate()
        .map(|(i, diff)| {
            let is_selected = i == app.commit_diff_file_index;
            let file_name = diff.file_name();
            let status_char = format!("{}", diff.status);

            let status_color = match diff.status.color_category() {
                gitkraft_core::StatusColorCategory::Added => theme.success,
                gitkraft_core::StatusColorCategory::Modified => theme.warning,
                gitkraft_core::StatusColorCategory::Deleted => theme.error,
                gitkraft_core::StatusColorCategory::Renamed => theme.accent,
            };

            let name_color = if is_selected {
                theme.text_primary
            } else {
                theme.text_secondary
            };

            let line = Line::from(vec![
                Span::styled(
                    format!("{} ", status_char),
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(file_name.to_string(), Style::default().fg(name_color)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(Some(app.commit_diff_file_index));

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

/// Render the diff content for the currently selected file.
fn render_diff_content(
    app: &mut App,
    frame: &mut Frame,
    area: Rect,
    border_color: ratatui::style::Color,
) {
    let theme = app.theme();

    let title = match &app.selected_diff {
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

    match &app.selected_diff {
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
                    let styled_line = match line {
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
                    };
                    lines.push(styled_line);
                }
            }

            // Clamp scroll so it doesn't go past the content
            let content_height = lines.len() as u16;
            let visible_height = area.height.saturating_sub(2); // subtract border rows
            if content_height > visible_height {
                if app.diff_scroll > content_height.saturating_sub(visible_height) {
                    app.diff_scroll = content_height.saturating_sub(visible_height);
                }
            } else {
                app.diff_scroll = 0;
            }

            let paragraph = Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false })
                .scroll((app.diff_scroll, 0));

            frame.render_widget(paragraph, area);
        }
    }
}
