use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

use crate::app::App;

/// Theme names in the same order as tui-file-explorer's `all_presets`.
pub const THEME_NAMES: &[&str] = &[
    "Default",
    "Grape",
    "Ocean",
    "Sunset",
    "Forest",
    "Rose",
    "Mono",
    "Neon",
    "Dracula",
    "Nord",
    "Solarized Dark",
    "Solarized Light",
    "Gruvbox Dark",
    "Gruvbox Light",
    "Catppuccin Latte",
    "Catppuccin Frappé",
    "Catppuccin Macchiato",
    "Catppuccin Mocha",
    "Tokyo Night",
    "Tokyo Night Storm",
    "Tokyo Night Light",
    "Kanagawa Wave",
    "Kanagawa Dragon",
    "Kanagawa Lotus",
    "Moonfly",
    "Nightfly",
    "Oxocarbon",
];

/// A representative accent color for each theme, used to tint the theme name
/// in the picker list.
pub const THEME_COLORS: &[Color] = &[
    Color::Rgb(80, 200, 255),  // Default - cyan
    Color::Rgb(180, 130, 255), // Grape - violet
    Color::Rgb(80, 200, 180),  // Ocean - teal
    Color::Rgb(255, 140, 60),  // Sunset - amber
    Color::Rgb(80, 180, 80),   // Forest - green
    Color::Rgb(255, 130, 160), // Rose - pink
    Color::Rgb(180, 180, 180), // Mono - grey
    Color::Rgb(0, 255, 200),   // Neon - bright cyan
    Color::Rgb(189, 147, 249), // Dracula - purple
    Color::Rgb(136, 192, 208), // Nord - frost blue
    Color::Rgb(42, 161, 152),  // Solarized Dark - cyan
    Color::Rgb(38, 139, 210),  // Solarized Light - blue
    Color::Rgb(250, 189, 47),  // Gruvbox Dark - yellow
    Color::Rgb(215, 153, 33),  // Gruvbox Light - yellow
    Color::Rgb(136, 57, 239),  // Catppuccin Latte - mauve
    Color::Rgb(202, 158, 230), // Catppuccin Frappé - mauve
    Color::Rgb(198, 160, 246), // Catppuccin Macchiato - mauve
    Color::Rgb(203, 166, 247), // Catppuccin Mocha - mauve
    Color::Rgb(122, 162, 247), // Tokyo Night - blue
    Color::Rgb(122, 162, 247), // Tokyo Night Storm - blue
    Color::Rgb(46, 126, 233),  // Tokyo Night Light - blue
    Color::Rgb(126, 156, 216), // Kanagawa Wave - crystal blue
    Color::Rgb(139, 164, 176), // Kanagawa Dragon - dragon blue
    Color::Rgb(77, 105, 155),  // Kanagawa Lotus - lotus blue
    Color::Rgb(128, 160, 255), // Moonfly - blue
    Color::Rgb(130, 170, 255), // Nightfly - blue
    Color::Rgb(78, 154, 232),  // Oxocarbon - blue
];

/// Render the theme picker panel on the right side of the main content area.
///
/// Shows a bordered block titled "Themes" with navigation hints at the top.
/// Lists all theme names numbered 1–27. The current theme is highlighted with
/// a reversed style and a `← ` arrow prefix.
pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let current = app.current_theme_index;

    let items: Vec<ListItem> = THEME_NAMES
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let number = format!("{:>2}. ", i + 1);
            let color = THEME_COLORS.get(i).copied().unwrap_or(Color::White);

            if i == current {
                // Highlighted row: arrow prefix, bold, custom background
                let line = Line::from(vec![
                    Span::styled(
                        "← ",
                        Style::default()
                            .fg(Color::Yellow)
                            .bg(Color::Rgb(40, 60, 80))
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        number,
                        Style::default()
                            .fg(Color::White)
                            .bg(Color::Rgb(40, 60, 80))
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        (*name).to_string(),
                        Style::default()
                            .fg(color)
                            .bg(Color::Rgb(40, 60, 80))
                            .add_modifier(Modifier::BOLD),
                    ),
                ]);
                ListItem::new(line)
            } else {
                let line = Line::from(vec![
                    Span::styled("  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(number, Style::default().fg(Color::DarkGray)),
                    Span::styled((*name).to_string(), Style::default().fg(color)),
                ]);
                ListItem::new(line)
            }
        })
        .collect();

    let block = Block::default()
        .title(" Themes  ↑ prev  ↓/t next ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let highlight_style = Style::default()
        .bg(Color::Rgb(40, 60, 80))
        .add_modifier(Modifier::BOLD);

    let list = List::new(items)
        .block(block)
        .highlight_style(highlight_style);

    let mut state = app.theme_list_state.clone();
    frame.render_stateful_widget(list, area, &mut state);
    app.theme_list_state = state;
}
