use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, InputMode, InputPurpose};

/// Render the bottom status bar — a single line showing the current mode,
/// status message, or error message.
///
/// Format: ` [{mode}] {status_or_error}`
///  - Error messages are rendered in the theme's error color.
///  - Status messages are rendered in the theme's success color.
///  - The mode indicator is rendered in the theme's warning color.
pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let theme = app.theme();

    let mode_str = match app.input_mode {
        InputMode::Normal => "NORMAL",
        InputMode::Input => match app.input_purpose {
            InputPurpose::CommitMessage => "INPUT: Commit",
            InputPurpose::BranchName => "INPUT: Branch",
            InputPurpose::RepoPath => "INPUT: Path",
            InputPurpose::SearchQuery => "INPUT: Search",
            InputPurpose::None => "INPUT",
        },
    };

    let mut spans: Vec<Span> = vec![
        Span::styled(" [", Style::default().fg(theme.text_muted)),
        Span::styled(
            mode_str,
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("] ", Style::default().fg(theme.text_muted)),
    ];

    // Show input buffer contents when in input mode
    if app.input_mode == InputMode::Input {
        spans.push(Span::styled(
            &app.input_buffer,
            Style::default().fg(theme.text_primary),
        ));
        let cursor_char = if app.tick_count % 10 < 5 { "█" } else { " " };
        spans.push(Span::styled(
            cursor_char,
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            "  (Enter: submit │ Esc: cancel)",
            Style::default().fg(theme.text_muted),
        ));
    }

    // Show error or status message (error takes precedence)
    if let Some(ref err) = app.error_message {
        spans.push(Span::styled(
            err,
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD),
        ));
    } else if let Some(ref msg) = app.status_message {
        spans.push(Span::styled(msg, Style::default().fg(theme.success)));
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(theme.border_inactive));

    frame.render_widget(paragraph, area);
}
