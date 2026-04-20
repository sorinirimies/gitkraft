//! GitKraft TUI — terminal user interface built with Ratatui.
//!
//! This crate provides a full-featured Git IDE experience in the terminal,
//! powered by [`gitkraft_core`] for all Git operations and [`ratatui`] for
//! rendering.
//!
//! # Entry point
//!
//! Call [`run`] with an optional repository path to start the TUI application.

pub mod app;
pub mod events;
pub mod features;
pub mod layout;
pub mod utils;
pub mod widgets;

use std::io;
use std::panic;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::app::App;

/// Run the TUI application.
///
/// If `repo_path` is `Some`, the repository at that path is opened immediately.
/// Otherwise the Welcome screen is shown, letting the user choose a repository.
pub fn run(mut repo_path: Option<PathBuf>) -> Result<()> {
    // If no repo path was given, try loading the last-opened repo from settings.
    if repo_path.is_none() {
        repo_path = gitkraft_core::features::persistence::get_last_repo()
            .ok()
            .flatten();
    }

    // Install a panic hook that restores the terminal before printing the
    // panic message.  Without this the user is left with a broken terminal
    // after an unexpected panic.
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        default_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, repo_path);

    restore_terminal()?;
    terminal.show_cursor()?;
    result
}

/// The inner event loop — draw, poll for input, dispatch, repeat.
pub fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    repo_path: Option<PathBuf>,
) -> Result<()>
where
    B::Error: Send + Sync + 'static,
{
    let mut app = App::new();

    if let Some(path) = repo_path {
        // CLI argument takes priority — open in the first tab
        app.open_repo(path);
    } else {
        // Try to restore the saved session (multiple tabs)
        if let Ok(settings) = gitkraft_core::features::persistence::load_settings() {
            let paths: Vec<PathBuf> = settings
                .open_tabs
                .into_iter()
                .filter(|p| p.exists())
                .collect();
            let active = settings.active_tab_index;
            if !paths.is_empty() {
                app.tabs.clear();
                for _ in &paths {
                    app.tabs.push(crate::app::RepoTab::new());
                }
                // Open each repo in its corresponding tab
                for (i, path) in paths.into_iter().enumerate() {
                    app.active_tab_index = i;
                    app.open_repo(path);
                }
                // Restore the originally active tab
                app.active_tab_index = active.min(app.tabs.len().saturating_sub(1));
                app.screen = crate::app::AppScreen::Main;
            }
        }
    }

    loop {
        app.tick_count = app.tick_count.wrapping_add(1);

        // Drain any results from background tasks (open_repo, refresh,
        // stage, commit, etc.) before drawing so the UI reflects the
        // latest state.
        app.poll_background();

        terminal.draw(|frame| layout::render(&mut app, frame))?;

        // Use a shorter poll interval (33 ms ≈ 30 fps) so background-task
        // results are picked up promptly and the loading indicator updates
        // without a noticeable lag.
        if event::poll(Duration::from_millis(33))? {
            if let Event::Key(key) = event::read()? {
                // Ignore key-release events on platforms that send them
                if key.kind == crossterm::event::KeyEventKind::Press {
                    events::handle_key(&mut app, key);
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

/// Restore the terminal to its original state (disable raw mode, leave the
/// alternate screen).
fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}
