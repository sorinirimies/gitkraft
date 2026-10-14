//! Responsive layout: pure functions mapping window size + stored pane sizes
//! to the sizes actually rendered. Stored sizes are never modified.

/// Below this window width the sidebar is hidden.
pub const NARROW_WIDTH: f32 = 720.0;
/// Max sidebar share of window width.
pub const SIDEBAR_MAX_FRAC: f32 = 0.35;
/// Max commit-log share of width remaining after the sidebar.
pub const COMMIT_MAX_FRAC: f32 = 0.55;
/// Max staging-area share of window height.
pub const STAGING_MAX_FRAC: f32 = 0.5;

/// Pane sizes to render.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutDims {
    /// Sidebar docked beside the panes.
    pub show_sidebar: bool,
    /// Sidebar expanded but window narrow: float it over the panes.
    pub overlay_sidebar: bool,
    pub sidebar_width: f32,
    pub commit_log_width: f32,
    pub staging_height: f32,
}

impl LayoutDims {
    pub fn compute(
        window_w: f32,
        window_h: f32,
        sidebar_expanded: bool,
        sidebar_w: f32,
        commit_w: f32,
        staging_h: f32,
    ) -> Self {
        let show_sidebar = sidebar_expanded && window_w >= NARROW_WIDTH;
        let overlay_sidebar = sidebar_expanded && !show_sidebar;
        let sidebar_width = sidebar_w.min(window_w * SIDEBAR_MAX_FRAC).max(0.0);
        let used = if show_sidebar { sidebar_width } else { 0.0 };
        Self {
            show_sidebar,
            overlay_sidebar,
            sidebar_width,
            commit_log_width: commit_w.min((window_w - used) * COMMIT_MAX_FRAC).max(0.0),
            staging_height: staging_h.min(window_h * STAGING_MAX_FRAC).max(0.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dims(w: f32, h: f32, exp: bool) -> LayoutDims {
        LayoutDims::compute(w, h, exp, 250.0, 600.0, 300.0)
    }

    #[test]
    fn wide_window_keeps_stored_sizes() {
        let d = dims(1920.0, 1080.0, true);
        assert!(d.show_sidebar);
        assert_eq!(d.sidebar_width, 250.0);
        assert_eq!(d.commit_log_width, 600.0);
        assert_eq!(d.staging_height, 300.0);
    }

    #[test]
    fn narrow_window_hides_sidebar() {
        assert!(!dims(NARROW_WIDTH - 1.0, 800.0, true).show_sidebar);
        assert!(dims(NARROW_WIDTH, 800.0, true).show_sidebar);
    }

    #[test]
    fn narrow_expanded_sidebar_overlays() {
        let d = dims(600.0, 800.0, true);
        assert!(d.overlay_sidebar && !d.show_sidebar);
        assert!(!dims(1200.0, 800.0, true).overlay_sidebar);
        assert!(!dims(600.0, 800.0, false).overlay_sidebar);
    }

    #[test]
    fn collapsed_sidebar_stays_hidden() {
        assert!(!dims(1920.0, 1080.0, false).show_sidebar);
    }

    #[test]
    fn sidebar_and_commit_clamped() {
        let d = LayoutDims::compute(800.0, 800.0, true, 400.0, 900.0, 300.0);
        assert_eq!(d.sidebar_width, 280.0); // capped: 35% of 800
        assert!(d.commit_log_width <= (800.0 - d.sidebar_width) * COMMIT_MAX_FRAC);
    }

    #[test]
    fn commit_uses_full_width_when_sidebar_hidden() {
        let d = dims(600.0, 800.0, true);
        assert_eq!(d.commit_log_width, 330.0);
    }

    #[test]
    fn staging_clamped_to_half_height() {
        assert_eq!(dims(1200.0, 400.0, true).staging_height, 200.0);
    }

    #[test]
    fn tiny_window_never_negative() {
        let d = dims(0.0, 0.0, true);
        assert!(d.sidebar_width >= 0.0 && d.commit_log_width >= 0.0 && d.staging_height >= 0.0);
    }
}
