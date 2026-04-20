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

    if app.tab().remotes.is_empty() {
        let items: Vec<ListItem> = vec![ListItem::new(Line::from(Span::styled(
            "  No remotes",
            Style::default().fg(theme.text_muted),
        )))];
        let list = List::new(items).block(block);
        frame.render_widget(list, area);
        return;
    }

    let items: Vec<ListItem> = app
        .tab()
        .remotes
        .iter()
        .map(|remote| {
            let url_part = remote.url.as_deref().unwrap_or("<no url>");
            let max_url_len = area.width.saturating_sub(remote.name.len() as u16 + 6) as usize;

            let line = Line::from(vec![
                Span::styled(
                    format!("  {} ", remote.name),
                    Style::default().fg(theme.accent),
                ),
                Span::styled(
                    gitkraft_core::truncate_str(url_part, max_url_len),
                    Style::default().fg(theme.text_muted),
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}
