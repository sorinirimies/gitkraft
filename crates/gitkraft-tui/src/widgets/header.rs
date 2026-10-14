use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, AppScreen};

/// Header shortcut hints in priority order: (key, label).
const HINTS: [(&str, &str); 11] = [
    ("[q]", "quit"),
    ("[←→]", "pane"),
    ("[r]", "refresh"),
    ("[f]", "fetch"),
    ("[t]", "theme"),
    ("[o]", "open"),
    ("[W]", "close"),
    ("[E]", "editor"),
    ("[O]", "options"),
    ("[,]", "settings"),
    ("[A]", "about"),
];

/// Display width of one hint as rendered: key + " label ".
fn hint_width(h: &(&str, &str)) -> usize {
    h.0.chars().count() + h.1.chars().count() + 2
}

/// How many leading hints fit into `avail` columns.
fn visible_hints(avail: usize) -> usize {
    let mut used = 0;
    for (i, h) in HINTS.iter().enumerate() {
        used += hint_width(h);
        if used > avail {
            return i;
        }
    }
    HINTS.len()
}

/// Render the top header bar showing repo name, current branch, state, and
/// keyboard shortcuts.
pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    if app.screen != AppScreen::Main {
        return;
    }

    let theme = app.theme();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_inactive))
        .style(Style::default().bg(theme.border_inactive));

    let tab = app.tab();

    let repo_name = tab
        .repo_path
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let branch_name = tab
        .repo_info
        .as_ref()
        .and_then(|info| info.head_branch.clone())
        .unwrap_or_else(|| "detached".to_string());

    let state = tab
        .repo_info
        .as_ref()
        .map(|info| format!("{}", info.state))
        .unwrap_or_else(|| "?".to_string());

    let mut spans: Vec<Span> = Vec::new();

    // Show tab indicators if there are multiple tabs
    if app.tabs.len() > 1 {
        for (i, t) in app.tabs.iter().enumerate() {
            let name = t.display_name();
            let style = if i == app.active_tab_index {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_muted)
            };
            spans.push(Span::styled(format!(" {} ", name), style));
            if i < app.tabs.len() - 1 {
                spans.push(Span::styled("|", Style::default().fg(theme.text_muted)));
            }
        }
        // Tab switching hint
        spans.push(Span::styled(
            " [/]",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            " switch tab ",
            Style::default().fg(theme.text_primary),
        ));
        spans.push(Span::styled(
            "│ ",
            Style::default().fg(theme.text_secondary),
        ));
    }

    // When tabs are shown, the active tab name already displays the repo.
    // Only show the standalone repo name when there's a single tab.
    if app.tabs.len() <= 1 {
        spans.push(Span::styled(
            format!(" {} ", repo_name),
            Style::default()
                .fg(theme.text_primary)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled("│", Style::default().fg(theme.text_secondary)));
    }

    spans.extend([
        Span::styled(
            format!("  {} ", branch_name),
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│", Style::default().fg(theme.text_secondary)),
        Span::styled(format!(" {} ", state), Style::default().fg(theme.accent)),
        Span::styled("│", Style::default().fg(theme.text_secondary)),
    ]);

    // Shortcut hints, in priority order; drop from the end when the header
    // is too narrow (2 columns of border).
    let used: usize = spans.iter().map(|sp| sp.content.chars().count()).sum();
    let avail = (area.width as usize).saturating_sub(2 + used);
    for (key, label) in &HINTS[..visible_hints(avail)] {
        spans.push(Span::styled(
            *key,
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {label} "),
            Style::default().fg(theme.text_primary),
        ));
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).block(block);

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_space_no_hints() {
        assert_eq!(visible_hints(0), 0);
    }

    #[test]
    fn plenty_of_space_all_hints() {
        assert_eq!(visible_hints(500), HINTS.len());
    }

    #[test]
    fn hints_grow_monotonically_and_fit() {
        let mut prev = 0;
        for w in 0..300 {
            let n = visible_hints(w);
            assert!(n >= prev);
            let used: usize = HINTS[..n].iter().map(hint_width).sum();
            assert!(used <= w);
            prev = n;
        }
    }
}
