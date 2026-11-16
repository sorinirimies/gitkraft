//! Mouse on the main screen: click to focus and select, double-click to open, the wheel to scroll,
//! right-click for a menu of what can be done to the thing under the pointer.
//!
//! Every action the menu offers is a key the pane already understands, so a click runs exactly
//! what the key would and the two cannot drift apart.

use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::{ActivePane, App, InputMode, StagingFocus};

/// Two clicks on the same cell within this are a double-click.
const DOUBLE: Duration = Duration::from_millis(400);

/// Where each pane was drawn in the last frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PaneRects {
    /// The whole terminal.
    pub screen: Rect,
    /// Branch list.
    pub branches: Rect,
    /// Stash list.
    pub stash: Rect,
    /// Commit log.
    pub commits: Rect,
    /// Diff pane.
    pub diff: Rect,
    /// Unstaged files.
    pub unstaged: Rect,
    /// Staged files.
    pub staged: Rect,
    /// The editor picker, when shown as a pane.
    pub editor: Rect,
}

/// A clickable part of the header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderHit {
    /// A repository tab.
    Tab(usize),
    /// A shortcut hint: presses its key.
    Key(KeyCode),
}

/// What a right-click menu entry does: the key it presses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Item {
    /// Shown text.
    pub label: &'static str,
    /// Shown key hint.
    pub hint: &'static str,
    /// The key pressed when it is chosen.
    pub key: KeyCode,
}

const fn item(label: &'static str, hint: &'static str, key: KeyCode) -> Item {
    Item { label, hint, key }
}

/// An open right-click menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainMenu {
    /// The cell it hangs from.
    pub anchor: (u16, u16),
    /// Its entries.
    pub items: Vec<Item>,
    /// The highlighted entry.
    pub row: usize,
}

fn c(ch: char) -> KeyCode {
    KeyCode::Char(ch)
}

/// What can be done to a branch.
pub fn branch_items() -> Vec<Item> {
    vec![
        item("Check out", "Enter", KeyCode::Enter),
        item("New branch…", "b", c('b')),
        item("Merge into the current branch", "m", c('m')),
        item("Rebase onto it", "R", c('R')),
        item("Rename…", "e", c('e')),
        item("Delete", "D", c('D')),
        item("Delete on the remote", "X", c('X')),
    ]
}

/// What can be done to a commit.
pub fn commit_items() -> Vec<Item> {
    vec![
        item("Show its changes", "Enter", KeyCode::Enter),
        item("Cherry-pick", "C", c('C')),
        item("Revert", "e", c('e')),
        item("Reset soft to here", "x", c('x')),
        item("Reset mixed to here", "n", c('n')),
        item("Reset hard to here", "X", c('X')),
        item("Copy the id", "y", c('y')),
        item("Force push the branch", "F", c('F')),
    ]
}

/// What can be done to an unstaged file.
pub fn unstaged_items() -> Vec<Item> {
    vec![
        item("Stage", "s", c('s')),
        item("Show the diff", "Enter", KeyCode::Enter),
        item("Open in the editor", "e", c('e')),
        item("Discard changes", "d", c('d')),
        item("File history", "H", c('H')),
        item("Blame", "B", c('B')),
        item("Delete the file", "D", c('D')),
        item("Stage everything", "S", c('S')),
        item("Stash…", "z", c('z')),
    ]
}

/// What can be done to a staged file.
pub fn staged_items() -> Vec<Item> {
    vec![
        item("Unstage", "u", c('u')),
        item("Show the diff", "Enter", KeyCode::Enter),
        item("Open in the editor", "e", c('e')),
        item("File history", "H", c('H')),
        item("Blame", "B", c('B')),
        item("Commit…", "c", c('c')),
        item("Draft the message with the AI", "g", c('g')),
        item("Unstage everything", "U", c('U')),
    ]
}

/// What can be done to a stash.
pub fn stash_items() -> Vec<Item> {
    vec![
        item("Pop (apply and drop)", "Enter", KeyCode::Enter),
        item("Apply (keep it)", "a", c('a')),
        item("Drop", "d", c('d')),
    ]
}

