use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;

/// Render the Welcome screen — a centered box with the GitKraft title,
/// available actions, and recent repositories.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let theme = app.theme();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_active))
        .title(" GitKraft ")
        .title_alignment(Alignment::Center);

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(
            "GitKraft",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        Line::from(Span::styled(
            "Git IDE for terminal",
            Style::default().fg(theme.text_primary),
        ))
        .alignment(Alignment::Center),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "[o]",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Open Repository", Style::default().fg(theme.text_primary)),
        ])
        .alignment(Alignment::Center),
        Line::from(vec![
            Span::styled(
                "[i]",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Init Repository", Style::default().fg(theme.text_primary)),
        ])
        .alignment(Alignment::Center),
        Line::from(vec![
            Span::styled(
                "[q]",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Quit", Style::default().fg(theme.text_primary)),
        ])
        .alignment(Alignment::Center),
        Line::from(""),
    ];

    if !app.recent_repos.is_empty() {
        lines.push(
            Line::from(Span::styled(
                "Recent repositories:",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Center),
        );

        for (i, entry) in app.recent_repos.iter().take(9).enumerate() {
            let number = format!("[{}]", i + 1);
            let path_str = format!(" {}", entry.path.display());
            lines.push(
                Line::from(vec![
                    Span::styled(
                        number,
                        Style::default()
                            .fg(theme.warning)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(path_str, Style::default().fg(theme.text_secondary)),
                ])
                .alignment(Alignment::Center),
            );
        }
    }

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(block);

    let centered = centered_rect(50, 50, area);
    frame.render_widget(paragraph, centered);
}

/// Return a `Rect` centered within `area` occupying the given percentage of
/// width and height.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1]);

    horizontal[1]
}
