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
            let auto_refresh = auto_refresh_subscription(state.has_repo());
            iced::Subscription::batch([keyboard, auto_refresh])
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

/// Periodic auto-refresh subscription — emits `FileSystemChanged` every ~2 seconds.
///
/// Uses `Subscription::run` with `futures::stream::unfold` and a blocking
/// `std::thread::sleep` so it works with any async backend (thread-pool, tokio,
/// smol, etc.).  The handler in `update.rs` short-circuits when no repo is open
/// or when the tab is already loading, so a no-repo guard here just avoids
/// spawning the stream at all.
fn auto_refresh_subscription(has_repo: bool) -> iced::Subscription<gitkraft_gui::Message> {
    if !has_repo {
        return iced::Subscription::none();
    }

    iced::Subscription::run(auto_refresh_stream)
}

/// Build a never-ending stream that yields `FileSystemChanged` every 2 seconds.
fn auto_refresh_stream() -> impl futures::Stream<Item = gitkraft_gui::Message> {
    futures::stream::unfold((), |()| async {
        // Blocking sleep is acceptable here — iced's thread-pool backend
        // drives each subscription on its own worker thread.
        std::thread::sleep(std::time::Duration::from_secs(2));
        Some((gitkraft_gui::Message::FileSystemChanged, ()))
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
