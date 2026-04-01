use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph};
use ratatui::Frame;

use crate::app::App;

/// Render the Welcome screen — a centered box with the GitKraft logo and
/// available actions.
pub fn render(_app: &mut App, frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" GitKraft ")
        .title_alignment(Alignment::Center)
        .padding(Padding::new(2, 2, 1, 1))
        .style(Style::default().bg(Color::Black));

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "╔═══════════════════════════════╗",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            "║          GitKraft             ║",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "║     Git IDE for terminal      ║",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "║                               ║",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(vec![
            Span::styled("║   ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "[o]",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Open Repository       ", Style::default().fg(Color::White)),
            Span::styled("║", Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("║   ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "[i]",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Init Repository       ", Style::default().fg(Color::White)),
            Span::styled("║", Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("║   ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "[q]",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Quit                  ", Style::default().fg(Color::White)),
            Span::styled("║", Style::default().fg(Color::Cyan)),
        ]),
        Line::from(Span::styled(
            "╚═══════════════════════════════╝",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(""),
    ];

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(block);

    // Center the block in the available area
    let centered = centered_rect(50, 60, area);
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
