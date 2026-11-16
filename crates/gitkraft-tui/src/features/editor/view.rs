use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::app::App;

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let theme = app.theme();

    let block = Block::default()
        .title(" Editor · Enter choose · Esc close ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_active))
        .style(Style::default().bg(theme.bg));

    let on_custom = crate::features::editor::events::on_custom(app);
    let custom_label = if app.editor_custom.is_empty() {
        "Custom command…  (type here, Enter applies)".to_string()
    } else {
        format!(
            "Custom: {}{}",
            app.editor_custom,
            if on_custom { "▏" } else { "" }
        )
    };
    let rows = std::iter::once(("none".to_string(), gitkraft_core::Editor::None))
        .chain(
            gitkraft_core::EDITOR_NAMES
                .iter()
                .enumerate()
                .map(|(i, name)| (name.to_string(), gitkraft_core::Editor::from_index(i))),
        )
        .chain(std::iter::once((
            custom_label,
            gitkraft_core::Editor::Custom(app.editor_custom.trim().to_string()),
        )));
    let items: Vec<ListItem> = rows
        .map(|(name, editor)| {
            let is_selected = match (&editor, &app.editor) {
                (gitkraft_core::Editor::Custom(_), gitkraft_core::Editor::Custom(_)) => true,
                (a, b) => a == b,
            };
            let marker = if is_selected { "✔ " } else { "  " };
            let style = if is_selected {
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_primary)
            };
            let mut spans = vec![
                Span::styled(marker, style),
                Span::styled(name.clone(), style),
            ];
            if !matches!(
                editor,
                gitkraft_core::Editor::None | gitkraft_core::Editor::Custom(_)
            ) {
                spans.push(Span::styled(
                    format!("  {}", editor.kind_label()),
                    Style::default().fg(theme.text_muted),
                ));
                if !editor.is_installed() {
                    spans.push(Span::styled(
                        "  not installed",
                        Style::default().fg(theme.warning),
                    ));
                }
            }
            ListItem::new(Line::from(spans))
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
