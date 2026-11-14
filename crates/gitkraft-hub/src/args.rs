//! Words a user types, turned into the platform types: the same rules for the command line, the MCP
//! server and the web page, so a flag means the same thing everywhere.

use gitkraft_ci::{RerunScope, Status};
use gitkraft_platforms::RepoId;
use gitkraft_platforms::model::{
    MergeMethod, MergeTiming, ReviewVerdict, StateFilter, UpdateStyle,
};
use gitkraft_settings::dashboard::{Involvement, Section, SectionKind, SectionState};

/// `open` (the default), `closed` or `all`.
///
/// # Errors
/// Anything else.
pub fn state_filter(s: Option<&str>) -> Result<StateFilter, String> {
    match s {
        None | Some("open") => Ok(StateFilter::Open),
        Some("closed") => Ok(StateFilter::Closed),
        Some("all") => Ok(StateFilter::All),
        Some(o) => Err(format!("state must be open, closed or all, not '{o}'")),
    }
}

/// `queued`, `running`, `success`, `failed` or `cancelled`.
///
/// # Errors
/// Anything else.
pub fn status(s: &str) -> Result<Status, String> {
    Ok(match s {
        "queued" => Status::Queued,
        "running" => Status::Running,
        "success" => Status::Success,
        "failed" => Status::Failed,
        "cancelled" => Status::Cancelled,
        o => {
            return Err(format!(
                "status must be queued, running, success, failed or cancelled, not '{o}'"
            ));
        }
    })
}

/// `authored` (the default), `assigned` or `review_requested` (`review` for short).
///
/// # Errors
/// Anything else.
pub fn involvement(s: Option<&str>) -> Result<Involvement, String> {
    match s {
        None | Some("authored") => Ok(Involvement::Authored),
        Some("assigned") => Ok(Involvement::Assigned),
        Some("review" | "review_requested") => Ok(Involvement::ReviewRequested),
        Some(o) => Err(format!(
            "involvement must be authored, assigned or review_requested, not '{o}'"
        )),
    }
}

/// The section a one-off query is loaded through.
#[must_use]
pub fn section(
    name: &str,
    kind: SectionKind,
    involvement: Option<Involvement>,
    state: StateFilter,
) -> Section {
    Section {
        name: name.into(),
        kind,
        repos: vec![],
        involvement,
        state: match state {
            StateFilter::Open => SectionState::Open,
            StateFilter::Closed => SectionState::Closed,
            StateFilter::All => SectionState::All,
        },
        label: None,
        branch: None,
        hidden: false,
    }
}

/// `merge` (the default), `squash` or `rebase`.
///
/// # Errors
/// Anything else.
pub fn merge_method(s: Option<&str>) -> Result<MergeMethod, String> {
    match s {
        None | Some("merge") => Ok(MergeMethod::Merge),
        Some("squash") => Ok(MergeMethod::Squash),
        Some("rebase") => Ok(MergeMethod::Rebase),
        Some(o) => Err(format!("method must be merge, squash or rebase, not '{o}'")),
    }
}

/// `now` (the default), `when_green` or `queue`.
///
/// # Errors
/// Anything else.
pub fn merge_timing(s: Option<&str>) -> Result<MergeTiming, String> {
    match s {
        None | Some("now") => Ok(MergeTiming::Now),
        Some("when_green") => Ok(MergeTiming::WhenGreen),
        Some("queue") => Ok(MergeTiming::Queue),
        Some(o) => Err(format!("when must be now, when_green or queue, not '{o}'")),
    }
}

/// `default` (the platform's own way, also when absent), `merge` or `rebase`.
///
/// # Errors
/// Anything else.
pub fn update_style(s: Option<&str>) -> Result<UpdateStyle, String> {
    match s {
        None | Some("default") => Ok(UpdateStyle::Default),
        Some("merge") => Ok(UpdateStyle::Merge),
        Some("rebase") => Ok(UpdateStyle::Rebase),
        Some(o) => Err(format!("style must be merge or rebase, not '{o}'")),
    }
}

/// `approve`, `request_changes` or `comment`.
///
/// # Errors
/// Anything else.
pub fn verdict(s: &str) -> Result<ReviewVerdict, String> {
    match s {
        "approve" => Ok(ReviewVerdict::Approve),
        "request_changes" => Ok(ReviewVerdict::RequestChanges),
        "comment" => Ok(ReviewVerdict::Comment),
        o => Err(format!(
            "verdict must be approve, request_changes or comment, not '{o}'"
        )),
    }
}

/// `failed` or `all`.
///
/// # Errors
/// Anything else.
pub fn rerun_scope(s: &str) -> Result<RerunScope, String> {
    match s {
        "failed" => Ok(RerunScope::Failed),
        "all" => Ok(RerunScope::All),
        o => Err(format!("scope must be failed or all, not '{o}'")),
    }
}

/// `host/owner/name`, `owner/name` (when there is one account or a github.com one), or the default
/// repository (the checkout the program runs in).
///
/// # Errors
/// Nothing given and no default, or not a repository name.
pub fn resolve_repo(
    given: Option<&str>,
    default: Option<&RepoId>,
    hosts: &[&str],
    no_default: &str,
) -> Result<RepoId, String> {
    let Some(text) = given.map(str::trim).filter(|t| !t.is_empty()) else {
        return default.cloned().ok_or_else(|| no_default.to_string());
    };
    if let Some(id) = RepoId::from_key(text) {
        return Ok(id);
    }
    let host = match hosts {
        [only] => Some(*only),
        many => many.iter().copied().find(|h| *h == "github.com"),
    };
    match (text.split('/').collect::<Vec<_>>().as_slice(), host) {
        ([owner, name], Some(host)) => Ok(RepoId::new(host, *owner, *name)),
        _ => Err(format!("'{text}' is not host/owner/name")),
    }
}

