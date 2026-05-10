//! Utility helpers shared across the crate — OID formatting, relative time, text, etc.

pub mod text;
pub mod time;

pub use text::truncate_str;
pub use time::{fmt_oid, relative_time, short_oid, short_oid_str};

/// Build an ascending `Vec<usize>` spanning from `anchor` to `target`
/// (inclusive), regardless of which is larger.
///
/// Used for range-selection in commit and file lists in both GUI and TUI.
///
/// # Examples
/// ```
/// assert_eq!(gitkraft_core::ascending_range(3, 7), vec![3, 4, 5, 6, 7]);
/// assert_eq!(gitkraft_core::ascending_range(7, 3), vec![3, 4, 5, 6, 7]);
/// assert_eq!(gitkraft_core::ascending_range(5, 5), vec![5]);
/// ```
pub fn ascending_range(anchor: usize, target: usize) -> Vec<usize> {
    let (start, end) = if anchor <= target {
        (anchor, target)
    } else {
        (target, anchor)
    };
    (start..=end).collect()
}

/// Next index in a list, wrapping around to 0 after the last element.
/// Returns 0 if `len` is 0.
pub fn wrap_next(current: usize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (current + 1) % len
}

/// Previous index in a list, wrapping around to `len - 1` from index 0.
/// Returns 0 if `len` is 0.
pub fn wrap_prev(current: usize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    if current == 0 {
        len - 1
    } else {
        current - 1
    }
}

/// Next index in a list, clamped at `len - 1` (no wrapping).
/// Returns 0 if `len` is 0.
pub fn clamp_next(current: usize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (current + 1).min(len - 1)
}

/// Human-readable label for a repository path — the last path component,
/// or `"New Tab"` when `path` is `None` or has no filename.
pub fn repo_display_name(path: Option<&std::path::Path>) -> String {
    path.and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "New Tab".into())
}

/// Clamp a selection index after a list has been resized.
///
/// - If the list is empty → `None` (nothing to select).
/// - If no index was selected → `Some(0)` (auto-select first item).
/// - Otherwise → the existing index clamped to `0..len`.
pub fn clamp_selection(current: Option<usize>, len: usize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    Some(current.unwrap_or(0).min(len - 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascending_range_low_to_high() {
        assert_eq!(ascending_range(2, 5), vec![2, 3, 4, 5]);
    }

    #[test]
    fn ascending_range_high_to_low_is_still_ascending() {
        assert_eq!(ascending_range(5, 2), vec![2, 3, 4, 5]);
    }

    #[test]
    fn ascending_range_same_returns_single() {
        assert_eq!(ascending_range(4, 4), vec![4]);
    }

    // ── wrap_next ─────────────────────────────────────────────────────────

    #[test]
    fn wrap_next_normal() {
        assert_eq!(wrap_next(0, 3), 1);
    }

    #[test]
    fn wrap_next_wraps_around() {
        assert_eq!(wrap_next(2, 3), 0);
    }

    #[test]
    fn wrap_next_empty_list() {
        assert_eq!(wrap_next(0, 0), 0);
    }

    #[test]
    fn wrap_next_single_element() {
        assert_eq!(wrap_next(0, 1), 0);
    }

    // ── wrap_prev ─────────────────────────────────────────────────────────

    #[test]
    fn wrap_prev_normal() {
        assert_eq!(wrap_prev(2, 3), 1);
    }

    #[test]
    fn wrap_prev_wraps_around() {
        assert_eq!(wrap_prev(0, 3), 2);
    }

    #[test]
    fn wrap_prev_empty_list() {
        assert_eq!(wrap_prev(0, 0), 0);
    }

    #[test]
    fn wrap_prev_single_element() {
        assert_eq!(wrap_prev(0, 1), 0);
    }

    // ── clamp_next ────────────────────────────────────────────────────────

    #[test]
    fn clamp_next_normal() {
        assert_eq!(clamp_next(0, 3), 1);
    }

    #[test]
    fn clamp_next_at_end() {
        assert_eq!(clamp_next(2, 3), 2);
    }

    #[test]
    fn clamp_next_empty_list() {
        assert_eq!(clamp_next(0, 0), 0);
    }

    // ── clamp_selection ───────────────────────────────────────────────────

    #[test]
    fn clamp_selection_empty_list() {
        assert_eq!(clamp_selection(Some(5), 0), None);
    }

    #[test]
    fn clamp_selection_none_becomes_first() {
        assert_eq!(clamp_selection(None, 3), Some(0));
    }

    #[test]
    fn clamp_selection_within_range() {
        assert_eq!(clamp_selection(Some(1), 3), Some(1));
    }

    #[test]
    fn clamp_selection_overflow_clamped() {
        assert_eq!(clamp_selection(Some(10), 3), Some(2));
    }

    // ── repo_display_name ─────────────────────────────────────────────────

    #[test]
    fn repo_display_name_with_path() {
        let p = std::path::Path::new("/home/user/my-project");
        assert_eq!(repo_display_name(Some(p)), "my-project");
    }

    #[test]
    fn repo_display_name_none() {
        assert_eq!(repo_display_name(None), "New Tab");
    }

    #[test]
    fn repo_display_name_root_path() {
        let p = std::path::Path::new("/");
        assert_eq!(repo_display_name(Some(p)), "New Tab");
    }
}
