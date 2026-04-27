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
}
