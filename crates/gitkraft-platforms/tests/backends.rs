//! Adapter tests against recorded responses; nothing touches the network.

use gitkraft_platforms::http::Method;
use gitkraft_platforms::model::{
    ActivityKind, Board, BoardCard, BoardColumn, ChecksState, ItemState, ListQuery, MergeMethod,
    MergeOptions, MergeOutcome, MergeTiming, NotificationReason, ReviewVerdict, StateFilter,
    SubjectKind, UpdateStyle,
};
use gitkraft_platforms::testing::FakeClient;
use gitkraft_platforms::{Account, Backend, ItemKind, PlatformError, PlatformKind, RepoId, Secret};

fn github(fake: &FakeClient) -> Backend<FakeClient> {
    let account = Account::new(PlatformKind::GitHub, "github.com").with_token(Secret::new("tok"));
    Backend::new(&account, fake.clone())
}
fn gitea(fake: &FakeClient) -> Backend<FakeClient> {
    Backend::new(
        &Account::new(PlatformKind::Gitea, "codeberg.org"),
        fake.clone(),
    )
}
fn gitlab(fake: &FakeClient) -> Backend<FakeClient> {
    Backend::new(
        &Account::new(PlatformKind::GitLab, "gitlab.com"),
        fake.clone(),
    )
}
fn repo() -> RepoId {
    RepoId::new("github.com", "o", "r")
}

#[tokio::test]
async fn github_lists_pull_requests_with_normalized_fields() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/repos/o/r/pulls",
        include_str!("fixtures/github_pulls.json"),
    );
    let prs = github(&fake)
        .list_pull_requests(&repo(), &ListQuery::default())
        .await
        .unwrap_or_default();
    assert_eq!(prs.len(), 2);
    let first = &prs[0];
    assert_eq!((first.number, first.title.as_str()), (12, "Add pulse view"));
    assert!(first.draft);
    assert_eq!(first.state, ItemState::Open);
    assert_eq!(first.author.login, "alice");
    assert_eq!(
        (first.source_branch.as_str(), first.target_branch.as_str()),
        ("feat/pulse", "master")
    );
    assert_eq!(first.labels[0].name, "enhancement");
    assert_eq!(first.reviewers[0].login, "carol");
    assert_eq!(prs[1].state, ItemState::Merged);
    let req = &fake.requests()[0];
    assert!(req.url.contains("state=open&per_page=50&page=1"));
    assert!(
        req.headers
            .contains(&("Authorization".into(), "Bearer tok".into()))
    );
}

#[tokio::test]
async fn pull_request_list_filters_by_author_client_side() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/repos/o/r/pulls",
        include_str!("fixtures/github_pulls.json"),
    );
    let q = ListQuery {
        author: Some("BOB".into()),
        state: StateFilter::All,
        ..ListQuery::default()
    };
    let prs = github(&fake)
        .list_pull_requests(&repo(), &q)
        .await
        .unwrap_or_default();
    assert_eq!(prs.iter().map(|p| p.number).collect::<Vec<_>>(), vec![11]);
}

#[tokio::test]
async fn github_issue_list_drops_pull_requests() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/repos/o/r/issues",
        include_str!("fixtures/github_issues.json"),
    );
    let q = ListQuery {
        author: Some("dave".into()),
        ..ListQuery::default()
    };
    let issues = github(&fake)
        .list_issues(&repo(), &q)
        .await
        .unwrap_or_default();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].milestone.as_deref(), Some("v1"));
    assert_eq!(issues[0].body, "");
    assert!(fake.requests()[0].url.contains("creator=dave"));
}

#[tokio::test]
async fn gitea_issue_list_uses_its_own_query_names() {
    let fake = FakeClient::new();
    fake.push_json("https://codeberg.org/api/v1/repos/o/r/issues", "[]");
    let q = ListQuery {
        author: Some("a".into()),
        assignee: Some("b".into()),
        ..ListQuery::default()
    };
    let _ = gitea(&fake)
        .list_issues(&RepoId::new("codeberg.org", "o", "r"), &q)
        .await;
    let url = &fake.requests()[0].url;
    assert!(
        url.contains("type=issues")
            && url.contains("created_by=a")
            && url.contains("assigned_by=b"),
        "{url}"
    );
}

#[tokio::test]
async fn single_pull_request_carries_check_state() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/repos/o/r/pulls/12",
        r#"{"number":12,"title":"t","state":"open","head":{"ref":"a","sha":"abc"},"base":{"ref":"m"},"additions":10,"deletions":2,"mergeable":false,"html_url":"u"}"#,
    );
    fake.push_json(
        "https://api.github.com/repos/o/r/commits/abc/status",
        r#"{"state":"failure"}"#,
    );
    let pr = github(&fake)
        .pull_request(&repo(), 12)
        .await
        .expect("pull request");
    assert_eq!(pr.checks, Some(ChecksState::Failure));
    assert_eq!(
        (pr.additions, pr.deletions, pr.mergeable),
        (Some(10), Some(2), Some(false))
    );
}

#[tokio::test]
async fn actions_send_the_right_requests() {
    let fake = FakeClient::new();
    for url in [
        "https://api.github.com/repos/o/r/issues/12/comments",
        "https://api.github.com/repos/o/r/issues/12",
        "https://api.github.com/repos/o/r/pulls/12/merge",
        "https://api.github.com/repos/o/r/pulls/12/reviews",
    ] {
        fake.push_json(url, "{}");
    }
    let b = github(&fake);
    let r = repo();
    assert!(
        b.post_comment(&r, ItemKind::PullRequest, 12, "hi")
            .await
            .is_ok()
    );
    assert!(
        b.set_open(&r, ItemKind::PullRequest, 12, false)
            .await
            .is_ok()
    );
    assert!(b.merge(&r, 12, MergeMethod::Squash).await.is_ok());
    assert!(
        b.submit_review(&r, 12, ReviewVerdict::Approve, "lgtm")
            .await
            .is_ok()
    );
    let sent = fake.requests();
    let body = |i: usize| {
        String::from_utf8_lossy(sent[i].body.as_deref().unwrap_or_default()).into_owned()
    };
    assert_eq!(body(0), r#"{"body":"hi"}"#);
    assert_eq!(body(1), r#"{"state":"closed"}"#);
    assert_eq!(body(2), r#"{"merge_method":"squash"}"#);
    assert!(body(3).contains("APPROVE"));
}

#[tokio::test]
async fn gitea_merge_uses_do_field() {
    let fake = FakeClient::new();
    fake.push_json("https://codeberg.org/api/v1/repos/o/r/pulls/1/merge", "{}");
    let b = gitea(&fake);
    assert!(
        b.merge(
            &RepoId::new("codeberg.org", "o", "r"),
            1,
            MergeMethod::Rebase
        )
        .await
        .is_ok()
    );
    let sent = &fake.requests()[0];
    assert_eq!(
        String::from_utf8_lossy(sent.body.as_deref().unwrap_or_default()),
        r#"{"Do":"rebase"}"#
    );
}

#[tokio::test]
async fn github_notifications_map_reason_kind_and_url() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/notifications",
        include_str!("fixtures/github_notifications.json"),
    );
    let n = github(&fake).notifications().await.unwrap_or_default();
    assert_eq!(n.len(), 2);
    assert_eq!(n[0].reason, NotificationReason::ReviewRequested);
    assert_eq!(
        (n[0].kind, n[0].number, n[0].unread),
        (SubjectKind::PullRequest, Some(12), true)
    );
    assert_eq!(n[0].url, "https://github.com/o/r/pull/12");
    assert_eq!(
        n[0].repo.as_ref().map(RepoId::key),
        Some("github.com/o/r".into())
    );
    assert_eq!(n[1].reason, NotificationReason::CiActivity);
    assert_eq!(n[1].kind, SubjectKind::Other);
}

#[tokio::test]
async fn gitea_notifications_use_numeric_ids_and_html_url() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://codeberg.org/api/v1/notifications",
        include_str!("fixtures/gitea_notifications.json"),
    );
    let n = gitea(&fake).notifications().await.unwrap_or_default();
    assert_eq!(
        (n[0].id.as_str(), n[0].number, n[0].kind),
        ("9", Some(4), SubjectKind::Issue)
    );
    assert_eq!(n[0].url, "https://codeberg.org/o/r/issues/4");
}

