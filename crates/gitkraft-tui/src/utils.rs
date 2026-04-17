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
