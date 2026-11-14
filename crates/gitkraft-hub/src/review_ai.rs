//! Drafts about pull requests and the inbox: a summary, a catch-up on a thread, review notes and
//! a triage of notifications. Like every AI feature here they are drafts (text for a box, a hint
//! for a list), never an action, and a host must have opted in to each of them ([`Policy`]).

use std::collections::BTreeMap;
use std::path::Path;

use chrono::{DateTime, Utc};
use gitkraft_platforms::model::Notification;
use serde::{Deserialize, Serialize};

use crate::PrDetail;
use crate::ai::{AiError, Llm, redact};

/// Characters of diff a prompt may carry.
pub const MAX_DIFF_CHARS: usize = 14_000;
/// Characters of discussion a prompt may carry.
pub const MAX_THREAD_CHARS: usize = 8_000;

/// What a model may be asked to do with a host's data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Action {
    /// Summarise a pull request.
    Summary,
    /// What happened in a thread since a time.
    CatchUp,
    /// Draft review notes.
    Review,
    /// Sort the inbox.
    Triage,
}

impl Action {
    /// Every action.
    pub const ALL: [Self; 4] = [Self::Summary, Self::CatchUp, Self::Review, Self::Triage];

    /// The word used in the policy file and on the command line.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::CatchUp => "catch_up",
            Self::Review => "review",
            Self::Triage => "triage",
        }
    }

    /// The action called `s`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.name() == s)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct HostPolicy {
    #[serde(default)]
    actions: Vec<String>,
}

/// Which hosts let a model see their data, per action. Off until switched on; kept in
/// `<config dir>/gitkraft/ai-policy.toml` (no secrets in it).
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Policy {
    #[serde(default)]
    hosts: BTreeMap<String, HostPolicy>,
}

impl Policy {
    /// The file under `dir`.
    #[must_use]
    pub fn path(dir: &Path) -> std::path::PathBuf {
        dir.join("ai-policy.toml")
    }

