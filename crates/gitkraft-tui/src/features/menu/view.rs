//! Draws the menu: a centred popup whose height and width ease open over a dimmed screen.

use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph};

use super::Row;
use crate::anim::lerp;
use crate::app::App;

const WIDTH: u16 = 64;
/// Borders (2) + header gap (1) + rows + gap (1) + help line (1).
fn full_height() -> u16 {
    Row::ALL.len() as u16 + 5
}

/// The popup rectangle for an openness of `progress` inside `area`.
#[must_use]
pub fn popup_rect(area: Rect, progress: f32) -> Rect {
    let full_w = WIDTH.min(area.width.saturating_sub(2));
    let full_h = full_height().min(area.height.saturating_sub(2));
    let w = lerp(f32::from(full_w) * 0.6, f32::from(full_w), progress).round() as u16;
    let h = lerp(3.0, f32::from(full_h), progress).round() as u16;
    let (w, h) = (w.max(3).min(area.width), h.max(3).min(area.height));
    Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    }
}

/// The value shown on the right of a row.
fn value(app: &App, row: Row) -> String {
    match row {
        Row::Theme => format!("◀ {} ▶", app.current_theme_name()),
        Row::ReducedMotion => if app.reduced_motion { "on" } else { "off" }.to_string(),
        Row::CommitStyle => "edit ›".to_string(),
        Row::SettingsFile => "open ›".to_string(),
        Row::Keys => "show ›".to_string(),
    }
}

/// Renders the menu if any of it is on screen.
pub fn render(app: &App, frame: &mut Frame, now: Instant) {
    if !app.menu.is_shown() {
        return;
    }
    let progress = app.menu.progress(now);
    let area = frame.area();
    let theme = app.theme();

    // Dim what is behind, more as the popup opens.
    if progress > 0.25 {
        let buf = frame.buffer_mut();
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].modifier.insert(Modifier::DIM);
            }
        }
    }

    let rect = popup_rect(area, progress);
    frame.render_widget(Clear, rect);
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled("⚙ ", Style::default().fg(theme.accent)),
            Span::styled(
                "Menu",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_active))
        .style(Style::default().bg(theme.bg))
        .padding(Padding::new(1, 1, 0, 0));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    // Only what fits is drawn, so rows appear one by one as the popup grows.
    let width = usize::from(inner.width);
    let mut lines: Vec<Line> = vec![Line::raw("")];
    for (i, row) in Row::ALL.iter().enumerate() {
        let selected = i == app.menu.row;
        let label = row.label();
        let val = value(app, *row);
        let gap = width
            .saturating_sub(label.chars().count() + val.chars().count() + 2)
            .max(1);
        let marker = if selected { "▸ " } else { "  " };
        let (label_style, value_style) = if selected {
            (
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (
                Style::default().fg(theme.text_primary),
                Style::default().fg(theme.text_muted),
            )
        };
        lines.push(Line::from(vec![
            Span::styled(marker, label_style),
            Span::styled(label, label_style),
            Span::raw(" ".repeat(gap)),
            Span::styled(val, value_style),
        ]));
    }
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        app.menu.selected().help(),
        Style::default().fg(theme.text_muted),
    ));
    frame.render_widget(Paragraph::new(lines), inner);
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::features::menu::DURATION;

    fn draw(app: &App, now: Instant) -> String {
        let mut term = Terminal::new(TestBackend::new(80, 24)).expect("terminal");
        term.draw(|f| render(app, f, now)).expect("draw");
        let buf = term.backend().buffer().clone();
        (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn the_popup_grows_from_a_slit_to_full_size_and_stays_centred() {
        let area = Rect::new(0, 0, 80, 24);
        let closed = popup_rect(area, 0.0);
        let open = popup_rect(area, 1.0);
        assert!(closed.height <= 3 && open.height > closed.height);
        assert!(open.width > closed.width);
        for r in [closed, open] {
            assert_eq!(r.x * 2 + r.width, 80, "centred horizontally");
        }
        let tiny = popup_rect(Rect::new(0, 0, 10, 4), 1.0);
        assert!(tiny.width <= 10 && tiny.height <= 4);
    }

    #[test]
    fn nothing_is_drawn_while_closed_and_rows_appear_as_it_opens() {
        let t0 = Instant::now();
        let mut app = App::new();
        assert!(!draw(&app, t0).contains("Menu"));
        app.menu.open(t0, false);
        let early = draw(&app, t0 + Duration::from_millis(10));
        let full = draw(&app, t0 + DURATION * 2);
        assert!(!early.contains("Keys"), "last row is not there yet");
        for want in [
            "Menu",
            "Theme",
            "Reduced motion",
            "Commit message style",
            "Keys",
            "▸",
        ] {
            assert!(full.contains(want), "{want}\n{full}");
        }
        assert!(
            full.contains("Space toggles") || full.contains("change"),
            "{full}"
        );
    }
}
