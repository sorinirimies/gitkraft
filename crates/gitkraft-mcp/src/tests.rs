//! The tools against a fake platform: no network.

use gitkraft_hub::Hub;
use gitkraft_platforms::testing::FakeClient;
use gitkraft_platforms::{Account, DynClient, PlatformKind, RepoId, Secret};
use rmcp::handler::server::wrapper::Parameters;
use serde_json::Value;

use crate::Policy;
use crate::read::*;
use crate::server::GitkraftServer;
use crate::write::*;

const BASE: &str = "https://api.github.com/repos/o/r";

fn rig(policy: Policy) -> (GitkraftServer, FakeClient) {
    let fake = FakeClient::new();
    let mut account = Account::new(PlatformKind::GitHub, "github.com").with_token(Secret::new("t"));
    account.me = Some("me".into());
    let hub = Hub::new(vec![account], &DynClient::new(fake.clone()));
    let repo = Some(RepoId::new("github.com", "o", "r"));
    (GitkraftServer::new(hub, repo, policy), fake)
}

fn write() -> Policy {
    Policy {
        write: true,
        allow_force: false,
    }
}

fn json(s: &str) -> Value {
    serde_json::from_str(s).unwrap_or_else(|e| panic!("{e}: {s}"))
}

fn sent(fake: &FakeClient) -> Vec<(String, String)> {
    fake.requests()
        .into_iter()
        .map(|r| {
            (
                r.url,
                String::from_utf8_lossy(r.body.as_deref().unwrap_or_default()).into_owned(),
            )
        })
        .collect()
}

#[test]
fn read_only_servers_have_no_write_tools_and_write_servers_have_all() {
    let (ro, _) = rig(Policy::default());
    let names = ro.tool_names();
    for t in [
        "accounts",
        "my_pull_requests",
        "get_pull_request",
        "ci_runs",
        "ci_job_log",
        "pulse",
        "discussions",
        "wiki",
        "boards",
    ] {
        assert!(names.iter().any(|n| n == t), "{t} in {names:?}");
    }
    for t in [
        "comment",
        "review",
        "merge_pull_request",
        "set_open",
        "edit_labels",
        "ci_rerun",
        "ci_cancel",
        "dispatch_workflow",
        "update_branch",
        "comment_discussion",
    ] {
        assert!(
            !names.iter().any(|n| n == t),
            "{t} must not exist read-only"
        );
    }
    let (rw, _) = rig(write());
    let names = rw.tool_names();
    for t in [
        "comment",
        "review",
        "merge_pull_request",
        "update_branch",
        "cancel_auto_merge",
        "set_open",
        "edit_labels",
        "ci_rerun",
        "ci_cancel",
        "dispatch_workflow",
        "comment_discussion",
    ] {
        assert!(names.iter().any(|n| n == t), "{t} in {names:?}");
    }
}

#[tokio::test]
async fn repo_resolution_uses_the_default_a_short_form_or_refuses() {
    let (s, _) = rig(Policy::default());
    assert_eq!(
        s.repo(None).map(|r| r.key()).ok().as_deref(),
        Some("github.com/o/r")
    );
    assert_eq!(
        s.repo(Some("x/y")).map(|r| r.key()).ok().as_deref(),
        Some("github.com/x/y")
    );
    assert_eq!(
        s.repo(Some("codeberg.org/a/b"))
            .map(|r| r.key())
            .ok()
            .as_deref(),
        Some("codeberg.org/a/b")
    );
    assert!(s.repo(Some("nonsense")).is_err());
    let (hubless, _) = rig(Policy::default());
    let none = GitkraftServer::new(
        Hub::new(vec![], &DynClient::new(FakeClient::new())),
        None,
        Policy::default(),
    );
    assert!(none.repo(None).is_err());
    assert!(
        none.repo(Some("x/y")).is_err(),
        "no account to take the host from"
    );
    drop(hubless);
}

