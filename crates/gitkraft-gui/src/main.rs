use gitkraft_gui::GitKraft;
use iced::Settings;

fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    iced::application("GitKraft — Git IDE", GitKraft::update, GitKraft::view)
        .theme(|state: &GitKraft| state.iced_theme())
        .subscription(|_state| iced::keyboard::on_key_press(handle_key_press))
        .scale_factor(|state: &GitKraft| state.ui_scale as f64)
        .settings(Settings {
            fonts: vec![iced_fonts::BOOTSTRAP_FONT_BYTES.into()],
            ..Default::default()
        })
        .window(iced::window::Settings {
            size: iced::Size::new(1400.0, 800.0),
            ..Default::default()
        })
        .run_with(|| {
            let (mut state, open_tabs) = GitKraft::new_with_session_paths();

            let maximize_task: iced::Task<gitkraft_gui::Message> =
                iced::window::get_oldest().and_then(|id| iced::window::maximize(id, true));

            let restore_task = if !open_tabs.is_empty() {
                // Restore every saved tab in parallel, each loading into its own index.
                let tasks: Vec<iced::Task<gitkraft_gui::Message>> = open_tabs
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.exists())
                    .map(|(i, p)| {
                        gitkraft_gui::features::repo::commands::load_repo_at(i, p.clone())
                    })
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
        })
}

/// Map keyboard shortcuts to messages.
fn handle_key_press(
    key: iced::keyboard::Key,
    modifiers: iced::keyboard::Modifiers,
) -> Option<gitkraft_gui::Message> {
    use iced::keyboard::Key;

    if modifiers.control() || modifiers.command() {
        match key {
            // Ctrl/Cmd + Plus (or Ctrl/Cmd + =)
            Key::Character(ref c) if c.as_str() == "+" || c.as_str() == "=" => {
                Some(gitkraft_gui::Message::ZoomIn)
            }
            // Ctrl/Cmd + Minus
            Key::Character(ref c) if c.as_str() == "-" => {
                Some(gitkraft_gui::Message::ZoomOut)
            }
            // Ctrl/Cmd + 0 — reset zoom
            Key::Character(ref c) if c.as_str() == "0" => {
                Some(gitkraft_gui::Message::ZoomReset)
            }
            _ => None,
        }
    } else {
        None
    }
}
