//! Prompts: canned starting points a client can offer as slash commands. Each one only tells the
//! model which tools to call and what to produce; nothing here touches the network.

use rmcp::ErrorData as McpError;
use rmcp::model::{GetPromptResult, Prompt, PromptArgument, PromptMessage, Role};
use serde_json::{Map, Value};

use crate::server::invalid;

fn arg(name: &str, description: &str, required: bool) -> PromptArgument {
    PromptArgument::new(name)
        .with_description(description)
        .with_required(required)
}

fn repo_arg() -> PromptArgument {
    arg(
        "repo",
        "host/owner/name; default: the checkout the server runs in",
        false,
    )
}

/// The prompts the server offers.
pub(crate) fn catalog(write: bool) -> Vec<Prompt> {
    let mut v = vec![
        Prompt::new(
            "fix_failing_ci",
            Some("Find why CI is red and plan the fix"),
            Some(vec![
                repo_arg(),
                arg(
                    "branch",
                    "branch to look at; default: the PR's or the default branch",
                    false,
                ),
            ]),
        ),
        Prompt::new(
            "review_queue",
            Some("What needs my attention, most urgent first"),
            Some(vec![]),
        ),
        Prompt::new(
            "review_pull_request",
            Some("Review a pull request: summary, risks, what to look at first"),
            Some(vec![repo_arg(), arg("number", "pull request number", true)]),
        ),
        Prompt::new(
            "catch_up",
            Some("Summarise a PR or issue thread: decisions, open questions, what is asked of me"),
            Some(vec![
                repo_arg(),
                arg("number", "PR or issue number", true),
                arg("kind", "pr (default) or issue", false),
            ]),
        ),
        Prompt::new(
            "triage_issues",
            Some("Group open issues, spot duplicates and suggest labels"),
            Some(vec![repo_arg()]),
        ),
    ];
    if write {
        v.push(Prompt::new(
            "release_notes",
            Some("Draft release notes from the pull requests merged lately"),
            Some(vec![repo_arg()]),
        ));
    }
    v
}

fn text<'a>(args: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn number(args: &Map<String, Value>) -> Result<u64, McpError> {
    let v = args
        .get("number")
        .ok_or_else(|| invalid("number is required"))?;
    v.as_u64()
        .or_else(|| {
            v.as_str()
                .and_then(|s| s.trim().trim_start_matches('#').parse().ok())
        })
        .ok_or_else(|| invalid("number must be a positive integer"))
}

fn of(args: &Map<String, Value>) -> String {
    match text(args, "repo") {
        Some(r) => format!("the repository `{r}`"),
        None => "the current repository (leave `repo` out of tool calls)".into(),
    }
}