#[tokio::test]
async fn gitea_heatmap_sums_per_day() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://codeberg.org/api/v1/users/me/heatmap",
        include_str!("fixtures/gitea_heatmap.json"),
    );
    let days = gitea(&fake).contributions("me").await.unwrap_or_default();
    assert_eq!(days.iter().map(|d| d.count).collect::<Vec<_>>(), vec![5, 1]);
    assert!(days[0].date < days[1].date);
}

#[tokio::test]
async fn github_contributions_use_graphql_and_need_a_token() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/graphql",
        include_str!("fixtures/github_graphql.json"),
    );
    let days = github(&fake).contributions("me").await.unwrap_or_default();
    assert_eq!(
        days.iter().map(|d| d.count).collect::<Vec<_>>(),
        vec![4, 0, 7]
    );

    let anon = Backend::new(
        &Account::new(PlatformKind::GitHub, "github.com"),
        fake.clone(),
    );
    assert!(matches!(
        anon.contributions("me").await,
        Err(PlatformError::Auth(_))
    ));
}

#[tokio::test]
async fn gitlab_merge_requests_map_state_draft_and_pipeline() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://gitlab.com/api/v4/projects/g%2Fs%2Fp/merge_requests",
        include_str!("fixtures/gitlab_mrs.json"),
    );
    let r = RepoId::new("gitlab.com", "g/s", "p");
    let q = ListQuery {
        state: StateFilter::Closed,
        ..ListQuery::default()
    };
    let prs = gitlab(&fake)
        .list_pull_requests(&r, &q)
        .await
        .unwrap_or_default();
    // `Closed` keeps merged, drops opened.
    assert_eq!(
        prs.iter().map(|p| (p.number, p.state)).collect::<Vec<_>>(),
        vec![(2, ItemState::Merged)]
    );

    let q = ListQuery {
        state: StateFilter::All,
        ..ListQuery::default()
    };
    let all = gitlab(&fake)
        .list_pull_requests(&r, &q)
        .await
        .unwrap_or_default();
    assert!(all[0].draft);
    assert_eq!(all[0].checks, Some(ChecksState::Failure));
    assert_eq!(all[0].mergeable, Some(false));
    assert_eq!(all[0].labels[0].name, "ui");
}

#[tokio::test]
async fn gitlab_unsupported_actions_say_so() {
    let fake = FakeClient::new();
    let b = gitlab(&fake);
    let r = RepoId::new("gitlab.com", "g", "p");
    assert!(matches!(
        b.merge(&r, 1, MergeMethod::Rebase).await,
        Err(PlatformError::Unsupported(_))
    ));
    assert!(matches!(
        b.submit_review(&r, 1, ReviewVerdict::RequestChanges, "")
            .await,
        Err(PlatformError::Unsupported(_))
    ));
    assert!(fake.requests().is_empty());
}

#[tokio::test]
async fn gitlab_todos_and_calendar() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://gitlab.com/api/v4/todos",
        include_str!("fixtures/gitlab_todos.json"),
    );
    fake.push_json(
        "https://gitlab.com/users/me/calendar.json",
        include_str!("fixtures/gitlab_calendar.json"),
    );
    let b = gitlab(&fake);
    let n = b.notifications().await.unwrap_or_default();
    assert_eq!(n[0].reason, NotificationReason::CiActivity);
    assert_eq!(n[0].repo.as_ref().map(|r| r.owner.as_str()), Some("g/s"));
    let days = b.contributions("me").await.unwrap_or_default();
    assert_eq!(days.iter().map(|d| d.count).collect::<Vec<_>>(), vec![1, 3]);
}

#[tokio::test]
async fn gitlab_comments_skip_system_notes() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://gitlab.com/api/v4/projects/g%2Fp/issues/4/notes",
        r#"[{"id":1,"body":"assigned","system":true},{"id":2,"body":"real","system":false,"author":{"username":"x"}}]"#,
    );
    let c = gitlab(&fake)
        .comments(&RepoId::new("gitlab.com", "g", "p"), ItemKind::Issue, 4)
        .await
        .unwrap_or_default();
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].body, "real");
}

#[tokio::test]
async fn http_errors_surface_as_typed_errors() {
    let fake = FakeClient::new();
    fake.push_status("https://api.github.com/user", 401, "Bad credentials");
    fake.push_status("https://api.github.com/repos/o/r/pulls/9", 404, "Not Found");
    let b = github(&fake);
    assert!(matches!(
        b.current_user().await,
        Err(PlatformError::Auth(_))
    ));
    assert!(matches!(
        b.pull_request(&repo(), 9).await,
        Err(PlatformError::NotFound(_))
    ));
}

#[tokio::test]
async fn malformed_json_is_a_parse_error_not_a_panic() {
    let fake = FakeClient::new();
    fake.push_json("https://api.github.com/user", "<html>");
    assert!(matches!(
        github(&fake).current_user().await,
        Err(PlatformError::Parse(_))
    ));
}

use gitkraft_settings::dashboard::Involvement;

#[tokio::test]
async fn github_search_finds_prs_across_repos() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/search/issues",
        r#"{"items":[
          {"number":4,"title":"Mine","state":"open","user":{"login":"me"},"repository_url":"https://api.github.com/repos/acme/app","pull_request":{"merged_at":null},"draft":true,"html_url":"u4"},
          {"number":9,"title":"Done","state":"closed","user":{"login":"me"},"repository_url":"https://api.github.com/repos/acme/lib","pull_request":{"merged_at":"2026-01-01T00:00:00Z"},"html_url":"u9"},
          {"number":2,"title":"An issue","state":"open","repository_url":"https://api.github.com/repos/acme/app","html_url":"u2"}]}"#,
    );
    let prs = github(&fake)
        .search_pull_requests(Involvement::ReviewRequested, StateFilter::All, "me")
        .await
        .unwrap_or_default();
    assert_eq!(prs.len(), 2, "issues are not PRs");
    assert_eq!(prs[0].repo.key(), "github.com/acme/app");
    assert!(prs[0].draft);
    assert_eq!(prs[1].state, ItemState::Merged);
    let url = &fake.requests()[0].url;
    assert!(
        url.contains("q=is:pr+review-requested:@me") && !url.contains("is:open"),
        "{url}"
    );
}

#[tokio::test]
async fn github_issue_search_drops_prs() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/search/issues",
        r#"{"items":[{"number":2,"title":"i","state":"open","repository_url":"https://api.github.com/repos/a/b","html_url":"u"},{"number":3,"title":"p","state":"open","repository_url":"https://api.github.com/repos/a/b","pull_request":{},"html_url":"u"}]}"#,
    );
    let issues = github(&fake)
        .search_issues(Involvement::Assigned, StateFilter::Open, "me")
        .await
        .unwrap_or_default();
    assert_eq!(issues.iter().map(|i| i.number).collect::<Vec<_>>(), vec![2]);
    assert!(
        fake.requests()[0]
            .url
            .contains("q=is:issue+is:open+assignee:@me")
    );
}