/// What can be done anywhere.
pub fn global_items() -> Vec<Item> {
    vec![
        item("Refresh", "r", c('r')),
        item("Fetch", "f", c('f')),
        item("Pull", "p", c('p')),
        item("Push", "P", c('P')),
        item("Actions pane", "A", c('A')),
        item("Choose the editor", "E", c('E')),
        item("Theme", "T", c('T')),
        item("Options", "O", c('O')),
        item("Settings", "M", c('M')),
        item("Info", "I", c('I')),
        item("Open the hub", "H", c('H')),
    ]
}

/// The row of a bordered one-line-per-item list that `y` falls on, if it is an item.
pub fn row_at(rect: Rect, offset: usize, y: u16, len: usize) -> Option<usize> {
    let top = rect.y.checked_add(1)?;
    let bottom = rect.y.checked_add(rect.height)?.checked_sub(1)?;
    if y < top || y >= bottom {
        return None;
    }
    let i = offset + usize::from(y - top);
    (i < len).then_some(i)
}

fn inside(r: Rect, x: u16, y: u16) -> bool {
    x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
}

/// The pane under a cell.
fn pane_at(app: &App, x: u16, y: u16) -> Option<(ActivePane, Option<StagingFocus>)> {
    let r = &app.rects;
    if inside(r.branches, x, y) {
        Some((ActivePane::Branches, None))
    } else if inside(r.stash, x, y) {
        Some((ActivePane::Stash, None))
    } else if inside(r.commits, x, y) {
        Some((ActivePane::CommitLog, None))
    } else if inside(r.diff, x, y) {
        Some((ActivePane::DiffView, None))
    } else if inside(r.unstaged, x, y) {
        Some((ActivePane::Staging, Some(StagingFocus::Unstaged)))
    } else if inside(r.staged, x, y) {
        Some((ActivePane::Staging, Some(StagingFocus::Staged)))
    } else {
        None
    }
}

/// Focuses the pane under the pointer and selects the row it points at. Returns the row.
fn focus_and_select(app: &mut App, x: u16, y: u16) -> Option<usize> {
    let (pane, focus) = pane_at(app, x, y)?;
    app.active_pane = pane;
    app.tab_mut().confirm_discard = false;
    let r = app.rects;
    match pane {
        ActivePane::Branches => {
            let t = app.tab();
            let i = row_at(
                r.branches,
                t.branch_list_state.offset(),
                y,
                t.branches.len(),
            )?;
            app.tab_mut().branch_list_state.select(Some(i));
            Some(i)
        }
        ActivePane::Stash => {
            let t = app.tab();
            let i = row_at(r.stash, t.stash_list_state.offset(), y, t.stashes.len())?;
            app.tab_mut().stash_list_state.select(Some(i));
            Some(i)
        }
        ActivePane::CommitLog => {
            let len = crate::features::commits::events::active_commits_len(app);
            let i = row_at(r.commits, app.tab().commit_list_state.offset(), y, len)?;
            crate::features::commits::events::select_at(app, i);
            Some(i)
        }
        ActivePane::Staging => {
            let focus = focus?;
            app.tab_mut().staging_focus = focus;
            let t = app.tab();
            let (rect, off, len) = match focus {
                StagingFocus::Unstaged => (
                    r.unstaged,
                    t.unstaged_list_state.offset(),
                    t.unstaged_changes.len(),
                ),
                StagingFocus::Staged => (
                    r.staged,
                    t.staged_list_state.offset(),
                    t.staged_changes.len(),
                ),
            };
            let i = row_at(rect, off, y, len)?;
            let t = app.tab_mut();
            match focus {
                StagingFocus::Unstaged => {
                    t.unstaged_list_state.select(Some(i));
                    t.anchor_unstaged = Some(i);
                    t.selected_unstaged.clear();
                }
                StagingFocus::Staged => {
                    t.staged_list_state.select(Some(i));
                    t.anchor_staged = Some(i);
                    t.selected_staged.clear();
                }
            }
            app.load_staging_diff();
            Some(i)
        }
        ActivePane::DiffView => None,
    }
}

