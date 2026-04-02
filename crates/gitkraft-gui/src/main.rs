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
            let state = GitKraft::new();

            // If there is a recently opened repo in persisted settings, auto-open it.
            let startup_task = match gitkraft_core::features::persistence::ops::get_last_repo() {
                Ok(Some(path)) if path.exists() => {
                    gitkraft_gui::features::repo::commands::load_repo(path)
                }
                _ => iced::Task::none(),
            };

            (state, startup_task)
        })
}
