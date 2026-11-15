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
    /// Opened with `?`: only the actions of the screen you are on.
    pub scoped: bool,
}

/// What the open palette lists: everything, or (opened with `?`) the actions of this screen.
pub fn shown_entries(hub: &HubState) -> Vec<Entry> {
    let all = entries_for(hub);
    if hub.palette.as_ref().is_some_and(|p| p.scoped) {
        all.into_iter().filter(|e| e.view == hub.view).collect()
    } else {
        all
    }
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
    if hub.is_multi() {
        out.push(e("Remote: show the next forge (all → each → all)", 'o', d));
    }
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
            e("CI run: ask the AI why it failed", 'E', d),
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
    if hub.selected_pr().is_some() {
        out.extend([
            e("Pull request: AI summary", 'B', d),
            e("Pull request: AI catch-up on the thread", 'I', d),
            e("Pull request: AI review notes", 'V', d),
        ]);
    }
    if matches!(hub.active_data(), Some(gitkraft_hub::SectionData::Inbox(_))) {
        out.push(e("Inbox: AI triage (now / fyi / ignore)", 'P', d));
    }
    out.push(e("Settings menu (theme, motion, commit style)", ',', d));
    out.extend([
        e("Section: move left", '{', d),
        e("Section: move right", '}', d),
        e("Go to pulse", 'p', d),
        e("Go to SSH keys", 'K', d),
        e("Go to agents (scheduled CI watchers)", 'J', d),
        e("Agents: run the selected job now", 'r', View::Agents),
        e("Agents: create a job", 'n', View::Agents),
        e("Agents: pause or resume the job", 'e', View::Agents),
        e("Agents: next schedule", 's', View::Agents),
        e(
            "Agents: next harness (builtin, pi, copilot, vscode, zed)",
            'h',
            View::Agents,
        ),
        e(
            "Agents: next visibility (all, public, private)",
            'v',
            View::Agents,
        ),
        e("Agents: next host filter", 'H', View::Agents),
        e(
            "Agents: watch only the checked repositories",
            'o',
            View::Agents,
        ),
        e("Agents: show the job's last report", 'l', View::Agents),
        e("Agents: delete the job (asks first)", 'd', View::Agents),
        e("Agents: set the model", 'm', View::Agents),
        e("Agents: edit the instructions", 'p', View::Agents),
        e("Agents: edit the project globs", '/', View::Agents),
        e("Agents: watch forks too (on/off)", 'f', View::Agents),
        e(
            "Agents: watch archived repositories (on/off)",
            'x',
            View::Agents,
        ),
        e(
            "Agents: file findings in the inbox (on/off)",
            'i',
            View::Agents,
        ),
        e("Agents: system notification (on/off)", 'y', View::Agents),
        e(
            "Agents: scheduler inside this app (on/off)",
            'u',
            View::Agents,
        ),
        e("Agents: reload from disk", 'R', View::Agents),
        e("Go to discussions and wiki", 'T', d),
        e("Discussions and wiki: show the wiki", 'w', View::Community),
        e(
            "Discussions and wiki: show discussions",
            'd',
            View::Community,
        ),
        e("Discussions and wiki: show boards", 'b', View::Community),
        e("Discussions and wiki: reload", 'r', View::Community),
        e(
            "Discussions and wiki: next repository",
            ']',
            View::Community,
        ),
        e(
            "Discussions and wiki: previous repository",
            '[',
            View::Community,
        ),
        e("Discussion: comment", 'c', View::Community),
        e("Discussion: reply to a comment", 'R', View::Community),
        e("Discussion: mark the answer", 'A', View::Community),
        e("Wiki: edit the page in your editor", 'e', View::Community),
        e("Wiki: upload the edit", 'U', View::Community),
        e("Wiki: new page", 'n', View::Community),
        e("Board: move the card to a column", 'm', View::Community),
        e("Board: new card", 'n', View::Community),
        e("Board: move the card left", '<', View::Community),
        e("Board: move the card right", '>', View::Community),
        e("Refresh", 'r', d),
        e("Notifications: toggle for this project", 'n', d),
        e("Notifications: toggle for everything", 'N', d),
        e("SSH keys: set up this machine", 's', View::Keys),
        e("SSH keys: test connection", 't', View::Keys),
        e("SSH keys: remove the selected key", 'd', View::Keys),
        e("SSH keys: refresh", 'r', View::Keys),
        e("Pulse: last 7 days", '1', View::Pulse),
        e("Pulse: last 30 days", '2', View::Pulse),
        e("Pulse: last 90 days", '3', View::Pulse),
        e("Pulse: last year", '4', View::Pulse),
        e("Pulse: today", 't', View::Pulse),
        e("Pulse: reload", 'r', View::Pulse),
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
