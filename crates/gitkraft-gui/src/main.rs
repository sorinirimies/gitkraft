use gitkraft_gui::GitKraft;
use iced::Settings;

fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    // Read saved window geometry before starting the app.
    let saved_layout = gitkraft_core::features::persistence::ops::get_saved_layout()
        .ok()
        .flatten()
        .unwrap_or_default();

    let window_size = iced::Size::new(
        saved_layout.window_width.unwrap_or(1400.0),
        saved_layout.window_height.unwrap_or(800.0),
    );

    let window_position = match (saved_layout.window_x, saved_layout.window_y) {
        // Only restore position when it looks like a valid on-screen location.
        (Some(x), Some(y)) if x > -200.0 && y > -200.0 && x < 8000.0 && y < 8000.0 => {
            iced::window::Position::Specific(iced::Point::new(x, y))
        }
        _ => iced::window::Position::Default,
    };

    iced::application(boot, GitKraft::update, GitKraft::view)
        .title("GitKraft — Git IDE")
        .theme(|state: &GitKraft| state.iced_theme())
        .subscription(|state: &GitKraft| {
            let keyboard = iced::event::listen_with(|event, status, _window| {
                // ModifiersChanged fires regardless of focus/status — always track it.
                if let iced::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(mods)) = event
                {
                    return Some(gitkraft_gui::Message::ModifiersChanged(mods));
                }
                // Track window geometry for persistence.
                if let iced::Event::Window(iced::window::Event::Resized(size)) = &event {
                    return Some(gitkraft_gui::Message::WindowResized(
                        size.width,
                        size.height,
                    ));
                }
                if let iced::Event::Window(iced::window::Event::Moved(point)) = &event {
                    return Some(gitkraft_gui::Message::WindowMoved(point.x, point.y));
                }
                // Ctrl/Cmd shortcuts fire regardless of widget focus so that
                // Ctrl+, (settings), Ctrl+F (search), Ctrl++/- (zoom) etc.
                // always work even when a text input has keyboard focus.
                if let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key,
                    modifiers,
                    ..
                }) = event
                {
                    // Always handle Ctrl/Cmd shortcuts regardless of widget focus.
                    if modifiers.control() || modifiers.command() {
                        return handle_key_press(key, modifiers);
                    }
                    // Always handle Shift+arrow shortcuts for range selection.
                    if modifiers.shift()
                        && matches!(
                            key,
                            iced::keyboard::Key::Named(
                                iced::keyboard::key::Named::ArrowDown
                                    | iced::keyboard::key::Named::ArrowUp
                            )
                        )
                    {
                        return handle_key_press(key, modifiers);
                    }
                    // Other shortcuts only when no widget has keyboard focus.
                    if let iced::event::Status::Ignored = status {
                        return handle_key_press(key, modifiers);
                    }
                }
                None
            });
            let repo_path = state.active_tab().repo_path.clone();
            let git_watcher = git_watch_subscription(repo_path);
            iced::Subscription::batch([keyboard, git_watcher])
        })
        .scale_factor(|state: &GitKraft| state.ui_scale)
        .settings(Settings {
            fonts: vec![iced_fonts::BOOTSTRAP_FONT_BYTES.into()],
            ..Default::default()
        })
        .window(iced::window::Settings {
            size: window_size,
            position: window_position,
            ..Default::default()
        })
        .run()
}

/// Boot function — initializes application state and returns startup tasks.
fn boot() -> (GitKraft, iced::Task<gitkraft_gui::Message>) {
    let (mut state, open_tabs) = GitKraft::new_with_session_paths();

    let maximize_task: iced::Task<gitkraft_gui::Message> =
        iced::window::oldest().and_then(|id| iced::window::maximize(id, true));

    let restore_task = if !open_tabs.is_empty() {
        // Restore every saved tab in parallel, each loading into its own index.
        let tasks: Vec<iced::Task<gitkraft_gui::Message>> = open_tabs
            .iter()
            .enumerate()
            .filter(|(_, p)| p.exists())
            .map(|(i, p)| gitkraft_gui::features::repo::commands::load_repo_at(i, p.clone()))
            .collect();
        iced::Task::batch(tasks)
    } else {
        // Legacy fallback: restore the single last-opened repo.
        match gitkraft_core::features::persistence::ops::get_last_repo() {
            Ok(Some(path)) if path.exists() => {
                let tab = state.active_tab_mut();
                tab.is_loading = true;
                tab.status_message = Some("Loading repository...".to_string());
                gitkraft_gui::features::repo::commands::load_repo(path)
            }
            _ => iced::Task::none(),
        }
    };

    (state, iced::Task::batch([maximize_task, restore_task]))
}

