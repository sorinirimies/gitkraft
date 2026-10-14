//! Responsive layout: pure functions mapping window size + stored pane sizes
//! to the sizes actually rendered. Stored sizes are never modified.

/// Below this window width the sidebar is hidden (or floated as an overlay).
pub const NARROW_WIDTH: f32 = 720.0;
/// Below this width toolbars switch to icon-only buttons.
pub const COMPACT_WIDTH: f32 = 1000.0;
/// Below this width secondary toolbar widgets (editor picker, version) go away.
pub const MINIMAL_WIDTH: f32 = 800.0;
/// Below this width even the theme picker is dropped.
pub const TINY_WIDTH: f32 = 520.0;

/// Max sidebar share of window width.
pub const SIDEBAR_MAX_FRAC: f32 = 0.35;
/// Max commit-log share of width remaining after the sidebar.
pub const COMMIT_MAX_FRAC: f32 = 0.55;
/// Max staging-area share of window height.
pub const STAGING_MAX_FRAC: f32 = 0.5;
/// Max staging share on short windows, so the commit log keeps room.
pub const STAGING_MAX_FRAC_SHORT: f32 = 0.4;
/// Windows shorter than this use [`STAGING_MAX_FRAC_SHORT`].
pub const SHORT_HEIGHT: f32 = 640.0;
/// Smallest useful staging height.
pub const STAGING_MIN: f32 = 110.0;

/// Width reserved for the diff viewer, however small the window.
pub const DIFF_MIN_WIDTH: f32 = 280.0;
/// Narrowest commit log we ever render.
pub const COMMIT_MIN_WIDTH: f32 = 160.0;
/// Width of a vertical divider.
pub const DIVIDER_W: f32 = 6.0;
/// Max share of the diff pane taken by its file list.
pub const DIFF_LIST_MAX_FRAC: f32 = 0.4;

/// Fixed width of the author sub-column in the commit log.
pub const AUTHOR_COLUMN_PX: f32 = 110.0;
/// Fixed width of the date sub-column in the commit log.
pub const TIME_COLUMN_PX: f32 = 72.0;
/// Commit-log width from which the author column is shown.
pub const SHOW_AUTHOR_FROM: f32 = 380.0;
/// Commit-log width from which the date column is shown.
pub const SHOW_TIME_FROM: f32 = 480.0;

/// Coarse window-width class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Breakpoint {
    /// < [`NARROW_WIDTH`]
    Narrow,
    /// < [`COMPACT_WIDTH`]
    Medium,
    /// everything wider
    Wide,
}

impl Breakpoint {
    pub fn of(window_w: f32) -> Self {
        if window_w < NARROW_WIDTH {
            Self::Narrow
        } else if window_w < COMPACT_WIDTH {
            Self::Medium
        } else {
            Self::Wide
        }
    }
}

/// Pane sizes to render.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutDims {
    pub breakpoint: Breakpoint,
    /// Sidebar docked beside the panes.
    pub show_sidebar: bool,
    /// Sidebar expanded but window narrow: float it over the panes.
    pub overlay_sidebar: bool,
    pub sidebar_width: f32,
    pub commit_log_width: f32,
    /// Width left for the diff viewer.
    pub diff_width: f32,
    /// Width of the file list inside the diff viewer.
    pub diff_file_list_width: f32,
    pub staging_height: f32,
    /// Stack the commit box under the staged list instead of a third column.
    pub staging_stacked: bool,
    /// Toolbars show icons only.
    pub compact_toolbar: bool,
    /// Drop secondary toolbar widgets (editor picker, version label).
    pub minimal_toolbar: bool,
    /// Drop even the theme picker.
    pub tiny_toolbar: bool,
}

