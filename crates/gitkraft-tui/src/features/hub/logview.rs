//! State of the job log viewer: scrolling, collapsible groups, search, error jumps.

use std::collections::HashSet;

use gitkraft_ci::annotations::{Annotation, extract};
use gitkraft_ci::log::{LineKind, LogLine, first_failure, parse_log, search, visible_lines};

/// A job log being read.
#[derive(Debug, Clone)]
pub struct LogView {
    /// Title line (job name).
    pub title: String,
    /// Every line of the log.
    pub lines: Vec<LogLine>,
    /// Indexes of collapsed `GroupStart` lines.
    pub collapsed: HashSet<usize>,
    /// Indexes into `lines` that are shown.
    pub visible: Vec<usize>,
    /// Position in `visible`.
    pub cursor: usize,
    /// First shown row (kept by the renderer).
    pub offset: usize,
    /// Current search text.
    pub query: String,
    /// Indexes into `lines` that match `query`.
    pub matches: Vec<usize>,
    /// Which match the cursor last jumped to.
    pub match_pos: usize,
    /// Errors and warnings found in the log.
    pub annotations: Vec<Annotation>,
    /// Show the annotation list above the log.
    pub show_annotations: bool,
}

impl LogView {
    /// Parses `raw`. The cursor starts on the first failure when there is one.
    pub fn new(title: impl Into<String>, raw: &str) -> Self {
        let lines = parse_log(raw);
        let annotations = extract(&lines);
        let visible: Vec<usize> = (0..lines.len()).collect();
        let mut v = Self {
            title: title.into(),
            lines,
            collapsed: HashSet::new(),
            visible,
            cursor: 0,
            offset: 0,
            query: String::new(),
            matches: Vec::new(),
            match_pos: 0,
            show_annotations: !annotations.is_empty(),
            annotations,
        };
        v.jump_failure();
        v
    }

    /// Line index under the cursor.
    pub fn current_line(&self) -> Option<usize> {
        self.visible.get(self.cursor).copied()
    }

    /// Moves the cursor by `delta` rows.
    pub fn move_cursor(&mut self, delta: isize) {
        self.cursor = super::list::step(self.cursor, self.visible.len(), delta);
    }

    /// First row.
    pub fn top(&mut self) {
        self.cursor = 0;
    }

    /// Last row.
    pub fn bottom(&mut self) {
        self.cursor = self.visible.len().saturating_sub(1);
    }

    fn recompute_visible(&mut self) {
        let keep = self.current_line();
        self.visible = visible_lines(&self.lines, &self.collapsed);
        self.cursor = keep
            .and_then(|l| self.visible.iter().position(|&v| v == l))
            .unwrap_or(0);
    }

    /// Collapses or expands the group the cursor is on (or in).
    pub fn toggle_group(&mut self) {
        let Some(mut i) = self.current_line() else {
            return;
        };
        // From inside a group, walk back to the line that opened it.
        if !matches!(self.lines[i].kind, LineKind::GroupStart(_)) {
            let depth = self.lines[i].depth;
            if depth == 0 {
                return;
            }
            match self.lines[..i]
                .iter()
                .rposition(|l| matches!(l.kind, LineKind::GroupStart(_)) && l.depth + 1 == depth)
            {
                Some(open) => i = open,
                None => return,
            }
        }
        if !self.collapsed.remove(&i) {
            self.collapsed.insert(i);
        }
        let keep = i;
        self.visible = visible_lines(&self.lines, &self.collapsed);
        self.cursor = self.visible.iter().position(|&v| v == keep).unwrap_or(0);
    }

    /// Sets the search text and jumps to the first match at or after the cursor.
    pub fn set_query(&mut self, query: &str) {
        self.query = query.to_string();
        self.matches = search(&self.lines, query);
        self.match_pos = 0;
        if let Some(from) = self.current_line()
            && let Some(p) = self.matches.iter().position(|&m| m >= from)
        {
            self.match_pos = p;
        }
        if !self.matches.is_empty() {
            self.jump_to(self.matches[self.match_pos]);
        }
    }

    /// Next match, wrapping.
    pub fn next_match(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        self.match_pos = (self.match_pos + 1) % self.matches.len();
        self.jump_to(self.matches[self.match_pos]);
    }

    /// Previous match, wrapping.
    pub fn prev_match(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        self.match_pos = (self.match_pos + self.matches.len() - 1) % self.matches.len();
        self.jump_to(self.matches[self.match_pos]);
    }

