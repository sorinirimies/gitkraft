//! The hub against fake responses: sections, partial failure, details, contributions.

use gitkraft_hub::{Hub, SectionData};
use gitkraft_platforms::testing::FakeClient;
use gitkraft_platforms::{Account, PlatformKind, RepoId};
use gitkraft_settings::dashboard::{Dashboard, Involvement, Section, SectionKind, SectionState};

fn hub(fake: &FakeClient, hosts: &[(PlatformKind, &str)]) -> Hub<FakeClient> {
    let accounts = hosts
        .iter()
        .map(|(k, h)| {
            let mut a = Account::new(*k, *h);
            a.me = Some("me".into());
            a
        })
        .collect();
    Hub::new(accounts, fake)
}

fn section(kind: SectionKind, repos: &[&str], involvement: Option<Involvement>) -> Section {
    Section {
        name: "s".into(),
        kind,
        repos: repos.iter().map(|r| r.to_string()).collect(),
        involvement,
        state: SectionState::Open,
        label: None,
        branch: None,
        hidden: false,
    }
}

const PULLS: &str = r#"[
 {"number":1,"title":"mine","state":"open","user":{"login":"me"},"updated_at":"2026-01-02T00:00:00Z","html_url":"u1"},
 {"number":2,"title":"theirs","state":"open","user":{"login":"other"},"requested_reviewers":[{"login":"me"}],"updated_at":"2026-01-03T00:00:00Z","html_url":"u2"}]"#;

#[tokio::test]
async fn repo_section_filters_by_involvement_and_sorts_newest_first() {
    let fake = FakeClient::new();
    fake.push_json("https://api.github.com/repos/o/r/pulls", PULLS);
    let h = hub(&fake, &[(PlatformKind::GitHub, "github.com")]);

    let all = h
        .load_section(&section(SectionKind::Prs, &["github.com/o/r"], None))
        .await;
    let SectionData::Prs(all) = all else {
        panic!("prs")
    };
    assert_eq!(
        all.items.iter().map(|p| p.number).collect::<Vec<_>>(),
        vec![2, 1],
        "newest activity first"
    );

    let review = h
        .load_section(&section(
            SectionKind::Prs,
            &["github.com/o/r"],
            Some(Involvement::ReviewRequested),
        ))
        .await;
    let SectionData::Prs(review) = review else {
        panic!("prs")
    };
    assert_eq!(
        review.items.iter().map(|p| p.number).collect::<Vec<_>>(),
        vec![2]
    );

    let mine = h
        .load_section(&section(
            SectionKind::Prs,
            &["github.com/o/r"],
            Some(Involvement::Authored),
        ))
        .await;
    assert_eq!(mine.len(), 1);
}

#[tokio::test]
async fn search_section_asks_every_account_and_keeps_going_when_one_fails() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/search/issues",
        r#"{"items":[{"number":4,"title":"x","state":"open","repository_url":"https://api.github.com/repos/a/b","pull_request":{},"html_url":"u"}]}"#,
    );
    fake.push_status(
        "https://codeberg.org/api/v1/repos/issues/search",
        500,
        "down",
    );
    let h = hub(
        &fake,
        &[
            (PlatformKind::GitHub, "github.com"),
            (PlatformKind::Gitea, "codeberg.org"),
        ],
    );
    let data = h
        .load_section(&section(SectionKind::Prs, &[], Some(Involvement::Authored)))
        .await;
    assert_eq!(data.len(), 1);
    assert_eq!(data.errors().len(), 1);
    assert!(
        data.errors()[0].starts_with("codeberg.org"),
        "{:?}",
        data.errors()
    );
}

#[tokio::test]
async fn unknown_host_and_bad_keys_are_reported_not_fatal() {
    let fake = FakeClient::new();
    let h = hub(&fake, &[(PlatformKind::GitHub, "github.com")]);
    let data = h
        .load_section(&section(SectionKind::Prs, &["gitlab.com/o/r"], None))
        .await;
    assert!(data.is_empty());
    assert_eq!(data.errors(), ["gitlab.com: no account"]);
    let none = h
        .load_section(&section(SectionKind::Prs, &["garbage"], None))
        .await;
    assert!(
        none.is_empty() && none.errors().is_empty(),
        "unparseable keys are skipped"
    );
}

#[tokio::test]
async fn me_is_asked_once_and_cached() {
    let fake = FakeClient::new();
    fake.push_json("https://api.github.com/user", r#"{"login":"alice"}"#);
    let h = Hub::new(
        vec![Account::new(PlatformKind::GitHub, "github.com")],
        &fake,
    );
    assert_eq!(h.me("github.com").await.as_deref(), Ok("alice"));
    assert_eq!(h.me("GitHub.com").await.as_deref(), Ok("alice"));
    assert_eq!(fake.requests().len(), 1);
    assert!(h.me("nowhere").await.is_err());
}

#[tokio::test]
async fn ci_and_inbox_sections() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/repos/o/r/actions/runs",
        r#"{"workflow_runs":[{"id":1,"name":"CI","status":"completed","conclusion":"failure","created_at":"2026-01-01T00:00:00Z","head_branch":"master"}]}"#,
    );
    fake.push_json(
        "https://api.github.com/notifications",
        r#"[{"id":"1","unread":true,"reason":"mention","subject":{"title":"t","type":"Issue"},"repository":{"full_name":"o/r"}}]"#,
    );
    let h = hub(&fake, &[(PlatformKind::GitHub, "github.com")]);
    let mut ci = section(SectionKind::Ci, &["github.com/o/r"], None);
    ci.branch = Some("master".into());
    let runs = h.load_section(&ci).await;
    assert!(matches!(&runs, SectionData::Runs(r) if r.items[0].name == "CI"));
    assert!(fake.requests()[0].url.contains("branch=master"));
    let inbox = h
        .load_section(&section(SectionKind::Inbox, &[], None))
        .await;
    assert_eq!(inbox.len(), 1);
}

