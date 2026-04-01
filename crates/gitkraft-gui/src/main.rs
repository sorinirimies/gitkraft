use gitkraft_gui::GitKraft;
use iced::Settings;

fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    iced::application("GitKraft — Git IDE", GitKraft::update, GitKraft::view)
        .theme(|state: &GitKraft| state.theme.clone())
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
            (state, iced::Task::none())
        })
}