#[tokio::test]
async fn gitea_search_uses_the_flag_per_involvement() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://codeberg.org/api/v1/repos/issues/search",
        r#"[{"number":1,"title":"t","state":"open","user":{"login":"me"},"repository":{"full_name":"o/r"},"pull_request":{"merged":true},"html_url":"u"}]"#,
    );
    let prs = gitea(&fake)
        .search_pull_requests(Involvement::Authored, StateFilter::Open, "me")
        .await
        .unwrap_or_default();
    assert_eq!(
        (prs[0].repo.key().as_str(), prs[0].state),
        ("codeberg.org/o/r", ItemState::Merged)
    );
    let url = &fake.requests()[0].url;
    assert!(
        url.contains("type=pulls") && url.contains("created=true") && url.contains("state=open"),
        "{url}"
    );
}

#[tokio::test]
async fn gitlab_search_derives_the_project_from_references() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://gitlab.com/api/v4/merge_requests",
        r#"[{"iid":3,"title":"t","state":"opened","references":{"full":"grp/sub/proj!3"},"web_url":"u"},{"iid":4,"title":"x","state":"opened","web_url":"u"}]"#,
    );
    let prs = gitlab(&fake)
        .search_pull_requests(Involvement::ReviewRequested, StateFilter::Open, "erin")
        .await
        .unwrap_or_default();
    assert_eq!(prs.len(), 1, "no reference, no project, skipped");
    assert_eq!(
        (prs[0].repo.owner.as_str(), prs[0].repo.name.as_str()),
        ("grp/sub", "proj")
    );
    let url = &fake.requests()[0].url;
    assert!(
        url.contains("reviewer_username=erin") && url.contains("state=opened"),
        "{url}"
    );
}

fn new(path: &str, line: u32) -> gitkraft_platforms::diffpos::Anchor {
    gitkraft_platforms::diffpos::Anchor {
        path: path.into(),
        line,
        side: gitkraft_platforms::diffpos::Side::New,
    }
}

fn old(path: &str, line: u32) -> gitkraft_platforms::diffpos::Anchor {
    gitkraft_platforms::diffpos::Anchor {
        side: gitkraft_platforms::diffpos::Side::Old,
        ..new(path, line)
    }
}

#[tokio::test]
async fn comments_on_removed_lines_use_the_old_side_on_each_platform() {
    let body = |req: &gitkraft_platforms::http::Request| -> serde_json::Value {
        serde_json::from_slice(req.body.as_deref().unwrap_or_default()).expect("json")
    };
    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/repos/o/r/pulls/3",
        r#"{"number":3,"head":{"ref":"f","sha":"abc"},"base":{"ref":"main"}}"#,
    );
    fake.push_json("https://api.github.com/repos/o/r/pulls/3/comments", "{}");
    github(&fake)
        .comment_on_line(&repo(), 3, &old("a.rs", 7), "gone?")
        .await
        .expect("github");
    let v = body(&fake.requests()[1]);
    assert_eq!(
        (v["side"].as_str(), v["line"].as_u64()),
        (Some("LEFT"), Some(7))
    );

    let fake = FakeClient::new();
    fake.push_json(
        "https://codeberg.org/api/v1/repos/o/r/pulls/3",
        r#"{"number":3,"head":{"ref":"f","sha":"d"},"base":{"ref":"main"}}"#,
    );
    fake.push_json(
        "https://codeberg.org/api/v1/repos/o/r/pulls/3/reviews",
        "{}",
    );
    gitea(&fake)
        .comment_on_line(&repo(), 3, &old("a.rs", 7), "gone?")
        .await
        .expect("gitea");
    let v = body(&fake.requests()[1]);
    assert_eq!(v["comments"][0]["old_position"], 7);
    assert!(v["comments"][0].get("new_position").is_none());

    let fake = FakeClient::new();
    fake.push_json(
        "https://gitlab.com/api/v4/projects/g%2Fp/merge_requests/3",
        r#"{"iid":3,"diff_refs":{"base_sha":"b","start_sha":"s","head_sha":"h"}}"#,
    );
    fake.push_json(
        "https://gitlab.com/api/v4/projects/g%2Fp/merge_requests/3/discussions",
        "{}",
    );
    gitlab(&fake)
        .comment_on_line(
            &RepoId::new("gitlab.com", "g", "p"),
            3,
            &old("x.rs", 7),
            "gone?",
        )
        .await
        .expect("gitlab");
    let v = body(&fake.requests()[1]);
    assert_eq!(v["position"]["old_line"], 7);
    assert!(v["position"].get("new_line").is_none());
}

#[tokio::test]
async fn inline_comments_anchor_to_the_head_commit_on_each_platform() {
    let body = |req: &gitkraft_platforms::http::Request| {
        String::from_utf8_lossy(req.body.as_deref().unwrap_or_default()).into_owned()
    };

    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/repos/o/r/pulls/3",
        r#"{"number":3,"head":{"ref":"f","sha":"abc123"},"base":{"ref":"main"}}"#,
    );
    fake.push_json("https://api.github.com/repos/o/r/pulls/3/comments", "{}");
    github(&fake)
        .comment_on_line(&repo(), 3, &new("src/a.rs", 12), "nit")
        .await
        .expect("github");
    let sent = fake.requests();
    assert_eq!(
        sent[1].url,
        "https://api.github.com/repos/o/r/pulls/3/comments"
    );
    let v: serde_json::Value = serde_json::from_str(&body(&sent[1])).expect("json");
    assert_eq!(
        (
            v["commit_id"].as_str(),
            v["path"].as_str(),
            v["line"].as_u64(),
            v["side"].as_str()
        ),
        (Some("abc123"), Some("src/a.rs"), Some(12), Some("RIGHT"))
    );

    let fake = FakeClient::new();
    fake.push_json(
        "https://codeberg.org/api/v1/repos/o/r/pulls/3",
        r#"{"number":3,"head":{"ref":"f","sha":"def456"},"base":{"ref":"main"}}"#,
    );
    fake.push_json(
        "https://codeberg.org/api/v1/repos/o/r/pulls/3/reviews",
        "{}",
    );
    gitea(&fake)
        .comment_on_line(&repo(), 3, &new("a.rs", 4), "why")
        .await
        .expect("gitea");
    let v: serde_json::Value = serde_json::from_str(&body(&fake.requests()[1])).expect("json");
    assert_eq!(v["event"], "COMMENT");
    assert_eq!(v["comments"][0]["new_position"], 4);
    assert_eq!(v["comments"][0]["path"], "a.rs");

    let fake = FakeClient::new();
    fake.push_json(
        "https://gitlab.com/api/v4/projects/g%2Fp/merge_requests/3",
        r#"{"iid":3,"diff_refs":{"base_sha":"b","start_sha":"s","head_sha":"h"}}"#,
    );
    fake.push_json(
        "https://gitlab.com/api/v4/projects/g%2Fp/merge_requests/3/discussions",
        "{}",
    );
    let r = RepoId::new("gitlab.com", "g", "p");
    gitlab(&fake)
        .comment_on_line(&r, 3, &new("x.rs", 9), "hm")
        .await
        .expect("gitlab");
    let v: serde_json::Value = serde_json::from_str(&body(&fake.requests()[1])).expect("json");
    assert_eq!(
        (
            v["position"]["head_sha"].as_str(),
            v["position"]["new_line"].as_u64(),
            v["position"]["new_path"].as_str()
        ),
        (Some("h"), Some(9), Some("x.rs"))
    );
}

#[tokio::test]
async fn inline_comments_explain_a_missing_head_commit() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/repos/o/r/pulls/3",
        r#"{"number":3}"#,
    );
    assert!(matches!(
        github(&fake)
            .comment_on_line(&repo(), 3, &new("a", 1), "x")
            .await,
        Err(PlatformError::Unsupported(_))
    ));
}