#[tokio::test]
async fn accounts_reports_the_policy_and_the_login() {
    let (s, _) = rig(write());
    let v = json(&s.accounts().await.expect("tool"));
    assert_eq!(v["accounts"][0]["host"], "github.com");
    assert_eq!(v["accounts"][0]["me"], "me");
    assert_eq!(v["write"], true);
    assert_eq!(v["allow_force"], false);
    assert_eq!(v["default_repo"], "github.com/o/r");
}

fn pr_json() -> &'static str {
    r#"{"number":7,"title":"Add pulse","body":"long text","state":"open","user":{"login":"alice","avatar_url":"a"},"head":{"ref":"feat","sha":"abc"},"base":{"ref":"master"},"additions":3,"deletions":1,"html_url":"u"}"#
}

#[tokio::test]
async fn lists_are_slim_and_details_carry_the_text() {
    let (s, fake) = rig(Policy::default());
    fake.push_json(&format!("{BASE}/pulls"), &format!("[{}]", pr_json()));
    let list = s
        .list_pull_requests(Parameters(ListArgs::default()))
        .await
        .expect("tool");
    let v = json(&list);
    assert_eq!(v["items"][0]["title"], "Add pulse");
    assert!(
        v["items"][0].get("body").is_none(),
        "lists drop descriptions: {list}"
    );
    assert!(!list.contains("avatar_url"));

    fake.push_json(&format!("{BASE}/pulls/7"), pr_json());
    fake.push_json(
        &format!("{BASE}/commits/abc/status"),
        r#"{"state":"success"}"#,
    );
    fake.push_json(
        &format!("{BASE}/issues/7/comments"),
        r#"[{"id":1,"body":"nice","user":{"login":"bob"}}]"#,
    );
    fake.push_json(
        &format!("{BASE}/pulls/7/reviews"),
        r#"[{"state":"APPROVED","user":{"login":"carol"}}]"#,
    );
    fake.push_status(
        &format!("{BASE}/pulls/7.diff"),
        200,
        "diff --git a/x b/x\n+new line",
    );
    let item = |diff| ItemArgs {
        repo: None,
        number: 7,
        include_diff: diff,
        max_diff_chars: Some(12),
    };
    let one = json(
        &s.get_pull_request(Parameters(item(None)))
            .await
            .expect("tool"),
    );
    assert_eq!(one["pull_request"]["body"], "long text");
    assert_eq!(one["comments"][0]["body"], "nice");
    assert!(one["diff"].is_null() && one["diff_chars"].as_u64() > Some(10));
}

#[tokio::test]
async fn a_diff_is_included_on_request_and_cut() {
    let (s, fake) = rig(Policy::default());
    fake.push_json(&format!("{BASE}/pulls/7"), pr_json());
    fake.push_json(
        &format!("{BASE}/commits/abc/status"),
        r#"{"state":"success"}"#,
    );
    fake.push_json(&format!("{BASE}/issues/7/comments"), "[]");
    fake.push_json(&format!("{BASE}/pulls/7/reviews"), "[]");
    fake.push_status(
        &format!("{BASE}/pulls/7.diff"),
        200,
        "diff --git a/x b/x\n+new line",
    );
    let v = json(
        &s.get_pull_request(Parameters(ItemArgs {
            repo: None,
            number: 7,
            include_diff: Some(true),
            max_diff_chars: Some(12),
        }))
        .await
        .expect("tool"),
    );
    assert_eq!(v["diff"], "diff --git a");
    assert_eq!(v["diff_truncated"], true);
}

#[tokio::test]
async fn bad_filters_are_invalid_params_before_any_call() {
    let (s, fake) = rig(Policy::default());
    let e = s
        .list_pull_requests(Parameters(ListArgs {
            state: Some("sideways".into()),
            ..ListArgs::default()
        }))
        .await
        .expect_err("refused");
    assert!(e.message.contains("state must be"), "{}", e.message);
    let e = s
        .ci_runs(Parameters(RunsArgs {
            status: Some("wat".into()),
            ..RunsArgs::default()
        }))
        .await
        .expect_err("refused");
    assert!(e.message.contains("status must be"));
    assert!(fake.requests().is_empty());
}