/// Renders one prompt.
pub(crate) fn render(
    name: &str,
    args: Option<&Map<String, Value>>,
    write: bool,
) -> Result<GetPromptResult, McpError> {
    let empty = Map::new();
    let args = args.unwrap_or(&empty);
    let repo_hint = text(args, "repo")
        .map(|r| format!(" Pass repo=`{r}` to every tool call."))
        .unwrap_or_default();
    let body = match name {
        "fix_failing_ci" => {
            let branch = text(args, "branch")
                .map(|b| format!(" on branch `{b}`"))
                .unwrap_or_default();
            format!(
                "Find out why CI fails in {}{branch}.\n\
                 1. `ci_runs` for the latest runs; take the newest failed one and `ci_run` for its jobs.\n\
                 2. `ci_job_log` for each failed job; read from the first failing step.\n\
                 3. If a pull request is involved, `get_pull_request` with the diff to see what changed.\n\
                 Then answer with: the root cause in one or two sentences, the file and line if the log \
                 gives them, a concrete fix plan as numbered steps, and whether the failure looks flaky \
                 (passes on a re-run) rather than caused by the change. Do not guess beyond the log.{repo_hint}",
                of(args)
            )
        }
        "review_queue" => "List what needs my attention.\n\
             1. `my_pull_requests` with involvement=review_requested, and again with involvement=authored.\n\
             2. `inbox` for unread notifications.\n\
             Order the result by urgency: reviews I owe that have waited longest, my PRs with failing \
             checks or conflicts or a branch behind its target, then mentions. One line each: \
             repo#number, title, why it needs me, and the next action."
            .to_string(),
        "review_pull_request" => format!(
            "Review pull request #{} in {}.\n\
             `get_pull_request` with include_diff=true, then `ci_runs` for its branch if checks are not green.\n\
             Answer with: what the change does in three sentences; the riskiest parts, each with file and \
             reason; missing tests or docs; questions for the author; and a verdict (approve, request \
             changes, comment). {}{repo_hint}",
            number(args)?,
            of(args),
            if write {
                "Show me the draft comments before posting anything with `review` or `comment`."
            } else {
                "The server is read-only, so only draft the comments."
            }
        ),
        "catch_up" => {
            let kind = text(args, "kind").unwrap_or("pr");
            format!(
                "Catch me up on {kind} #{} in {}.\n\
                 `get_{}` with its comments, then summarise: the goal, decisions made, what is still open, \
                 and what is being asked of me, newest first.{repo_hint}",
                number(args)?,
                of(args),
                if kind == "issue" { "issue" } else { "pull_request" },
            )
        }
        "triage_issues" => format!(
            "Triage the open issues of {}.\n\
             `list_issues` (state=open) and `list_labels`. Group the issues by theme, point out likely \
             duplicates, issues with no labels or no reply, and suggest labels from the existing set. \
             Do not change anything.{repo_hint}",
            of(args)
        ),
        "release_notes" if write => format!(
            "Draft release notes for {}.\n\
             `list_pull_requests` with state=closed, keep the merged ones since the last release, and \
             group them as Features, Fixes and Other, one line each with the PR number and author.{repo_hint}",
            of(args)
        ),
        other => return Err(invalid(format!("no prompt named '{other}'"))),
    };
    Ok(GetPromptResult::new(vec![PromptMessage::new_text(
        Role::User,
        body,
    )]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn args(v: Value) -> Map<String, Value> {
        v.as_object().cloned().unwrap_or_default()
    }

    fn body(r: &GetPromptResult) -> String {
        serde_json::to_string(&r.messages).unwrap_or_default()
    }

    #[test]
    fn every_listed_prompt_renders_with_its_required_arguments() {
        let given = args(json!({"number": 7, "repo": "github.com/o/r"}));
        for p in catalog(true) {
            let r = render(&p.name, Some(&given), true);
            assert!(r.is_ok(), "{} did not render", p.name);
        }
    }

    #[test]
    fn release_notes_exist_only_with_writes_on() {
        assert!(catalog(false).iter().all(|p| p.name != "release_notes"));
        assert!(render("release_notes", None, false).is_err());
        assert!(render("release_notes", None, true).is_ok());
    }

    #[test]
    fn a_missing_or_bad_number_is_an_error_and_hash_numbers_are_accepted() {
        assert!(render("review_pull_request", None, false).is_err());
        let bad = args(json!({"number": "x"}));
        assert!(render("catch_up", Some(&bad), false).is_err());
        let ok = args(json!({"number": "#12"}));
        assert!(body(&render("catch_up", Some(&ok), false).unwrap_or_default()).contains("#12"));
    }

    #[test]
    fn the_repo_is_passed_on_to_the_tool_calls_and_review_follows_the_write_mode() {
        let a = args(json!({"number": 3, "repo": "gitlab.com/g/p"}));
        let ro = body(&render("review_pull_request", Some(&a), false).unwrap_or_default());
        assert!(ro.contains("repo=`gitlab.com/g/p`") && ro.contains("read-only"));
        let rw = body(&render("review_pull_request", Some(&a), true).unwrap_or_default());
        assert!(rw.contains("before posting"));
    }

    #[test]
    fn unknown_prompts_are_refused() {
        assert!(render("nope", None, true).is_err());
    }
}
