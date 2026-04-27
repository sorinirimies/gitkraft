use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

pub fn handle_key(app: &mut App, key: KeyEvent) {
    let total = gitkraft_core::EDITOR_NAMES.len() + 1; // +1 for "none"

    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            let i = app.editor_list_state.selected().unwrap_or(0);
            app.editor_list_state.select(Some((i + 1) % total));
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let i = app.editor_list_state.selected().unwrap_or(0);
            app.editor_list_state
                .select(Some(if i == 0 { total - 1 } else { i - 1 }));
        }
        KeyCode::Enter => {
            if let Some(idx) = app.editor_list_state.selected() {
                app.editor = if idx == 0 {
                    gitkraft_core::Editor::None
                } else {
                    gitkraft_core::Editor::from_index(idx - 1)
                };
                app.tab_mut().status_message = Some(format!("Editor set to {}", app.editor));
                let _ = gitkraft_core::features::persistence::save_editor_tui(
                    app.editor.display_name(),
                );
                app.show_editor_panel = false;
            }
        }
        KeyCode::Char('E') | KeyCode::Esc => {
            app.show_editor_panel = false;
        }
        _ => {}
    }
}