#[tokio::test]
async fn a_platform_failure_names_the_tool_and_the_reason() {
    let (s, fake) = rig(Policy::default());
    fake.push_status(&format!("{BASE}/pulls"), 404, "nope");
    let e = s
        .list_pull_requests(Parameters(ListArgs::default()))
        .await
        .expect_err("fails");
    assert!(
        e.message.starts_with("list_pull_requests:"),
        "{}",
        e.message
    );
}

#[tokio::test]
async fn the_job_log_is_clean_tailed_or_grepped() {
    let (s, fake) = rig(Policy::default());
    let log = "start\n\u{1b}[31merror: boom\u{1b}[0m\nmid\nend";
    fake.push_status(&format!("{BASE}/actions/jobs/11/logs"), 200, log);
    fake.push_status(&format!("{BASE}/actions/jobs/11/logs"), 200, log);
    let args = |tail, grep: Option<&str>| LogArgs {
        repo: None,
        job_id: 11,
        tail_lines: tail,
        grep: grep.map(Into::into),
    };
    let tail = json(
        &s.ci_job_log(Parameters(args(Some(2), None)))
            .await
            .expect("tool"),
    );
    assert_eq!(tail["total_lines"], 4);
    assert_eq!(tail["lines"], serde_json::json!(["mid", "end"]));
    let grep = json(
        &s.ci_job_log(Parameters(args(None, Some("ERROR"))))
            .await
            .expect("tool"),
    );
    assert_eq!(grep["lines"], serde_json::json!(["2: error: boom"]));
}

#[tokio::test]
async fn writes_reach_the_platform_with_what_was_asked() {
    let (s, fake) = rig(write());
    fake.push_json(&format!("{BASE}/issues/7/comments"), "{}");
    s.comment(Parameters(CommentArgs {
        repo: None,
        kind: "pr".into(),
        number: 7,
        body: "ship it".into(),
    }))
    .await
    .expect("comment");
    fake.push_json(&format!("{BASE}/pulls/7/merge"), r#"{"merged":true}"#);
    let m = s
        .merge_pull_request(Parameters(MergeArgs {
            repo: None,
            number: 7,
            method: Some("squash".into()),
            when: None,
            force: None,
        }))
        .await
        .expect("merge");
    assert!(
        json(&m)["message"]
            .as_str()
            .is_some_and(|t| t.contains("merged")),
        "{m}"
    );
    let sent = sent(&fake);
    assert!(sent[0].1.contains("ship it"));
    assert!(sent[1].1.contains("squash"), "{sent:?}");
}

#[tokio::test]
async fn a_force_merge_is_refused_without_allow_force_and_nothing_is_sent() {
    let (s, fake) = rig(write());
    let e = s
        .merge_pull_request(Parameters(MergeArgs {
            repo: None,
            number: 7,
            method: None,
            when: None,
            force: Some(true),
        }))
        .await
        .expect_err("refused");
    assert!(e.message.contains("--allow-force"), "{}", e.message);
    assert!(fake.requests().is_empty());

    let forced = GitkraftServer::new(
        Hub::new(vec![], &DynClient::new(FakeClient::new())),
        None,
        Policy {
            write: true,
            allow_force: true,
        },
    );
    // allowed by policy: it then fails on the missing account, not on the policy
    let e = forced
        .merge_pull_request(Parameters(MergeArgs {
            repo: Some("github.com/o/r".into()),
            number: 7,
            method: None,
            when: None,
            force: Some(true),
        }))
        .await
        .expect_err("no account");
    assert!(e.message.contains("no account"), "{}", e.message);
}

#[tokio::test]
async fn write_arguments_are_validated_first() {
    let (s, fake) = rig(write());
    let bad =
        |e: rmcp::ErrorData, needle: &str| assert!(e.message.contains(needle), "{}", e.message);
    bad(
        s.comment(Parameters(CommentArgs {
            repo: None,
            kind: "pr".into(),
            number: 1,
            body: "  ".into(),
        }))
        .await
        .expect_err("empty"),
        "empty",
    );
    bad(
        s.review(Parameters(ReviewArgs {
            repo: None,
            number: 1,
            verdict: "request_changes".into(),
            body: None,
        }))
        .await
        .expect_err("needs body"),
        "needs a body",
    );
    bad(
        s.merge_pull_request(Parameters(MergeArgs {
            repo: None,
            number: 1,
            method: Some("ff".into()),
            when: None,
            force: None,
        }))
        .await
        .expect_err("method"),
        "method must be",
    );
    bad(
        s.edit_labels(Parameters(LabelsArgs {
            repo: None,
            kind: "pr".into(),
            number: 1,
            add: vec![],
            remove: vec![],
        }))
        .await
        .expect_err("nothing"),
        "labels",
    );
    bad(
        s.ci_rerun(Parameters(RerunArgs {
            repo: None,
            run_id: 1,
            scope: Some("some".into()),
        }))
        .await
        .expect_err("scope"),
        "scope must be",
    );
    assert!(fake.requests().is_empty());
}

#[tokio::test]
async fn activity_day_groups_the_day_and_refuses_days_outside_the_calendar() {
    let (s, fake) = rig(Policy::default());
    let today = chrono::Utc::now().date_naive();
    fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"user":{"contributionsCollection":{"contributionCalendar":{"totalContributions":2},"commitContributionsByRepository":[{"repository":{"nameWithOwner":"o/r","url":"u"},"contributions":{"totalCount":2}}],"pullRequestContributions":{"nodes":[]},"issueContributions":{"nodes":[]},"pullRequestReviewContributions":{"nodes":[]}}}}}"#,
    );
    let v = json(
        &s.activity_day(Parameters(DayArgs { date: None }))
            .await
            .expect("tool"),
    );
    assert_eq!(v["headline"], format!("2 contributions on {today}"));
    assert_eq!(v["groups"][0]["items"][0]["repo"], "github.com/o/r");
    let before = fake.requests().len();
    for bad in ["soon", "2999-01-01", "2001-01-01"] {
        assert!(
            s.activity_day(Parameters(DayArgs {
                date: Some(bad.into())
            }))
            .await
            .is_err(),
            "{bad}"
        );
    }
    assert_eq!(fake.requests().len(), before);
}

