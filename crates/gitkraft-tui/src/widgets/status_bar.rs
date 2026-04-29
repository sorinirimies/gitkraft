use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_spinner::{FluxFrames, FluxSpinner};

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

    // Fill the whole status-bar row with the background colour first.
    frame.render_widget(
        Paragraph::new("").style(Style::default().bg(theme.border_inactive)),
        area,
    );

    // ── Layout ────────────────────────────────────────────────────────────
    // [3 cols: spinner/dot/blank] [rest: mode + message]
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);
    let left = chunks[0];
    let right = chunks[1];

    // ── Spinner area ──────────────────────────────────────────────────────
    // Centre a 1×1 cell inside the 3-col left block (1 col padding each side).
    let spinner_cell = Rect {
        x: left.x + 1,
        y: left.y,
        width: 1,
        height: 1,
    };

    if app.tab().is_loading {
        // Animated CORNERS spinner while a background task is in flight.
        frame.render_widget(
            FluxSpinner::new(app.tick_count)
                .frames(FluxFrames::CORNERS)
                .color(theme.accent),
            spinner_cell,
        );
    }
    // else: blank — no spinner rendered

    // ── Mode indicator + message ──────────────────────────────────────────
    let mode_str = match app.input_mode {
        InputMode::Normal => "NORMAL",
        InputMode::Input => match app.input_purpose {
            InputPurpose::CommitMessage => "INPUT: Commit",
            InputPurpose::BranchName => "INPUT: Branch",
            InputPurpose::RepoPath => "INPUT: Path",
            InputPurpose::SearchQuery => "INPUT: Search",
            InputPurpose::StashMessage => "INPUT: Stash Msg",
            InputPurpose::CommitActionInput1 => "INPUT: Action",
            InputPurpose::CommitActionInput2 => "INPUT: Action (2)",
            InputPurpose::None => "INPUT",
        },
    };

    let mut spans: Vec<Span> = vec![
        Span::styled("[", Style::default().fg(theme.text_muted)),
        Span::styled(
            mode_str,
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("] ", Style::default().fg(theme.text_muted)),
    ];

    if app.input_mode == InputMode::Input {
        spans.push(Span::styled(
            &app.input_buffer,
            Style::default().fg(theme.text_primary),
        ));
        let cursor = if app.tick_count % 10 < 5 { "█" } else { " " };
        spans.push(Span::styled(
            cursor,
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            "  (Enter: submit │ Esc: cancel)",
            Style::default().fg(theme.text_muted),
        ));
    }

    if let Some(ref err) = app.tab().error_message {
        spans.push(Span::styled(
            err,
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD),
        ));
    } else if let Some(ref msg) = app.tab().status_message {
        spans.push(Span::styled(msg, Style::default().fg(theme.success)));
    }

    let paragraph =
        Paragraph::new(Line::from(spans)).style(Style::default().bg(theme.border_inactive));
    frame.render_widget(paragraph, right);
}
