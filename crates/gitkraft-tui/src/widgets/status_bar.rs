use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, InputMode, InputPurpose};

/// Render the bottom status bar — a single line showing the current mode,
/// status message, or error message.
///
/// Format: ` [{mode}] {status_or_error}`
///  - Error messages are rendered in Red.
///  - Status messages are rendered in Green.
///  - The mode indicator is rendered in Yellow.
pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
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
        Span::styled(" [", Style::default().fg(Color::DarkGray)),
        Span::styled(
            mode_str,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("] ", Style::default().fg(Color::DarkGray)),
    ];

    // Show input buffer contents when in input mode
    if app.input_mode == InputMode::Input {
        spans.push(Span::styled(
            &app.input_buffer,
            Style::default().fg(Color::White),
        ));
        let cursor_char = if app.tick_count % 10 < 5 { "█" } else { " " };
        spans.push(Span::styled(
            cursor_char,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            "  (Enter: submit │ Esc: cancel)",
            Style::default().fg(Color::DarkGray),
        ));
    }

    // Show error or status message (error takes precedence)
    if let Some(ref err) = app.error_message {
        spans.push(Span::styled(
            err,
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
    } else if let Some(ref msg) = app.status_message {
        spans.push(Span::styled(msg, Style::default().fg(Color::Green)));
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(Color::DarkGray));

    frame.render_widget(paragraph, area);
}
