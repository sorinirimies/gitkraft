//! Where each line of a unified diff lives in the new file, so a cursor on the text can
//! become a `(path, line)` for an inline review comment.

/// Which version of the file a comment is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Side {
    /// The new version (added and unchanged lines).
    #[default]
    New,
    /// The old version (removed lines).
    Old,
}

/// A line a review comment can be put on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchor {
    /// The file.
    pub path: String,
    /// Line number in the version `side` names.
    pub line: u32,
    /// New or old version.
    pub side: Side,
}

/// One displayed diff line.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiffPos {
    /// The file (new path, or the old one for a deleted file); `None` before the first file header.
    pub path: Option<String>,
    /// Line number in the new file; `None` for headers, hunk lines and removed lines.
    pub new_line: Option<u32>,
    /// Line number in the old file, for removed lines.
    pub old_line: Option<u32>,
}

impl DiffPos {
    /// Where a comment on this line goes, if it can be anchored here.
    pub fn anchor(&self) -> Option<Anchor> {
        let path = self.path.clone()?;
        match (self.new_line, self.old_line) {
            (Some(line), _) => Some(Anchor {
                path,
                line,
                side: Side::New,
            }),
            (None, Some(line)) => Some(Anchor {
                path,
                line,
                side: Side::Old,
            }),
            _ => None,
        }
    }
}

fn hunk_start(h: &str, sign: char) -> Option<u32> {
    h.split_whitespace()
        .find_map(|w| w.strip_prefix(sign))
        .and_then(|w| w.split(',').next())
        .and_then(|n| n.parse().ok())
}

/// One entry per line of `diff` (same indexes as `diff.lines()`).
pub fn positions(diff: &str) -> Vec<DiffPos> {
    let mut out = Vec::new();
    let mut path: Option<String> = None;
    let mut old_path: Option<String> = None;
    let mut next: Option<u32> = None;
    let mut old_next: u32 = 0;
    for l in diff.lines() {
        let mut pos = DiffPos {
            path: path.clone(),
            ..DiffPos::default()
        };
        if l.starts_with("diff ") {
            path = None;
            old_path = None;
            next = None;
            pos.path = None;
        } else if let Some(p) = l.strip_prefix("+++ ") {
            path = match p.strip_prefix("b/").unwrap_or(p) {
                "/dev/null" => old_path.clone(),
                p => Some(p.to_string()),
            };
            pos.path = path.clone();
        } else if let Some(p) = l.strip_prefix("--- ").filter(|_| next.is_none()) {
            old_path = match p.strip_prefix("a/").unwrap_or(p) {
                "/dev/null" => None,
                p => Some(p.to_string()),
            };
        } else if let Some(h) = l.strip_prefix("@@") {
            next = hunk_start(h, '+');
            old_next = hunk_start(h, '-').unwrap_or(0);
        } else if let Some(n) = next {
            match l.chars().next() {
                Some('+') => {
                    pos.new_line = Some(n);
                    next = Some(n + 1);
                }
                Some(' ') | None => {
                    // a blank line inside a hunk is a context line whose space was trimmed
                    pos.new_line = Some(n);
                    pos.old_line = Some(old_next);
                    next = Some(n + 1);
                    old_next += 1;
                }
                Some('-') => {
                    pos.old_line = Some(old_next);
                    old_next += 1;
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

    fn a(path: &str, line: u32, side: Side) -> Option<Anchor> {
        Some(Anchor {
            path: path.into(),
            line,
            side,
        })
    }

    const DIFF: &str = "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -1,3 +10,4 @@ fn x\n ctx\n-old\n+new\n+more\n tail\n\\ No newline at end of file\ndiff --git a/gone b/gone\n--- a/gone\n+++ /dev/null\n@@ -1 +0,0 @@\n-x\n";

    #[test]
    fn lines_map_to_new_file_numbers() {
        let p = positions(DIFF);
        assert_eq!(p.len(), DIFF.lines().count());
        assert_eq!(p[0].anchor(), None, "file header");
        assert_eq!(p[3].anchor(), None, "hunk header");
        assert_eq!(p[4].anchor(), a("a.rs", 10, Side::New), "context");
        assert_eq!(
            p[5].anchor(),
            a("a.rs", 2, Side::Old),
            "removed lines are on the old side"
        );
        assert_eq!(p[6].anchor(), a("a.rs", 11, Side::New));
        assert_eq!(p[7].anchor(), a("a.rs", 12, Side::New));
        assert_eq!(p[8].anchor(), a("a.rs", 13, Side::New));
        assert_eq!(p[9].anchor(), None, "no-newline marker");
    }

    #[test]
    fn deleted_files_comment_on_the_old_side_and_empty_input_has_no_anchors() {
        let p = positions(DIFF);
        assert!(p[10..14].iter().all(|x| x.anchor().is_none()));
        assert_eq!(p[14].anchor(), a("gone", 1, Side::Old), "{p:?}");
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
        assert_eq!(p[2].anchor(), a("f", 1, Side::New));
        assert_eq!(p[3].anchor(), a("f", 2, Side::New));
    }
}
