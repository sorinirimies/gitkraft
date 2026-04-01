use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
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

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .style(Style::default().bg(Color::DarkGray));

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
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("  {} ", branch_name),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│", Style::default().fg(Color::Gray)),
        Span::styled(format!(" {} ", state), Style::default().fg(Color::Cyan)),
        Span::styled("│", Style::default().fg(Color::Gray)),
        Span::styled(
            " [Tab]",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" pane ", Style::default().fg(Color::White)),
        Span::styled(
            "[r]",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" refresh ", Style::default().fg(Color::White)),
        Span::styled(
            "[f]",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" fetch ", Style::default().fg(Color::White)),
        Span::styled(
            "[q]",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" quit", Style::default().fg(Color::White)),
    ];

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).block(block);

    frame.render_widget(paragraph, area);
}