#[tokio::test]
async fn labels_are_added_and_removed_by_name_or_id_depending_on_the_platform() {
    use gitkraft_platforms::ItemKind;
    let names = |v: &[&str]| v.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();

    let fake = FakeClient::new();
    fake.push_json("https://api.github.com/repos/o/r/issues/3/labels", "[]");
    fake.push_json(
        "https://api.github.com/repos/o/r/issues/3/labels/good%20first",
        "[]",
    );
    github(&fake)
        .edit_labels(
            &repo(),
            ItemKind::Issue,
            3,
            &names(&["bug"]),
            &names(&["good first"]),
        )
        .await
        .expect("github");
    let sent = fake.requests();
    assert_eq!(
        String::from_utf8_lossy(sent[0].body.as_deref().unwrap_or_default()),
        r#"{"labels":["bug"]}"#
    );
    assert_eq!(sent[1].method, gitkraft_platforms::http::Method::Delete);
    assert!(sent[1].url.ends_with("/labels/good%20first"));

    let fake = FakeClient::new();
    fake.push_json(
        "https://codeberg.org/api/v1/repos/o/r/labels",
        r#"[{"id":11,"name":"bug"},{"id":12,"name":"wontfix"}]"#,
    );
    fake.push_json(
        "https://codeberg.org/api/v1/repos/o/r/issues/3/labels/12",
        "[]",
    );
    gitea(&fake)
        .edit_labels(&repo(), ItemKind::Issue, 3, &[], &names(&["wontfix"]))
        .await
        .expect("gitea");
    assert!(
        fake.requests()
            .last()
            .expect("sent")
            .url
            .ends_with("/issues/3/labels/12")
    );
    assert!(matches!(
        gitea(&fake)
            .edit_labels(&repo(), ItemKind::Issue, 3, &[], &names(&["nope"]))
            .await,
        Err(PlatformError::NotFound(_))
    ));

    let fake = FakeClient::new();
    fake.push_json(
        "https://gitlab.com/api/v4/projects/g%2Fp/merge_requests/3",
        "{}",
    );
    gitlab(&fake)
        .edit_labels(
            &RepoId::new("gitlab.com", "g", "p"),
            ItemKind::PullRequest,
            3,
            &names(&["a", "b"]),
            &names(&["c"]),
        )
        .await
        .expect("gitlab");
    let v: serde_json::Value =
        serde_json::from_slice(fake.requests()[0].body.as_deref().unwrap_or_default())
            .expect("json");
    assert_eq!(
        (v["add_labels"].as_str(), v["remove_labels"].as_str()),
        (Some("a,b"), Some("c"))
    );

    fake.push_json(
        "https://gitlab.com/api/v4/projects/g%2Fp/labels",
        r#"[{"name":"x"}]"#,
    );
    assert_eq!(
        gitlab(&fake)
            .list_labels(&RepoId::new("gitlab.com", "g", "p"))
            .await
            .expect("labels"),
        ["x"]
    );
}

fn body_of(req: &gitkraft_platforms::http::Request) -> serde_json::Value {
    serde_json::from_slice(req.body.as_deref().unwrap_or_default()).unwrap_or_default()
}

fn opts(timing: MergeTiming, force: bool) -> MergeOptions {
    MergeOptions {
        method: MergeMethod::Squash,
        timing,
        force,
    }
}

