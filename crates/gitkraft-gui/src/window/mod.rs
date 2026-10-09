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

#[cfg(test)]
mod tests {
    #[test]
    fn setup_builds_a_task_for_any_message_type() {
        let _: iced::Task<u8> = super::setup();
        let _: iced::Task<String> = super::setup();
    }
}