fn press(app: &mut App, code: KeyCode) {
    crate::events::handle_key(app, KeyEvent::new(code, KeyModifiers::NONE));
}

/// The menu's rectangle on a screen: hangs from the anchor, kept on screen.
pub fn menu_rect(m: &MainMenu, screen: Rect) -> Rect {
    let longest = m
        .items
        .iter()
        .map(|i| i.label.chars().count() + i.hint.chars().count() + 3)
        .max()
        .unwrap_or(10) as u16;
    let w = (longest + 4).min(screen.width);
    let h = (m.items.len() as u16 + 2).min(screen.height);
    Rect::new(
        m.anchor.0.min(screen.x + screen.width - w),
        m.anchor.1.min(screen.y + screen.height - h),
        w,
        h,
    )
}

/// The entries for what is under `(x, y)`: the thing's own, then the global ones.
fn menu_for(app: &App, x: u16, y: u16) -> Vec<Item> {
    let own = match pane_at(app, x, y) {
        Some((ActivePane::Branches, _)) if !app.tab().branches.is_empty() => branch_items(),
        Some((ActivePane::Stash, _)) if !app.tab().stashes.is_empty() => stash_items(),
        Some((ActivePane::CommitLog, _)) if !app.tab().commits.is_empty() => commit_items(),
        Some((ActivePane::Staging, Some(StagingFocus::Unstaged)))
            if !app.tab().unstaged_changes.is_empty() =>
        {
            unstaged_items()
        }
        Some((ActivePane::Staging, Some(StagingFocus::Staged)))
            if !app.tab().staged_changes.is_empty() =>
        {
            staged_items()
        }
        Some((ActivePane::DiffView, _)) => vec![item("Open in the editor", "e", c('e'))],
        _ => Vec::new(),
    };
    own.into_iter().chain(global_items()).collect()
}

/// Handles a mouse event on the main screen.
pub fn handle(app: &mut App, ev: MouseEvent) {
    let (x, y) = (ev.column, ev.row);
    if app.menu.is_open() {
        return settings_mouse(app, ev);
    }
    if app.input_mode == InputMode::Input || crate::features::commits::events::popup_is_open(app) {
        return;
    }
    if app.main_menu.is_some() {
        return menu_mouse(app, ev);
    }
    if app.show_editor_panel {
        return editor_mouse(app, ev);
    }
    if app.show_about_panel || app.show_theme_panel || app.show_options_panel {
        return panel_mouse(app, ev);
    }
    if app.tab().file_history_path.is_some() || app.tab().blame_path.is_some() {
        return; // these overlays have their own keys
    }
    if let MouseEventKind::Down(MouseButton::Left) = ev.kind
        && let Some(hit) = app
            .header_hits
            .iter()
            .find(|(r, _)| inside(*r, x, y))
            .map(|(_, h)| *h)
    {
        match hit {
            HeaderHit::Tab(i) if i < app.tabs.len() => {
                app.active_tab_index = i;
                app.screen = if app.tab().repo_path.is_some() {
                    crate::app::AppScreen::Main
                } else {
                    crate::app::AppScreen::Welcome
                };
            }
            HeaderHit::Tab(_) => {}
            HeaderHit::Key(k) => press(app, k),
        }
        return;
    }
    match ev.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let now = Instant::now();
            let double = app
                .last_click
                .is_some_and(|(t, cx, cy)| (cx, cy) == (x, y) && now.duration_since(t) < DOUBLE);
            app.last_click = Some((now, x, y));
            let row = focus_and_select(app, x, y);
            if double && row.is_some() {
                let open = match app.active_pane {
                    ActivePane::Branches | ActivePane::CommitLog | ActivePane::Staging => {
                        Some(KeyCode::Enter)
                    }
                    _ => None,
                };
                if let Some(k) = open {
                    press(app, k);
                }
            }
        }
        MouseEventKind::Down(MouseButton::Right) => {
            focus_and_select(app, x, y);
            app.main_menu = Some(MainMenu {
                anchor: (x, y),
                items: menu_for(app, x, y),
                row: 0,
            });
        }
        MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
            let Some((pane, focus)) = pane_at(app, x, y) else {
                return;
            };
            app.active_pane = pane;
            if let Some(f) = focus {
                app.tab_mut().staging_focus = f;
            }
            let key = if ev.kind == MouseEventKind::ScrollDown {
                c('j')
            } else {
                c('k')
            };
            for _ in 0..3 {
                press(app, key);
            }
        }
        _ => {}
    }
}