    /// Reads the policy; a missing file is an empty one (everything off).
    ///
    /// # Errors
    /// The file exists but is not valid.
    pub fn load(dir: &Path) -> Result<Self, String> {
        match std::fs::read_to_string(Self::path(dir)) {
            Ok(t) => toml::from_str(&t).map_err(|e| format!("ai-policy.toml: {e}")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Writes the policy under `dir`.
    ///
    /// # Errors
    /// The file cannot be written.
    pub fn save(&self, dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let text = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(Self::path(dir), text).map_err(|e| e.to_string())
    }

    /// May `action` send data of `host` to a model?
    #[must_use]
    pub fn allows(&self, host: &str, action: Action) -> bool {
        self.hosts
            .get(host)
            .is_some_and(|h| h.actions.iter().any(|a| a == "all" || a == action.name()))
    }

    /// Switches `actions` on for `host`.
    pub fn allow(&mut self, host: &str, actions: &[Action]) {
        let h = self.hosts.entry(host.to_string()).or_default();
        for a in actions {
            if !h.actions.iter().any(|x| x == a.name()) {
                h.actions.push(a.name().to_string());
            }
        }
        h.actions.retain(|a| a != "all");
    }

    /// Switches `actions` off for `host`; an empty list switches all of them off.
    pub fn deny(&mut self, host: &str, actions: &[Action]) {
        if actions.is_empty() {
            self.hosts.remove(host);
            return;
        }
        if let Some(h) = self.hosts.get_mut(host) {
            let all = h.actions.iter().any(|a| a == "all");
            if all {
                h.actions = Action::ALL.iter().map(|a| a.name().to_string()).collect();
            }
            h.actions.retain(|a| !actions.iter().any(|x| x.name() == a));
            if h.actions.is_empty() {
                self.hosts.remove(host);
            }
        }
    }

    /// `host: summary, review` per line; empty when nothing is on.
    #[must_use]
    pub fn describe(&self) -> String {
        self.hosts
            .iter()
            .map(|(h, p)| format!("{h}: {}\n", p.actions.join(", ")))
            .collect()
    }

    /// The refusal for `host`/`action`, or `Ok`.
    ///
    /// # Errors
    /// The host has not opted in; the message says how to.
    pub fn check(&self, host: &str, action: Action) -> Result<(), String> {
        if self.allows(host, action) {
            Ok(())
        } else {
            Err(format!(
                "AI {} is off for {host}: run `gitkraft-tui ai-allow {host} {}` to let a model see its data",
                action.name(),
                action.name()
            ))
        }
    }
}

fn clip(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let cut: String = text.chars().take(max).collect();
    format!("{cut}\n[... cut ...]")
}

fn thread(detail: &PrDetail, since: Option<DateTime<Utc>>) -> String {
    let mut out = String::new();
    for c in &detail.comments {
        if since.is_some_and(|s| c.created_at.is_some_and(|t| t < s)) {
            continue;
        }
        let at = c
            .created_at
            .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_default();
        out.push_str(&format!("[{at}] {}: {}\n", c.author.login, c.body.trim()));
    }
    for r in &detail.reviews {
        if since.is_some_and(|s| r.submitted_at.is_some_and(|t| t < s)) {
            continue;
        }
        let v = r.verdict.map_or("commented", |v| match v {
            gitkraft_platforms::model::ReviewVerdict::Approve => "approved",
            gitkraft_platforms::model::ReviewVerdict::RequestChanges => "requested changes",
            gitkraft_platforms::model::ReviewVerdict::Comment => "commented",
        });
        out.push_str(&format!(
            "[review] {} {v}: {}\n",
            r.author.login,
            r.body.trim()
        ));
    }
    out
}

/// The redacted prompt of a pull request: title, description, thread and diff. `since` keeps only
/// the thread after that time.
#[must_use]
pub fn pr_prompt(detail: &PrDetail, since: Option<DateTime<Utc>>, with_diff: bool) -> String {
    let pr = &detail.pr;
    let mut out = format!(
        "Repository: {}\nPull request #{}: {}\nBranch: {} -> {}\nAuthor: {}\n\nDescription:\n{}\n\nThread:\n{}",
        pr.repo.key(),
        pr.number,
        pr.title,
        pr.source_branch,
        pr.target_branch,
        pr.author.login,
        pr.body.trim(),
        clip(&redact(&thread(detail, since)), MAX_THREAD_CHARS),
    );
    if with_diff {
        out.push_str("\n\nDiff:\n");
        out.push_str(&clip(&redact(&detail.diff), MAX_DIFF_CHARS));
    }
    redact_all(&out)
}

fn redact_all(s: &str) -> String {
    redact(s)
}

const SUMMARY_SYSTEM: &str = "You summarise a pull request for a maintainer. Answer in plain text: What (two sentences), \
Why (if the description says), Risk (what could break, from the diff), and Open questions (from the thread). \
Use only what the text shows; if something is not shown, say so. Never invent files or behaviour.";

const CATCH_UP_SYSTEM: &str = "You catch a reader up on a pull request thread. Say in at most six bullet points what \
was decided, what was asked of the reader and what is still open, newest last. Use only the text shown.";

const REVIEW_SYSTEM: &str = "You draft review notes for a pull request diff, for the reviewer to edit. Output one note per \
line in exactly the form `path:line: note` for a note on a changed line (the line number in the new file), or \
`general: note` otherwise. At most eight notes, only real problems or questions the diff supports, no praise, \
no style nits a formatter would fix. If you see nothing, output `general: no issues found`.";

/// A summary of the pull request.
///
/// # Errors
/// The model call.
pub async fn summarize_pr(llm: &impl Llm, detail: &PrDetail) -> Result<String, AiError> {
    llm.complete(SUMMARY_SYSTEM, &pr_prompt(detail, None, true))
        .await
}

/// What happened in the thread since `since` (all of it when `None`).
///
/// # Errors
/// The model call.
pub async fn catch_up(
    llm: &impl Llm,
    detail: &PrDetail,
    since: Option<DateTime<Utc>>,
) -> Result<String, AiError> {
    llm.complete(CATCH_UP_SYSTEM, &pr_prompt(detail, since, false))
        .await
}

/// One drafted review note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Note {
    /// File, for a note on a line.
    pub path: Option<String>,
    /// Line in the new file.
    pub line: Option<u32>,
    /// The note.
    pub text: String,
}

impl Note {
    /// `path:line: text`, or the text alone.
    #[must_use]
    pub fn render(&self) -> String {
        match (&self.path, self.line) {
            (Some(p), Some(l)) => format!("{p}:{l}: {}", self.text),
            _ => self.text.clone(),
        }
    }
}

/// Reads the notes of a review draft; a line that is not in the asked form becomes a general note.
#[must_use]
pub fn parse_notes(answer: &str) -> Vec<Note> {
    answer
        .lines()
        .map(|l| l.trim().trim_start_matches(['-', '*', ' ']).trim())
        .filter(|l| !l.is_empty())
        .map(|l| {
            let general = |t: &str| Note {
                path: None,
                line: None,
                text: t.trim().to_string(),
            };
            if let Some(rest) = l.strip_prefix("general:") {
                return general(rest);
            }
            let mut parts = l.splitn(3, ':');
            match (parts.next(), parts.next(), parts.next()) {
                (Some(p), Some(n), Some(t))
                    if !p.contains(' ') && n.trim().parse::<u32>().is_ok() =>
                {
                    Note {
                        path: Some(p.to_string()),
                        line: n.trim().parse().ok(),
                        text: t.trim().to_string(),
                    }
                }
                _ => general(l),
            }
        })
        .collect()
}

/// Review notes for the diff.
///
/// # Errors
/// No diff, or the model call.
pub async fn review_draft(llm: &impl Llm, detail: &PrDetail) -> Result<Vec<Note>, AiError> {
    if detail.diff.trim().is_empty() {
        return Err(AiError::Transport("the pull request has no diff".into()));
    }
    let answer = llm
        .complete(REVIEW_SYSTEM, &pr_prompt(detail, None, true))
        .await?;
    Ok(parse_notes(&answer))
}

/// How urgent a notification is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Urgency {
    /// Needs you today.
    Now,
    /// Good to know.
    Fyi,
    /// Can be dismissed.
    Ignore,
}

/// One triaged notification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Triaged {
    /// Notification id.
    pub id: String,
    /// The verdict.
    pub urgency: Urgency,
    /// Why, in a few words.
    pub why: String,
}

