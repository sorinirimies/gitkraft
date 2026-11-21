//! Shared TUI utility functions.

/// Pad a string to `width` characters with trailing spaces.
pub fn pad_right(s: &str, width: usize) -> String {
    let char_count = s.chars().count();
    if char_count >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - char_count))
    }
}

/// Hands `lines` scrolled down by `scroll` lines to a ratatui `Paragraph`, whose own scroll is
/// a `u16`: lines beyond what that can reach are dropped from the top first, so scrolling works
/// to the end of any length (big diffs pass 65 535 lines). Returns the lines and the remaining
/// offset for `Paragraph::scroll`.
pub fn scroll_lines<'a>(
    lines: Vec<ratatui::text::Line<'a>>,
    scroll: usize,
) -> (Vec<ratatui::text::Line<'a>>, u16) {
    let skip = scroll.saturating_sub(usize::from(u16::MAX));
    let rest = u16::try_from(scroll - skip).unwrap_or(u16::MAX);
    if skip == 0 {
        (lines, rest)
    } else {
        (lines.into_iter().skip(skip).collect(), rest)
    }
}

/// Rows `lines` take when wrapped to `width` columns (each line at least one).
pub fn wrapped_rows(lines: &[ratatui::text::Line<'_>], width: u16) -> usize {
    let w = usize::from(width.max(1));
    lines.iter().map(|l| l.width().max(1).div_ceil(w)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::text::Line;

    #[test]
    fn scrolling_reaches_lines_past_what_a_u16_holds() {
        let lines: Vec<Line> = (0..100_000).map(|i| Line::from(i.to_string())).collect();
        let (shown, rest) = scroll_lines(lines.clone(), 10);
        assert_eq!(
            (shown.len(), rest),
            (100_000, 10),
            "small offsets stay with ratatui"
        );
        let (shown, rest) = scroll_lines(lines, 99_990);
        assert_eq!(rest, u16::MAX);
        assert_eq!(
            shown[usize::from(rest)].to_string(),
            "99990",
            "the line at the offset is on top"
        );
    }

    #[test]
    fn wrapped_rows_count_long_lines_several_times() {
        let lines = [Line::from("x".repeat(25)), Line::from(""), Line::from("ab")];
        assert_eq!(wrapped_rows(&lines, 10), 3 + 1 + 1);
    }
}
