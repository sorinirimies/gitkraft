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

/// Indexes into `entries` that match `query`, best first.
pub fn filter_entries(entries: &[Entry], query: &str) -> Vec<usize> {
    gitkraft_hub::fuzzy::filter(entries.iter().map(|e| e.label.as_str()), query)
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
            e("Pull request: merge when the checks pass", 'W', d),
            e("Pull request: add to the merge queue", 'Q', d),
            e("Pull request: force merge (admin)", '!', d),
            e("Pull request: cancel auto-merge or leave the queue", 'Z', d),
            e("Pull request: update branch", 'u', d),
            e("Pull request: cycle merge method", 'S', d),
            e("Pull request: close or reopen", 'C', d),
            e("Pull request: check out in a worktree", 'w', d),
            e("Pull request: show images", 'i', d),
            e("Pull request: change labels", 'L', d),
            e("Pull request: copy link", 'y', d),
            e("Pull request: next tab", 'v', d),
        ]);
    }
    if hub.focus == super::state::Focus::Detail
        && hub.detail.tab == Some(super::state::DetailTab::Diff)
    {
        out.push(e("Diff: comment on the line under the cursor", 'c', d));
    }
    if hub.selected_issue().is_some() {
        out.extend([
            e("Issue: comment", 'c', d),
            e("Issue: close or reopen", 'C', d),
            e("Issue: change labels", 'L', d),
            e("Issue: show images", 'i', d),
            e("Issue: copy link", 'y', d),
        ]);
    }
    if hub.selected_run().is_some() {
        out.extend([
            e("CI run: re-run failed jobs", 'f', d),
            e("CI run: re-run everything", 'F', d),
            e("CI run: run the workflow again with inputs", 'D', d),
            e("CI run: download artifacts", 'A', d),
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

    fn list() -> Vec<Entry> {
        vec![
            e("Refresh", 'r', View::Dashboard),
            e("Go to pulse", 'p', View::Dashboard),
            e("Pulse: next repository", ']', View::Pulse),
        ]
    }

    #[test]
    fn filtering_ranks_and_keeps_order_on_ties() {
        assert_eq!(filter_entries(&list(), ""), [0, 1, 2]);
        assert_eq!(
            filter_entries(&list(), "pulse"),
            [1, 2],
            "word-start 'Pulse:' and 'pulse' tie on letters; list order decides"
        );
        assert_eq!(filter_entries(&list(), "next"), [2]);
        assert!(filter_entries(&list(), "xyz").is_empty());
    }
}
