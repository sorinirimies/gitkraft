use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph};
use ratatui::Frame;

use crate::app::App;

/// Render the options panel with grouped sections in bordered inner blocks,
/// matching the tui-file-explorer options style.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let theme = app.theme();

    let key_style = Style::default()
        .fg(theme.warning)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(theme.text_primary);
    let value_style = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD);
    let section_title_style = Style::default().fg(theme.text_muted);
    let muted_style = Style::default().fg(theme.text_muted);

    // ── Outer block ───────────────────────────────────────────────────────
    let outer_block = Block::default()
        .title(Line::from(vec![
            Span::styled("⚙ ", Style::default().fg(theme.accent)),
            Span::styled(
                "Options",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_active))
        .padding(Padding::new(1, 1, 0, 0));

    let inner_area = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    // ── Close hint ────────────────────────────────────────────────────────
    let close_line = Paragraph::new(Line::from(vec![
        Span::styled("Shift + O", key_style),
        Span::styled(" close", desc_style),
    ]));

    // ── Layout: close hint, then sections ─────────────────────────────────
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // close hint
            Constraint::Length(1), // spacer
            Constraint::Length(6), // Settings section
            Constraint::Length(1), // spacer
            Constraint::Length(8), // Navigation section
            Constraint::Length(1), // spacer
            Constraint::Length(8), // Staging section
            Constraint::Length(1), // spacer
            Constraint::Length(7), // Git section
            Constraint::Min(0),    // remaining
        ])
        .split(inner_area);

    frame.render_widget(close_line, sections[0]);

    // ── Settings section ──────────────────────────────────────────────────
    {
        let block = Block::default()
            .title(Span::styled(" Settings ", section_title_style))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_inactive))
            .padding(Padding::new(1, 1, 0, 0));

        let theme_name = app.current_theme_name();
        let last_repo = app
            .repo_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "none".to_string());

        let on_style = Style::default()
            .fg(theme.success)
            .add_modifier(Modifier::BOLD);

        let lines = vec![
            Line::from(vec![
                Span::styled(pad_right("Shift + T", 14), key_style),
                Span::styled(pad_right("theme", 16), desc_style),
                Span::styled(theme_name, value_style),
            ]),
            Line::from(vec![
                Span::styled(pad_right("commits", 14), muted_style),
                Span::styled(pad_right("max loaded", 16), desc_style),
                Span::styled("500", value_style),
            ]),
            Line::from(vec![
                Span::styled(pad_right("repo", 14), muted_style),
                Span::styled(pad_right("current", 16), desc_style),
                Span::styled(last_repo, on_style),
            ]),
        ];

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, sections[2]);
    }

    // ── Navigation section ────────────────────────────────────────────────
    {
        let block = Block::default()
            .title(Span::styled(" Navigation ", section_title_style))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_inactive))
            .padding(Padding::new(1, 1, 0, 0));

        let lines = vec![
            Line::from(vec![
                Span::styled(pad_right("Tab", 14), key_style),
                Span::styled("cycle pane forward", desc_style),
            ]),
            Line::from(vec![
                Span::styled(pad_right("Shift + Tab", 14), key_style),
                Span::styled("cycle pane backward", desc_style),
            ]),
            Line::from(vec![
                Span::styled(pad_right("j / ↓", 14), key_style),
                Span::styled("next item", desc_style),
            ]),
            Line::from(vec![
                Span::styled(pad_right("k / ↑", 14), key_style),
                Span::styled("previous item", desc_style),
            ]),
            Line::from(vec![
                Span::styled(pad_right("Enter", 14), key_style),
                Span::styled("select / activate", desc_style),
            ]),
            Line::from(vec![
                Span::styled(pad_right("Esc", 14), key_style),
                Span::styled("dismiss / cancel", desc_style),
            ]),
        ];

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, sections[4]);
    }

    // ── Staging section ───────────────────────────────────────────────────
    {
        let block = Block::default()
            .title(Span::styled(" Staging ", section_title_style))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_inactive))
            .padding(Padding::new(1, 1, 0, 0));

        let lines = vec![
            Line::from(vec![
                Span::styled(pad_right("s", 14), key_style),
                Span::styled(pad_right("stage file", 18), desc_style),
                Span::styled("selected", value_style),
            ]),
            Line::from(vec![
                Span::styled(pad_right("u", 14), key_style),
                Span::styled(pad_right("unstage file", 18), desc_style),
                Span::styled("selected", value_style),
            ]),
            Line::from(vec![
                Span::styled(pad_right("S", 14), key_style),
                Span::styled(pad_right("stage all", 18), desc_style),
                Span::styled("all files", value_style),
            ]),
            Line::from(vec![
                Span::styled(pad_right("U", 14), key_style),
                Span::styled(pad_right("unstage all", 18), desc_style),
                Span::styled("all files", value_style),
            ]),
            Line::from(vec![
                Span::styled(pad_right("d", 14), key_style),
                Span::styled(pad_right("discard", 18), desc_style),
                Span::styled("confirm ×2", value_style),
            ]),
            Line::from(vec![
                Span::styled(pad_right("Tab", 14), key_style),
                Span::styled("toggle focus", desc_style),
            ]),
        ];

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, sections[6]);
    }

    // ── Git section ───────────────────────────────────────────────────────
    {
        let block = Block::default()
            .title(Span::styled(" Git ", section_title_style))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_inactive))
            .padding(Padding::new(1, 1, 0, 0));

        let lines = vec![
            Line::from(vec![
                Span::styled(pad_right("c", 14), key_style),
                Span::styled(pad_right("commit", 18), desc_style),
                Span::styled("staged changes", value_style),
            ]),
            Line::from(vec![
                Span::styled(pad_right("b", 14), key_style),
                Span::styled(pad_right("create branch", 18), desc_style),
                Span::styled("from HEAD", value_style),
            ]),
            Line::from(vec![
                Span::styled(pad_right("z", 14), key_style),
                Span::styled(pad_right("stash save", 18), desc_style),
                Span::styled("working dir", value_style),
            ]),
            Line::from(vec![
                Span::styled(pad_right("Z", 14), key_style),
                Span::styled(pad_right("stash pop", 18), desc_style),
                Span::styled("latest", value_style),
            ]),
            Line::from(vec![
                Span::styled(pad_right("r", 14), key_style),
                Span::styled(pad_right("refresh", 18), desc_style),
                Span::styled("all data", value_style),
            ]),
        ];

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, sections[8]);
    }
}

/// Pad a string to a fixed width with trailing spaces.
fn pad_right(s: &str, width: usize) -> String {
    if s.len() >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - s.len()))
    }
}