/// Mouse on the settings menu (`M`): click a row to change it, outside to close.
fn settings_mouse(app: &mut App, ev: MouseEvent) {
    let rect = crate::features::menu::view::popup_rect(app.rects.screen, 1.0);
    let rows = crate::features::menu::Row::ALL.len();
    match ev.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if !inside(rect, ev.column, ev.row) {
                app.close_menu(Instant::now());
                return;
            }
            // border, then a blank line, then the rows
            let first = rect.y + 2;
            if ev.row >= first && usize::from(ev.row - first) < rows {
                crate::features::menu::events::click_row(app, usize::from(ev.row - first));
            }
        }
        MouseEventKind::Down(MouseButton::Right) => app.close_menu(Instant::now()),
        MouseEventKind::ScrollDown => app.menu.step(1),
        MouseEventKind::ScrollUp => app.menu.step(-1),
        _ => {}
    }
}

/// Mouse on the theme, options and info panels: pick a theme, click outside or right-click to
/// close.
fn panel_mouse(app: &mut App, ev: MouseEvent) {
    let rect = app.panel_rect;
    match ev.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if !inside(rect, ev.column, ev.row) {
                app.show_about_panel = false;
                app.show_theme_panel = false;
                app.show_options_panel = false;
            } else if app.show_theme_panel
                && let Some(i) = row_at(
                    rect,
                    app.theme_list_state.offset(),
                    ev.row,
                    crate::features::theme::palette::picker_len(),
                )
            {
                app.select_picker(i);
                app.save_theme();
            }
        }
        MouseEventKind::Down(MouseButton::Right) => {
            app.show_about_panel = false;
            app.show_theme_panel = false;
            app.show_options_panel = false;
        }
        MouseEventKind::ScrollDown | MouseEventKind::ScrollUp if app.show_theme_panel => {
            app.cycle_theme(if ev.kind == MouseEventKind::ScrollDown {
                1
            } else {
                -1
            });
            app.save_theme();
        }
        _ => {}
    }
}

fn menu_mouse(app: &mut App, ev: MouseEvent) {
    let Some(menu) = app.main_menu.clone() else {
        return;
    };
    let rect = menu_rect(&menu, app.rects.screen);
    let (x, y) = (ev.column, ev.row);
    match ev.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let row = (inside(rect, x, y) && y > rect.y && y + 1 < rect.y + rect.height)
                .then(|| usize::from(y - rect.y - 1));
            app.main_menu = None;
            if let Some(it) = row.and_then(|r| menu.items.get(r)) {
                press(app, it.key);
            }
        }
        MouseEventKind::Down(MouseButton::Right) => {
            app.main_menu = None;
            handle(app, ev);
        }
        MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
            if let Some(m) = app.main_menu.as_mut() {
                let last = m.items.len().saturating_sub(1);
                m.row = if ev.kind == MouseEventKind::ScrollDown {
                    (m.row + 1).min(last)
                } else {
                    m.row.saturating_sub(1)
                };
            }
        }
        _ => {}
    }
}

/// Keys while a main-screen menu is open: arrows and Enter, Esc closes.
pub fn menu_key(app: &mut App, key: KeyEvent) {
    let Some(m) = app.main_menu.as_mut() else {
        return;
    };
    let last = m.items.len().saturating_sub(1);
    match key.code {
        KeyCode::Down | KeyCode::Char('j') => m.row = (m.row + 1).min(last),
        KeyCode::Up | KeyCode::Char('k') => m.row = m.row.saturating_sub(1),
        KeyCode::Enter => {
            let pick = m.items.get(m.row).copied();
            app.main_menu = None;
            if let Some(it) = pick {
                press(app, it.key);
            }
        }
        _ => app.main_menu = None,
    }
}

