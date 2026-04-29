use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

use crate::app::App;

/// Render the stash list in the sidebar below the branches list.
pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let theme = app.theme();
    let is_active = app.active_pane == crate::app::ActivePane::Stash;
    let border_color = if is_active {
        theme.border_active
    } else {
        theme.border_inactive
    };

    let title = if is_active && !app.tab().stashes.is_empty() {
        format!(
            " Stashes ({}) [Enter=pop  d=drop] ",
            app.tab().stashes.len()
        )
    } else {
        format!(" Stashes ({}) ", app.tab().stashes.len())
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    if app.tab().stashes.is_empty() {
        let items: Vec<ListItem> = vec![ListItem::new(Line::from(Span::styled(
            "  No stashes",
            Style::default().fg(theme.text_muted),
        )))];
        let list = List::new(items).block(block);
        frame.render_widget(list, area);
        return;
    }

    let items: Vec<ListItem> = app
        .tab()
        .stashes
        .iter()
        .map(|entry| {
            let truncated_msg = entry.short_message(40);

            let line = Line::from(vec![
                Span::styled(
                    format!("{}:", entry.index),
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" {}", truncated_msg),
                    Style::default().fg(theme.text_primary),
                ),
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

    frame.render_stateful_widget(list, area, &mut app.tab_mut().stash_list_state);
}