impl LayoutDims {
    pub fn compute(
        window_w: f32,
        window_h: f32,
        sidebar_expanded: bool,
        sidebar_w: f32,
        commit_w: f32,
        staging_h: f32,
        diff_list_w: f32,
    ) -> Self {
        let window_w = window_w.max(0.0);
        let window_h = window_h.max(0.0);
        let show_sidebar = sidebar_expanded && window_w >= NARROW_WIDTH;
        let overlay_sidebar = sidebar_expanded && !show_sidebar;
        let sidebar_width = sidebar_w.min(window_w * SIDEBAR_MAX_FRAC).max(0.0);
        let used = if show_sidebar {
            sidebar_width + DIVIDER_W
        } else {
            0.0
        };
        let avail = (window_w - used).max(0.0);

        // Commit log: at most 55% of what is left, and always leave the diff
        // viewer its minimum; never collapse below a readable minimum.
        let cap = (avail * COMMIT_MAX_FRAC).min(avail - DIVIDER_W - DIFF_MIN_WIDTH);
        let floor = COMMIT_MIN_WIDTH.min(avail * 0.6);
        let commit_log_width = commit_w.min(cap).max(floor).min(avail);
        let diff_width = (avail - commit_log_width - DIVIDER_W).max(0.0);

        let staging_frac = if window_h < SHORT_HEIGHT {
            STAGING_MAX_FRAC_SHORT
        } else {
            STAGING_MAX_FRAC
        };
        let staging_cap = window_h * staging_frac;
        let staging_height = staging_h
            .min(staging_cap)
            .max(STAGING_MIN.min(staging_cap))
            .max(0.0);

        Self {
            breakpoint: Breakpoint::of(window_w),
            show_sidebar,
            overlay_sidebar,
            sidebar_width,
            commit_log_width,
            diff_width,
            diff_file_list_width: diff_list_w.min(diff_width * DIFF_LIST_MAX_FRAC).max(0.0),
            staging_height,
            staging_stacked: window_w < NARROW_WIDTH,
            compact_toolbar: window_w < COMPACT_WIDTH,
            minimal_toolbar: window_w < MINIMAL_WIDTH,
            tiny_toolbar: window_w < TINY_WIDTH,
        }
    }
}

/// Commit-log column sizes for a given pane width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommitColumns {
    pub ref_w: f32,
    pub graph_w: f32,
    pub show_author: bool,
    pub show_time: bool,
    /// Pixels left for the commit summary text.
    pub msg_px: f32,
}

impl CommitColumns {
    pub fn compute(commit_w: f32, ref_stored: f32, graph_stored: f32) -> Self {
        let commit_w = commit_w.max(0.0);
        let show_author = commit_w >= SHOW_AUTHOR_FROM;
        let show_time = commit_w >= SHOW_TIME_FROM;
        let ref_w = ref_stored.min(commit_w * 0.28).max(0.0);
        let graph_w = graph_stored.min(commit_w * 0.22).max(0.0);
        let mut fixed = ref_w + DIVIDER_W + graph_w + DIVIDER_W + 30.0;
        if show_author {
            fixed += AUTHOR_COLUMN_PX + 6.0;
        }
        if show_time {
            fixed += TIME_COLUMN_PX + 6.0;
        }
        Self {
            ref_w,
            graph_w,
            show_author,
            show_time,
            msg_px: (commit_w - fixed).max(60.0),
        }
    }
}

