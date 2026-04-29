use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

/// Handle keys when the Stash pane is active.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        // Navigate down
        KeyCode::Char('j') | KeyCode::Down => {
            if app.tab().stashes.is_empty() {
                return;
            }
            let len = app.tab().stashes.len();
            let i = match app.tab().stash_list_state.selected() {
                Some(i) => (i + 1).min(len - 1),
                None => 0,
            };
            app.tab_mut().stash_list_state.select(Some(i));
        }
        // Navigate up
        KeyCode::Char('k') | KeyCode::Up => {
            if app.tab().stashes.is_empty() {
                return;
            }
            let i = match app.tab().stash_list_state.selected() {
                Some(i) => i.saturating_sub(1),
                None => 0,
            };
            app.tab_mut().stash_list_state.select(Some(i));
        }
        // Pop selected stash (apply + drop)
        KeyCode::Enter | KeyCode::Char('p') => {
            app.stash_pop_selected();
        }
        // Drop selected stash (delete without applying)
        KeyCode::Char('d') => {
            app.stash_drop_selected();
        }
        // Deselect
        KeyCode::Esc => {
            app.tab_mut().stash_list_state.select(None);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn app_with_stashes(count: usize) -> App {
        let mut app = App::new();
        app.tab_mut().stashes = (0..count)
            .map(|i| gitkraft_core::StashEntry {
                index: i,
                message: format!("WIP stash {i}"),
                oid: format!("abc{i:04}deadbeef"),
            })
            .collect();
        app.tab_mut().stash_list_state.select(Some(0));
        app
    }

    #[test]
    fn j_navigates_down() {
        let mut app = app_with_stashes(3);
        handle_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.tab().stash_list_state.selected(), Some(1));
    }

    #[test]
    fn k_navigates_up() {
        let mut app = app_with_stashes(3);
        app.tab_mut().stash_list_state.select(Some(2));
        handle_key(&mut app, key(KeyCode::Char('k')));
        assert_eq!(app.tab().stash_list_state.selected(), Some(1));
    }

    #[test]
    fn j_clamps_at_last() {
        let mut app = app_with_stashes(2);
        app.tab_mut().stash_list_state.select(Some(1));
        handle_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.tab().stash_list_state.selected(), Some(1));
    }

    #[test]
    fn k_clamps_at_first() {
        let mut app = app_with_stashes(2);
        app.tab_mut().stash_list_state.select(Some(0));
        handle_key(&mut app, key(KeyCode::Char('k')));
        assert_eq!(app.tab().stash_list_state.selected(), Some(0));
    }

    #[test]
    fn j_noop_on_empty_stash_list() {
        let mut app = App::new();
        handle_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.tab().stash_list_state.selected(), None);
    }

    #[test]
    fn enter_queues_stash_pop() {
        let mut app = app_with_stashes(2);
        app.tab_mut().repo_path = Some(std::path::PathBuf::from("/tmp/fake-repo"));
        handle_key(&mut app, key(KeyCode::Enter));
        assert!(
            app.tab().is_loading,
            "Enter must trigger stash pop (is_loading=true)"
        );
    }

    #[test]
    fn p_queues_stash_pop() {
        let mut app = app_with_stashes(2);
        app.tab_mut().repo_path = Some(std::path::PathBuf::from("/tmp/fake-repo"));
        handle_key(&mut app, key(KeyCode::Char('p')));
        assert!(app.tab().is_loading);
    }

    #[test]
    fn d_queues_stash_drop() {
        let mut app = app_with_stashes(2);
        app.tab_mut().repo_path = Some(std::path::PathBuf::from("/tmp/fake-repo"));
        handle_key(&mut app, key(KeyCode::Char('d')));
        assert!(
            app.tab().is_loading,
            "d must trigger stash drop (is_loading=true)"
        );
    }

    #[test]
    fn esc_deselects() {
        let mut app = app_with_stashes(2);
        handle_key(&mut app, key(KeyCode::Esc));
        assert_eq!(app.tab().stash_list_state.selected(), None);
    }
}