const TRIAGE_SYSTEM: &str = "You triage a developer's notifications. For every numbered line answer one line \
`N | now|fyi|ignore | reason (max 8 words)`. now = needs the developer today (review requested, a failure on their work, \
a direct question), fyi = good to know, ignore = noise. Judge only from the text shown.";

/// The prompt that lists the notifications by number.
#[must_use]
pub fn triage_prompt(items: &[Notification]) -> String {
    items
        .iter()
        .enumerate()
        .map(|(i, n)| {
            format!(
                "{}. [{:?}] {} {} {}\n",
                i + 1,
                n.reason,
                n.repo.as_ref().map(|r| r.key()).unwrap_or_default(),
                n.number.map(|x| format!("#{x}")).unwrap_or_default(),
                redact(&n.title)
            )
        })
        .collect()
}

/// Reads a triage answer; lines that name no notification or no urgency are skipped (a model that
/// rambles must not mislabel anything).
#[must_use]
pub fn parse_triage(answer: &str, items: &[Notification]) -> Vec<Triaged> {
    answer
        .lines()
        .filter_map(|l| {
            let mut p = l.splitn(3, '|').map(str::trim);
            let n: usize = p.next()?.trim_end_matches('.').parse().ok()?;
            let urgency = match p.next()?.to_lowercase().as_str() {
                "now" => Urgency::Now,
                "fyi" => Urgency::Fyi,
                "ignore" => Urgency::Ignore,
                _ => return None,
            };
            Some(Triaged {
                id: items.get(n.checked_sub(1)?)?.id.clone(),
                urgency,
                why: p.next().unwrap_or_default().to_string(),
            })
        })
        .collect()
}