#[tokio::test]
async fn github_merges_now_schedules_auto_merge_and_queues_through_graphql() {
    let fake = FakeClient::new();
    let pr = "https://api.github.com/repos/o/r/pulls/5";
    fake.push_json(&format!("{pr}/merge"), "{}");
    fake.push_json(pr, r#"{"node_id":"PR_x"}"#);
    fake.push_json("https://api.github.com/graphql", r#"{"data":{}}"#);
    fake.push_json(pr, r#"{"node_id":"PR_x"}"#);
    fake.push_json("https://api.github.com/graphql", r#"{"data":{}}"#);
    let b = github(&fake);

    assert_eq!(
        b.merge_with(&repo(), 5, &opts(MergeTiming::Now, true))
            .await,
        Ok(MergeOutcome::Merged)
    );
    assert_eq!(
        b.merge_with(&repo(), 5, &opts(MergeTiming::WhenGreen, false))
            .await,
        Ok(MergeOutcome::Scheduled)
    );
    assert_eq!(
        b.merge_with(&repo(), 5, &opts(MergeTiming::Queue, true))
            .await,
        Ok(MergeOutcome::Queued)
    );
    let sent = fake.requests();
    assert_eq!(
        body_of(&sent[0]),
        serde_json::json!({"merge_method": "squash"})
    );
    let auto = body_of(&sent[2]);
    assert!(
        auto["query"]
            .as_str()
            .is_some_and(|q| q.contains("enablePullRequestAutoMerge"))
    );
    assert_eq!(
        auto["variables"],
        serde_json::json!({"id": "PR_x", "m": "SQUASH"})
    );
    let queue = body_of(&sent[4]);
    assert!(
        queue["query"]
            .as_str()
            .is_some_and(|q| q.contains("enqueuePullRequest"))
    );
    assert_eq!(
        queue["variables"],
        serde_json::json!({"id": "PR_x", "jump": true})
    );
}

#[tokio::test]
async fn graphql_errors_inside_a_200_are_reported() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/repos/o/r/pulls/5",
        r#"{"node_id":"PR_x"}"#,
    );
    fake.push_json(
        "https://api.github.com/graphql",
        r#"{"errors":[{"message":"Pull request is not in the correct state to enable auto-merge"}]}"#,
    );
    let err = github(&fake)
        .merge_with(&repo(), 5, &opts(MergeTiming::WhenGreen, false))
        .await
        .expect_err("refused");
    assert!(
        err.to_string().contains("not in the correct state"),
        "{err}"
    );
}

#[tokio::test]
async fn a_refused_force_merge_says_what_force_needs_and_nonsense_is_rejected() {
    let fake = FakeClient::new();
    fake.push_status(
        "https://api.github.com/repos/o/r/pulls/5/merge",
        405,
        "Required status check is expected",
    );
    let b = github(&fake);
    let err = b
        .merge_with(&repo(), 5, &opts(MergeTiming::Now, true))
        .await
        .expect_err("refused");
    assert!(err.to_string().contains("admin rights"), "{err}");
    assert!(matches!(
        b.merge_with(&repo(), 5, &opts(MergeTiming::WhenGreen, true))
            .await,
        Err(PlatformError::Unsupported(_))
    ));
}

#[tokio::test]
async fn enterprise_graphql_lives_under_api_graphql() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://ghe.corp/api/v3/repos/o/r/pulls/5",
        r#"{"node_id":"PR_x"}"#,
    );
    fake.push_json("https://ghe.corp/api/graphql", r#"{"data":{}}"#);
    let account = Account::new(PlatformKind::GitHub, "ghe.corp");
    let b = Backend::new(&account, fake.clone());
    b.merge_with(
        &RepoId::new("ghe.corp", "o", "r"),
        5,
        &opts(MergeTiming::Queue, false),
    )
    .await
    .expect("queued");
    assert_eq!(fake.requests()[1].url, "https://ghe.corp/api/graphql");
}

#[tokio::test]
async fn gitea_force_schedule_and_update_use_their_own_fields() {
    let fake = FakeClient::new();
    let base = "https://codeberg.org/api/v1/repos/o/r/pulls/2";
    fake.push_json(&format!("{base}/merge"), "{}");
    fake.push_json(&format!("{base}/merge"), "{}");
    fake.push_json(&format!("{base}/update?style=rebase"), "{}");
    fake.push_json(&format!("{base}/update?style=merge"), "{}");
    fake.push_json(&format!("{base}/merge"), "{}");
    let r = RepoId::new("codeberg.org", "o", "r");
    let b = gitea(&fake);

    assert_eq!(
        b.merge_with(&r, 2, &opts(MergeTiming::Now, true)).await,
        Ok(MergeOutcome::Merged)
    );
    assert_eq!(
        b.merge_with(&r, 2, &opts(MergeTiming::WhenGreen, false))
            .await,
        Ok(MergeOutcome::Scheduled)
    );
    assert!(b.update_branch(&r, 2, UpdateStyle::Rebase).await.is_ok());
    assert!(b.update_branch(&r, 2, UpdateStyle::Default).await.is_ok());
    assert!(b.cancel_auto_merge(&r, 2).await.is_ok());
    assert!(matches!(
        b.merge_with(&r, 2, &opts(MergeTiming::Queue, true)).await,
        Err(PlatformError::Unsupported(_))
    ));
    let sent = fake.requests();
    assert_eq!(
        body_of(&sent[0]),
        serde_json::json!({"Do": "squash", "force_merge": true})
    );
    assert_eq!(
        body_of(&sent[1]),
        serde_json::json!({"Do": "squash", "merge_when_checks_succeed": true})
    );
    assert_eq!(sent[4].method, gitkraft_platforms::http::Method::Delete);
}

#[tokio::test]
async fn gitlab_schedules_queues_rebases_and_cancels() {
    let fake = FakeClient::new();
    let mr = "https://gitlab.com/api/v4/projects/g%2Fp/merge_requests/3";
    fake.push_json(&format!("{mr}/merge"), r#"{"state":"merged"}"#);
    fake.push_json(&format!("{mr}/merge"), r#"{"state":"opened"}"#);
    fake.push_json(&format!("{mr}/merge"), r#"{"state":"opened"}"#);
    fake.push_json(&format!("{mr}/rebase"), r#"{"rebase_in_progress":true}"#);
    fake.push_json(&format!("{mr}/cancel_merge_when_pipeline_succeeds"), "{}");
    let r = RepoId::new("gitlab.com", "g", "p");
    let b = gitlab(&fake);

    assert_eq!(
        b.merge_with(&r, 3, &opts(MergeTiming::Now, false)).await,
        Ok(MergeOutcome::Merged)
    );
    assert_eq!(
        b.merge_with(&r, 3, &opts(MergeTiming::WhenGreen, false))
            .await,
        Ok(MergeOutcome::Scheduled)
    );
    assert_eq!(
        b.merge_with(&r, 3, &opts(MergeTiming::Queue, false)).await,
        Ok(MergeOutcome::Queued)
    );
    assert!(b.update_branch(&r, 3, UpdateStyle::Default).await.is_ok());
    assert!(b.cancel_auto_merge(&r, 3).await.is_ok());
    assert!(matches!(
        b.update_branch(&r, 3, UpdateStyle::Merge).await,
        Err(PlatformError::Unsupported(_))
    ));
    assert!(matches!(
        b.merge_with(&r, 3, &opts(MergeTiming::Now, true)).await,
        Err(PlatformError::Unsupported(_))
    ));
    let sent = fake.requests();
    assert_eq!(body_of(&sent[0]), serde_json::json!({"squash": true}));
    assert_eq!(body_of(&sent[1])["merge_when_pipeline_succeeds"], true);
    assert_eq!(sent[3].method, gitkraft_platforms::http::Method::Put);
}

#[tokio::test]
async fn github_updates_its_branch_and_cancels_auto_merge_falling_back_to_dequeue() {
    let fake = FakeClient::new();
    let pr = "https://api.github.com/repos/o/r/pulls/5";
    fake.push_json(
        &format!("{pr}/update-branch"),
        r#"{"message":"Updating pull request branch."}"#,
    );
    fake.push_json(pr, r#"{"node_id":"PR_x"}"#);
    fake.push_json(
        "https://api.github.com/graphql",
        r#"{"errors":[{"message":"not auto-merging"}]}"#,
    );
    fake.push_json("https://api.github.com/graphql", r#"{"data":{}}"#);
    let b = github(&fake);
    assert!(
        b.update_branch(&repo(), 5, UpdateStyle::Default)
            .await
            .is_ok()
    );
    assert!(matches!(
        b.update_branch(&repo(), 5, UpdateStyle::Rebase).await,
        Err(PlatformError::Unsupported(_))
    ));
    assert!(b.cancel_auto_merge(&repo(), 5).await.is_ok());
    let sent = fake.requests();
    assert!(
        body_of(&sent[3])["query"]
            .as_str()
            .is_some_and(|q| q.contains("dequeuePullRequest"))
    );
}

fn day(y: i32, m: u32, d: u32) -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(y, m, d).expect("date")
}

#[tokio::test]
async fn github_day_activity_asks_for_that_day_and_groups_what_was_done() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"user":{"contributionsCollection":{
          "contributionCalendar":{"totalContributions":6},
          "commitContributionsByRepository":[
            {"repository":{"nameWithOwner":"o/r","url":"https://github.com/o/r"},"contributions":{"totalCount":3}},
            {"repository":{"nameWithOwner":"o/none","url":"u"},"contributions":{"totalCount":0}}],
          "pullRequestContributions":{"nodes":[{"pullRequest":{"number":7,"title":"Add pulse","url":"https://github.com/o/r/pull/7","repository":{"nameWithOwner":"o/r"}}}]},
          "issueContributions":{"nodes":[{"issue":{"number":2,"title":"Bug","url":"i","repository":{"nameWithOwner":"o/r"}}}]},
          "pullRequestReviewContributions":{"nodes":[{"pullRequest":{"number":9,"title":"Theirs","url":"p","repository":{"nameWithOwner":"x/y"}}}]}}}}}"#,
    );
    let a = github(&fake)
        .day_activity("me", day(2026, 3, 10))
        .await
        .expect("activity");
    let body = String::from_utf8_lossy(fake.requests()[0].body.as_deref().unwrap_or_default())
        .into_owned();
    assert!(
        body.contains("2026-03-10T00:00:00Z") && body.contains("2026-03-10T23:59:59Z"),
        "{body}"
    );
    assert_eq!(a.total, 6);
    assert_eq!(a.headline(), "6 contributions on 2026-03-10");
    let groups = a.groups();
    assert_eq!(
        groups.iter().map(|g| (g.0, g.1)).collect::<Vec<_>>(),
        [
            (ActivityKind::Commits, 3),
            (ActivityKind::PullRequest, 1),
            (ActivityKind::Issue, 1),
            (ActivityKind::Review, 1)
        ],
        "a repo with no commits is left out"
    );
    assert_eq!(a.entries[1].number, Some(7));
    assert_eq!(a.entries[1].repo, "o/r");
}

#[tokio::test]
async fn github_day_activity_needs_a_token_and_a_user() {
    let fake = FakeClient::new();
    let anon = Backend::new(
        &Account::new(PlatformKind::GitHub, "github.com"),
        fake.clone(),
    );
    assert!(matches!(
        anon.day_activity("me", day(2026, 3, 10)).await,
        Err(PlatformError::Auth(_))
    ));
    fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"user":null}}"#,
    );
    assert!(matches!(
        github(&fake).day_activity("ghost", day(2026, 3, 10)).await,
        Err(PlatformError::NotFound(_))
    ));
}

#[tokio::test]
async fn gitea_day_activity_reads_the_feed_of_that_day() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://codeberg.org/api/v1/users/me/activities/feeds?only-performed-by=true&date=2026-03-10&limit=50",
        r#"[
          {"op_type":"commit_repo","ref_name":"refs/heads/main","content":"{\"Len\":4}","repo":{"full_name":"o/r"}},
          {"op_type":"create_pull_request","content":"12|Add thing","repo":{"full_name":"o/r"}},
          {"op_type":"close_issue","content":"3|Old","repo":{"full_name":"o/r"}},
          {"op_type":"approve_pull_request","content":"5|Theirs","repo":{"full_name":"x/y"}},
          {"op_type":"comment_issue","content":"3|Old","repo":{"full_name":"o/r"}},
          {"op_type":"create_repo","content":"","repo":{"full_name":"o/new"}}]"#,
    );
    let a = gitea(&fake)
        .day_activity("me", day(2026, 3, 10))
        .await
        .expect("activity");
    assert_eq!(a.total, 9);
    let kinds: Vec<_> = a.entries.iter().map(|e| e.kind).collect();
    assert_eq!(
        kinds,
        [
            ActivityKind::Commits,
            ActivityKind::PullRequest,
            ActivityKind::Issue,
            ActivityKind::Review,
            ActivityKind::Comment,
            ActivityKind::Other
        ]
    );
    assert_eq!(a.entries[0].title, "4 commits to main");
    assert_eq!(a.entries[1].url, "https://codeberg.org/o/r/pulls/12");
    assert_eq!(a.entries[2].url, "https://codeberg.org/o/r/issues/3");
}

