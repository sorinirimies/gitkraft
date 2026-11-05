//! Native window behaviour: the commands a user expects from any desktop window —
//! **resize, zoom, minimize, tile** — that the window library does not set up by itself.
//!
//! * **macOS**: the system builds *Zoom*, *Fill*, *Move & Resize* (the ⌃⌘← tiling
//!   shortcuts), *Minimize* and *Bring All to Front* from the app's **Window menu**, and
//!   winit's default menu bar has none. `macos` adds it.
//! * **Windows and Linux**: the OS or the compositor draws the frame and does all of this
//!   already (Win+Arrow snapping, X11 window managers, Wayland client-side decorations),
//!   so there is nothing to add.
//!
//! Call [`setup`] once the window exists and run the task it returns.

#[cfg(target_os = "macos")]
pub mod macos;

/// One-time window setup as an iced task (a no-op where the platform needs nothing).
///
/// Generic over the app's message type: the task produces no messages.
#[cfg(target_os = "macos")]
pub fn setup<M: Send + 'static>() -> iced::Task<M> {
    iced::window::oldest()
        .and_then(|id| {
            iced::window::run(id, |_window| {
                macos::install_window_menu();
            })
        })
        .discard()
}

#[cfg(not(target_os = "macos"))]
pub fn setup<M: Send + 'static>() -> iced::Task<M> {
    iced::Task::none()
}

/// Wayland `app_id` and X11 `WM_CLASS` of the window. Desktops match it against the
/// `.desktop` file name and its `StartupWMClass` for the icon and window grouping.
pub const APP_ID: &str = "io.github.sorinirimies.GitKraft";

/// The main window at `size` and `position`, named [`APP_ID`] on Linux.
#[must_use]
pub fn settings(size: iced::Size, position: iced::window::Position) -> iced::window::Settings {
    iced::window::Settings {
        size,
        position,
        #[cfg(target_os = "linux")]
        platform_specific: iced::window::settings::PlatformSpecific {
            application_id: APP_ID.to_string(),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn settings_keep_size_and_position() {
        let p = iced::window::Position::Specific(iced::Point::new(10.0, 20.0));
        let s = super::settings(iced::Size::new(1400.0, 800.0), p);
        assert_eq!(s.size, iced::Size::new(1400.0, 800.0));
        assert!(matches!(
            s.position,
            iced::window::Position::Specific(iced::Point { x: 10.0, y: 20.0 })
        ));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn settings_name_the_desktop_entry_on_linux() {
        let s = super::settings(iced::Size::new(1.0, 1.0), iced::window::Position::Default);
        assert_eq!(s.platform_specific.application_id, super::APP_ID);
    }

    #[test]
    fn app_id_matches_the_desktop_entry() {
        let desktop =
            include_str!("../../../../packaging/linux/io.github.sorinirimies.GitKraft.desktop");
        assert!(
            desktop
                .lines()
                .any(|l| l == format!("StartupWMClass={}", super::APP_ID))
        );
    }

    #[test]
    fn setup_builds_a_task_for_any_message_type() {
        let _: iced::Task<u8> = super::setup();
        let _: iced::Task<String> = super::setup();
    }
}