impl crate::state::GitKraft {
    /// Pane sizes to render for the current window and stored sizes.
    pub fn layout(&self) -> LayoutDims {
        LayoutDims::compute(
            self.window_width,
            self.window_height,
            self.sidebar_expanded,
            self.sidebar_width,
            self.commit_log_width,
            self.staging_height,
            self.diff_file_list_width,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dims(w: f32, h: f32, exp: bool) -> LayoutDims {
        LayoutDims::compute(w, h, exp, 250.0, 600.0, 300.0, 180.0)
    }

    #[test]
    fn wide_window_keeps_stored_sizes() {
        let d = dims(1920.0, 1080.0, true);
        assert!(d.show_sidebar);
        assert_eq!(d.sidebar_width, 250.0);
        assert_eq!(d.commit_log_width, 600.0);
        assert_eq!(d.staging_height, 300.0);
        assert_eq!(d.diff_file_list_width, 180.0);
        assert!(!d.compact_toolbar && !d.staging_stacked);
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
        let d = LayoutDims::compute(800.0, 800.0, true, 400.0, 900.0, 300.0, 180.0);
        assert_eq!(d.sidebar_width, 280.0); // capped: 35% of 800
        assert!(d.commit_log_width <= (800.0 - d.sidebar_width) * COMMIT_MAX_FRAC);
    }

    #[test]
    fn diff_always_keeps_minimum_width() {
        for w in [721.0, 800.0, 1000.0, 1280.0, 1600.0] {
            let d = dims(w, 900.0, true);
            assert!(
                d.diff_width >= DIFF_MIN_WIDTH - 0.01,
                "w={w} diff={}",
                d.diff_width
            );
        }
        let d = dims(600.0, 800.0, true); // sidebar floats; full width shared
        assert!(d.diff_width >= DIFF_MIN_WIDTH - 0.01);
    }

    #[test]
    fn panes_add_up_to_window() {
        for w in [400.0, 600.0, 900.0, 1400.0] {
            let d = dims(w, 800.0, true);
            let sb = if d.show_sidebar {
                d.sidebar_width + DIVIDER_W
            } else {
                0.0
            };
            let total = sb + d.commit_log_width + DIVIDER_W + d.diff_width;
            assert!(total <= w + 0.01, "w={w} total={total}");
        }
    }

    #[test]
    fn diff_file_list_limited_to_share_of_diff() {
        let d = LayoutDims::compute(800.0, 800.0, false, 250.0, 300.0, 300.0, 400.0);
        assert!(d.diff_file_list_width <= d.diff_width * DIFF_LIST_MAX_FRAC + 0.01);
    }

    #[test]
    fn staging_clamped_with_floor() {
        assert_eq!(dims(1200.0, 400.0, true).staging_height, 160.0); // short: 40%
        assert_eq!(dims(1200.0, 800.0, true).staging_height, 300.0);
        let d = LayoutDims::compute(1200.0, 1000.0, true, 250.0, 600.0, 20.0, 180.0);
        assert_eq!(d.staging_height, STAGING_MIN);
    }

    #[test]
    fn toolbar_modes_by_width() {
        assert!(!dims(1200.0, 800.0, true).compact_toolbar);
        let d = dims(900.0, 800.0, true);
        assert!(d.compact_toolbar && !d.minimal_toolbar);
        let d = dims(700.0, 800.0, true);
        assert!(d.minimal_toolbar && d.staging_stacked && !d.tiny_toolbar);
        assert!(dims(500.0, 800.0, true).tiny_toolbar);
    }

    #[test]
    fn breakpoints() {
        assert_eq!(Breakpoint::of(500.0), Breakpoint::Narrow);
        assert_eq!(Breakpoint::of(900.0), Breakpoint::Medium);
        assert_eq!(Breakpoint::of(1400.0), Breakpoint::Wide);
    }

    #[test]
    fn tiny_window_never_negative() {
        let d = dims(0.0, 0.0, true);
        assert!(d.sidebar_width >= 0.0 && d.commit_log_width >= 0.0);
        assert!(d.staging_height >= 0.0 && d.diff_width >= 0.0 && d.diff_file_list_width >= 0.0);
    }

    #[test]
    fn commit_columns_drop_as_pane_shrinks() {
        let wide = CommitColumns::compute(700.0, 120.0, 100.0);
        assert!(wide.show_author && wide.show_time);
        let mid = CommitColumns::compute(420.0, 120.0, 100.0);
        assert!(mid.show_author && !mid.show_time);
        let small = CommitColumns::compute(300.0, 120.0, 100.0);
        assert!(!small.show_author && !small.show_time);
        assert!(small.ref_w <= 300.0 * 0.28 + 0.01 && small.graph_w <= 300.0 * 0.22 + 0.01);
        assert!(small.msg_px >= 60.0);
    }

    #[test]
    fn commit_message_gets_room_when_wide() {
        let c = CommitColumns::compute(900.0, 120.0, 100.0);
        assert_eq!(c.ref_w, 120.0);
        assert!(c.msg_px > 300.0);
    }
}