#[tokio::test]
async fn gitlab_day_activity_resolves_projects_once_and_skips_empty_pushes() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://gitlab.com/api/v4/users?username=me",
        r#"[{"id":42}]"#,
    );
    fake.push_json(
        "https://gitlab.com/api/v4/users/42/events?after=2026-03-09&before=2026-03-11&per_page=100",
        r#"[
          {"project_id":5,"action_name":"pushed to","push_data":{"commit_count":2,"ref":"main"}},
          {"project_id":5,"action_name":"pushed to","push_data":{"commit_count":0,"ref":"gone"}},
          {"project_id":5,"action_name":"opened","target_type":"MergeRequest","target_iid":8,"target_title":"Add x"},
          {"project_id":5,"action_name":"approved","target_type":"MergeRequest","target_iid":9,"target_title":"Theirs"},
          {"project_id":5,"action_name":"opened","target_type":"Issue","target_iid":3,"target_title":"Bug"},
          {"project_id":5,"action_name":"commented on","target_type":"Note","note":{"noteable_type":"MergeRequest","noteable_iid":8},"target_title":"hi"}]"#,
    );
    fake.push_json(
        "https://gitlab.com/api/v4/projects/5",
        r#"{"path_with_namespace":"g/s"}"#,
    );
    let a = gitlab(&fake)
        .day_activity("me", day(2026, 3, 10))
        .await
        .expect("activity");
    assert_eq!(a.total, 6);
    assert_eq!(fake.requests().len(), 3, "the project is looked up once");
    assert_eq!(a.entries[0].title, "2 commits to main");
    assert_eq!(a.entries[0].repo, "g/s");
    assert_eq!(
        a.entries[1].url,
        "https://gitlab.com/g/s/-/merge_requests/8"
    );
    assert_eq!(a.entries[2].kind, ActivityKind::Review);
    assert_eq!(a.entries[3].url, "https://gitlab.com/g/s/-/issues/3");
    assert_eq!(a.entries[4].kind, ActivityKind::Comment);
    assert_eq!(
        a.entries[4].url,
        "https://gitlab.com/g/s/-/merge_requests/8"
    );
}

#[test]
fn headlines_are_singular_for_one_and_say_so_for_none() {
    use gitkraft_platforms::model::DayActivity;
    let mut d = DayActivity::empty(day(2026, 3, 10));
    assert_eq!(d.headline(), "No contributions on 2026-03-10");
    d.total = 1;
    assert_eq!(d.headline(), "1 contribution on 2026-03-10");
    assert!(d.groups().is_empty());
}

#[tokio::test]
async fn github_discussions_list_read_and_comment_through_graphql() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"repository":{"hasDiscussionsEnabled":true,"discussions":{"nodes":[
            {"id":"D_1","number":5,"title":"How do I clone?","url":"https://github.com/o/r/discussions/5",
             "body":"help","updatedAt":"2026-05-01T10:00:00Z","isAnswered":true,
             "category":{"name":"Q&A"},"author":{"login":"alice","avatarUrl":null},"commentCount":{"totalCount":3}},
            {"id":"D_2","number":6,"title":"Gone author","url":"u","body":"","updatedAt":"2026-05-02T10:00:00Z",
             "isAnswered":false,"category":{"name":"Ideas"},"author":null,"commentCount":{"totalCount":0}}]}}}}"#,
    );
    let list = github(&fake)
        .discussions(&repo(), 30)
        .await
        .unwrap_or_default();
    assert_eq!(list.len(), 2);
    assert_eq!(
        (
            list[0].number,
            list[0].category.as_str(),
            list[0].answered,
            list[0].comments
        ),
        (5, "Q&A", true, 3)
    );
    assert_eq!(
        list[1].author.login, "ghost",
        "a deleted account reads as ghost"
    );

    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"repository":{"discussion":{"id":"D_1","number":5,"title":"How do I clone?","url":"u","body":"help",
            "updatedAt":"2026-05-01T10:00:00Z","isAnswered":true,"category":{"name":"Q&A"},
            "author":{"login":"alice"},"commentCount":{"totalCount":1},"comments":{"nodes":[
              {"databaseId":9,"body":"git clone","createdAt":"2026-05-01T11:00:00Z","author":{"login":"bob"}}]}}}}}"#,
    );
    let d = github(&fake)
        .discussion(&repo(), 5)
        .await
        .unwrap_or_default();
    assert_eq!(d.comments.len(), 1);
    assert_eq!(
        (
            d.comments[0].author.login.as_str(),
            d.comments[0].body.as_str()
        ),
        ("bob", "git clone")
    );

    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"repository":{"discussion":{"id":"D_1"}}}}"#,
    );
    fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"addDiscussionComment":{"comment":{"id":"C_1"}}}}"#,
    );
    assert!(
        github(&fake)
            .comment_on_discussion(&repo(), 5, "thanks")
            .await
            .is_ok()
    );
    let body = fake
        .requests()
        .last()
        .and_then(|r| r.body.clone())
        .unwrap_or_default();
    let sent = String::from_utf8_lossy(&body).into_owned();
    assert!(
        sent.contains("addDiscussionComment") && sent.contains("thanks") && sent.contains("D_1")
    );
}

#[tokio::test]
async fn discussions_are_a_github_thing_and_wikis_are_not() {
    let fake = FakeClient::new();
    assert!(matches!(
        gitea(&fake).discussions(&repo(), 10).await,
        Err(PlatformError::Unsupported(_))
    ));
    assert!(matches!(
        gitlab(&fake).discussion(&repo(), 1).await,
        Err(PlatformError::Unsupported(_))
    ));
    let err = github(&fake)
        .wiki_pages(&repo())
        .await
        .err()
        .map(|e| e.to_string());
    assert!(err.unwrap_or_default().contains(".wiki.git"));
    assert!(
        fake.requests().is_empty(),
        "nothing was sent for an unsupported call"
    );
}

#[tokio::test]
async fn gitea_wiki_pages_and_content_decode_base64() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://codeberg.org/api/v1/repos/o/r/wiki/pages",
        r#"[{"title":"Home","sub_url":"Home"},{"title":"Setup guide","sub_url":"Setup-guide"}]"#,
    );
    let pages = gitea(&fake).wiki_pages(&repo()).await.unwrap_or_default();
    assert_eq!(
        pages.iter().map(|p| p.slug.as_str()).collect::<Vec<_>>(),
        ["Home", "Setup-guide"]
    );
    // "# Hello\nworld" in base64, wrapped over two lines like some servers do
    fake.push_json(
        "https://codeberg.org/api/v1/repos/o/r/wiki/page/Home",
        r##"{"title":"Home","sub_url":"Home","content_base64":"IyBIZWxs\nbwp3b3JsZA=="}"##,
    );
    let page = gitea(&fake)
        .wiki_page(&repo(), "Home")
        .await
        .unwrap_or_default();
    assert_eq!(page.content, "# Hello\nworld");
}

