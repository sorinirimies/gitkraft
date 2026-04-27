use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

/// Handle keys on the Welcome screen.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('o') => {
            let start = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/"));
            app.open_browser(start);
        }
        KeyCode::Char('i') => {
            // Init a repo in the current working directory
            match std::env::current_dir() {
                Ok(cwd) => match gitkraft_core::features::repo::init_repo(&cwd) {
                    Ok(_repo) => {
                        app.tab_mut().status_message =
                            Some(format!("Initialized repo at {}", cwd.display()));
                        app.open_repo(cwd);
                    }
                    Err(e) => {
                        app.tab_mut().error_message = Some(format!("Failed to init repo: {e}"));
                    }
                },
                Err(e) => {
                    app.tab_mut().error_message =
                        Some(format!("Cannot determine current directory: {e}"));
                }
            }
        }
        KeyCode::Char(c @ '1'..='9') => {
            let idx = (c as usize) - ('1' as usize);
            if idx < app.recent_repos.len() {
                let path = app.recent_repos[idx].path.clone();
                app.open_repo(path);
            }
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Esc => {
            app.should_quit = true;
        }
        // Tab management
        // Open settings file in editor (works even before a repo is loaded)
        KeyCode::Char(',') => {
            app.open_settings_in_editor();
        }
        KeyCode::Char('N') => {
            app.new_tab();
        }
        _ => {}
    }
}
