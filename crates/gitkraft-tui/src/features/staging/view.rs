use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Padding, Paragraph};
use ratatui::Frame;

use gitkraft_core::FileStatus;

use crate::app::{ActivePane, App, InputMode, InputPurpose, StagingFocus};
use crate::utils::pad_right;

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
    let theme = app.theme();
    let is_focused = pane_active && app.staging_focus == StagingFocus::Unstaged;

    let border_color = if is_focused {
        theme.border_active
    } else if pane_active {
        theme.accent
    } else {
        theme.border_inactive
    };

    let title = format!(" Unstaged ({}) ", app.unstaged_changes.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    if app.unstaged_changes.is_empty() {
        let items: Vec<ListItem> = vec![ListItem::new(Line::from(Span::styled(
            "  No unstaged changes",
            Style::default().fg(theme.text_muted),
        )))];
        let list = List::new(items).block(block);
        frame.render_widget(list, area);
        return;
    }

    let items: Vec<ListItem> = app
        .unstaged_changes
        .iter()
        .map(|diff| {
            let file_name = diff.display_path();
            let (status_char, status_color) = status_display(&diff.status, &theme);

            let line = Line::from(vec![
                Span::styled(
                    format!(" {} ", status_char),
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(file_name, Style::default().fg(theme.text_primary)),
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

    frame.render_stateful_widget(list, area, &mut app.unstaged_list_state);
}

/// Render the staged changes list.
fn render_staged(app: &mut App, frame: &mut Frame, area: Rect, pane_active: bool) {
    let theme = app.theme();
    let is_focused = pane_active && app.staging_focus == StagingFocus::Staged;

    let border_color = if is_focused {
        theme.border_active
    } else if pane_active {
        theme.accent
    } else {
        theme.border_inactive
    };

    let title = format!(" Staged ({}) ", app.staged_changes.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    if app.staged_changes.is_empty() {
        let items: Vec<ListItem> = vec![ListItem::new(Line::from(Span::styled(
            "  No staged changes",
            Style::default().fg(theme.text_muted),
        )))];
        let list = List::new(items).block(block);
        frame.render_widget(list, area);
        return;
    }

    let items: Vec<ListItem> = app
        .staged_changes
        .iter()
        .map(|diff| {
            let file_name = diff.display_path();
            let (status_char, status_color) = status_display(&diff.status, &theme);

            let line = Line::from(vec![
                Span::styled(
                    format!(" {} ", status_char),
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(file_name, Style::default().fg(theme.text_primary)),
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

    frame.render_stateful_widget(list, area, &mut app.staged_list_state);
}

/// Render either the commit message input (if in input mode) or key hints.
fn render_commit_or_hints(app: &mut App, frame: &mut Frame, area: Rect, pane_active: bool) {
    let theme = app.theme();
    let border_color = if pane_active {
        theme.border_active
    } else {
        theme.border_inactive
    };

    let is_commit_input =
        app.input_mode == InputMode::Input && app.input_purpose == InputPurpose::CommitMessage;

    if is_commit_input {
        // Show commit message editor
        let block = Block::default()
            .title(" Commit Message ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.warning));

        let cursor_char = if app.tick_count % 10 < 5 { "█" } else { " " };

        let lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(" ", Style::default()),
                Span::styled(&app.input_buffer, Style::default().fg(theme.text_primary)),
                Span::styled(
                    cursor_char,
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                " Enter: commit │ Esc: cancel",
                Style::default().fg(theme.text_muted),
            )),
        ];

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    } else {
        // Show key hints in bordered inner sections (tui-file-explorer style)
        let outer_block = Block::default()
            .title(Line::from(vec![
                Span::styled("⚡", Style::default().fg(theme.accent)),
                Span::styled(
                    "Actions",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
            ]))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .padding(Padding::new(1, 1, 0, 0));

        let inner_area = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        let key_style = Style::default()
            .fg(theme.warning)
            .add_modifier(Modifier::BOLD);
        let desc_style = Style::default().fg(theme.text_primary);
        let value_style = Style::default().fg(theme.accent);
        let section_title = Style::default().fg(theme.text_muted);

        // Split inner area into sections
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Staging section
                Constraint::Length(4), // Git section
                Constraint::Min(2),    // remaining / warnings
            ])
            .split(inner_area);

        // ── Staging section ───────────────────────────────────────────
        {
            let block = Block::default()
                .title(Span::styled(" Staging ", section_title))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border_inactive));

            let lines = vec![
                Line::from(vec![
                    Span::styled(pad_right("s", 8), key_style),
                    Span::styled(pad_right("stage", 12), desc_style),
                    Span::styled(pad_right("u", 8), key_style),
                    Span::styled("unstage", desc_style),
                ]),
                Line::from(vec![
                    Span::styled(pad_right("S", 8), key_style),
                    Span::styled(pad_right("stage all", 12), desc_style),
                    Span::styled(pad_right("U", 8), key_style),
                    Span::styled("unstage all", desc_style),
                ]),
            ];

            let paragraph = Paragraph::new(lines).block(block);
            frame.render_widget(paragraph, sections[0]);
        }

        // ── Git section ───────────────────────────────────────────────
        {
            let block = Block::default()
                .title(Span::styled(" Git ", section_title))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border_inactive));

            let lines = vec![
                Line::from(vec![
                    Span::styled(pad_right("c", 8), key_style),
                    Span::styled(pad_right("commit", 12), desc_style),
                    Span::styled(pad_right("z", 8), key_style),
                    Span::styled("stash", desc_style),
                ]),
                Line::from(vec![
                    Span::styled(pad_right("d", 8), key_style),
                    Span::styled(pad_right("discard", 12), desc_style),
                    Span::styled(pad_right("Z", 8), key_style),
                    Span::styled("stash pop", desc_style),
                ]),
            ];

            let paragraph = Paragraph::new(lines).block(block);
            frame.render_widget(paragraph, sections[1]);
        }

        // ── Remaining area: navigation hint + discard warning ─────────
        {
            let mut lines = vec![Line::from(vec![
                Span::styled(" Tab", key_style),
                Span::styled(" focus  ", desc_style),
                Span::styled("Enter", key_style),
                Span::styled(" diff  ", desc_style),
                Span::styled("O", key_style),
                Span::styled(" options", value_style),
            ])];

            if app.confirm_discard {
                lines.push(Line::from(Span::styled(
                    " ⚠ Press d again to confirm discard",
                    Style::default()
                        .fg(theme.error)
                        .add_modifier(Modifier::BOLD),
                )));
            }

            let paragraph = Paragraph::new(lines);
            frame.render_widget(paragraph, sections[2]);
        }
    }
}

/// Map a `FileStatus` to a display character and color.
fn status_display(
    status: &FileStatus,
    theme: &crate::features::theme::palette::UiTheme,
) -> (&'static str, ratatui::style::Color) {
    match status {
        FileStatus::Modified => ("M", theme.warning),
        FileStatus::New => ("A", theme.success),
        FileStatus::Deleted => ("D", theme.error),
        FileStatus::Renamed => ("R", theme.accent),
        FileStatus::Copied => ("C", theme.accent),
        FileStatus::Typechange => ("T", theme.text_secondary),
        FileStatus::Untracked => ("?", theme.text_secondary),
    }
}