#[tokio::test]
async fn gitlab_wiki_pages_and_content() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://gitlab.com/api/v4/projects/o%2Fr/wikis",
        r#"[{"title":"Home","slug":"home","format":"markdown"},{"title":"Dev/Setup","slug":"dev/setup","format":"markdown"}]"#,
    );
    let pages = gitlab(&fake).wiki_pages(&repo()).await.unwrap_or_default();
    assert_eq!(pages[1].slug, "dev/setup");
    fake.push_json(
        "https://gitlab.com/api/v4/projects/o%2Fr/wikis/dev%2Fsetup",
        r##"{"title":"Dev/Setup","slug":"dev/setup","content":"# Setup"}"##,
    );
    let page = gitlab(&fake)
        .wiki_page(&repo(), "dev/setup")
        .await
        .unwrap_or_default();
    assert_eq!(page.content, "# Setup");
}

#[tokio::test]
async fn github_projects_become_boards_with_the_status_order_and_unreadable_cards_skipped() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"repository":{"projectsV2":{"nodes":[{"title":"Roadmap",
          "field":{"options":[{"name":"Todo"},{"name":"Doing"},{"name":"Done"}]},
          "items":{"nodes":[
            {"fieldValueByName":{"name":"Doing"},"content":{"number":3,"title":"Fix it","url":"u3","assignees":{"nodes":[{"login":"alice"}]}}},
            {"fieldValueByName":{"name":"Todo"},"content":{"number":4,"title":"Add it","url":"u4","assignees":{"nodes":[]}}},
            {"fieldValueByName":null,"content":{"title":"A draft"}},
            {"fieldValueByName":{"name":"Todo"},"content":{}}]}}]}}}}"#,
    );
    let boards = github(&fake).boards(&repo()).await.unwrap_or_default();
    assert_eq!(boards[0].name, "Roadmap");
    let names: Vec<_> = boards[0].columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["Todo", "Doing", "Done", "No status"]);
    assert_eq!(boards[0].columns[1].cards[0].assignees, ["alice"]);
    assert_eq!(
        boards[0].columns[0].cards.len(),
        1,
        "the unreadable card is skipped"
    );
    assert_eq!(
        boards[0].columns[3].cards[0].number, None,
        "a draft has no number"
    );
}

#[tokio::test]
async fn gitlab_boards_put_issues_in_the_column_of_their_list_label() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://gitlab.com/api/v4/projects/o%2Fr/boards",
        r#"[{"id":1,"name":"Development","lists":[{"label":{"name":"doing"}},{"label":{"name":"review"}}]}]"#,
    );
    fake.push_json(
        "https://gitlab.com/api/v4/projects/o%2Fr/issues",
        r#"[{"iid":1,"title":"a","web_url":"u1","labels":["doing"],"assignees":[{"username":"bob"}]},
            {"iid":2,"title":"b","web_url":"u2","labels":["bug"],"assignees":[]},
            {"iid":3,"title":"c","web_url":"u3","labels":["review","bug"],"assignees":[]}]"#,
    );
    let boards = gitlab(&fake).boards(&repo()).await.unwrap_or_default();
    let col = |n: &str| {
        boards[0]
            .columns
            .iter()
            .find(|c| c.name == n)
            .map(|c| c.cards.iter().filter_map(|c| c.number).collect::<Vec<_>>())
    };
    assert_eq!(col("Open"), Some(vec![2]));
    assert_eq!(col("doing"), Some(vec![1]));
    assert_eq!(col("review"), Some(vec![3]));
    // no board configured: nothing, and the issues are not even fetched
    let fake = FakeClient::new();
    fake.push_json("https://gitlab.com/api/v4/projects/o%2Fr/boards", "[]");
    assert!(
        gitlab(&fake)
            .boards(&repo())
            .await
            .unwrap_or_default()
            .is_empty()
    );
    assert_eq!(fake.requests().len(), 1);
    // Gitea has no API for them
    assert!(matches!(
        gitea(&FakeClient::new()).boards(&repo()).await,
        Err(PlatformError::Unsupported(_))
    ));
}

#[tokio::test]
async fn a_pull_request_knows_when_its_branch_is_behind_its_target() {
    let pr = |state: &str| {
        let fake = FakeClient::new();
        fake.push_json(
            "https://api.github.com/repos/o/r/pulls/12",
            &format!(
                r#"{{"number":12,"title":"t","state":"open","head":{{"ref":"a","sha":"abc"}},"base":{{"ref":"m"}},"mergeable_state":"{state}","html_url":"u"}}"#
            ),
        );
        fake.push_json(
            "https://api.github.com/repos/o/r/commits/abc/status",
            r#"{"state":"success"}"#,
        );
        async move {
            github(&fake)
                .pull_request(&repo(), 12)
                .await
                .map(|p| p.behind)
        }
    };
    assert_eq!(pr("behind").await.ok(), Some(Some(true)));
    assert_eq!(pr("clean").await.ok(), Some(Some(false)));
    assert_eq!(
        pr("unknown").await.ok(),
        Some(None),
        "unknown is not an answer"
    );

    let mr = |body: &str| {
        let fake = FakeClient::new();
        fake.push_json(
            "https://gitlab.com/api/v4/projects/g%2Fp/merge_requests/3",
            &format!(r#"{{"iid":3,"title":"t","state":"opened","source_branch":"a","target_branch":"m",{body}}}"#),
        );
        async move {
            gitlab(&fake)
                .pull_request(&RepoId::new("gitlab.com", "g", "p"), 3)
                .await
                .map(|p| p.behind)
        }
    };
    assert_eq!(
        mr(r#""diverged_commits_count":2"#).await.ok(),
        Some(Some(true))
    );
    assert_eq!(
        mr(r#""diverged_commits_count":0"#).await.ok(),
        Some(Some(false))
    );
    assert_eq!(
        mr(r#""detailed_merge_status":"need_rebase""#).await.ok(),
        Some(Some(true))
    );
    assert_eq!(mr(r#""draft":false"#).await.ok(), Some(None));
}

#[tokio::test]
async fn an_actions_only_commit_is_judged_by_its_check_runs_not_the_empty_commit_status() {
    let state = |status: &str, runs: &str| {
        let fake = FakeClient::new();
        fake.push_json(
            "https://api.github.com/repos/o/r/pulls/12",
            r#"{"number":12,"title":"t","state":"open","head":{"ref":"a","sha":"abc"},"base":{"ref":"m"},"html_url":"u"}"#,
        );
        fake.push_json(
            "https://api.github.com/repos/o/r/commits/abc/status",
            status,
        );
        fake.push_json(
            "https://api.github.com/repos/o/r/commits/abc/check-runs",
            runs,
        );
        async move {
            github(&fake)
                .pull_request(&repo(), 12)
                .await
                .map(|p| p.checks)
        }
    };
    let none = r#"{"state":"pending","total_count":0}"#;
    let run =
        |s: &str, c: &str| format!(r#"{{"check_runs":[{{"status":"{s}","conclusion":{c}}}]}}"#);
    assert_eq!(
        state(none, &run("completed", "\"success\"")).await.ok(),
        Some(Some(ChecksState::Success))
    );
    assert_eq!(
        state(none, &run("completed", "\"skipped\"")).await.ok(),
        Some(Some(ChecksState::Success))
    );
    assert_eq!(
        state(none, &run("completed", "\"failure\"")).await.ok(),
        Some(Some(ChecksState::Failure))
    );
    assert_eq!(
        state(none, &run("in_progress", "null")).await.ok(),
        Some(Some(ChecksState::Pending))
    );
    assert_eq!(
        state(none, r#"{"check_runs":[]}"#).await.ok(),
        Some(None),
        "no checks at all is not pending"
    );
    // a real failing status wins over green runs
    assert_eq!(
        state(
            r#"{"state":"failure","total_count":1}"#,
            &run("completed", "\"success\"")
        )
        .await
        .ok(),
        Some(Some(ChecksState::Failure))
    );
}

fn sent_bodies(fake: &FakeClient) -> Vec<String> {
    fake.requests()
        .iter()
        .map(|r| String::from_utf8_lossy(r.body.as_deref().unwrap_or_default()).into_owned())
        .collect()
}

#[tokio::test]
async fn a_discussion_thread_reads_as_comment_then_replies_and_a_reply_goes_to_its_thread() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"repository":{"discussion":{"id":"D_1","number":5,"title":"t","url":"u","body":"b",
            "updatedAt":"2026-05-01T10:00:00Z","isAnswered":false,"category":{"name":"Q&A"},
            "author":{"login":"alice"},"commentCount":{"totalCount":3},"comments":{"nodes":[
              {"databaseId":9,"id":"C_9","body":"top","createdAt":"2026-05-01T11:00:00Z","author":{"login":"bob"},
               "replies":{"nodes":[{"databaseId":10,"body":"re","createdAt":"2026-05-01T12:00:00Z","author":{"login":"carol"}}]}},
              {"databaseId":11,"id":"C_11","body":"other","createdAt":"2026-05-01T13:00:00Z","author":{"login":"dave"},"replies":{"nodes":[]}}]}}}}}"#,
    );
    let d = github(&fake).discussion(&repo(), 5).await.expect("detail");
    let shape: Vec<(u64, Option<u64>)> = d.comments.iter().map(|c| (c.id, c.parent)).collect();
    assert_eq!(shape, vec![(9, None), (10, Some(9)), (11, None)]);

    // Answering the reply (10) goes to its thread's top-level comment (C_9).
    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"repository":{"discussion":{"id":"D_1","comments":{"nodes":[
            {"databaseId":9,"id":"C_9","replies":{"nodes":[{"databaseId":10}]}},
            {"databaseId":11,"id":"C_11","replies":{"nodes":[]}}]}}}}}"#,
    );
    fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"addDiscussionComment":{"comment":{"id":"C_new"}}}}"#,
    );
    github(&fake)
        .reply_to_discussion(&repo(), 5, 10, "thanks")
        .await
        .expect("reply");
    let sent = sent_bodies(&fake).pop().unwrap_or_default();
    assert!(
        sent.contains("replyToId") && sent.contains("\"to\":\"C_9\""),
        "{sent}"
    );
    assert!(
        sent.contains("\"id\":\"D_1\"") && sent.contains("thanks"),
        "{sent}"
    );

    // A comment the discussion does not have, and a platform without discussions.
    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"repository":{"discussion":{"id":"D_1","comments":{"nodes":[]}}}}}"#,
    );
    let err = github(&fake)
        .reply_to_discussion(&repo(), 5, 99, "x")
        .await
        .expect_err("unknown comment");
    assert!(err.to_string().contains("comment 99"), "{err}");
    assert!(matches!(
        gitlab(&FakeClient::new())
            .reply_to_discussion(&repo(), 5, 1, "x")
            .await,
        Err(PlatformError::Unsupported(_))
    ));
}

