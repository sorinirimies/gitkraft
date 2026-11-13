//! Where each line of a unified diff lives in the new file, so a cursor on the text can
//! become a `(path, line)` for an inline review comment.

/// One displayed diff line.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiffPos {
    /// The file (new path) this line belongs to; `None` on file headers before the first `+++`.
    pub path: Option<String>,
    /// Line number in the new file; `None` for headers, hunk lines, removed lines.
    pub new_line: Option<u32>,
}

impl DiffPos {
    /// `(path, line)` when a comment can be anchored here.
    pub fn anchor(&self) -> Option<(&str, u32)> {
        Some((self.path.as_deref()?, self.new_line?))
    }
}

/// One entry per line of `diff` (same indexes as `diff.lines()`).
pub fn positions(diff: &str) -> Vec<DiffPos> {
    let mut out = Vec::new();
    let mut path: Option<String> = None;
    let mut next: Option<u32> = None;
    for l in diff.lines() {
        let mut pos = DiffPos {
            path: path.clone(),
            new_line: None,
        };
        if l.starts_with("diff ") {
            path = None;
            next = None;
            pos.path = None;
        } else if let Some(p) = l.strip_prefix("+++ ") {
            path = match p.strip_prefix("b/").unwrap_or(p) {
                "/dev/null" => None,
                p => Some(p.to_string()),
            };
            pos.path = path.clone();
        } else if l.starts_with("--- ") && next.is_none() {
            // old-file header
        } else if let Some(h) = l.strip_prefix("@@") {
            next = h
                .split_whitespace()
                .find_map(|w| w.strip_prefix('+'))
                .and_then(|w| w.split(',').next())
                .and_then(|n| n.parse().ok());
        } else if let Some(n) = next {
            match l.chars().next() {
                Some('+') | Some(' ') | None => {
                    // a blank line inside a hunk is a context line whose space was trimmed
                    pos.new_line = Some(n);
                    next = Some(n + 1);
                }
                _ => {}
            }
        }
        out.push(pos);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIFF: &str = "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -1,3 +10,4 @@ fn x\n ctx\n-old\n+new\n+more\n tail\n\\ No newline at end of file\ndiff --git a/gone b/gone\n--- a/gone\n+++ /dev/null\n@@ -1 +0,0 @@\n-x\n";

    #[test]
    fn lines_map_to_new_file_numbers() {
        let p = positions(DIFF);
        assert_eq!(p.len(), DIFF.lines().count());
        assert_eq!(p[0].anchor(), None, "file header");
        assert_eq!(p[3].anchor(), None, "hunk header");
        assert_eq!(p[4].anchor(), Some(("a.rs", 10)), "context");
        assert_eq!(p[5].anchor(), None, "removed lines have no new line");
        assert_eq!(p[6].anchor(), Some(("a.rs", 11)));
        assert_eq!(p[7].anchor(), Some(("a.rs", 12)));
        assert_eq!(p[8].anchor(), Some(("a.rs", 13)));
        assert_eq!(p[9].anchor(), None, "no-newline marker");
    }

    #[test]
    fn deleted_files_and_empty_input_have_no_anchors() {
        let p = positions(DIFF);
        assert!(p[10..].iter().all(|x| x.anchor().is_none()));
        assert!(positions("").is_empty());
        assert!(
            positions("just text\nno diff")
                .iter()
                .all(|x| x.anchor().is_none())
        );
    }

    #[test]
    fn a_blank_context_line_counts() {
        let p = positions("+++ b/f\n@@ -1,2 +1,2 @@\n\n+x\n");
        assert_eq!(p[2].anchor(), Some(("f", 1)));
        assert_eq!(p[3].anchor(), Some(("f", 2)));
    }
}
