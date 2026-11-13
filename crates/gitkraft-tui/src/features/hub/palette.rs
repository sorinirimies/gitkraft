//! The command palette: every action of the hub in one fuzzy-searchable list.
//!
//! An entry is a label plus the key that already does the thing, so the palette cannot
//! drift from the keyboard: running an entry feeds that key to the view it belongs to.

use super::state::{HubState, View};
use gitkraft_settings::dashboard::SectionKind;

/// One action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// What the list shows.
    pub label: String,
    /// The key that does it in `view`.
    pub key: char,
    /// The view that key belongs to.
    pub view: View,
}

/// The palette's state while open.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Palette {
    /// What was typed.
    pub query: String,
    /// Selected row of the filtered list.
    pub row: usize,
}

/// How well `query` matches `text`: `None` unless every query letter appears in order.
/// Consecutive letters and letters at a word start score higher; gaps cost a little.
pub fn fuzzy_score(query: &str, text: &str) -> Option<i32> {
    let q: Vec<char> = query
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if q.is_empty() {
        return Some(0);
    }
    let t: Vec<char> = text.to_lowercase().chars().collect();
    let mut score = 0;
    let mut qi = 0;
    let mut last: Option<usize> = None;
    for (ti, c) in t.iter().enumerate() {
        if qi < q.len() && *c == q[qi] {
            score += 10;
            if last.is_some_and(|l| l + 1 == ti) {
                score += 15;
            }
            if ti == 0 || !t[ti - 1].is_alphanumeric() {
                score += 12;
            }
            if let Some(l) = last {
                score -= ((ti - l - 1) as i32).min(6);
            }
            last = Some(ti);
            qi += 1;
        }
    }
    (qi == q.len()).then_some(score - t.len() as i32 / 8)
}

/// Indexes into `entries` that match `query`, best first (ties keep list order).
pub fn filter(entries: &[Entry], query: &str) -> Vec<usize> {
    let mut hits: Vec<(i32, usize)> = entries
        .iter()
        .enumerate()
        .filter_map(|(i, e)| fuzzy_score(query, &e.label).map(|s| (s, i)))
        .collect();
    hits.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    hits.into_iter().map(|(_, i)| i).collect()
}

fn e(label: impl Into<String>, key: char, view: View) -> Entry {
    Entry {
        label: label.into(),
        key,
        view,
    }
}

/// The actions that make sense right now.
pub fn entries_for(hub: &HubState) -> Vec<Entry> {
    let d = View::Dashboard;
    let mut out = Vec::new();
    if hub.selected_pr().is_some() {
        out.extend([
            e("Pull request: approve", 'a', d),
            e("Pull request: request changes", 'x', d),
            e("Pull request: comment", 'c', d),
            e("Pull request: merge", 'm', d),
            e("Pull request: squash and merge", 'M', d),
            e("Pull request: close or reopen", 'C', d),
            e("Pull request: check out in a worktree", 'w', d),
            e("Pull request: show images", 'i', d),
            e("Pull request: copy link", 'y', d),
            e("Pull request: next tab", 'v', d),
        ]);
    }
    if hub.selected_issue().is_some() {
        out.extend([
            e("Issue: comment", 'c', d),
            e("Issue: close or reopen", 'C', d),
            e("Issue: show images", 'i', d),
            e("Issue: copy link", 'y', d),
        ]);
    }
    if hub.selected_run().is_some() {
        out.extend([
            e("CI run: re-run failed jobs", 'f', d),
            e("CI run: re-run everything", 'F', d),
            e("CI run: run the workflow again with inputs", 'D', d),
            e("CI run: cancel", 'X', d),
            e("CI run: copy link", 'y', d),
        ]);
    }
    for (i, idx) in hub.visible_sections().into_iter().take(9).enumerate() {
        let s = &hub.dashboard.sections[idx];
        let kind = match s.kind {
            SectionKind::Prs => "pull requests",
            SectionKind::Issues => "issues",
            SectionKind::Ci => "CI",
            SectionKind::Inbox => "inbox",
        };
        out.push(e(
            format!("Go to section: {} ({kind})", s.name),
            char::from_digit(i as u32 + 1, 10).unwrap_or('1'),
            d,
        ));
    }
    out.extend([
        e("Section: move left", '{', d),
        e("Section: move right", '}', d),
        e("Go to pulse", 'p', d),
        e("Go to SSH keys", 'K', d),
        e("Refresh", 'r', d),
        e("Notifications: toggle for this project", 'n', d),
        e("Notifications: toggle for everything", 'N', d),
        e("SSH keys: set up this machine", 's', View::Keys),
        e("SSH keys: test connection", 't', View::Keys),
        e("SSH keys: remove the selected key", 'd', View::Keys),
        e("Pulse: last 7 days", '1', View::Pulse),
        e("Pulse: last 30 days", '2', View::Pulse),
        e("Pulse: last 90 days", '3', View::Pulse),
        e("Pulse: last year", '4', View::Pulse),
        e("Pulse: next repository", ']', View::Pulse),
        e("Pulse: previous repository", '[', View::Pulse),
    ]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_must_appear_in_order_and_case_does_not_matter() {
        assert!(fuzzy_score("mrg", "Pull request: merge").is_some());
        assert!(fuzzy_score("MERGE", "Pull request: merge").is_some());
        assert!(fuzzy_score("gem", "merge").is_none());
        assert!(fuzzy_score("zzz", "merge").is_none());
        assert_eq!(fuzzy_score("", "anything"), Some(0));
        assert_eq!(fuzzy_score("   ", "anything"), Some(0));
    }

    #[test]
    fn consecutive_letters_and_word_starts_win() {
        let tight = fuzzy_score("rerun", "CI run: re-run failed jobs").expect("m");
        let loose = fuzzy_score("rerun", "Pull request: close or reopen, then run").expect("m");
        assert!(tight > loose);
        assert!(
            fuzzy_score("ssh", "Go to SSH keys")
                > fuzzy_score("ssh", "Pulse: previous repository, see slashes here")
        );
    }

    fn list() -> Vec<Entry> {
        vec![
            e("Refresh", 'r', View::Dashboard),
            e("Go to pulse", 'p', View::Dashboard),
            e("Pulse: next repository", ']', View::Pulse),
        ]
    }

    #[test]
    fn filtering_ranks_and_keeps_order_on_ties() {
        assert_eq!(filter(&list(), ""), [0, 1, 2]);
        assert_eq!(
            filter(&list(), "pulse"),
            [1, 2],
            "word-start 'Pulse:' and 'pulse' tie on letters; list order decides"
        );
        assert_eq!(filter(&list(), "next"), [2]);
        assert!(filter(&list(), "xyz").is_empty());
    }
}
