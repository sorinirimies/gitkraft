use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

use crate::app::App;

/// Render the stash list in the sidebar below the branches list.
pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let border_color = Color::DarkGray;

    let title = format!(" Stashes ({}) ", app.stashes.len());

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    if app.stashes.is_empty() {
        let items: Vec<ListItem> = vec![ListItem::new(Line::from(Span::styled(
            "  No stashes",
            Style::default().fg(Color::DarkGray),
        )))];
        let list = List::new(items).block(block);
        frame.render_widget(list, area);
        return;
    }

    let items: Vec<ListItem> = app
        .stashes
        .iter()
        .map(|entry| {
            let truncated_msg = if entry.message.len() > 20 {
                format!("{}…", &entry.message[..19])
            } else {
                entry.message.clone()
            };

            let line = Line::from(vec![
                Span::styled(
                    format!("{}:", entry.index),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" {}", truncated_msg),
                    Style::default().fg(Color::White),
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(block);

    frame.render_widget(list, area);
}
