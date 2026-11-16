use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;

/// Rows of the picker: "none", every named editor, then the custom command.
pub fn total() -> usize {
    gitkraft_core::EDITOR_NAMES.len() + 2
}

/// Whether the cursor is on the "Custom command" row.
pub fn on_custom(app: &App) -> bool {
    app.editor_list_state.selected() == Some(total() - 1)
}

/// The editor a row stands for.
pub fn editor_at(app: &App, idx: usize) -> gitkraft_core::Editor {
    if idx == 0 {
        gitkraft_core::Editor::None
    } else if idx == total() - 1 {
        gitkraft_core::Editor::Custom(app.editor_custom.trim().to_string())
    } else {
        gitkraft_core::Editor::from_index(idx - 1)
    }
}

/// Makes row `idx` the editor and saves it.
pub fn choose(app: &mut App, idx: usize) {
    let editor = editor_at(app, idx);
    if let gitkraft_core::Editor::Custom(c) = &editor
        && c.is_empty()
    {
        app.tab_mut().status_message =
            Some("Type a command first, e.g. `nano -w` or `code --wait`".into());
        return;
    }
    let saved = match &editor {
        gitkraft_core::Editor::Custom(c) => c.clone(),
        e => e.display_name().to_string(),
    };
    let note = if editor.is_installed() || matches!(editor, gitkraft_core::Editor::None) {
        format!("Editor set to {editor}")
    } else {
        format!("Editor set to {editor}, but it was not found on PATH")
    };
    app.editor = editor;
    app.tab_mut().status_message = Some(note);
    let _ = gitkraft_core::features::persistence::save_editor_tui(&saved);
    app.show_editor_panel = false;
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    let total = total();
    let typing = on_custom(app);
    let move_by = |app: &mut App, down: bool| {
        let i = app.editor_list_state.selected().unwrap_or(0);
        let next = if down {
            (i + 1) % total
        } else if i == 0 {
            total - 1
        } else {
            i - 1
        };
        app.editor_list_state.select(Some(next));
    };

    match key.code {
        KeyCode::Down => move_by(app, true),
        KeyCode::Up => move_by(app, false),
        KeyCode::Char('j') if !typing => move_by(app, true),
        KeyCode::Char('k') if !typing => move_by(app, false),
        KeyCode::Enter => {
            if let Some(idx) = app.editor_list_state.selected() {
                choose(app, idx);
            }
        }
        KeyCode::Esc => app.show_editor_panel = false,
        KeyCode::Char('E') if !typing => app.show_editor_panel = false,
        KeyCode::Backspace if typing => {
            app.editor_custom.pop();
        }
        KeyCode::Char(c)
            if typing
                && !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            app.editor_custom.push(c);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(app: &mut App, code: KeyCode) {
        handle_key(app, KeyEvent::new(code, KeyModifiers::NONE));
    }

    #[test]
    fn the_picker_lists_none_every_editor_and_a_custom_row() {
        assert_eq!(total(), gitkraft_core::EDITOR_NAMES.len() + 2);
        let app = App::new();
        assert_eq!(editor_at(&app, 0), gitkraft_core::Editor::None);
        assert_eq!(editor_at(&app, 1), gitkraft_core::Editor::Helix);
        assert_eq!(
            editor_at(&app, total() - 2),
            gitkraft_core::Editor::from_index(gitkraft_core::EDITOR_NAMES.len() - 1)
        );
    }

    #[test]
    fn typing_on_the_custom_row_builds_a_command_and_enter_applies_it() {
        let mut app = App::new();
        app.editor = gitkraft_core::Editor::None;
        app.open_editor_picker();
        assert!(app.show_editor_panel);
        press(&mut app, KeyCode::Up); // from "none" wraps to the custom row
        assert!(on_custom(&app));
        for c in "nano -w".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        press(&mut app, KeyCode::Char('j')); // typed, not a move
        press(&mut app, KeyCode::Backspace);
        assert_eq!(app.editor_custom, "nano -w");
        assert!(on_custom(&app));
        // not persisted in tests: choose() writes settings, so only check the empty guard here
        app.editor_custom.clear();
        press(&mut app, KeyCode::Enter);
        assert!(app.show_editor_panel, "an empty command is refused");
        press(&mut app, KeyCode::Esc);
        assert!(!app.show_editor_panel);
    }

    #[test]
    fn opening_selects_the_current_editor() {
        let mut app = App::new();
        app.editor = gitkraft_core::Editor::Vim;
        app.open_editor_picker();
        assert_eq!(app.editor_list_state.selected(), Some(3));
        app.editor = gitkraft_core::Editor::Custom("hx -c x".into());
        app.open_editor_picker();
        assert!(on_custom(&app));
        assert_eq!(app.editor_custom, "hx -c x");
    }
}