/// Sorts the notifications; anything the model skipped is left out of the result.
///
/// # Errors
/// The model call.
pub async fn triage(llm: &impl Llm, items: &[Notification]) -> Result<Vec<Triaged>, AiError> {
    if items.is_empty() {
        return Ok(vec![]);
    }
    let answer = llm.complete(TRIAGE_SYSTEM, &triage_prompt(items)).await?;
    Ok(parse_triage(&answer, items))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::FakeLlm;
    use gitkraft_platforms::RepoId;
    use gitkraft_platforms::model::{
        Comment, ItemState, NotificationReason, PullRequest, Review, ReviewVerdict, SubjectKind,
        User,
    };

    fn user(n: &str) -> User {
        User {
            login: n.into(),
            avatar_url: None,
        }
    }

    fn at(d: u32) -> Option<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(&format!("2026-05-{d:02}T10:00:00Z"))
            .ok()
            .map(|t| t.with_timezone(&Utc))
    }

    fn detail() -> PrDetail {
        PrDetail {
            pr: PullRequest {
                repo: RepoId::new("github.com", "o", "r"),
                number: 7,
                title: "Add cache".into(),
                body: "Speeds up lookups. token=ghp_abcdefghijklmnopqrstuvwxyz0123456789".into(),
                author: user("ann"),
                state: ItemState::Open,
                source_branch: "cache".into(),
                target_branch: "main".into(),
                ..PullRequest::default()
            },
            comments: vec![
                Comment {
                    id: 1,
                    author: user("bob"),
                    body: "old remark".into(),
                    created_at: at(1),
                    parent: None,
                },
                Comment {
                    id: 2,
                    author: user("cy"),
                    body: "new remark".into(),
                    created_at: at(9),
                    parent: None,
                },
            ],
            reviews: vec![Review {
                author: user("dee"),
                verdict: Some(ReviewVerdict::RequestChanges),
                body: "needs tests".into(),
                submitted_at: at(10),
            }],
            diff: "+let x = 1;".into(),
            errors: vec![],
        }
    }

    #[test]
    fn nothing_is_allowed_until_a_host_opts_in_per_action() {
        let dir = tempfile::tempdir().expect("tmp");
        let mut p = Policy::load(dir.path()).expect("empty");
        assert!(!p.allows("github.com", Action::Summary));
        assert!(
            p.check("github.com", Action::Summary)
                .unwrap_err()
                .contains("ai-allow github.com summary")
        );
        p.allow("github.com", &[Action::Summary, Action::Review]);
        p.allow("github.com", &[Action::Summary]);
        p.save(dir.path()).expect("save");
        let q = Policy::load(dir.path()).expect("load");
        assert!(q.allows("github.com", Action::Review));
        assert!(!q.allows("github.com", Action::Triage), "per action");
        assert!(!q.allows("codeberg.org", Action::Review), "per host");
        assert_eq!(q.describe(), "github.com: summary, review\n");
        let mut q = q;
        q.deny("github.com", &[Action::Review]);
        assert!(!q.allows("github.com", Action::Review) && q.allows("github.com", Action::Summary));
        q.deny("github.com", &[]);
        assert_eq!(q.describe(), "");
    }

    #[test]
    fn a_hand_written_all_means_every_action_and_survives_a_partial_deny() {
        let dir = tempfile::tempdir().expect("tmp");
        std::fs::write(
            Policy::path(dir.path()),
            "[hosts.\"gitea.lan\"]\nactions = [\"all\"]\n",
        )
        .expect("write");
        let mut p = Policy::load(dir.path()).expect("load");
        assert!(Action::ALL.iter().all(|a| p.allows("gitea.lan", *a)));
        p.deny("gitea.lan", &[Action::Triage]);
        assert!(p.allows("gitea.lan", Action::Summary) && !p.allows("gitea.lan", Action::Triage));
        std::fs::write(Policy::path(dir.path()), "hosts = 3").expect("write");
        assert!(Policy::load(dir.path()).is_err());
    }

    #[tokio::test]
    async fn the_summary_prompt_carries_the_pr_without_secrets() {
        let llm = FakeLlm::saying("What: a cache");
        let out = summarize_pr(&llm, &detail()).await.expect("ok");
        assert_eq!(out, "What: a cache");
        let (_, user) = &llm.prompts()[0];
        assert!(
            user.contains("#7: Add cache")
                && user.contains("new remark")
                && user.contains("+let x")
        );
        assert!(!user.contains("ghp_abcdefghijklmnopqrstuvwxyz"), "{user}");
    }

    #[tokio::test]
    async fn a_catch_up_keeps_only_what_came_after() {
        let llm = FakeLlm::saying("- done");
        catch_up(&llm, &detail(), at(5)).await.expect("ok");
        let user = &llm.prompts()[0].1;
        assert!(user.contains("new remark") && user.contains("needs tests"));
        assert!(!user.contains("old remark"), "{user}");
        assert!(!user.contains("Diff:"), "a catch-up needs no diff");
    }

    #[test]
    fn review_notes_are_read_leniently() {
        let n = parse_notes(
            "- src/a.rs:12: unchecked index\ngeneral: add a test\nrambling line\n* src/b.rs: no line here\n",
        );
        assert_eq!(n.len(), 4);
        assert_eq!(
            (n[0].path.as_deref(), n[0].line),
            (Some("src/a.rs"), Some(12))
        );
        assert_eq!(n[0].render(), "src/a.rs:12: unchecked index");
        assert_eq!(
            (n[1].path.clone(), n[1].text.as_str()),
            (None, "add a test")
        );
        assert_eq!(n[2].text, "rambling line");
        assert_eq!(n[3].path, None, "a path without a line is a general note");
    }

    #[tokio::test]
    async fn a_review_draft_needs_a_diff() {
        let mut d = detail();
        d.diff.clear();
        let llm = FakeLlm::saying("general: x");
        assert!(review_draft(&llm, &d).await.is_err());
        assert!(llm.prompts().is_empty(), "no call without a diff");
    }

    fn note(id: &str, title: &str) -> Notification {
        Notification {
            id: id.into(),
            repo: Some(RepoId::new("github.com", "o", "r")),
            kind: SubjectKind::PullRequest,
            number: Some(3),
            title: title.into(),
            reason: NotificationReason::ReviewRequested,
            unread: true,
            updated_at: None,
            url: String::new(),
        }
    }

    #[tokio::test]
    async fn triage_maps_numbers_to_ids_and_skips_nonsense() {
        let items = vec![note("a", "Fix"), note("b", "Bump"), note("c", "Chore")];
        let llm = FakeLlm::saying(
            "1 | now | review asked\n2. | IGNORE | bot bump\n9 | now | no such\n3 | maybe | ?\nblah",
        );
        let t = triage(&llm, &items).await.expect("ok");
        assert_eq!(
            t,
            vec![
                Triaged {
                    id: "a".into(),
                    urgency: Urgency::Now,
                    why: "review asked".into()
                },
                Triaged {
                    id: "b".into(),
                    urgency: Urgency::Ignore,
                    why: "bot bump".into()
                },
            ]
        );
        assert!(
            llm.prompts()[0]
                .1
                .starts_with("1. [ReviewRequested] github.com/o/r #3 Fix")
        );
        let none = FakeLlm::default();
        assert!(
            triage(&none, &[]).await.expect("ok").is_empty(),
            "no call for an empty inbox"
        );
    }
}
