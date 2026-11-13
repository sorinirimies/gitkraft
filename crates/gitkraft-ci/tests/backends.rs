//! CI adapter tests against recorded responses.

use gitkraft_ci::log::parse_log;
use gitkraft_ci::{CiBackend, RerunScope, RunQuery, Status};
use gitkraft_platforms::testing::FakeClient;
use gitkraft_platforms::{Account, PlatformError, PlatformKind, RepoId, Secret};

fn github(fake: &FakeClient) -> CiBackend<FakeClient> {
    CiBackend::new(
        &Account::new(PlatformKind::GitHub, "github.com").with_token(Secret::new("t")),
        fake.clone(),
    )
}
fn gitlab(fake: &FakeClient) -> CiBackend<FakeClient> {
    CiBackend::new(
        &Account::new(PlatformKind::GitLab, "gitlab.com"),
        fake.clone(),
    )
}
fn repo() -> RepoId {
    RepoId::new("github.com", "o", "r")
}

const RUNS: &str = r#"{"total_count":2,"workflow_runs":[
 {"id":101,"name":"CI","display_title":"fix: thing","run_number":42,"status":"completed","conclusion":"failure",
  "head_branch":"master","head_sha":"abc","event":"push","actor":{"login":"alice"},
  "created_at":"2026-03-01T10:00:00Z","run_started_at":"2026-03-01T10:00:05Z","updated_at":"2026-03-01T10:05:05Z",
  "html_url":"https://github.com/o/r/actions/runs/101"},
 {"id":102,"name":"CI","display_title":"wip","run_number":43,"status":"in_progress","conclusion":null,
  "head_branch":"dev","head_sha":"def","event":"pull_request","actor":{"login":"bob"},
  "created_at":"2026-03-01T11:00:00Z","updated_at":"2026-03-01T11:00:30Z","html_url":"u"}]}"#;

const JOBS: &str = r#"{"jobs":[
 {"id":7,"name":"build","status":"completed","conclusion":"success","started_at":"2026-03-01T10:00:10Z","completed_at":"2026-03-01T10:01:10Z",
  "steps":[{"number":1,"name":"checkout","status":"completed","conclusion":"success","started_at":"2026-03-01T10:00:10Z","completed_at":"2026-03-01T10:00:15Z"}],
  "html_url":"j7","runner_name":"GitHub Actions 1"},
 {"id":8,"name":"test","status":"completed","conclusion":"failure","started_at":"2026-03-01T10:01:20Z","completed_at":"2026-03-01T10:05:00Z","steps":[],"html_url":"j8"}]}"#;

#[tokio::test]
async fn github_runs_map_status_timing_and_actor() {
    let fake = FakeClient::new();
    fake.push_json("https://api.github.com/repos/o/r/actions/runs", RUNS);
    let q = RunQuery {
        branch: Some("master".into()),
        ..RunQuery::default()
    };
    let runs = github(&fake)
        .list_runs(&repo(), &q)
        .await
        .unwrap_or_default();
    assert_eq!(runs.len(), 2);
    assert_eq!(
        (
            runs[0].status,
            runs[0].name.as_str(),
            runs[0].actor.as_str()
        ),
        (Status::Failed, "CI", "alice")
    );
    assert_eq!(runs[0].duration().map(|d| d.num_seconds()), Some(300));
    assert_eq!(runs[1].status, Status::Running);
    assert!(runs[1].finished_at.is_none(), "running runs have no end");
    assert!(fake.requests()[0].url.contains("branch=master"));
}

#[tokio::test]
async fn status_filter_is_applied_after_the_server() {
    let fake = FakeClient::new();
    fake.push_json("https://api.github.com/repos/o/r/actions/runs", RUNS);
    let q = RunQuery {
        status: Some(Status::Failed),
        ..RunQuery::default()
    };
    let runs = github(&fake)
        .list_runs(&repo(), &q)
        .await
        .unwrap_or_default();
    assert_eq!(runs.iter().map(|r| r.id).collect::<Vec<_>>(), vec![101]);
}

