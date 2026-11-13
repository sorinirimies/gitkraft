//! Pure helpers for scrolling lists and finding what is under the mouse.

use ratatui::layout::Rect;

/// Keeps `selected` inside the window `[offset, offset + height)` with the least scrolling.
pub fn ensure_visible(selected: usize, offset: usize, height: usize) -> usize {
    if height == 0 {
        return 0;
    }
    if selected < offset {
        selected
    } else if selected >= offset + height {
        selected + 1 - height
    } else {
        offset
    }
}

/// `selected + delta`, clamped to `0..len`. An empty list gives 0.
pub fn step(selected: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }
    selected.saturating_add_signed(delta).min(len - 1)
}

/// Whether `(x, y)` is inside `rect`.
pub fn contains(rect: Rect, x: u16, y: u16) -> bool {
    x >= rect.x
        && x < rect.x.saturating_add(rect.width)
        && y >= rect.y
        && y < rect.y.saturating_add(rect.height)
}

/// Splits `width` columns into a list part of `percent` and the rest, leaving one column for the divider.
/// Returns `(list_width, detail_width)`.
pub fn split_widths(width: u16, percent: u16) -> (u16, u16) {
    if width < 3 {
        return (width, 0);
    }
    let usable = width - 1;
    let list = (u32::from(usable) * u32::from(percent.clamp(20, 80)) / 100) as u16;
    let list = list.clamp(1, usable - 1);
    (list, usable - list)
}

/// The percentage of `total` columns that column `x` (relative to the area's left edge) stands for.
pub fn percent_at(x: u16, left: u16, total: u16) -> u16 {
    if total == 0 {
        return 50;
    }
    let rel = x.saturating_sub(left);
    ((u32::from(rel) * 100) / u32::from(total)).clamp(20, 80) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrolls_just_enough() {
        assert_eq!(ensure_visible(0, 0, 5), 0);
        assert_eq!(ensure_visible(4, 0, 5), 0);
        assert_eq!(ensure_visible(5, 0, 5), 1);
        assert_eq!(ensure_visible(2, 10, 5), 2);
        assert_eq!(ensure_visible(12, 10, 5), 10);
        assert_eq!(ensure_visible(9, 3, 0), 0, "no room, no offset");
    }

    #[test]
    fn step_clamps_at_both_ends_and_survives_empty() {
        assert_eq!(step(0, 5, -1), 0);
        assert_eq!(step(4, 5, 1), 4);
        assert_eq!(step(2, 5, 2), 4);
        assert_eq!(step(2, 5, -10), 0);
        assert_eq!(step(0, 0, 3), 0);
        assert_eq!(step(usize::MAX, 3, 1), 2);
    }

    #[test]
    fn contains_is_half_open() {
        let r = Rect::new(2, 3, 4, 2);
        assert!(contains(r, 2, 3) && contains(r, 5, 4));
        assert!(!contains(r, 6, 3) && !contains(r, 2, 5) && !contains(r, 1, 3));
        assert!(!contains(Rect::new(0, 0, 0, 0), 0, 0));
    }

    #[test]
    fn widths_leave_a_divider_column() {
        for width in [3u16, 10, 80, 200] {
            for pct in [0u16, 20, 50, 80, 100] {
                let (l, d) = split_widths(width, pct);
                assert_eq!(l + d + 1, width, "{width} {pct}");
                assert!(l >= 1 && d >= 1);
            }
        }
        assert_eq!(split_widths(2, 50), (2, 0));
    }

    #[test]
    fn percent_is_clamped_to_the_allowed_range() {
        assert_eq!(percent_at(0, 0, 100), 20);
        assert_eq!(percent_at(100, 0, 100), 80);
        assert_eq!(percent_at(50, 0, 100), 50);
        assert_eq!(percent_at(60, 10, 100), 50);
        assert_eq!(percent_at(5, 0, 0), 50);
    }
}