/// Reactive git-state watcher subscription.
///
/// Spawns a background OS thread that uses `notify` to watch the repo's `.git`
/// directory for any file change.  Changes are debounced (300 ms) so that a
/// single git operation (which writes several files) triggers only one refresh.
/// A 5-second fallback poll is included so the UI stays current on network
/// file systems or environments where inotify events are not delivered.
///
/// Returns `Subscription::none()` when no repository is open.
fn git_watch_subscription(
    repo_path: Option<std::path::PathBuf>,
) -> iced::Subscription<gitkraft_gui::Message> {
    let Some(path) = repo_path else {
        return iced::Subscription::none();
    };
    // `run_with` hashes both `path` and the function pointer as the subscription
    // ID, so switching repos automatically tears down the old watcher and starts
    // a fresh one.
    iced::Subscription::run_with(path, git_watch_builder)
}

/// Indirection required by `Subscription::run_with` (needs `fn` pointer, not closure).
fn git_watch_builder(
    path: &std::path::PathBuf,
) -> impl futures::Stream<Item = gitkraft_gui::Message> {
    git_watch_stream(path.clone())
}

/// Stream that emits `FileSystemChanged` whenever the `.git` directory changes.
///
/// Uses `spawn_git_watcher` from core (debounced, fallback poll) and a
/// blocking `mpsc::Receiver` — no spin-wait, zero CPU overhead when idle.
fn git_watch_stream(
    repo_path: std::path::PathBuf,
) -> impl futures::Stream<Item = gitkraft_gui::Message> {
    use std::sync::mpsc;

    let (tx, rx) = mpsc::channel::<()>();
    let git_dir = repo_path.join(".git");

    // Spawn the core watcher; the thread exits when tx.send() fails (rx dropped).
    gitkraft_core::spawn_git_watcher(git_dir, move || tx.send(()).is_ok());

    // The stream blocks on recv() — safe because iced runs each subscription
    // on its own worker thread (same reasoning as the existing blocking sleep).
    futures::stream::unfold(rx, |rx| async move {
        // Block until the watcher signals a change or the sender is dropped.
        match rx.recv() {
            Ok(()) => Some((gitkraft_gui::Message::FileSystemChanged, rx)),
            Err(_) => None, // watcher thread exited — end the stream
        }
    })
}

/// Map keyboard shortcuts to messages.
fn handle_key_press(
    key: iced::keyboard::Key,
    modifiers: iced::keyboard::Modifiers,
) -> Option<gitkraft_gui::Message> {
    use iced::keyboard::Key;

    use iced::keyboard::key::Named;

    if modifiers.control() || modifiers.command() {
        match key {
            // Ctrl/Cmd + Plus (or Ctrl/Cmd + =)
            Key::Character(ref c) if c.as_str() == "+" || c.as_str() == "=" => {
                Some(gitkraft_gui::Message::ZoomIn)
            }
            // Ctrl/Cmd + Minus
            Key::Character(ref c) if c.as_str() == "-" => Some(gitkraft_gui::Message::ZoomOut),
            // Ctrl/Cmd + 0 — reset zoom
            Key::Character(ref c) if c.as_str() == "0" => Some(gitkraft_gui::Message::ZoomReset),
            // Ctrl/Cmd + F — toggle search
            Key::Character(ref c) if c.as_str() == "f" => Some(gitkraft_gui::Message::ToggleSearch),
            // Ctrl/Cmd + , — open settings.json in the configured editor (like Zed)
            Key::Character(ref c) if c.as_str() == "," => {
                Some(gitkraft_gui::Message::OpenSettingsFile)
            }
            _ => None,
        }
    } else if modifiers.shift() {
        // Shift+arrow: extend range selection in the file list or commit log.
        match key {
            Key::Named(Named::ArrowDown) => Some(gitkraft_gui::Message::ShiftArrowDown),
            Key::Named(Named::ArrowUp) => Some(gitkraft_gui::Message::ShiftArrowUp),
            _ => None,
        }
    } else {
        // Bare-key shortcuts (only when no widget has focus / Status::Ignored)
        match key {
            Key::Named(iced::keyboard::key::Named::Escape) => {
                // Esc closes the blame overlay, context menu, or other overlays.
                Some(gitkraft_gui::Message::CloseFileBlame)
            }
            _ => None,
        }
    }
}
