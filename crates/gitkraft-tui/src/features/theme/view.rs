use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

use gitkraft_core::Rgb;

use super::palette::{picker_name, PICKER_LEN};

use crate::app::App;

/// Convert a core [`Rgb`] to a ratatui [`Color`].
fn rgb_to_color(rgb: Rgb) -> Color {
    Color::Rgb(rgb.r, rgb.g, rgb.b)
}

/// Build the accent-colour swatch for each theme by pulling the `accent` field
/// from the canonical core definition.  This replaces the old hand-maintained
/// `THEME_COLORS` constant.
///
/// `row` is a picker row (`0` = System, previewing the preset the OS currently
/// resolves to; `n + 1` = preset `n`).
fn accent_color_for_row(row: usize) -> Color {
    let index = if row == 0 {
        gitkraft_core::current_system_theme_index()
    } else {
        row - 1
    };
    rgb_to_color(gitkraft_core::theme_by_index(index).accent)
}

/// Render the theme picker panel on the right side of the main content area.
///
/// Shows a bordered block titled "Themes" with navigation hints at the top.
/// Lists "System" followed by every core preset. The current theme is highlighted with
/// a reversed style and a `← ` arrow prefix.
pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let theme = app.theme();
    let current = app.picker_index();

    let items: Vec<ListItem> = (0..PICKER_LEN)
        .map(|i| {
            let name = picker_name(i);
            let number = format!("{:>2}. ", i);
            let color = accent_color_for_row(i);

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
                        name.to_string(),
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
                    Span::styled(name.to_string(), Style::default().fg(color)),
                ]);
                ListItem::new(line)
            }
        })
        .collect();

    let block = Block::default()
        .title(" Themes  ↑/k prev  ↓/j/t next  Enter/Esc close ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_active))
        .style(Style::default().bg(theme.bg));

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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn draw(app: &mut App) -> String {
        let mut term = Terminal::new(TestBackend::new(60, 52)).unwrap();
        term.draw(|f| render(app, f, f.area())).unwrap();
        let buf = term.backend().buffer().clone();
        buf.content().iter().map(|c| c.symbol()).collect::<String>()
    }

    #[test]
    fn picker_renders_system_and_presets() {
        let mut app = App::new();
        app.set_follow_system();
        let out = draw(&mut app);
        assert!(out.contains("System"));
        assert!(out.contains("Default"));
        assert!(out.contains("←"));
    }

    #[test]
    fn picker_marks_selected_preset() {
        let mut app = App::new();
        app.set_theme(9);
        let out = draw(&mut app);
        assert!(out.contains("Dracula"));
        assert_eq!(app.theme_list_state.selected(), Some(10));
    }
}
