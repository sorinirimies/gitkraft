use gitkraft_gui::GitKraft;
use iced::Settings;

fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    iced::application("GitKraft — Git IDE", GitKraft::update, GitKraft::view)
        .theme(|state: &GitKraft| state.iced_theme())
        .settings(Settings {
            fonts: vec![iced_fonts::BOOTSTRAP_FONT_BYTES.into()],
            ..Default::default()
        })
        .window(iced::window::Settings {
            size: iced::Size::new(1400.0, 800.0),
            ..Default::default()
        })
        .run_with(|| {
            let mut state = GitKraft::new();

            // Maximize the window on startup by finding the main window ID
            // and then requesting maximization.
            let maximize_task: iced::Task<gitkraft_gui::Message> =
                iced::window::get_oldest().and_then(|id| iced::window::maximize(id, true));

            // If there is a recently opened repo in persisted settings, auto-open it.
            let repo_task = match gitkraft_core::features::persistence::ops::get_last_repo() {
                Ok(Some(path)) if path.exists() => {
                    let tab = state.active_tab_mut();
                    tab.is_loading = true;
                    tab.status_message = Some("Loading repository...".to_string());
                    gitkraft_gui::features::repo::commands::load_repo(path)
                }
                _ => iced::Task::none(),
            };

            let startup_task = iced::Task::batch([maximize_task, repo_task]);

            (state, startup_task)
        })
}