#[tokio::test]
async fn discussions_list_slim_and_read_with_comments_and_wiki_says_why_not() {
    let (s, fake) = rig(Policy::default());
    fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"repository":{"hasDiscussionsEnabled":true,"discussions":{"nodes":[{"id":"D","number":5,"title":"Q","url":"u","body":"long text","updatedAt":"2026-05-01T10:00:00Z","isAnswered":false,"category":{"name":"Q&A"},"author":{"login":"a","avatarUrl":"x"},"commentCount":{"totalCount":2}}]}}}}"#,
    );
    let v = json(
        &s.discussions(Parameters(DiscussionsArgs::default()))
            .await
            .expect("tool"),
    );
    assert_eq!(v["discussions"][0]["title"], "Q");
    assert!(v["discussions"][0].get("body").is_none(), "lists are slim");
    let err = s
        .wiki(Parameters(WikiArgs::default()))
        .await
        .expect_err("github has no wiki api");
    assert!(err.message.contains(".wiki.git"), "{err:?}");
}

#[tokio::test]
async fn boards_come_back_as_columns_of_cards() {
    let (s, fake) = rig(Policy::default());
    fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"repository":{"projectsV2":{"nodes":[{"title":"Roadmap","field":{"options":[{"name":"Todo"}]},"items":{"nodes":[{"fieldValueByName":{"name":"Todo"},"content":{"number":4,"title":"Add it","url":"u","assignees":{"nodes":[]}}}]}}]}}}}"#,
    );
    let v = json(
        &s.boards(Parameters(RepoArgs::default()))
            .await
            .expect("tool"),
    );
    assert_eq!(v["boards"][0]["name"], "Roadmap");
    assert_eq!(v["boards"][0]["columns"][0]["cards"][0]["number"], 4);
}
