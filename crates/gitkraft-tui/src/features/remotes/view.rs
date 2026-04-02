use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

use crate::app::App;

/// Render the remotes list in the sidebar (below stashes).
pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let theme = app.theme();
    let border_color = theme.border_inactive;

    let block = Block::default()
        .title(" Remotes ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    if app.remotes.is_empty() {
        let items: Vec<ListItem> = vec![ListItem::new(Line::from(Span::styled(
            "  No remotes",
            Style::default().fg(theme.text_muted),
        )))];
        let list = List::new(items).block(block);
        frame.render_widget(list, area);
        return;
    }

    let items: Vec<ListItem> = app
        .remotes
        .iter()
        .map(|remote| {
            let url_part = remote.url.as_deref().unwrap_or("<no url>");

            let line = Line::from(vec![
                Span::styled(
                    format!("  {} ", remote.name),
                    Style::default().fg(theme.accent),
                ),
                Span::styled(
                    truncate_str(
                        url_part,
                        area.width.saturating_sub(remote.name.len() as u16 + 6) as usize,
                    ),
                    Style::default().fg(theme.text_muted),
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

/// Truncate a string to `max_len` characters, appending `…` if truncated.
fn truncate_str(s: &str, max_len: usize) -> String {
    if max_len == 0 {
        return String::new();
    }
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 1 {
        "…".to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}
