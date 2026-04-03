use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

use gitkraft_core::{Rgb, THEME_NAMES};

use crate::app::App;

/// Convert a core [`Rgb`] to a ratatui [`Color`].
fn rgb_to_color(rgb: Rgb) -> Color {
    Color::Rgb(rgb.r, rgb.g, rgb.b)
}

/// Build the accent-colour swatch for each theme by pulling the `accent` field
/// from the canonical core definition.  This replaces the old hand-maintained
/// `THEME_COLORS` constant.
fn accent_color_for_index(index: usize) -> Color {
    rgb_to_color(gitkraft_core::theme_by_index(index).accent)
}

/// Render the theme picker panel on the right side of the main content area.
///
/// Shows a bordered block titled "Themes" with navigation hints at the top.
/// Lists all theme names numbered 1–27. The current theme is highlighted with
/// a reversed style and a `← ` arrow prefix.
pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let theme = app.theme();
    let current = app.current_theme_index;

    let items: Vec<ListItem> = THEME_NAMES
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let number = format!("{:>2}. ", i + 1);
            let color = accent_color_for_index(i);

            if i == current {
                // Highlighted row: arrow prefix, bold, custom background
                let line = Line::from(vec![
                    Span::styled(
                        "← ",
                        Style::default()
                            .fg(theme.warning)
                            .bg(theme.sel_bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        number,
                        Style::default()
                            .fg(theme.text_primary)
                            .bg(theme.sel_bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        (*name).to_string(),
                        Style::default()
                            .fg(color)
                            .bg(theme.sel_bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]);
                ListItem::new(line)
            } else {
                let line = Line::from(vec![
                    Span::styled("  ", Style::default().fg(theme.text_muted)),
                    Span::styled(number, Style::default().fg(theme.text_muted)),
                    Span::styled((*name).to_string(), Style::default().fg(color)),
                ]);
                ListItem::new(line)
            }
        })
        .collect();

    let block = Block::default()
        .title(" Themes  ↑ prev  ↓/t next ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_active));

    let highlight_style = Style::default()
        .bg(theme.sel_bg)
        .add_modifier(Modifier::BOLD);

    let list = List::new(items)
        .block(block)
        .highlight_style(highlight_style);

    let mut state = app.theme_list_state;
    frame.render_stateful_widget(list, area, &mut state);
    app.theme_list_state = state;
}