#[tokio::test]
async fn run_detail_has_jobs_steps_and_inferred_needs() {
    let fake = FakeClient::new();
    fake.push_json("https://api.github.com/repos/o/r/actions/runs/101", r#"{"id":101,"name":"CI","status":"completed","conclusion":"failure","created_at":"2026-03-01T10:00:00Z","updated_at":"2026-03-01T10:05:05Z"}"#);
    fake.push_json(
        "https://api.github.com/repos/o/r/actions/runs/101/jobs",
        JOBS,
    );
    let d = github(&fake)
        .run_detail(&repo(), 101)
        .await
        .expect("detail");
    assert_eq!(d.jobs.len(), 2);
    assert_eq!(d.jobs[0].steps[0].name, "checkout");
    assert_eq!(d.jobs[0].runner.as_deref(), Some("GitHub Actions 1"));
    assert_eq!(d.jobs[1].status, Status::Failed);
    assert_eq!(
        d.jobs[1].needs,
        ["build"],
        "test started after build finished"
    );
    let bars = gitkraft_ci::timing::gantt(&d);
    assert!(bars.iter().any(|b| b.label == "build › checkout"));
}

#[tokio::test]
async fn log_is_fetched_as_text_and_parses() {
    let fake = FakeClient::new();
    fake.push_status("https://api.github.com/repos/o/r/actions/jobs/8/logs", 200, "2026-03-01T10:01:20.0000000Z ##[group]Run tests\n2026-03-01T10:01:21.0000000Z error: boom\n");
    let raw = github(&fake).job_log(&repo(), 8).await.expect("log");
    let lines = parse_log(&raw);
    assert_eq!(lines.len(), 2);
    assert!(lines[0].time.is_some());
    assert!(
        fake.requests()[0]
            .headers
            .contains(&("Accept".into(), "text/plain".into()))
    );
}

#[tokio::test]
async fn actions_hit_the_right_endpoints() {
    let fake = FakeClient::new();
    for p in [
        "runs/101/rerun",
        "runs/101/rerun-failed-jobs",
        "jobs/8/rerun",
        "runs/101/cancel",
        "workflows/ci.yml/dispatches",
    ] {
        fake.push_status(
            &format!("https://api.github.com/repos/o/r/actions/{p}"),
            201,
            "",
        );
    }
    let b = github(&fake);
    let run = gitkraft_ci::Run {
        id: 101,
        ..gitkraft_ci::Run::default()
    };
    assert!(b.rerun(&repo(), &run, RerunScope::All).await.is_ok());
    assert!(b.rerun(&repo(), &run, RerunScope::Failed).await.is_ok());
    assert!(b.rerun_job(&repo(), 8).await.is_ok());
    assert!(b.cancel(&repo(), 101).await.is_ok());
    assert!(
        b.dispatch(
            &repo(),
            "ci.yml",
            "master",
            &[("level".into(), "debug".into())]
        )
        .await
        .is_ok()
    );
    let sent = fake.requests();
    assert!(sent[4].body.as_deref().is_some_and(
        |b| String::from_utf8_lossy(b) == r#"{"inputs":{"level":"debug"},"ref":"master"}"#
    ));
    assert!(matches!(
        b.play_job(&repo(), 1).await,
        Err(PlatformError::Unsupported(_))
    ));
}

#[tokio::test]
async fn gitea_run_without_name_falls_back_to_the_workflow_file() {
    let fake = FakeClient::new();
    fake.push_json(
        "https://codeberg.org/api/v1/repos/o/r/actions/runs",
        r#"{"workflow_runs":[{"id":5,"display_title":"t","status":"success","path":".gitea/workflows/ci.yml@refs/heads/master","head_branch":"master","started_at":"2026-03-01T10:00:00Z","stopped":"2026-03-01T10:02:00Z"}]}"#,
    );
    let b = CiBackend::new(
        &Account::new(PlatformKind::Gitea, "codeberg.org"),
        fake.clone(),
    );
    let runs = b
        .list_runs(&RepoId::new("codeberg.org", "o", "r"), &RunQuery::default())
        .await
        .unwrap_or_default();
    assert_eq!(
        (runs[0].name.as_str(), runs[0].status),
        ("ci.yml", Status::Success)
    );
    assert_eq!(runs[0].duration().map(|d| d.num_seconds()), Some(120));
    assert_eq!(b.system_name(), "Gitea Actions");
}

#[tokio::test]
async fn gitlab_pipeline_detail_orders_stages_into_needs() {
    let fake = FakeClient::new();
    fake.push_json("https://gitlab.com/api/v4/projects/g%2Fp/pipelines/9", r#"{"id":9,"iid":3,"status":"failed","ref":"main","sha":"abcdef1234","source":"push","user":{"username":"erin"},"web_url":"u","created_at":"2026-03-01T10:00:00Z","started_at":"2026-03-01T10:00:02Z","finished_at":"2026-03-01T10:03:00Z"}"#);
    fake.push_json("https://gitlab.com/api/v4/projects/g%2Fp/pipelines/9/jobs", r#"[
      {"id":1,"name":"compile","stage":"build","status":"success","started_at":"2026-03-01T10:00:02Z","finished_at":"2026-03-01T10:01:00Z"},
      {"id":2,"name":"unit","stage":"test","status":"failed","allow_failure":false,"runner":{"description":"shared-1"}},
      {"id":3,"name":"lint","stage":"test","status":"manual"}]"#);
    let r = RepoId::new("gitlab.com", "g", "p");
    let d = gitlab(&fake).run_detail(&r, 9).await.expect("detail");
    assert_eq!(
        (d.run.status, d.run.actor.as_str(), d.run.branch.as_str()),
        (Status::Failed, "erin", "main")
    );
    assert_eq!(d.jobs[1].needs, ["compile"]);
    assert_eq!(d.jobs[2].needs, ["compile"]);
    assert!(d.jobs[0].needs.is_empty());
    assert_eq!(
        (d.jobs[1].runner.as_deref(), d.jobs[2].status),
        (Some("shared-1"), Status::Manual)
    );
}

#[tokio::test]
async fn gitlab_actions_and_rerun_all_creates_a_pipeline_on_the_ref() {
    let fake = FakeClient::new();
    let base = "https://gitlab.com/api/v4/projects/g%2Fp";
    for p in [
        "pipelines/9/retry",
        "pipelines/9/cancel",
        "jobs/3/play",
        "jobs/3/retry",
        "pipeline",
    ] {
        fake.push_status(&format!("{base}/{p}"), 201, "{}");
    }
    let b = gitlab(&fake);
    let r = RepoId::new("gitlab.com", "g", "p");
    let run = gitkraft_ci::Run {
        id: 9,
        branch: "main".into(),
        ..gitkraft_ci::Run::default()
    };
    assert!(b.rerun(&r, &run, RerunScope::Failed).await.is_ok());
    assert!(b.cancel(&r, 9).await.is_ok());
    assert!(b.play_job(&r, 3).await.is_ok());
    assert!(b.rerun_job(&r, 3).await.is_ok());
    assert!(b.rerun(&r, &run, RerunScope::All).await.is_ok());
    let last = fake.requests().pop().expect("request");
    assert!(last.url.ends_with("/pipeline"));
    assert_eq!(
        String::from_utf8_lossy(last.body.as_deref().unwrap_or_default()),
        r#"{"ref":"main","variables":[]}"#
    );
    let no_ref = gitkraft_ci::Run {
        id: 9,
        ..gitkraft_ci::Run::default()
    };
    assert!(matches!(
        b.rerun(&r, &no_ref, RerunScope::All).await,
        Err(PlatformError::Unsupported(_))
    ));
}

#[tokio::test]
async fn gitlab_list_maps_statuses_and_passes_filters() {
    let fake = FakeClient::new();
    fake.push_json("https://gitlab.com/api/v4/projects/g%2Fp/pipelines", r#"[{"id":2,"status":"running","ref":"main","sha":"abc"},{"id":1,"status":"canceled","ref":"main","sha":"abc"}]"#);
    let q = RunQuery {
        branch: Some("main".into()),
        status: Some(Status::Running),
        ..RunQuery::default()
    };
    let runs = gitlab(&fake)
        .list_runs(&RepoId::new("gitlab.com", "g", "p"), &q)
        .await
        .unwrap_or_default();
    assert_eq!(
        runs.iter().map(|r| r.status).collect::<Vec<_>>(),
        vec![Status::Running, Status::Cancelled]
    );
    let url = &fake.requests()[0].url;
    assert!(
        url.contains("ref=main") && url.contains("status=running"),
        "{url}"
    );
}

#[tokio::test]
async fn errors_pass_through() {
    let fake = FakeClient::new();
    fake.push_status(
        "https://api.github.com/repos/o/r/actions/runs",
        403,
        "Resource not accessible",
    );
    assert!(matches!(
        github(&fake).list_runs(&repo(), &RunQuery::default()).await,
        Err(PlatformError::Forbidden(_))
    ));
}