/// What a log request returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogExcerpt {
    /// Lines of the whole log.
    pub total_lines: usize,
    /// The picked lines; with a search each one starts with its line number.
    pub lines: Vec<String>,
}

/// The colourless log, cut down: lines containing `grep` (any case, at most `max_matches`, each
/// prefixed `N: `), else the last `tail` lines.
#[must_use]
pub fn log_excerpt(raw: &str, tail: usize, grep: Option<&str>, max_matches: usize) -> LogExcerpt {
    let clean = gitkraft_ci::log::strip_ansi(raw);
    let lines: Vec<&str> = clean.lines().collect();
    let picked = match grep.filter(|g| !g.is_empty()) {
        Some(g) => {
            let g = g.to_lowercase();
            lines
                .iter()
                .enumerate()
                .filter(|(_, l)| l.to_lowercase().contains(&g))
                .take(max_matches)
                .map(|(i, l)| format!("{}: {l}", i + 1))
                .collect()
        }
        None => lines[lines.len().saturating_sub(tail.max(1))..]
            .iter()
            .map(|l| (*l).to_string())
            .collect(),
    };
    LogExcerpt {
        total_lines: lines.len(),
        lines: picked,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_map_to_types_with_defaults_and_say_what_was_allowed() {
        assert_eq!(state_filter(None), Ok(StateFilter::Open));
        assert_eq!(state_filter(Some("all")), Ok(StateFilter::All));
        assert!(
            state_filter(Some("x"))
                .unwrap_err()
                .contains("open, closed or all")
        );
        assert_eq!(status("failed"), Ok(Status::Failed));
        assert!(status("x").unwrap_err().contains("queued"));
        assert_eq!(involvement(None), Ok(Involvement::Authored));
        assert_eq!(
            involvement(Some("review")),
            Ok(Involvement::ReviewRequested)
        );
        assert_eq!(
            involvement(Some("review_requested")),
            Ok(Involvement::ReviewRequested)
        );
        assert!(involvement(Some("x")).is_err());
        assert_eq!(merge_method(None), Ok(MergeMethod::Merge));
        assert_eq!(merge_method(Some("squash")), Ok(MergeMethod::Squash));
        assert!(merge_method(Some("ff")).unwrap_err().contains("squash"));
        assert_eq!(merge_timing(Some("queue")), Ok(MergeTiming::Queue));
        assert!(merge_timing(Some("later")).is_err());
        assert_eq!(update_style(None), Ok(UpdateStyle::Default));
        assert_eq!(update_style(Some("rebase")), Ok(UpdateStyle::Rebase));
        assert!(update_style(Some("x")).is_err());
        assert_eq!(verdict("approve"), Ok(ReviewVerdict::Approve));
        assert!(verdict("lgtm").unwrap_err().contains("request_changes"));
        assert_eq!(rerun_scope("all"), Ok(RerunScope::All));
        assert!(rerun_scope("some").is_err());
    }

    #[test]
    fn a_repository_is_named_in_full_short_or_by_default() {
        let default = RepoId::new("github.com", "me", "here");
        let r = |g: Option<&str>, hosts: &[&str]| resolve_repo(g, Some(&default), hosts, "none");
        assert_eq!(
            r(None, &[]).map(|x| x.key()),
            Ok("github.com/me/here".into())
        );
        assert_eq!(
            r(Some(" "), &[]).map(|x| x.key()),
            Ok("github.com/me/here".into())
        );
        assert_eq!(
            r(Some("codeberg.org/a/b"), &[]).map(|x| x.key()),
            Ok("codeberg.org/a/b".into())
        );
        assert_eq!(
            r(Some("a/b"), &["codeberg.org"]).map(|x| x.key()),
            Ok("codeberg.org/a/b".into()),
            "the only account"
        );
        assert_eq!(
            r(Some("a/b"), &["gitlab.com", "github.com"]).map(|x| x.key()),
            Ok("github.com/a/b".into()),
            "github.com wins among several"
        );
        assert!(r(Some("a/b"), &["gitlab.com", "codeberg.org"]).is_err());
        assert_eq!(
            r(Some("nonsense"), &["github.com"]),
            Err("'nonsense' is not host/owner/name".into())
        );
        assert_eq!(
            resolve_repo(None, None, &[], "pass -R"),
            Err("pass -R".into())
        );
    }

    #[test]
    fn a_log_is_tailed_or_searched_without_colour() {
        let raw = "one\n\u{1b}[31mERROR boom\u{1b}[0m\nthree\nfour";
        let t = log_excerpt(raw, 2, None, 200);
        assert_eq!(
            (t.total_lines, t.lines),
            (4, vec!["three".to_string(), "four".to_string()])
        );
        let g = log_excerpt(raw, 2, Some("error"), 200);
        assert_eq!(g.lines, vec!["2: ERROR boom".to_string()]);
        let capped = log_excerpt("a\na\na", 1, Some("a"), 2);
        assert_eq!(capped.lines.len(), 2);
        assert_eq!(
            log_excerpt("x", 0, None, 1).lines,
            vec!["x".to_string()],
            "tail is at least one"
        );
        assert_eq!(
            log_excerpt("x\ny", 5, Some(""), 1).lines.len(),
            2,
            "an empty search tails"
        );
    }

    #[test]
    fn a_one_off_section_carries_the_state() {
        let s = section(
            "cli",
            SectionKind::Prs,
            Some(Involvement::Assigned),
            StateFilter::Closed,
        );
        assert_eq!((s.name.as_str(), s.state), ("cli", SectionState::Closed));
        assert!(s.repos.is_empty() && !s.hidden);
    }
}
