use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use gitkraft_core::DiffLine;

use crate::app::{ActivePane, App};

/// Render the diff pane — shows colored hunks when a diff is selected,
/// or a placeholder message when nothing is selected.
pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let theme = app.theme();
    let is_active = app.active_pane == ActivePane::DiffView;
    let border_color = if is_active {
        theme.border_active
    } else {
        theme.border_inactive
    };

    let title = match &app.selected_diff {
        Some(diff) => {
            let name = diff.display_path();
            if name.is_empty() {
                " Diff ".to_string()
            } else if app.commit_diffs.len() > 1 {
                format!(
                    " Diff: {} [{}/{}] ",
                    name,
                    app.commit_diff_file_index + 1,
                    app.commit_diffs.len()
                )
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
