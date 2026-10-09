use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::utils::pad_right;

/// Render the About panel: app name, version, author and project info.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    use gitkraft_core::about;

    let theme = app.theme();

    let key_style = Style::default()
        .fg(theme.warning)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(theme.text_primary);
    let muted_style = Style::default().fg(theme.text_muted);
    let value_style = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD);

    let block = Block::default()
        .title(Line::from(vec![
            Span::styled("ⓘ ", Style::default().fg(theme.accent)),
            Span::styled(
                "About",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_active))
        .style(Style::default().bg(theme.bg))
        .padding(Padding::new(2, 2, 1, 0));

    let row = |label: &str, value: String| {
        Line::from(vec![
            Span::styled(pad_right(label, 10), muted_style),
            Span::styled(value, desc_style),
        ])
    };

    let lines = vec![
        Line::from(vec![
            Span::styled(about::APP_NAME, value_style),
            Span::styled(format!("  {}", about::version_label()), desc_style),
        ]),
        Line::from(""),
        Line::from(Span::styled(about::DESCRIPTION, desc_style)),
        Line::from(""),
        row("Author", about::AUTHOR_NAME.to_string()),
        row(
            "GitHub",
            format!("@{}  ({})", about::AUTHOR_GITHUB, about::AUTHOR_URL),
        ),
        row("Project", about::REPO_URL.to_string()),
        row("Version", about::VERSION.to_string()),
        row("License", about::LICENSE.to_string()),
        Line::from(""),
        Line::from(vec![
            Span::styled("g", key_style),
            Span::styled(" open GitHub profile   ", desc_style),
            Span::styled("p", key_style),
            Span::styled(" open project   ", desc_style),
            Span::styled("Esc / Shift + A", key_style),
            Span::styled(" close", desc_style),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}