#[tokio::test]
async fn pr_detail_survives_a_missing_diff() {
    let fake = FakeClient::new();
    let base = "https://api.github.com/repos/o/r";
    fake.push_json(&format!("{base}/pulls/7"), r#"{"number":7,"title":"t","state":"open","head":{"ref":"a"},"base":{"ref":"m"},"html_url":"u"}"#);
    fake.push_json(
        &format!("{base}/issues/7/comments"),
        r#"[{"id":1,"body":"hi","user":{"login":"x"}}]"#,
    );
    fake.push_json(
        &format!("{base}/pulls/7/reviews"),
        r#"[{"state":"APPROVED","user":{"login":"y"}}]"#,
    );
    fake.push_status(&format!("{base}/pulls/7.diff"), 500, "boom");
    let h = hub(&fake, &[(PlatformKind::GitHub, "github.com")]);
    let d = h
        .pr_detail(&RepoId::new("github.com", "o", "r"), 7)
        .await
        .expect("detail");
    assert_eq!((d.comments.len(), d.reviews.len()), (1, 1));
    assert!(d.diff.is_empty());
    assert_eq!(d.errors.len(), 1);
    assert!(d.errors[0].starts_with("diff"));
    assert!(
        h.pr_detail(&RepoId::new("nowhere", "o", "r"), 7)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn contributions_are_summed_across_accounts() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"user":{"contributionsCollection":{"contributionCalendar":{"weeks":[{"contributionDays":[{"date":"2026-01-01","contributionCount":4},{"date":"2026-01-02","contributionCount":1}]}]}}}}}"#,
    );
    fake.push_json(
        "https://codeberg.org/api/v1/users/me/heatmap",
        r#"[{"timestamp":1767225600,"contributions":3}]"#,
    );
    let mut gh = Account::new(PlatformKind::GitHub, "github.com");
    gh.me = Some("me".into());
    gh.token = Some(gitkraft_platforms::Secret::new("t"));
    let mut cb = Account::new(PlatformKind::Gitea, "codeberg.org");
    cb.me = Some("me".into());
    let h = Hub::new(vec![gh, cb], &fake);
    let c = h.contributions().await;
    // 1767225600 = 2026-01-01T00:00:00Z
    assert_eq!(
        c.items.iter().map(|d| d.count).collect::<Vec<_>>(),
        vec![7, 1]
    );
    assert!(c.errors.is_empty());
}

#[test]
fn starter_dashboard_is_loadable_by_the_hub_types() {
    assert!(
        Dashboard::starter()
            .sections
            .iter()
            .all(|s| !s.name.is_empty())
    );
}

#[tokio::test]
async fn a_day_is_merged_over_accounts_and_a_failing_one_is_reported() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"user":{"contributionsCollection":{"contributionCalendar":{"totalContributions":3},"commitContributionsByRepository":[{"repository":{"nameWithOwner":"o/r","url":"u"},"contributions":{"totalCount":3}}],"pullRequestContributions":{"nodes":[]},"issueContributions":{"nodes":[]},"pullRequestReviewContributions":{"nodes":[]}}}}}"#,
    );
    fake.push_json(
        "https://codeberg.org/api/v1/users/me/activities/feeds?only-performed-by=true&date=2026-01-01&limit=50",
        r#"[{"op_type":"create_issue","content":"1|Bug","repo":{"full_name":"o/r"}}]"#,
    );
    fake.push_status("https://gitlab.com/api/v4/users?username=me", 500, "down");
    let mut gh = Account::new(PlatformKind::GitHub, "github.com");
    gh.me = Some("me".into());
    gh.token = Some(gitkraft_platforms::Secret::new("t"));
    let mut cb = Account::new(PlatformKind::Gitea, "codeberg.org");
    cb.me = Some("me".into());
    let mut gl = Account::new(PlatformKind::GitLab, "gitlab.com");
    gl.me = Some("me".into());
    let h = Hub::new(vec![gh, cb, gl], &fake);
    let day = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).expect("date");
    let r = h.day_activity(day).await;
    assert_eq!(r.activity.total, 4);
    let repos: Vec<_> = r.activity.entries.iter().map(|e| e.repo.as_str()).collect();
    assert_eq!(repos, ["github.com/o/r", "codeberg.org/o/r"]);
    assert_eq!(r.errors.len(), 1, "{:?}", r.errors);
    assert!(r.errors[0].contains("gitlab.com"));
}
