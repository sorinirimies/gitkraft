//! Text manipulation utilities shared across the crate.

/// Truncate `s` to at most `max_chars` Unicode characters, appending "…" if shortened.
///
/// This is a framework-agnostic utility shared by both the GUI and TUI.
pub fn truncate_str(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else if max_chars <= 1 {
        "…".to_string()
    } else {
        let mut out: String = s.chars().take(max_chars - 1).collect();
        out.push('…');
        out
    }
}

/// Extract the filename (last component) from a `/`-separated path string.
///
/// Returns the entire input when there is no `/` separator.
///
/// # Examples
/// ```
/// assert_eq!(gitkraft_core::path_basename("src/main.rs"), "main.rs");
/// assert_eq!(gitkraft_core::path_basename("hello.txt"), "hello.txt");
/// assert_eq!(gitkraft_core::path_basename(""), "");
/// ```
pub fn path_basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_truncation_when_short() {
        assert_eq!(truncate_str("hello", 10), "hello");
    }

    #[test]
    fn exact_length_no_truncation() {
        assert_eq!(truncate_str("hello", 5), "hello");
    }

    #[test]
    fn truncation_adds_ellipsis() {
        assert_eq!(truncate_str("hello world", 5), "hell…");
    }

    #[test]
    fn max_one_gives_ellipsis() {
        assert_eq!(truncate_str("hello", 1), "…");
    }

    #[test]
    fn max_zero_gives_ellipsis() {
        assert_eq!(truncate_str("hello", 0), "…");
    }

    #[test]
    fn unicode_chars_counted_correctly() {
        // 4 chars: each emoji is 1 char
        assert_eq!(truncate_str("😀😁😂😃", 3), "😀😁…");
    }

    #[test]
    fn empty_string() {
        assert_eq!(truncate_str("", 5), "");
    }

    #[test]
    fn path_basename_with_slashes() {
        assert_eq!(path_basename("src/main.rs"), "main.rs");
    }

    #[test]
    fn path_basename_no_slash() {
        assert_eq!(path_basename("hello.txt"), "hello.txt");
    }

    #[test]
    fn path_basename_trailing_slash() {
        assert_eq!(path_basename("dir/"), "");
    }

    #[test]
    fn path_basename_empty() {
        assert_eq!(path_basename(""), "");
    }

    #[test]
    fn path_basename_deep_path() {
        assert_eq!(path_basename("a/b/c/d/file.rs"), "file.rs");
    }
}
