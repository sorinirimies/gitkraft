use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

use crate::app::App;

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let theme = app.theme();

    let block = Block::default()
        .title(" Editor — Shift+E close ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_active))
        .style(Style::default().bg(theme.bg));

    let items: Vec<ListItem> = std::iter::once(("none", gitkraft_core::Editor::None))
        .chain(
            gitkraft_core::EDITOR_NAMES
                .iter()
                .enumerate()
                .map(|(i, name)| (*name, gitkraft_core::Editor::from_index(i))),
        )
        .map(|(name, editor)| {
            let is_selected = editor == app.editor;
            let marker = if is_selected { "✔ " } else { "  " };
            let style = if is_selected {
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_primary)
            };
            ListItem::new(Line::from(vec![
                Span::styled(marker, style),
                Span::styled(name.to_string(), style),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(theme.sel_bg)
                .add_modifier(Modifier::REVERSED),
        )
        .highlight_symbol("▸ ");

    frame.render_stateful_widget(list, area, &mut app.editor_list_state);
}
