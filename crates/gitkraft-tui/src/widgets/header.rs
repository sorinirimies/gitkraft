use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, AppScreen};

/// Render the top header bar showing repo name, current branch, state, and
/// keyboard shortcuts.
pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    if app.screen != AppScreen::Main {
        return;
    }

    let theme = app.theme();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_inactive))
        .style(Style::default().bg(theme.border_inactive));

    let repo_name = app
        .repo_path
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let branch_name = app
        .repo_info
        .as_ref()
        .and_then(|info| info.head_branch.clone())
        .unwrap_or_else(|| "detached".to_string());

    let state = app
        .repo_info
        .as_ref()
        .map(|info| format!("{}", info.state))
        .unwrap_or_else(|| "?".to_string());

    let spans = vec![
        Span::styled(
            format!(" {} ", repo_name),
            Style::default()
                .fg(theme.text_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│", Style::default().fg(theme.text_secondary)),
        Span::styled(
            format!("  {} ", branch_name),
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│", Style::default().fg(theme.text_secondary)),
        Span::styled(format!(" {} ", state), Style::default().fg(theme.accent)),
        Span::styled("│", Style::default().fg(theme.text_secondary)),
        Span::styled(
            " [←→]",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" pane ", Style::default().fg(theme.text_primary)),
        Span::styled(
            "[r]",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" refresh ", Style::default().fg(theme.text_primary)),
        Span::styled(
            "[f]",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" fetch ", Style::default().fg(theme.text_primary)),
        Span::styled(
            "[T]",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" theme ", Style::default().fg(theme.text_primary)),
        Span::styled(
            "[O]",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" options ", Style::default().fg(theme.text_primary)),
        Span::styled(
            "[o]",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" open ", Style::default().fg(theme.text_primary)),
        Span::styled(
            "[W]",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" close ", Style::default().fg(theme.text_primary)),
        Span::styled(
            "[q]",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" quit", Style::default().fg(theme.text_primary)),
    ];

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).block(block);

    frame.render_widget(paragraph, area);
}
