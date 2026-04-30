use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

use crate::app::{ActivePane, App};
use gitkraft_core::BranchType;

/// Render the branches list in the sidebar area.
pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let theme = app.theme();
    let is_active = app.active_pane == ActivePane::Branches;

    let border_color = if is_active {
        theme.border_active
    } else {
        theme.border_inactive
    };

    let block = Block::default()
        .title(" Branches ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.bg));

    if app.tab().branches.is_empty() {
        let items: Vec<ListItem> = vec![ListItem::new(Line::from(Span::styled(
            "  No branches",
            Style::default().fg(theme.text_muted),
        )))];
        let list = List::new(items).block(block);
        frame.render_widget(list, area);
        return;
    }

    let items: Vec<ListItem> = app
        .tab()
        .branches
        .iter()
        .map(|branch| {
            let (prefix, style) = if branch.is_head {
                (
                    "* ",
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                match branch.branch_type {
                    BranchType::Local => ("  ", Style::default().fg(theme.text_primary)),
                    BranchType::Remote => ("  ", Style::default().fg(theme.text_muted)),
                }
            };

            let icon = match branch.branch_type {
                BranchType::Local => "",
                BranchType::Remote => "⇄ ",
            };

            let line = Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(icon, style),
                Span::styled(branch.name.clone(), style),
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

    frame.render_stateful_widget(list, area, &mut app.tab_mut().branch_list_state);
}
