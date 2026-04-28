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
        repo_path = gitkraft_core::features::persistence::get_last_tui_repo()
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

    // Always attempt to enable enhanced keyboard input.
    //
    // `PushKeyboardEnhancementFlags` writes a short escape sequence
    // (`\x1b[>flags u`).  Terminals that understand the Kitty keyboard
    // protocol (Kitty, Alacritty, WezTerm, recent iTerm2, …) will honour it
    // and start sending Shift+arrow keys with the SHIFT modifier flag, making
    // Shift+↑/↓ range-selection work natively.  Terminals that don't
    // understand the sequence simply ignore it — no garbage is produced.
    //
    // We unconditionally push (ignoring the result) and unconditionally pop on
    // exit so the terminal is always left in a clean state.
    let _ = execute!(
        stdout,
        crossterm::event::PushKeyboardEnhancementFlags(
            crossterm::event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES,
        )
    );

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, repo_path);

    // Always pop the keyboard enhancement flags so the terminal is left clean.
    let _ = execute!(io::stdout(), crossterm::event::PopKeyboardEnhancementFlags);

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
        if let Ok(settings) = gitkraft_core::features::persistence::load_tui_settings() {
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

    // Watcher thread handle — restarted whenever it exits or the repo changes.
    let mut git_watcher_thread: Option<std::thread::JoinHandle<()>> = None;

    loop {
        app.tick_count = app.tick_count.wrapping_add(1);

        // Drain any results from background tasks (open_repo, refresh,
        // stage, commit, etc.) before drawing so the UI reflects the
        // latest state.
        app.poll_background();
        app.maybe_auto_refresh();

        // ── Reactive git watcher ──────────────────────────────────────────
        // Spawn a background thread to watch the .git directory whenever:
        //   • no watcher is running yet, or
        //   • the previous watcher thread exited (e.g., repo was closed).
        // The thread sends GitStateChanged via bg_tx on every file-system
        // event (debounced 300 ms) and on a 5-second fallback poll.
        // It exits automatically when bg_tx.send() fails (TUI exited).
        if git_watcher_thread
            .as_ref()
            .map(|t| t.is_finished())
            .unwrap_or(true)
        {
            if let Some(ref path) = app.tab().repo_path.clone() {
                let git_dir = path.join(".git");
                let tx = app.bg_tx.clone();
                git_watcher_thread = Some(gitkraft_core::spawn_git_watcher(git_dir, move || {
                    tx.send(crate::app::BackgroundResult::GitStateChanged)
                        .is_ok()
                }));
            }
        }
        // ─────────────────────────────────────────────────────────────────

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

        // ── Terminal-editor handoff ───────────────────────────────────────
        // If a key handler set `pending_editor_open`, suspend the TUI now,
        // run the terminal editor synchronously (it inherits the real TTY),
        // then restore the TUI.  This works for Helix, Neovim, Vim, etc.
        if let Some(paths) = app.pending_editor_open.take() {
            // 1. Suspend: leave the alternate screen and restore the terminal
            //    to its normal (cooked, echo) state so the editor can use it.
            disable_raw_mode()?;
            execute!(io::stdout(), LeaveAlternateScreen)?;
            terminal.show_cursor()?;

            // 2. Try each binary candidate in order.
            let candidates = app.editor.binary_candidates();
            tracing::debug!(
                "[gitkraft] opening {:?} with {} (candidates: {})",
                paths,
                app.editor,
                candidates.join(", ")
            );

            let mut opened = false;
            let mut error_msg: Option<String> = None;

            for bin in &candidates {
                let parts: Vec<&str> = bin.split_whitespace().collect();
                if let Some((cmd, args)) = parts.split_first() {
                    tracing::debug!("[gitkraft] trying binary: {cmd}");
                    match std::process::Command::new(cmd)
                        .args(args.iter())
                        .args(paths.iter()) // ← pass ALL paths
                        .stdin(std::process::Stdio::inherit())
                        .stdout(std::process::Stdio::inherit())
                        .stderr(std::process::Stdio::inherit())
                        .status()
                    {
                        Ok(status) => {
                            tracing::debug!("[gitkraft] editor exited with {status}");
                            opened = true;
                            break;
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            tracing::debug!("[gitkraft] binary '{cmd}' not found, trying next");
                            continue;
                        }
                        Err(e) => {
                            let msg = format!("Editor '{cmd}' failed to launch: {e}");
                            tracing::warn!("[gitkraft] {msg}");
                            error_msg = Some(msg);
                            break;
                        }
                    }
                }
            }

            if !opened {
                let msg = error_msg.unwrap_or_else(|| {
                    format!(
                        "Could not find {} in PATH — tried: {} \
                         (check that the binary is installed and in your $PATH)",
                        app.editor,
                        candidates.join(", ")
                    )
                });
                tracing::warn!("[gitkraft] {msg}");
                app.tab_mut().error_message = Some(msg);
            } else {
                let count = paths.len();
                app.tab_mut().status_message = Some(format!(
                    "Returned from {} — {} file(s) edited",
                    app.editor, count
                ));
            }

            // 3. Resume: re-enter the alternate screen and raw mode.
            enable_raw_mode()?;
            execute!(io::stdout(), EnterAlternateScreen)?;
            terminal.clear()?;
        }
        // ─────────────────────────────────────────────────────────────────

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
