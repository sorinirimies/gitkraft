//! Native window behaviour: the commands a user expects from any desktop window —
//! **resize, zoom, minimize, tile** — that the window library does not set up by itself,
//! and the app icon.
//!
//! * **macOS**: the system builds *Zoom*, *Fill*, *Move & Resize* (the ⌃⌘← tiling
//!   shortcuts), *Minimize* and *Bring All to Front* from the app's **Window menu**, and
//!   winit's default menu bar has none. `macos` adds it.
//! * **Windows and Linux**: the OS or the compositor draws the frame and does all of this
//!   already (Win+Arrow snapping, X11 window managers, Wayland client-side decorations),
//!   so there is nothing to add.
//!
//! * **Icon**: Windows and X11 take the window's icon (title bar, taskbar, Alt-Tab) from
//!   [`settings`]; macOS ignores a window icon, so `setup` sets the Dock icon, which a
//!   bare `gitkraft` binary outside a `.app` bundle would otherwise lack. Wayland takes it
//!   from the `.desktop` entry named by [`APP_ID`]. The pixels come from
//!   `assets/icons/gitkraft.svg` via `just icons`.
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
                macos::set_dock_icon(DOCK_ICON_PNG);
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

/// Edge length in pixels of [`WINDOW_ICON_RGBA`].
const WINDOW_ICON_SIZE: u32 = 128;

/// The window icon as raw 8-bit RGBA, [`WINDOW_ICON_SIZE`] square.
const WINDOW_ICON_RGBA: &[u8] = include_bytes!("../../assets/gitkraft-128.rgba");

/// The Dock icon (PNG) on Apple's icon grid, margin and shadow included.
#[cfg(target_os = "macos")]
const DOCK_ICON_PNG: &[u8] = include_bytes!("../../assets/gitkraft-macos-512.png");

/// The app icon for the window; `None` only if the embedded pixels were malformed.
#[must_use]
pub fn icon() -> Option<iced::window::Icon> {
    iced::window::icon::from_rgba(
        WINDOW_ICON_RGBA.to_vec(),
        WINDOW_ICON_SIZE,
        WINDOW_ICON_SIZE,
    )
    .ok()
}

/// The main window at `size` and `position`, with the app icon, named [`APP_ID`] on Linux.
#[must_use]
pub fn settings(size: iced::Size, position: iced::window::Position) -> iced::window::Settings {
    iced::window::Settings {
        size,
        position,
        icon: icon(),
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

    #[test]
    fn the_window_icon_is_embedded_at_its_size() {
        let (rgba, size) = super::icon().expect("valid icon").into_raw();
        assert_eq!(size, iced::Size::new(128, 128));
        assert_eq!(rgba.len(), 128 * 128 * 4);
    }

    #[test]
    fn the_window_icon_is_opaque_in_the_middle_and_clear_in_the_corner() {
        let (rgba, _) = super::icon().expect("valid icon").into_raw();
        let alpha = |x: usize, y: usize| rgba[(y * 128 + x) * 4 + 3];
        assert_eq!(alpha(64, 64), 255, "the tile is solid");
        assert_eq!(alpha(0, 0), 0, "the rounded corner is transparent");
    }

    #[test]
    fn settings_carry_the_icon() {
        let s = super::settings(iced::Size::new(1.0, 1.0), iced::window::Position::Default);
        assert!(s.icon.is_some());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_dock_icon_is_a_png() {
        assert!(super::DOCK_ICON_PNG.starts_with(b"\x89PNG\r\n\x1a\n"));
    }
}