    /// Jumps to the first failure; false when the log has none.
    pub fn jump_failure(&mut self) -> bool {
        match first_failure(&self.lines) {
            Some(i) => {
                self.jump_to(i);
                true
            }
            None => false,
        }
    }

    /// Puts the cursor on log line `line`, opening any collapsed group around it.
    pub fn jump_to(&mut self, line: usize) {
        if line >= self.lines.len() {
            return;
        }
        if !self.visible.contains(&line) {
            // Expand every group that contains the line.
            let mut depth = self.lines[line].depth;
            for i in (0..line).rev() {
                if depth == 0 {
                    break;
                }
                if matches!(self.lines[i].kind, LineKind::GroupStart(_))
                    && self.lines[i].depth < depth
                {
                    self.collapsed.remove(&i);
                    depth = self.lines[i].depth;
                }
            }
            self.recompute_visible();
        }
        if let Some(p) = self.visible.iter().position(|&v| v == line) {
            self.cursor = p;
        }
    }

    /// Jumps to the `n`-th annotation's log line.
    pub fn jump_annotation(&mut self, n: usize) {
        if let Some(a) = self.annotations.get(n) {
            self.jump_to(a.log_index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = "start\n##[group]Build\ncompiling\nerror[E0425]: no x\n --> src/a.rs:1:2\n##[endgroup]\n##[group]Test\nrunning\n##[endgroup]\ndone";

    #[test]
    fn starts_on_the_first_failure_and_lists_annotations() {
        let v = LogView::new("build", LOG);
        assert_eq!(v.current_line(), Some(3));
        assert_eq!(v.annotations.len(), 1);
        assert!(v.show_annotations);
        let clean = LogView::new("ok", "a\nb");
        assert_eq!(clean.current_line(), Some(0));
        assert!(!clean.show_annotations);
    }

    #[test]
    fn collapsing_hides_the_body_and_keeps_the_cursor_on_the_header() {
        let mut v = LogView::new("b", LOG);
        v.toggle_group(); // cursor is inside Build
        assert_eq!(
            v.current_line(),
            Some(1),
            "header of the group the cursor was in"
        );
        assert_eq!(v.visible, vec![0, 1, 6, 7, 8, 9]);
        v.toggle_group();
        assert_eq!(v.visible.len(), v.lines.len());
        // Not in a group: a no-op.
        v.top();
        v.toggle_group();
        assert_eq!(v.visible.len(), v.lines.len());
    }

    #[test]
    fn jumping_into_a_collapsed_group_opens_it() {
        let mut v = LogView::new("b", LOG);
        v.toggle_group();
        assert!(!v.visible.contains(&3));
        v.jump_to(3);
        assert_eq!(v.current_line(), Some(3));
        assert!(v.collapsed.is_empty());
        v.jump_to(999);
        assert_eq!(v.current_line(), Some(3), "out of range is ignored");
    }

    #[test]
    fn search_jumps_and_wraps_both_ways() {
        let mut v = LogView::new("b", "a\nfoo 1\nb\nfoo 2\nc");
        v.top();
        v.set_query("FOO");
        assert_eq!(v.matches, vec![1, 3]);
        assert_eq!(v.current_line(), Some(1));
        v.next_match();
        assert_eq!(v.current_line(), Some(3));
        v.next_match();
        assert_eq!(v.current_line(), Some(1));
        v.prev_match();
        assert_eq!(v.current_line(), Some(3));
        v.set_query("nope");
        assert!(v.matches.is_empty());
        v.next_match();
        v.prev_match();
        assert_eq!(v.current_line(), Some(3), "no matches, no movement");
    }

    #[test]
    fn search_starts_at_the_cursor() {
        let mut v = LogView::new("b", "foo\nx\nfoo\ny");
        v.top();
        v.move_cursor(2);
        v.set_query("foo");
        assert_eq!(v.current_line(), Some(2));
    }

    #[test]
    fn annotation_jump_and_empty_log() {
        let mut v = LogView::new("b", LOG);
        v.top();
        v.jump_annotation(0);
        assert_eq!(v.current_line(), Some(3));
        v.jump_annotation(5);
        let mut e = LogView::new("e", "");
        e.move_cursor(3);
        e.bottom();
        e.toggle_group();
        e.next_match();
        assert_eq!(e.current_line(), None);
        assert!(!e.jump_failure());
    }
}
