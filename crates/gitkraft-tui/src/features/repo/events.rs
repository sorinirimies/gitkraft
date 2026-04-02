use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, InputMode, InputPurpose};

/// Handle keys on the Welcome screen.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('o') => {
            app.input_mode = InputMode::Input;
            app.input_purpose = InputPurpose::RepoPath;
            app.input_buffer.clear();
            app.status_message = Some("Enter repository path:".into());
        }
        KeyCode::Char('i') => {
            // Init a repo in the current working directory
            match std::env::current_dir() {
                Ok(cwd) => match gitkraft_core::features::repo::init_repo(&cwd) {
                    Ok(_repo) => {
                        app.status_message = Some(format!("Initialized repo at {}", cwd.display()));
                        app.open_repo(cwd);
                    }
                    Err(e) => {
                        app.error_message = Some(format!("Failed to init repo: {e}"));
                    }
                },
                Err(e) => {
                    app.error_message = Some(format!("Cannot determine current directory: {e}"));
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
        _ => {}
    }
}