#[tokio::test]
async fn wiki_pages_are_created_and_edited_on_gitea_and_gitlab_not_github() {
    let fake = FakeClient::new();
    fake.push_json("https://codeberg.org/api/v1/repos/o/r/wiki/new", "{}");
    fake.push_json("https://codeberg.org/api/v1/repos/o/r/wiki/page/Home", "{}");
    let g = gitea(&fake);
    g.save_wiki_page(&repo(), None, "Home", "# Hi", "create")
        .await
        .expect("create");
    g.save_wiki_page(&repo(), Some("Home"), "Home", "# Hi 2", "edit")
        .await
        .expect("edit");
    let reqs = fake.requests();
    assert_eq!(reqs[0].method, Method::Post);
    assert_eq!(reqs[1].method, Method::Patch);
    let sent = sent_bodies(&fake);
    assert!(
        sent[0].contains("\"title\":\"Home\"") && sent[0].contains("IyBIaQ=="),
        "{}",
        sent[0]
    );
    assert!(sent[1].contains("\"message\":\"edit\""));

    let fake = FakeClient::new();
    fake.push_json("https://gitlab.com/api/v4/projects/o%2Fr/wikis", "{}");
    fake.push_json("https://gitlab.com/api/v4/projects/o%2Fr/wikis/home", "{}");
    let l = gitlab(&fake);
    l.save_wiki_page(&repo(), None, "Home", "# Hi", "")
        .await
        .expect("create");
    l.save_wiki_page(&repo(), Some("home"), "Home", "# Hi 2", "")
        .await
        .expect("edit");
    let reqs = fake.requests();
    assert_eq!(
        (reqs[0].method, reqs[1].method),
        (Method::Post, Method::Put)
    );
    assert!(sent_bodies(&fake)[0].contains("\"format\":\"markdown\""));

    assert!(matches!(
        github(&FakeClient::new())
            .save_wiki_page(&repo(), None, "t", "c", "m")
            .await,
        Err(PlatformError::Unsupported(_))
    ));
}

fn two_column_board() -> Board {
    let col = |name: &str, id: &str| BoardColumn {
        name: name.into(),
        option_id: Some(id.into()),
        ..BoardColumn::default()
    };
    Board {
        name: "Roadmap".into(),
        columns: vec![col("Todo", "O1"), col("Doing", "O2"), col("Done", "O3")],
        project_id: Some("P".into()),
        field_id: Some("F".into()),
    }
}

#[tokio::test]
async fn a_github_card_moves_by_setting_its_status_and_a_gitlab_issue_by_swapping_list_labels() {
    let board = two_column_board();
    let card = BoardCard {
        number: Some(7),
        title: "t".into(),
        item_id: Some("I".into()),
        ..BoardCard::default()
    };
    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"updateProjectV2ItemFieldValue":{"projectV2Item":{"id":"I"}}}}"#,
    );
    github(&fake)
        .move_card(&repo(), &board, &card, &board.columns[1])
        .await
        .expect("move");
    let sent = sent_bodies(&fake).pop().unwrap_or_default();
    for want in ["\"p\":\"P\"", "\"i\":\"I\"", "\"f\":\"F\"", "\"o\":\"O2\""] {
        assert!(sent.contains(want), "{want} in {sent}");
    }
    // Without the ids (an old load) it refuses instead of guessing.
    let bare = BoardCard {
        item_id: None,
        ..card.clone()
    };
    assert!(
        github(&FakeClient::new())
            .move_card(&repo(), &board, &bare, &board.columns[1])
            .await
            .is_err()
    );

    let mut lboard = two_column_board();
    lboard.columns.insert(
        0,
        BoardColumn {
            name: "Open".into(),
            ..BoardColumn::default()
        },
    );
    let fake = FakeClient::new();
    fake.push_json("https://gitlab.com/api/v4/projects/o%2Fr/issues/7", "{}");
    gitlab(&fake)
        .move_card(&repo(), &lboard, &card, &lboard.columns[2])
        .await
        .expect("move");
    let sent = sent_bodies(&fake).pop().unwrap_or_default();
    assert!(sent.contains("\"add_labels\":\"Doing\""), "{sent}");
    assert!(sent.contains("\"remove_labels\":\"Todo,Done\""), "{sent}");
    // Into "Open": every list label comes off and none goes on.
    let fake = FakeClient::new();
    fake.push_json("https://gitlab.com/api/v4/projects/o%2Fr/issues/7", "{}");
    gitlab(&fake)
        .move_card(&repo(), &lboard, &card, &lboard.columns[0])
        .await
        .expect("move");
    let sent = sent_bodies(&fake).pop().unwrap_or_default();
    assert!(
        sent.contains("\"add_labels\":\"\"") && sent.contains("Todo,Doing,Done"),
        "{sent}"
    );
    // A draft card has no issue to label.
    let draft = BoardCard {
        number: None,
        ..card
    };
    assert!(
        gitlab(&FakeClient::new())
            .move_card(&repo(), &lboard, &draft, &lboard.columns[1])
            .await
            .is_err()
    );
}