/// Mouse on the editor picker (a pane on the main screen, a popup over the hub).
pub fn editor_mouse(app: &mut App, ev: MouseEvent) {
    use crate::features::editor::events;
    let total = events::total();
    match ev.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let rect = app.rects.editor;
            if !inside(rect, ev.column, ev.row) {
                app.show_editor_panel = false;
                return;
            }
            if let Some(i) = row_at(rect, app.editor_list_state.offset(), ev.row, total) {
                if i == total - 1 {
                    app.editor_list_state.select(Some(i)); // the custom row: type, then Enter
                } else {
                    events::choose(app, i);
                }
            }
        }
        MouseEventKind::Down(MouseButton::Right) => app.show_editor_panel = false,
        MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
            let i = app.editor_list_state.selected().unwrap_or(0);
            let next = if ev.kind == MouseEventKind::ScrollDown {
                (i + 1).min(total - 1)
            } else {
                i.saturating_sub(1)
            };
            app.editor_list_state.select(Some(next));
        }
        _ => {}
    }
}

/// Draws the open menu over everything.
pub fn render_menu(app: &App, frame: &mut Frame) {
    let Some(m) = &app.main_menu else { return };
    let theme = app.theme();
    let rect = menu_rect(m, frame.area());
    frame.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(" Actions ")
        .style(Style::default().bg(theme.bg));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);
    let lines: Vec<Line> = m
        .items
        .iter()
        .enumerate()
        .take(inner.height as usize)
        .map(|(i, it)| {
            let sel = i == m.row;
            let base = if sel {
                Style::default().fg(theme.bg).bg(theme.accent)
            } else {
                Style::default().fg(theme.text_primary)
            };
            let hint = if sel {
                base
            } else {
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD)
            };
            let pad = (inner.width as usize)
                .saturating_sub(it.label.chars().count() + it.hint.chars().count() + 2);
            Line::from(vec![
                Span::styled(format!(" {}{}", it.label, " ".repeat(pad)), base),
                Span::styled(format!("{} ", it.hint), hint),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(kind: MouseEventKind, x: u16, y: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn files(n: usize, prefix: &str) -> Vec<gitkraft_core::DiffFileEntry> {
        (0..n)
            .map(|i| gitkraft_core::DiffFileEntry {
                old_file: String::new(),
                new_file: format!("{prefix}{i}.rs"),
                status: gitkraft_core::FileStatus::Modified,
            })
            .collect()
    }

    fn rig() -> App {
        let mut app = App::new();
        app.screen = crate::app::AppScreen::Main;
        app.editor = gitkraft_core::Editor::None;
        app.tab_mut().unstaged_changes = files(3, "u");
        app.tab_mut().staged_changes = files(2, "s");
        app.rects = PaneRects {
            screen: Rect::new(0, 0, 100, 40),
            branches: Rect::new(0, 3, 20, 10),
            stash: Rect::new(0, 13, 20, 5),
            commits: Rect::new(20, 3, 30, 20),
            diff: Rect::new(50, 3, 50, 20),
            unstaged: Rect::new(0, 23, 50, 12),
            staged: Rect::new(50, 23, 50, 12),
            editor: Rect::default(),
        };
        app
    }

    #[test]
    fn rows_are_found_inside_the_border_and_offset() {
        let r = Rect::new(0, 5, 20, 6); // rows at y 6..=9
        assert_eq!(row_at(r, 0, 5, 9), None, "the top border");
        assert_eq!(row_at(r, 0, 6, 9), Some(0));
        assert_eq!(row_at(r, 3, 8, 9), Some(5), "scrolled by 3");
        assert_eq!(row_at(r, 0, 9, 9), Some(3));
        assert_eq!(row_at(r, 0, 10, 9), None, "the bottom border");
        assert_eq!(row_at(r, 0, 8, 2), None, "past the last item");
    }

    #[test]
    fn a_click_focuses_the_list_and_selects_the_row_under_it() {
        let mut app = rig();
        handle(
            &mut app,
            ev(MouseEventKind::Down(MouseButton::Left), 60, 25),
        );
        assert_eq!(app.active_pane, ActivePane::Staging);
        assert_eq!(app.tab().staging_focus, StagingFocus::Staged);
        assert_eq!(app.tab().staged_list_state.selected(), Some(1));
        handle(&mut app, ev(MouseEventKind::Down(MouseButton::Left), 5, 24));
        assert_eq!(app.tab().staging_focus, StagingFocus::Unstaged);
        assert_eq!(app.tab().unstaged_list_state.selected(), Some(0));
    }

    #[test]
    fn the_wheel_moves_the_list_under_the_pointer() {
        let mut app = rig();
        app.tab_mut().unstaged_changes = files(8, "u");
        app.tab_mut().unstaged_list_state.select(Some(0));
        handle(&mut app, ev(MouseEventKind::ScrollDown, 5, 25));
        assert_eq!(app.active_pane, ActivePane::Staging);
        assert_eq!(app.tab().unstaged_list_state.selected(), Some(3));
        handle(&mut app, ev(MouseEventKind::ScrollUp, 5, 25));
        assert_eq!(app.tab().unstaged_list_state.selected(), Some(0));
    }

    #[test]
    fn a_right_click_selects_the_row_and_offers_its_actions_then_the_global_ones() {
        let mut app = rig();
        handle(
            &mut app,
            ev(MouseEventKind::Down(MouseButton::Right), 5, 26),
        );
        assert_eq!(app.tab().unstaged_list_state.selected(), Some(2));
        let m = app.main_menu.clone().expect("menu");
        assert_eq!(m.items[0].label, "Stage");
        assert!(m.items.iter().any(|i| i.label == "Refresh"));
        // right-clicking an empty branch list offers only the global entries
        let mut app = rig();
        handle(&mut app, ev(MouseEventKind::Down(MouseButton::Right), 5, 5));
        let m = app.main_menu.expect("menu");
        assert_eq!(m.items.len(), global_items().len());
    }

    #[test]
    fn choosing_a_menu_entry_presses_its_key_and_a_click_outside_closes_it() {
        let mut app = rig();
        handle(
            &mut app,
            ev(MouseEventKind::Down(MouseButton::Right), 60, 5),
        );
        let menu = app.main_menu.clone().expect("menu");
        let rect = menu_rect(&menu, app.rects.screen);
        let n = menu
            .items
            .iter()
            .position(|i| i.label == "Actions pane")
            .expect("entry");
        assert!(!app.show_actions_panel);
        handle(
            &mut app,
            ev(
                MouseEventKind::Down(MouseButton::Left),
                rect.x + 2,
                rect.y + 1 + n as u16,
            ),
        );
        assert!(
            app.main_menu.is_none() && app.show_actions_panel,
            "ran the A key"
        );
        handle(
            &mut app,
            ev(MouseEventKind::Down(MouseButton::Right), 60, 5),
        );
        handle(&mut app, ev(MouseEventKind::Down(MouseButton::Left), 1, 39));
        assert!(app.main_menu.is_none());
        assert!(app.show_actions_panel, "nothing ran");
    }

    #[test]
    fn the_menu_has_a_keyboard_fallback() {
        let mut app = rig();
        handle(
            &mut app,
            ev(MouseEventKind::Down(MouseButton::Right), 60, 5),
        );
        let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
        menu_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.main_menu.as_ref().expect("open").row, 1);
        menu_key(&mut app, key(KeyCode::Esc));
        assert!(app.main_menu.is_none());
    }

    #[test]
    fn the_menu_stays_on_screen() {
        let m = MainMenu {
            anchor: (99, 39),
            items: global_items(),
            row: 0,
        };
        let screen = Rect::new(0, 0, 100, 40);
        let r = menu_rect(&m, screen);
        assert!(r.x + r.width <= 100 && r.y + r.height <= 40);
    }

    #[test]
    fn a_double_click_opens_and_other_screens_ignore_the_mouse() {
        let mut app = rig();
        app.input_mode = InputMode::Input;
        handle(&mut app, ev(MouseEventKind::Down(MouseButton::Left), 5, 24));
        assert_eq!(
            app.active_pane,
            ActivePane::Branches,
            "typing owns the input"
        );
    }
}
