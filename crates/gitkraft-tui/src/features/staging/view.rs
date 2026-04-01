use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use gitkraft_core::FileStatus;

use crate::app::{ActivePane, App, InputMode, InputPurpose, StagingFocus};

/// Render the staging area — split into three columns:
///  1. Unstaged changes list
///  2. Staged changes list
///  3. Commit message input OR key hints
pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let is_active = app.active_pane == ActivePane::Staging;

    // Split the staging area into three columns
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35), // unstaged
            Constraint::Percentage(35), // staged
            Constraint::Min(20),        // commit input / hints
        ])
        .split(area);

    render_unstaged(app, frame, cols[0], is_active);
    render_staged(app, frame, cols[1], is_active);
    render_commit_or_hints(app, frame, cols[2], is_active);
}

/// Render the unstaged changes list.
fn render_unstaged(app: &mut App, frame: &mut Frame, area: Rect, pane_active: bool) {
    let is_focused = pane_active && app.staging_focus == StagingFocus::Unstaged;

    let border_color = if is_focused {
        Color::Cyan
    } else if pane_active {
        Color::Blue
    } else {
        Color::DarkGray
    };

    let title = format!(" Unstaged ({}) ", app.unstaged_changes.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    if app.unstaged_changes.is_empty() {
        let items: Vec<ListItem> = vec![ListItem::new(Line::from(Span::styled(
            "  No unstaged changes",
            Style::default().fg(Color::DarkGray),
        )))];
        let list = List::new(items).block(block);
        frame.render_widget(list, area);
        return;
    }

    let items: Vec<ListItem> = app
        .unstaged_changes
        .iter()
        .map(|diff| {
            let file_name = if diff.new_file.is_empty() {
                &diff.old_file
            } else {
                &diff.new_file
            };

            let (status_char, status_color) = status_display(&diff.status);

            let line = Line::from(vec![
                Span::styled(
                    format!(" {} ", status_char),
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(file_name, Style::default().fg(Color::White)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::REVERSED),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.unstaged_list_state);
}

/// Render the staged changes list.
fn render_staged(app: &mut App, frame: &mut Frame, area: Rect, pane_active: bool) {
    let is_focused = pane_active && app.staging_focus == StagingFocus::Staged;

    let border_color = if is_focused {
        Color::Cyan
    } else if pane_active {
        Color::Blue
    } else {
        Color::DarkGray
    };

    let title = format!(" Staged ({}) ", app.staged_changes.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    if app.staged_changes.is_empty() {
        let items: Vec<ListItem> = vec![ListItem::new(Line::from(Span::styled(
            "  No staged changes",
            Style::default().fg(Color::DarkGray),
        )))];
        let list = List::new(items).block(block);
        frame.render_widget(list, area);
        return;
    }

    let items: Vec<ListItem> = app
        .staged_changes
        .iter()
        .map(|diff| {
            let file_name = if diff.new_file.is_empty() {
                &diff.old_file
            } else {
                &diff.new_file
            };

            let (status_char, status_color) = status_display(&diff.status);

            let line = Line::from(vec![
                Span::styled(
                    format!(" {} ", status_char),
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(file_name, Style::default().fg(Color::White)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::REVERSED),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.staged_list_state);
}

/// Render either the commit message input (if in input mode) or key hints.
fn render_commit_or_hints(app: &mut App, frame: &mut Frame, area: Rect, pane_active: bool) {
    let border_color = if pane_active {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let is_commit_input =
        app.input_mode == InputMode::Input && app.input_purpose == InputPurpose::CommitMessage;

    if is_commit_input {
        // Show commit message editor
        let block = Block::default()
            .title(" Commit Message ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        // Build the text with a blinking cursor
        let cursor_char = if app.tick_count % 10 < 5 { "█" } else { " " };

        let lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(" ", Style::default()),
                Span::styled(&app.input_buffer, Style::default().fg(Color::White)),
                Span::styled(
                    cursor_char,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                " Enter: commit │ Esc: cancel",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    } else {
        // Show key hints
        let block = Block::default()
            .title(" Actions ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(" [s]", Style::default().fg(Color::Yellow)),
                Span::styled("tage  ", Style::default().fg(Color::White)),
                Span::styled("[u]", Style::default().fg(Color::Yellow)),
                Span::styled("nstage", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled(" [S]", Style::default().fg(Color::Yellow)),
                Span::styled("tage all  ", Style::default().fg(Color::White)),
                Span::styled("[U]", Style::default().fg(Color::Yellow)),
                Span::styled("nstage all", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled(" [c]", Style::default().fg(Color::Yellow)),
                Span::styled("ommit  ", Style::default().fg(Color::White)),
                Span::styled("[d]", Style::default().fg(Color::Yellow)),
                Span::styled("iscard", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled(" [z]", Style::default().fg(Color::Yellow)),
                Span::styled(" stash  ", Style::default().fg(Color::White)),
                Span::styled("[Z]", Style::default().fg(Color::Yellow)),
                Span::styled(" stash pop", Style::default().fg(Color::White)),
            ]),
            if app.confirm_discard {
                Line::from(Span::styled(
                    " ⚠ Press d again to confirm discard",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(vec![
                    Span::styled(" [Tab]", Style::default().fg(Color::Yellow)),
                    Span::styled(" toggle focus", Style::default().fg(Color::White)),
                ])
            },
        ];

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }
}

/// Map a `FileStatus` to a display character and color.
fn status_display(status: &FileStatus) -> (&'static str, Color) {
    match status {
        FileStatus::Modified => ("M", Color::Yellow),
        FileStatus::New => ("A", Color::Green),
        FileStatus::Deleted => ("D", Color::Red),
        FileStatus::Renamed => ("R", Color::Blue),
        FileStatus::Copied => ("C", Color::Blue),
        FileStatus::Typechange => ("T", Color::Magenta),
        FileStatus::Untracked => ("?", Color::Magenta),
    }
}
