//! Adapter tests against recorded responses; nothing touches the network.

use gitkraft_platforms::model::{
    ChecksState, ItemState, ListQuery, MergeMethod, NotificationReason, ReviewVerdict, StateFilter,
    SubjectKind,
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
    assert!(url.contains("q=is:pr+review-requested:@me") && !url.contains("is:open"), "{url}");
}

#[tokio::test]
async fn github_issue_search_drops_prs() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/search/issues",
        r#"{"items":[{"number":2,"title":"i","state":"open","repository_url":"https://api.github.com/repos/a/b","html_url":"u"},{"number":3,"title":"p","state":"open","repository_url":"https://api.github.com/repos/a/b","pull_request":{},"html_url":"u"}]}"#,
    );
    let issues = github(&fake).search_issues(Involvement::Assigned, StateFilter::Open, "me").await.unwrap_or_default();
    assert_eq!(issues.iter().map(|i| i.number).collect::<Vec<_>>(), vec![2]);
    assert!(fake.requests()[0].url.contains("q=is:issue+is:open+assignee:@me"));
}

#[tokio::test]
async fn gitea_search_uses_the_flag_per_involvement() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://codeberg.org/api/v1/repos/issues/search",
        r#"[{"number":1,"title":"t","state":"open","user":{"login":"me"},"repository":{"full_name":"o/r"},"pull_request":{"merged":true},"html_url":"u"}]"#,
    );
    let prs = gitea(&fake).search_pull_requests(Involvement::Authored, StateFilter::Open, "me").await.unwrap_or_default();
    assert_eq!((prs[0].repo.key().as_str(), prs[0].state), ("codeberg.org/o/r", ItemState::Merged));
    let url = &fake.requests()[0].url;
    assert!(url.contains("type=pulls") && url.contains("created=true") && url.contains("state=open"), "{url}");
}

#[tokio::test]
async fn gitlab_search_derives_the_project_from_references() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://gitlab.com/api/v4/merge_requests",
        r#"[{"iid":3,"title":"t","state":"opened","references":{"full":"grp/sub/proj!3"},"web_url":"u"},{"iid":4,"title":"x","state":"opened","web_url":"u"}]"#,
    );
    let prs = gitlab(&fake).search_pull_requests(Involvement::ReviewRequested, StateFilter::Open, "erin").await.unwrap_or_default();
    assert_eq!(prs.len(), 1, "no reference, no project, skipped");
    assert_eq!((prs[0].repo.owner.as_str(), prs[0].repo.name.as_str()), ("grp/sub", "proj"));
    let url = &fake.requests()[0].url;
    assert!(url.contains("reviewer_username=erin") && url.contains("state=opened"), "{url}");
}
