use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::App;

/// Render the Welcome screen — a centered box with ASCII art logo,
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
        Line::from(""),
        Line::from(vec![Span::styled(
            "  ╔═╗╦╔╦╗╦╔═╦═╗╔═╗╔═╗╔╦╗",
            Style::default().fg(theme.accent),
        )])
        .alignment(Alignment::Center),
        Line::from(vec![Span::styled(
            "  ║ ╦║ ║ ╠╩╗╠╦╝╠═╣╠╣  ║ ",
            Style::default().fg(theme.accent),
        )])
        .alignment(Alignment::Center),
        Line::from(vec![Span::styled(
            "  ╚═╝╩ ╩ ╩ ╩╩╚═╩ ╩╚   ╩ ",
            Style::default().fg(theme.accent),
        )])
        .alignment(Alignment::Center),
        Line::from(""),
        Line::from(Span::styled(
            "A modern Git IDE for the terminal",
            Style::default().fg(theme.text_secondary),
        ))
        .alignment(Alignment::Center),
        Line::from(""),
        Line::from(""),
        // Actions section
        Line::from(Span::styled(
            "─── Actions ───",
            Style::default().fg(theme.text_muted),
        ))
        .alignment(Alignment::Center),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  [o]",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  Browse & Open Repository",
                Style::default().fg(theme.text_primary),
            ),
        ])
        .alignment(Alignment::Center),
        Line::from(vec![
            Span::styled(
                "  [i]",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  Init New Repository (cwd)",
                Style::default().fg(theme.text_primary),
            ),
        ])
        .alignment(Alignment::Center),
        Line::from(vec![
            Span::styled(
                "  [q]",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  Quit", Style::default().fg(theme.text_primary)),
        ])
        .alignment(Alignment::Center),
        Line::from(""),
    ];

    // Recent repos section
    if !app.recent_repos.is_empty() {
        lines.push(
            Line::from(Span::styled(
                "─── Recent Repositories ───",
                Style::default().fg(theme.text_muted),
            ))
            .alignment(Alignment::Center),
        );
        lines.push(Line::from(""));

        for (i, entry) in app.recent_repos.iter().take(9).enumerate() {
            let path_str = entry.path.display().to_string();
            // Extract just the repo name (last path component) for emphasis
            let repo_name = entry
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&path_str);
            let parent = entry
                .path
                .parent()
                .map(|p| format!("{}/", p.display()))
                .unwrap_or_default();

            lines.push(
                Line::from(vec![
                    Span::styled(
                        format!("  [{}]  ", i + 1),
                        Style::default()
                            .fg(theme.warning)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(parent, Style::default().fg(theme.text_muted)),
                    Span::styled(
                        repo_name.to_string(),
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
                .alignment(Alignment::Center),
            );
        }
    }

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(block);

    let centered = centered_rect(60, 70, area);
    frame.render_widget(paragraph, centered);
}

/// Render the directory browser screen.
pub fn render_browser(app: &mut App, frame: &mut Frame, area: Rect) {
    let theme = app.theme();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_active))
        .title(format!(" 📂 {} ", app.browser_dir.display()))
        .title_alignment(Alignment::Left);

    let items: Vec<ListItem> = app
        .browser_entries
        .iter()
        .map(|path| {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let is_git = path.join(".git").exists();
            let (icon, style) = if is_git {
                (
                    "⊙ ",
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                )
            } else if name.starts_with('.') {
                ("  ", Style::default().fg(theme.text_muted))
            } else {
                ("📁 ", Style::default().fg(theme.text_primary))
            };
            ListItem::new(Line::from(vec![
                Span::styled(icon, style),
                Span::styled(name, style),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD)
                .bg(theme.sel_bg),
        )
        .highlight_symbol("▸ ");

    // Help bar at the bottom
    let help = Paragraph::new(Line::from(vec![
        Span::styled(
            "↑↓",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" navigate  ", Style::default().fg(theme.text_muted)),
        Span::styled(
            "Enter",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" open  ", Style::default().fg(theme.text_muted)),
        Span::styled(
            "Backspace",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" go up  ", Style::default().fg(theme.text_muted)),
        Span::styled(
            "o",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" open as repo  ", Style::default().fg(theme.text_muted)),
        Span::styled(
            "Esc",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" cancel", Style::default().fg(theme.text_muted)),
    ]));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    frame.render_stateful_widget(list, chunks[0], &mut app.browser_list_state);
    frame.render_widget(help, chunks[1]);
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
