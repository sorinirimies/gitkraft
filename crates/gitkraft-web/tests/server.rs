//! End to end: a real server on a free port, fake platform responses, raw HTTP.

use std::net::SocketAddr;

use gitkraft_hub::Hub;
use gitkraft_platforms::testing::FakeClient;
use gitkraft_platforms::{Account, DynClient, PlatformKind, Secret};
use gitkraft_settings::dashboard::{Dashboard, Involvement, Section, SectionKind, SectionState};
use gitkraft_web::{Paths, Session, router};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// The token of each test server, which a real page reads from its `<meta>`.
static TOKENS: std::sync::Mutex<Vec<(SocketAddr, String)>> = std::sync::Mutex::new(Vec::new());

fn token_of(addr: SocketAddr) -> String {
    TOKENS
        .lock()
        .expect("lock")
        .iter()
        .find(|(a, _)| *a == addr)
        .map(|(_, t)| t.clone())
        .unwrap_or_default()
}

struct Response {
    status: u16,
    body: String,
}

fn dechunk(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut rest = raw;
    while let Some(nl) = rest.windows(2).position(|w| w == b"\r\n") {
        let size =
            usize::from_str_radix(std::str::from_utf8(&rest[..nl]).unwrap_or("0").trim(), 16)
                .unwrap_or(0);
        if size == 0 {
            break;
        }
        out.extend_from_slice(&rest[nl + 2..nl + 2 + size]);
        rest = &rest[nl + 2 + size + 2..];
    }
    out
}

async fn request(addr: SocketAddr, method: &str, path: &str, body: &str) -> Response {
    // Actions carry the page's token unless the test brings its own.
    let with_token;
    let body = if path == "/api/action" && body.starts_with('{') && !body.contains("\"token\"") {
        with_token = body.replacen('{', &format!("{{\"token\":\"{}\",", token_of(addr)), 1);
        with_token.as_str()
    } else {
        body
    };
    let mut s = TcpStream::connect(addr).await.expect("connect");
    let extra = if method == "POST" {
        format!(
            "Content-Type: application/json\r\nContent-Length: {}\r\n",
            body.len()
        )
    } else {
        String::new()
    };
    s.write_all(
        format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\n{extra}Connection: close\r\n\r\n{body}"
        )
        .as_bytes(),
    )
    .await
    .expect("write");
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).await.expect("read");
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").expect("head");
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("status");
    let body = &raw[split + 4..];
    let body = if head
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        dechunk(body)
    } else {
        body.to_vec()
    };
    Response {
        status,
        body: String::from_utf8_lossy(&body).into_owned(),
    }
}

async fn get(addr: SocketAddr, path: &str) -> Response {
    request(addr, "GET", path, "").await
}

fn section(name: &str, kind: SectionKind, repos: &[&str]) -> Section {
    Section {
        name: name.into(),
        kind,
        repos: repos.iter().map(|r| r.to_string()).collect(),
        involvement: if repos.is_empty() {
            Some(Involvement::Authored)
        } else {
            None
        },
        state: SectionState::Open,
        label: None,
        branch: None,
        hidden: false,
    }
}

struct Harness {
    addr: SocketAddr,
    fake: FakeClient,
    dir: tempfile::TempDir,
}

async fn start() -> Harness {
    start_with(Vec::new()).await
}

async fn start_with(checkouts: Vec<std::path::PathBuf>) -> Harness {
    let fake = FakeClient::new();
    let mut account = Account::new(PlatformKind::GitHub, "github.com").with_token(Secret::new("t"));
    account.me = Some("me".into());
    let hub = Hub::new(vec![account], &DynClient::new(fake.clone()));
    let dashboard = Dashboard {
        sections: vec![
            section("Mine", SectionKind::Prs, &["github.com/o/r"]),
            section("CI", SectionKind::Ci, &["github.com/o/r"]),
            section("Inbox", SectionKind::Inbox, &[]),
        ],
    };
    let dir = tempfile::tempdir().expect("tmp");
    let paths = Paths {
        dashboard: None,
        notify: Some(dir.path().join("notifications.toml")),
        use_state_dir: false,
        ssh_dir: None,
        checkouts,
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let session = Session::new(hub, dashboard, paths, 0);
    TOKENS
        .lock()
        .expect("lock")
        .push((addr, session.token().to_string()));
    tokio::spawn(async move {
        let _ = topcoat::serve(listener, router(session)).await;
    });
    Harness { addr, fake, dir }
}

const PULLS: &str = r#"[
 {"number":1,"title":"Fix <script>alert(1)</script> bug","state":"open","user":{"login":"alice"},"updated_at":"2026-01-02T00:00:00Z","html_url":"u1"},
 {"number":2,"title":"Second","state":"open","user":{"login":"bob"},"draft":true,"updated_at":"2026-01-01T00:00:00Z","html_url":"u2"}]"#;

#[tokio::test]
async fn the_dashboard_lists_rows_links_them_and_escapes_titles() {
    let h = start().await;
    h.fake
        .push_json("https://api.github.com/repos/o/r/pulls", PULLS);
    let page = get(h.addr, "/").await;
    assert_eq!(page.status, 200);
    for needle in [
        "<title>Mine · GitKraft</title>",
        "href=\"/?s=1\"",
        "CI</a>",
        "Pulse",
        "href=\"/pr?repo=github.com%2Fo%2Fr&amp;n=1\"",
        "#2 Second",
    ] {
        assert!(
            page.body.contains(needle),
            "missing {needle:?} in\n{}",
            page.body
        );
    }
    assert!(
        !page.body.contains("<script>alert(1)"),
        "titles are escaped"
    );
    assert!(page.body.contains("&lt;script&gt;alert(1)"));
}

#[tokio::test]
async fn a_failing_platform_shows_a_note_instead_of_a_500() {
    let h = start().await;
    h.fake
        .push_status("https://api.github.com/repos/o/r/pulls", 500, "boom");
    let page = get(h.addr, "/").await;
    assert_eq!(page.status, 200);
    assert!(
        page.body.contains("class=\"note err\"") && page.body.contains("500"),
        "{}",
        page.body
    );
}

fn pr_fixtures(h: &Harness) {
    let base = "https://api.github.com/repos/o/r";
    h.fake.push_json(&format!("{base}/pulls/7"), r#"{"number":7,"title":"Add pulse","body":"See ![shot](https://img.example/a.png)","state":"open","user":{"login":"alice"},"head":{"ref":"feat","sha":"abc"},"base":{"ref":"master"},"additions":3,"deletions":1,"html_url":"u"}"#);
    h.fake.push_json(
        &format!("{base}/commits/abc/status"),
        r#"{"state":"success"}"#,
    );
    h.fake.push_json(
        &format!("{base}/issues/7/comments"),
        r#"[{"id":1,"body":"looks <b>good</b>","user":{"login":"bob"}}]"#,
    );
    h.fake.push_json(
        &format!("{base}/pulls/7/reviews"),
        r#"[{"state":"APPROVED","user":{"login":"carol"}}]"#,
    );
    h.fake.push_status(
        &format!("{base}/pulls/7.diff"),
        200,
        "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-old <i>\n+new",
    );
}

#[tokio::test]
async fn the_pr_page_has_tabs_actions_images_and_escaped_text() {
    let h = start().await;
    pr_fixtures(&h);
    let overview = get(h.addr, "/pr?repo=github.com%2Fo%2Fr&n=7").await;
    assert_eq!(overview.status, 200, "{}", overview.body);
    for needle in [
        "Add pulse",
        "carol approved",
        "✔ checks passed",
        "+3 −1",
        "data-act=",
        "Approve",
        "Squash",
        "When green",
        "Queue",
        "Force merge",
        "Update branch",
        "Cancel auto-merge",
        "data-method=\"mm\"",
        "src=\"https://img.example/a.png\"",
        "feat → master",
    ] {
        assert!(overview.body.contains(needle), "missing {needle:?}");
    }
    assert!(
        overview
            .body
            .contains("&quot;action&quot;:&quot;review&quot;")
            || overview.body.contains("&quot;review&quot;"),
        "action payload is attribute-escaped JSON"
    );

    let diff = get(h.addr, "/pr?repo=github.com%2Fo%2Fr&n=7&tab=diff").await;
    assert!(
        diff.body
            .contains("<div class=\"del\">-old &lt;i&gt;</div>")
            && diff.body.contains("<div class=\"add\">+new</div>"),
        "{}",
        diff.body
    );

    let comments = get(h.addr, "/pr?repo=github.com%2Fo%2Fr&n=7&tab=comments").await;
    assert!(
        comments.body.contains("looks &lt;b&gt;good&lt;/b&gt;")
            && !comments.body.contains("<b>good</b>")
    );
    assert!(comments.body.contains("Comments 1"));
}

#[tokio::test]
async fn unknown_items_and_bad_parameters_are_404_not_a_crash() {
    let h = start().await;
    h.fake
        .push_status("https://api.github.com/repos/o/r/pulls/9", 404, "nope");
    assert_eq!(
        get(h.addr, "/pr?repo=github.com%2Fo%2Fr&n=9").await.status,
        404
    );
    assert_eq!(get(h.addr, "/pr?repo=garbage&n=1").await.status, 404);
    assert_eq!(get(h.addr, "/pr").await.status, 404);
    assert_eq!(
        get(h.addr, "/ci/log?repo=github.com%2Fo%2Fr&job=x")
            .await
            .status,
        404
    );
    assert_eq!(get(h.addr, "/nothing-here").await.status, 404);
}

#[tokio::test]
async fn actions_post_json_and_report_success_or_the_reason() {
    let h = start().await;
    h.fake
        .push_json("https://api.github.com/repos/o/r/issues/7/comments", "{}");
    let ok = request(
        h.addr,
        "POST",
        "/api/action",
        r#"{"action":"comment","repo":"github.com/o/r","number":7,"kind":"pr","body":"thanks"}"#,
    )
    .await;
    assert_eq!(ok.status, 200, "{}", ok.body);
    assert!(ok.body.contains("comment posted"));
    let sent = h.fake.requests();
    assert_eq!(
        String::from_utf8_lossy(sent[0].body.as_deref().unwrap_or_default()),
        r#"{"body":"thanks"}"#
    );

    for (body, needle) in [
        (
            r#"{"action":"comment","repo":"github.com/o/r","number":7,"kind":"pr","body":"  "}"#,
            "needs some text",
        ),
        (
            r#"{"action":"review","repo":"github.com/o/r","number":7,"verdict":"request_changes","body":""}"#,
            "needs a note",
        ),
        (
            r#"{"action":"merge","repo":"github.com/o/r","number":7,"method":"fast-forward"}"#,
            "method must be merge, squash or rebase",
        ),
        (
            r#"{"action":"comment","repo":"nope","number":7,"kind":"pr","body":"x"}"#,
            "not host/owner/name",
        ),
        (
            r#"{"action":"comment","repo":"gitlab.com/o/r","number":7,"kind":"pr","body":"x"}"#,
            "no account",
        ),
    ] {
        let r = request(h.addr, "POST", "/api/action", body).await;
        assert_eq!(r.status, 400, "{body}");
        assert!(r.body.contains(needle), "{body}: {}", r.body);
    }
    assert!(
        request(h.addr, "POST", "/api/action", "{not json")
            .await
            .status
            >= 400
    );
    assert!(
        request(h.addr, "POST", "/api/action", r#"{"action":"explode"}"#)
            .await
            .status
            >= 400
    );
    assert_eq!(
        h.fake.requests().len(),
        1,
        "failed actions never reach the platform"
    );
}

#[tokio::test]
async fn a_failing_platform_call_is_reported_to_the_page() {
    let h = start().await;
    h.fake.push_status(
        "https://api.github.com/repos/o/r/pulls/7/merge",
        405,
        "not mergeable",
    );
    let r = request(
        h.addr,
        "POST",
        "/api/action",
        r#"{"action":"merge","repo":"github.com/o/r","number":7}"#,
    )
    .await;
    assert_eq!(r.status, 400);
    assert!(r.body.contains("405"), "{}", r.body);
}

#[tokio::test]
async fn ci_run_and_log_pages() {
    let h = start().await;
    let base = "https://api.github.com/repos/o/r/actions";
    h.fake.push_json(&format!("{base}/runs/5"), r#"{"id":5,"name":"CI","run_number":9,"status":"in_progress","head_branch":"master","actor":{"login":"alice"},"created_at":"2026-03-01T10:00:00Z"}"#);
    h.fake.push_json(&format!("{base}/runs/5/jobs"), r#"{"jobs":[{"id":11,"name":"build","status":"completed","conclusion":"failure","started_at":"2026-03-01T10:00:10Z","completed_at":"2026-03-01T10:01:10Z","steps":[]}]}"#);
    let run = get(h.addr, "/ci/run?repo=github.com%2Fo%2Fr&id=5").await;
    assert_eq!(run.status, 200, "{}", run.body);
    for needle in [
        "CI #9",
        "Re-run failed",
        "Cancel",
        "build",
        "1m 00s",
        "<svg class=\"gantt\"",
        "http-equiv=\"refresh\"",
        "/ci/log?repo=github.com%2Fo%2Fr&amp;job=11&amp;id=5",
    ] {
        assert!(
            run.body.contains(needle),
            "missing {needle:?} in\n{}",
            run.body
        );
    }

    h.fake.push_status(&format!("{base}/jobs/11/logs"), 200, "##[group]Build <b>\n\u{1b}[31mred\u{1b}[0m\nerror[E0425]: no x\n --> src/a.rs:1:2\n##[endgroup]");
    let log = get(h.addr, "/ci/log?repo=github.com%2Fo%2Fr&job=11&id=5").await;
    assert_eq!(log.status, 200);
    for needle in [
        "Build &lt;b&gt;",
        "color:#cd3131",
        "1 problems found",
        "src/a.rs:1",
        "href=\"#L3\"",
        "← back to the run",
    ] {
        assert!(
            log.body.contains(needle),
            "missing {needle:?} in\n{}",
            log.body
        );
    }
    assert!(!log.body.contains("<b>\n") && !log.body.contains("Build <b>"));
}

#[tokio::test]
async fn the_pulse_page_draws_the_calendar_and_numbers() {
    let h = start().await;
    h.fake.push_json("https://api.github.com/graphql", r#"{"data":{"user":{"contributionsCollection":{"contributionCalendar":{"weeks":[{"contributionDays":[{"date":"2026-01-01","contributionCount":3}]}]}}}}}"#);
    h.fake
        .push_json("https://api.github.com/repos/o/r/pulls", PULLS);
    h.fake
        .push_json("https://api.github.com/repos/o/r/issues", "[]");
    h.fake.push_json(
        "https://api.github.com/repos/o/r/actions/runs",
        r#"{"workflow_runs":[]}"#,
    );
    let page = get(h.addr, "/pulse").await;
    assert_eq!(page.status, 200, "{}", page.body);
    for needle in [
        "<svg class=\"heat\"",
        "Contributions",
        "PRs opened",
        "class=\"stat\"",
        "href=\"/pulse?repo=github.com%2Fo%2Fr&amp;days=7\"",
        "PRs opened per day",
    ] {
        assert!(
            page.body.contains(needle),
            "missing {needle:?} in\n{}",
            page.body
        );
    }
    // A wild `days` falls back instead of failing.
    h.fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"user":{"contributionsCollection":{"contributionCalendar":{"weeks":[]}}}}}"#,
    );
    assert_eq!(get(h.addr, "/pulse?days=99999").await.status, 200);
}

#[tokio::test]
async fn notifications_can_be_switched_from_the_page() {
    let h = start().await;
    let page = get(h.addr, "/notify").await;
    assert_eq!(page.status, 200);
    assert!(
        page.body.contains("github.com/o/r")
            && page.body.contains("Turn on")
            && page.body.contains("notifications off"),
        "{}",
        page.body
    );

    let r = request(
        h.addr,
        "POST",
        "/api/action",
        r#"{"action":"notify_toggle","repo":"github.com/o/r","on":true}"#,
    )
    .await;
    assert_eq!(r.status, 200, "{}", r.body);
    let text = std::fs::read_to_string(h.dir.path().join("notifications.toml")).expect("written");
    assert!(
        text.contains("github.com/o/r") && text.contains("enabled = true"),
        "{text}"
    );
    let after = get(h.addr, "/notify").await;
    assert!(
        after.body.contains("Turn off") && after.body.contains("no poller is running"),
        "{}",
        after.body
    );

    let bad = request(
        h.addr,
        "POST",
        "/api/action",
        r#"{"action":"notify_toggle","repo":"junk","on":true}"#,
    )
    .await;
    assert_eq!(bad.status, 400);
}

#[tokio::test]
async fn json_and_static_routes() {
    let h = start().await;
    assert_eq!(get(h.addr, "/api/health").await.body, "\"ok\"");
    let s = get(h.addr, "/api/sections").await;
    assert!(
        s.body.contains("\"Mine\"") && s.body.contains("\"kind\":\"prs\""),
        "{}",
        s.body
    );
    let css = get(h.addr, "/style.css").await;
    assert!(css.body.contains(".diff") && css.status == 200);
    h.fake.push_json("https://api.github.com/graphql", r#"{"data":{"user":{"contributionsCollection":{"contributionCalendar":{"weeks":[{"contributionDays":[{"date":"2026-01-01","contributionCount":3}]}]}}}}}"#);
    let c = get(h.addr, "/api/contributions").await;
    assert!(
        c.body.contains("2026-01-01") && c.body.contains("\"count\":3"),
        "{}",
        c.body
    );
}

#[tokio::test]
async fn an_action_without_the_pages_token_is_refused_and_never_reaches_the_platform() {
    let h = start().await;
    let body = r#"{"action":"comment","repo":"github.com/o/r","number":7,"kind":"pr","body":"x"}"#;
    // No token at all, and a wrong one.
    for raw in [
        body.to_string(),
        body.replacen('{', r#"{"token":"guess","#, 1),
    ] {
        let mut s = TcpStream::connect(h.addr).await.expect("connect");
        s.write_all(
            format!(
                "POST /api/action HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{raw}",
                raw.len()
            )
            .as_bytes(),
        )
        .await
        .expect("write");
        let mut out = Vec::new();
        s.read_to_end(&mut out).await.expect("read");
        let text = String::from_utf8_lossy(&out).to_string();
        assert!(text.starts_with("HTTP/1.1 400"), "{text}");
        assert!(text.contains("wrong token"), "{text}");
    }
    assert!(h.fake.requests().is_empty());
    // Every page carries the token.
    let page = get(h.addr, "/notify").await;
    assert!(
        page.body
            .contains(&format!("content=\"{}\"", token_of(h.addr))),
        "{}",
        page.body
    );
}

#[tokio::test]
async fn the_keys_page_lists_keys_and_a_key_can_be_removed() {
    let h = start().await;
    h.fake.push_json(
        "https://api.github.com/user/keys?per_page=100&limit=50",
        r#"[{"id":5,"title":"my <laptop>","key":"ssh-ed25519 AAAA"}]"#,
    );
    let page = get(h.addr, "/keys").await;
    assert_eq!(page.status, 200);
    assert!(
        page.body.contains("my &lt;laptop&gt;")
            && page.body.contains("Remove")
            && !page.body.contains("Set up ssh"),
        "no ssh dir in a test session: {}",
        page.body
    );
    h.fake
        .push_status("https://api.github.com/user/keys/5", 204, "");
    let r = request(
        h.addr,
        "POST",
        "/api/action",
        r#"{"action":"ssh_revoke","host":"github.com","id":"5"}"#,
    )
    .await;
    assert_eq!(r.status, 200, "{}", r.body);
    assert!(r.body.contains("removed key 5"));
    for body in [
        r#"{"action":"ssh_setup","host":"github.com"}"#,
        r#"{"action":"ssh_test","host":"github.com"}"#,
    ] {
        let r = request(h.addr, "POST", "/api/action", body).await;
        assert_eq!(r.status, 400, "{body}");
        assert!(r.body.contains("may not"), "{}", r.body);
    }
    let r = request(
        h.addr,
        "POST",
        "/api/action",
        r#"{"action":"ssh_revoke","host":"nowhere","id":"1"}"#,
    )
    .await;
    assert!(r.body.contains("no account"), "{}", r.body);
}

#[tokio::test]
async fn merge_when_green_queue_update_and_cancel_reach_the_platform() {
    let h = start().await;
    let pr = "https://api.github.com/repos/o/r/pulls/7";
    h.fake.push_json(pr, r#"{"node_id":"PR_7"}"#);
    h.fake
        .push_json("https://api.github.com/graphql", r#"{"data":{}}"#);
    h.fake.push_json(pr, r#"{"node_id":"PR_7"}"#);
    h.fake
        .push_json("https://api.github.com/graphql", r#"{"data":{}}"#);
    h.fake.push_json(&format!("{pr}/update-branch"), "{}");
    h.fake.push_json(pr, r#"{"node_id":"PR_7"}"#);
    h.fake
        .push_json("https://api.github.com/graphql", r#"{"data":{}}"#);
    let post = |body: &'static str| request(h.addr, "POST", "/api/action", body);

    let r = post(r#"{"action":"merge","repo":"github.com/o/r","number":7,"when":"when_green","method":"squash"}"#).await;
    assert_eq!(r.status, 200, "{}", r.body);
    assert!(
        r.body.contains("will merge when the checks pass"),
        "{}",
        r.body
    );
    let r = post(
        r#"{"action":"merge","repo":"github.com/o/r","number":7,"when":"queue","force":true}"#,
    )
    .await;
    assert!(r.body.contains("merge queue"), "{}", r.body);
    let r = post(r#"{"action":"update_branch","repo":"github.com/o/r","number":7}"#).await;
    assert!(r.body.contains("being updated"), "{}", r.body);
    let r = post(r#"{"action":"cancel_auto_merge","repo":"github.com/o/r","number":7}"#).await;
    assert!(r.body.contains("no longer auto-merge"), "{}", r.body);

    let sent = h.fake.requests();
    let sent_body = |i: usize| {
        String::from_utf8_lossy(sent[i].body.as_deref().unwrap_or_default()).into_owned()
    };
    assert!(sent_body(1).contains("enablePullRequestAutoMerge") && sent_body(1).contains("SQUASH"));
    assert!(sent_body(3).contains("enqueuePullRequest") && sent_body(3).contains("\"jump\":true"));
    assert!(sent_body(6).contains("disablePullRequestAutoMerge"));
}

#[tokio::test]
async fn nonsense_merge_requests_are_refused_before_any_call() {
    let h = start().await;
    for (body, needle) in [
        (
            r#"{"action":"merge","repo":"github.com/o/r","number":7,"when":"soon"}"#,
            "when must be now, when_green or queue",
        ),
        (
            r#"{"action":"merge","repo":"github.com/o/r","number":7,"when":"when_green","force":true}"#,
            "cannot wait",
        ),
        (
            r#"{"action":"update_branch","repo":"github.com/o/r","number":7,"style":"squash"}"#,
            "style must be merge or rebase",
        ),
    ] {
        let r = request(h.addr, "POST", "/api/action", body).await;
        assert_eq!(r.status, 400, "{body}");
        assert!(r.body.contains(needle), "{}", r.body);
    }
    assert!(h.fake.requests().is_empty());
}

fn git(dir: &std::path::Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .current_dir(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .output()
        .expect("git");
    assert!(
        out.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

const WORKTREE: &str = r#"{"action":"worktree","repo":"github.com/o/r","number":7}"#;

#[tokio::test]
async fn a_worktree_of_a_repo_without_a_known_checkout_says_how_to_fix_it() {
    let h = start().await;
    let r = request(h.addr, "POST", "/api/action", WORKTREE).await;
    assert_eq!(r.status, 400, "{}", r.body);
    assert!(r.body.contains("--checkout DIR"), "{}", r.body);
    assert!(h.fake.requests().is_empty());
}

#[cfg(unix)]
#[tokio::test]
async fn a_pull_request_is_fetched_into_a_worktree_next_to_the_checkout() {
    let root = tempfile::tempdir().expect("tmp");
    // the "forge": a repository that keeps the PR head under refs/pull/7/head
    let forge = root.path().join("forge");
    std::fs::create_dir_all(&forge).expect("mk");
    git(&forge, &["init", "-q"]);
    std::fs::write(forge.join("f.txt"), "pr code").expect("write");
    git(&forge, &["add", "."]);
    git(&forge, &["commit", "-qm", "pr"]);
    git(&forge, &["update-ref", "refs/pull/7/head", "HEAD"]);
    // the checkout: origin is the GitHub URL; its "ssh" is a script that serves the forge above
    let mine = root.path().join("r");
    std::fs::create_dir_all(&mine).expect("mk");
    git(&mine, &["init", "-q"]);
    git(
        &mine,
        &["remote", "add", "origin", "git@github.com:o/r.git"],
    );
    let fake_ssh = root.path().join("fake-ssh");
    std::fs::write(
        &fake_ssh,
        format!(
            "#!/bin/sh\nexec git-upload-pack '{}/.git'\n",
            forge.display()
        ),
    )
    .expect("script");
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake_ssh, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    }
    git(
        &mine,
        &["config", "core.sshCommand", &fake_ssh.display().to_string()],
    );
    let h = start_with(vec![mine.clone()]).await;
    let r = request(h.addr, "POST", "/api/action", WORKTREE).await;
    assert_eq!(r.status, 200, "{}", r.body);
    assert!(r.body.contains("#7 is checked out"), "{}", r.body);
    let wt = root.path().join("r-pr-7");
    assert_eq!(
        std::fs::read_to_string(wt.join("f.txt")).expect("worktree"),
        "pr code"
    );
    assert!(h.fake.requests().is_empty(), "no platform call is needed");
}

#[tokio::test]
async fn every_page_carries_the_command_palette_and_the_pr_page_a_check_out_button() {
    let h = start().await;
    h.fake
        .push_json("https://api.github.com/repos/o/r/pulls", "[]");
    let page = get(h.addr, "/").await;
    for needle in ["function gkPalette", "gkEntries", "button[data-act]"] {
        assert!(page.body.contains(needle), "{needle} missing");
    }
    pr_fixtures(&h);
    let pr = get(h.addr, "/pr?repo=github.com%2Fo%2Fr&n=7").await;
    assert!(pr.body.contains("Check out"), "{}", pr.body);
    assert!(
        pr.body.contains("&quot;action&quot;:&quot;worktree&quot;") || pr.body.contains("worktree")
    );
}

#[tokio::test]
async fn a_day_of_the_calendar_answers_with_what_was_done_and_refuses_nonsense() {
    let h = start().await;
    let today = chrono::Utc::now().date_naive();
    h.fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"user":{"contributionsCollection":{"contributionCalendar":{"totalContributions":4},"commitContributionsByRepository":[{"repository":{"nameWithOwner":"o/r","url":"https://github.com/o/r"},"contributions":{"totalCount":3}}],"pullRequestContributions":{"nodes":[{"pullRequest":{"number":7,"title":"Add <b>pulse</b>","url":"https://github.com/o/r/pull/7","repository":{"nameWithOwner":"o/r"}}}]},"issueContributions":{"nodes":[]},"pullRequestReviewContributions":{"nodes":[]}}}}}"#,
    );
    let ok = get(h.addr, &format!("/api/day?date={today}")).await;
    assert_eq!(ok.status, 200, "{}", ok.body);
    let v: serde_json::Value = serde_json::from_str(&ok.body).expect("json");
    assert_eq!(v["total"], 4);
    assert_eq!(v["headline"], format!("4 contributions on {today}"));
    assert_eq!(v["groups"][0]["heading"], "Commits");
    assert_eq!(v["groups"][1]["items"][0]["title"], "Add <b>pulse</b>");
    assert_eq!(v["groups"][1]["items"][0]["repo"], "github.com/o/r");

    let before = h.fake.requests().len();
    for bad in ["nonsense", "2999-01-01", "2001-01-01"] {
        let r = get(h.addr, &format!("/api/day?date={bad}")).await;
        assert_eq!(r.status, 400, "{bad}: {}", r.body);
    }
    assert_eq!(get(h.addr, "/api/day").await.status, 400);
    assert_eq!(
        h.fake.requests().len(),
        before,
        "refused before any platform call"
    );
}

#[tokio::test]
async fn the_calendar_cells_are_focusable_days_and_the_page_has_the_day_panel() {
    let h = start().await;
    h.fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"user":{"contributionsCollection":{"contributionCalendar":{"weeks":[]}}}}}"#,
    );
    let page = get(h.addr, "/pulse").await;
    for needle in [
        "class=\"day\" data-date=\"",
        "tabindex=\"0\"",
        "id=\"day\"",
        "function gkDay",
    ] {
        assert!(page.body.contains(needle), "{needle} missing");
    }
}

#[tokio::test]
async fn the_community_page_lists_reads_and_comments_on_discussions() {
    let h = start().await;
    h.fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"repository":{"hasDiscussionsEnabled":true,"discussions":{"nodes":[{"id":"D","number":5,"title":"How do I <clone>?","url":"u","body":"b","updatedAt":"2026-05-01T10:00:00Z","isAnswered":true,"category":{"name":"Q&A"},"author":{"login":"alice"},"commentCount":{"totalCount":2}}]}}}}"#,
    );
    let page = get(h.addr, "/community").await;
    assert_eq!(page.status, 200, "{}", page.body);
    for needle in [
        "Discussions &amp; wiki",
        "#5 How do I &lt;clone&gt;?",
        "Q&amp;A",
        "answered",
        "tab=discussions&amp;n=5",
    ] {
        assert!(page.body.contains(needle), "missing {needle:?}");
    }

    let h = start().await;
    h.fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"repository":{"discussion":{"id":"D","number":5,"title":"T","url":"u","body":"the question","updatedAt":"2026-05-01T10:00:00Z","isAnswered":false,"category":{"name":"Q&A"},"author":{"login":"alice"},"commentCount":{"totalCount":1},"comments":{"nodes":[{"databaseId":9,"body":"an <b>answer</b>","createdAt":"2026-05-01T11:00:00Z","author":{"login":"bob"}}]}}}}}"#,
    );
    let page = get(
        h.addr,
        "/community?repo=github.com%2Fo%2Fr&tab=discussions&n=5",
    )
    .await;
    assert!(
        page.body.contains("the question") && page.body.contains("an &lt;b&gt;answer&lt;/b&gt;")
    );
    assert!(page.body.contains("discussion_comment"), "a comment box");
}

#[tokio::test]
async fn the_wiki_tab_explains_that_github_has_no_wiki_api() {
    let h = start().await;
    let page = get(h.addr, "/community?tab=wiki").await;
    assert_eq!(page.status, 200);
    assert!(page.body.contains(".wiki.git"), "{}", page.body);
    assert!(h.fake.requests().is_empty());
}

#[tokio::test]
async fn the_boards_tab_lists_cards_with_their_column_and_opens_one_as_its_issue() {
    let h = start().await;
    h.fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"repository":{"projectsV2":{"nodes":[{"title":"Roadmap","field":{"options":[{"name":"Todo"},{"name":"Done"}]},"items":{"nodes":[{"fieldValueByName":{"name":"Todo"},"content":{"number":4,"title":"Add <it>","url":"u","assignees":{"nodes":[{"login":"alice"}]}}}]}}]}}}}"#,
    );
    let page = get(h.addr, "/community?tab=boards").await;
    assert_eq!(page.status, 200, "{}", page.body);
    for needle in [
        "#4 Add &lt;it&gt;",
        "Roadmap · Todo · alice",
        "tab=boards&amp;n=4",
    ] {
        assert!(page.body.contains(needle), "missing {needle:?}");
    }
    let h = start().await;
    h.fake.push_json(
        "https://api.github.com/repos/o/r/issues/4",
        r#"{"number":4,"title":"Add it","body":"card text","state":"open","user":{"login":"alice"},"html_url":"u"}"#,
    );
    h.fake.push_json(
        "https://api.github.com/repos/o/r/issues/4/comments",
        r#"[{"id":1,"body":"on it","user":{"login":"bob"}}]"#,
    );
    let page = get(h.addr, "/community?repo=github.com%2Fo%2Fr&tab=boards&n=4").await;
    assert!(
        page.body.contains("card text") && page.body.contains("on it"),
        "{}",
        page.body
    );
    assert!(
        !page.body.contains("discussion_comment"),
        "cards are commented on in their issue"
    );
}

const RUN5: &str = r#"{"id":5,"name":"CI","run_number":9,"status":"completed","conclusion":"failure","head_branch":"main","actor":{"login":"alice"},"created_at":"2026-03-01T10:00:00Z"}"#;
const JOBS5: &str =
    r#"{"jobs":[{"id":11,"name":"build","status":"completed","conclusion":"failure","steps":[]}]}"#;

#[tokio::test]
async fn a_failed_run_offers_explain_artifacts_and_a_way_to_start_a_workflow() {
    let h = start().await;
    let base = "https://api.github.com/repos/o/r/actions";
    h.fake.push_json(&format!("{base}/runs/5"), RUN5);
    h.fake.push_json(&format!("{base}/runs/5/jobs"), JOBS5);
    h.fake.push_json(
        &format!("{base}/runs/5/artifacts"),
        r#"{"artifacts":[{"id":3,"name":"dist","size_in_bytes":2048},{"id":4,"name":"old","expired":true}]}"#,
    );
    let page = get(h.addr, "/ci/run?repo=github.com%2Fo%2Fr&id=5").await;
    assert_eq!(page.status, 200, "{}", page.body);
    for needle in [
        "Explain failure",
        "&quot;action&quot;:&quot;explain&quot;",
        "Run a workflow",
        "/ci/dispatch?repo=github.com%2Fo%2Fr",
        "Artifacts",
        "dist",
        "2.0 KB",
        "id=\"ai\"",
    ] {
        assert!(
            page.body.contains(needle),
            "missing {needle:?}\n{}",
            page.body
        );
    }
    assert!(
        !page.body.contains(">old<"),
        "expired artifacts are not offered"
    );

    // A run that did not fail has nothing to explain.
    let h = start().await;
    h.fake.push_json(
        &format!("{base}/runs/6"),
        &RUN5
            .replace("\"id\":5", "\"id\":6")
            .replace("failure", "success"),
    );
    h.fake.push_json(
        &format!("{base}/runs/6/jobs"),
        &JOBS5.replace("failure", "success"),
    );
    let ok = get(h.addr, "/ci/run?repo=github.com%2Fo%2Fr&id=6").await;
    assert!(!ok.body.contains("Explain failure"), "{}", ok.body);
}

#[tokio::test]
async fn an_artifact_is_downloaded_as_a_zip_with_a_safe_file_name() {
    let h = start().await;
    let base = "https://api.github.com/repos/o/r/actions";
    h.fake.push_json(
        &format!("{base}/runs/5/artifacts"),
        r#"{"artifacts":[{"id":3,"name":"../dist","size_in_bytes":4}]}"#,
    );
    h.fake.push(
        &format!("{base}/artifacts/3/zip"),
        gitkraft_platforms::http::Response {
            status: 200,
            headers: vec![],
            body: b"PK\x03\x04".to_vec(),
        },
    );
    let r = get(h.addr, "/ci/artifact?repo=github.com%2Fo%2Fr&id=5&job=3").await;
    assert_eq!(r.status, 200, "{}", r.body);
    assert!(r.body.starts_with("PK"));
    let missing = get(h.addr, "/ci/artifact?repo=github.com%2Fo%2Fr&id=5&job=99").await;
    assert_eq!(
        missing.status, 404,
        "an id the run does not list is refused"
    );
}

#[tokio::test]
async fn the_dispatch_page_lists_workflows_and_the_action_starts_one() {
    let h = start().await;
    let base = "https://api.github.com/repos/o/r/actions";
    h.fake.push_json(
        &format!("{base}/workflows"),
        r#"{"workflows":[{"id":1,"name":"CI","path":".github/workflows/ci.yml","state":"active"},{"id":2,"name":"Old","path":".github/workflows/old.yml","state":"disabled_manually"}]}"#,
    );
    let page = get(h.addr, "/ci/dispatch?repo=github.com%2Fo%2Fr").await;
    assert_eq!(page.status, 200, "{}", page.body);
    assert!(page.body.contains("CI (ci.yml)"), "{}", page.body);
    assert!(
        !page.body.contains("old.yml"),
        "disabled workflows are not offered"
    );
    assert!(page.body.contains("gkDispatch()"));
    assert_eq!(get(h.addr, "/ci/dispatch?repo=nope").await.status, 404);

    h.fake
        .push_status(&format!("{base}/workflows/ci.yml/dispatches"), 204, "");
    let started = request(
        h.addr,
        "POST",
        "/api/action",
        r#"{"action":"dispatch","repo":"github.com/o/r","workflow":"ci.yml","git_ref":"main","inputs":"level=a"}"#,
    )
    .await;
    assert_eq!(started.status, 200, "{}", started.body);
    assert!(started.body.contains("started ci.yml on main"));
    let sent = h.fake.requests();
    let last = sent.last().expect("request");
    assert!(last.url.ends_with("/workflows/ci.yml/dispatches"));
    let body = String::from_utf8_lossy(last.body.as_deref().unwrap_or_default()).to_string();
    assert!(
        body.contains("\"ref\":\"main\"") && body.contains("\"level\":\"a\""),
        "{body}"
    );

    let n = h.fake.requests().len();
    for body in [
        r#"{"action":"dispatch","repo":"github.com/o/r","workflow":"ci.yml","git_ref":"my branch"}"#,
        r#"{"action":"dispatch","repo":"github.com/o/r","workflow":"ci.yml","git_ref":"main","inputs":"oops"}"#,
    ] {
        assert_eq!(
            request(h.addr, "POST", "/api/action", body).await.status,
            400,
            "{body}"
        );
    }
    assert_eq!(
        h.fake.requests().len(),
        n,
        "bad input never reaches the platform"
    );
}

#[tokio::test]
async fn discussion_comments_have_reply_buttons_and_a_reply_reaches_the_platform() {
    let h = start().await;
    h.fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"repository":{"discussion":{"id":"D","number":5,"title":"T","url":"u","body":"q","updatedAt":"2026-05-01T10:00:00Z","isAnswered":false,"category":{"name":"Q&A"},"author":{"login":"alice"},"commentCount":{"totalCount":2},"comments":{"nodes":[{"databaseId":9,"id":"C_9","body":"top","createdAt":"2026-05-01T11:00:00Z","author":{"login":"bob"},"replies":{"nodes":[{"databaseId":10,"body":"nested","createdAt":"2026-05-01T12:00:00Z","author":{"login":"carol"}}]}}]}}}}}"#,
    );
    let page = get(
        h.addr,
        "/community?repo=github.com%2Fo%2Fr&tab=discussions&n=5",
    )
    .await;
    assert_eq!(page.status, 200, "{}", page.body);
    assert!(
        page.body
            .contains("&quot;action&quot;:&quot;discussion_reply&quot;"),
        "{}",
        page.body
    );
    assert!(
        page.body.contains("&quot;comment_id&quot;:9")
            && page.body.contains("&quot;comment_id&quot;:10")
    );
    assert!(page.body.contains("comment reply"), "replies are indented");
    assert!(page.body.contains("data-ask=\"Your reply\""));

    let h = start().await;
    h.fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"repository":{"discussion":{"id":"D_1","comments":{"nodes":[{"databaseId":9,"id":"C_9","replies":{"nodes":[]}}]}}}}}"#,
    );
    h.fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"addDiscussionComment":{"comment":{"id":"n"}}}}"#,
    );
    let ok = request(
        h.addr,
        "POST",
        "/api/action",
        r#"{"action":"discussion_reply","repo":"github.com/o/r","number":5,"comment_id":9,"body":"thanks"}"#,
    )
    .await;
    assert_eq!(ok.status, 200, "{}", ok.body);
    let sent = String::from_utf8_lossy(
        h.fake
            .requests()
            .last()
            .and_then(|r| r.body.clone())
            .unwrap_or_default()
            .as_slice(),
    )
    .into_owned();
    assert!(sent.contains("replyToId") && sent.contains("C_9"), "{sent}");
    let empty = request(
        h.addr,
        "POST",
        "/api/action",
        r#"{"action":"discussion_reply","repo":"github.com/o/r","number":5,"comment_id":9,"body":" "}"#,
    )
    .await;
    assert_eq!(empty.status, 400);
}

#[tokio::test]
async fn board_cards_have_a_move_button_and_a_move_sets_the_status() {
    let boards = r#"{"data":{"repository":{"projectsV2":{"nodes":[{"id":"P","title":"Roadmap","field":{"id":"F","options":[{"id":"O1","name":"Todo"},{"id":"O2","name":"Done"}]},"items":{"nodes":[{"id":"I4","fieldValueByName":{"name":"Todo"},"content":{"number":4,"title":"Add it","url":"u","assignees":{"nodes":[]}}}]}}]}}}}"#;
    let h = start().await;
    h.fake.push_json("https://api.github.com/graphql", boards);
    let page = get(h.addr, "/community?tab=boards").await;
    assert!(
        page.body
            .contains("&quot;action&quot;:&quot;move_card&quot;"),
        "{}",
        page.body
    );
    assert!(page.body.contains("data-field=\"column\"") && page.body.contains("Move…"));

    let h = start().await;
    h.fake.push_json("https://api.github.com/graphql", boards);
    h.fake.push_json(
        "https://api.github.com/graphql",
        r#"{"data":{"updateProjectV2ItemFieldValue":{"projectV2Item":{"id":"I4"}}}}"#,
    );
    let ok = request(
        h.addr,
        "POST",
        "/api/action",
        r#"{"action":"move_card","repo":"github.com/o/r","number":4,"column":"don","board":"Roadmap"}"#,
    )
    .await;
    assert_eq!(ok.status, 200, "{}", ok.body);
    assert!(
        ok.body.contains("#4 moved to Done on Roadmap"),
        "{}",
        ok.body
    );
    let sent = String::from_utf8_lossy(
        h.fake
            .requests()
            .last()
            .and_then(|r| r.body.clone())
            .unwrap_or_default()
            .as_slice(),
    )
    .into_owned();
    assert!(
        sent.contains("\"o\":\"O2\"") && sent.contains("\"i\":\"I4\""),
        "{sent}"
    );
}

#[tokio::test]
async fn a_wiki_save_is_refused_on_github_with_the_reason_and_validated_before_any_call() {
    let h = start().await;
    let r = request(
        h.addr,
        "POST",
        "/api/action",
        r#"{"action":"wiki_save","repo":"github.com/o/r","slug":"Home","title":"Home","content":"Hi"}"#,
    )
    .await;
    assert_eq!(r.status, 400);
    assert!(r.body.contains("git"), "{}", r.body);
    let blank = request(
        h.addr,
        "POST",
        "/api/action",
        r#"{"action":"wiki_save","repo":"github.com/o/r","title":" ","content":"x"}"#,
    )
    .await;
    assert!(blank.body.contains("title"), "{}", blank.body);
    assert!(h.fake.requests().is_empty(), "nothing reached the platform");
}

async fn start_gitea() -> Harness {
    let fake = FakeClient::new();
    let mut account =
        Account::new(PlatformKind::Gitea, "codeberg.org").with_token(Secret::new("t"));
    account.me = Some("me".into());
    let hub = Hub::new(vec![account], &DynClient::new(fake.clone()));
    let dashboard = Dashboard {
        sections: vec![section("Mine", SectionKind::Prs, &["codeberg.org/o/r"])],
    };
    let dir = tempfile::tempdir().expect("tmp");
    let paths = Paths {
        dashboard: None,
        notify: Some(dir.path().join("notifications.toml")),
        use_state_dir: false,
        ssh_dir: None,
        checkouts: Vec::new(),
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let session = Session::new(hub, dashboard, paths, 0);
    TOKENS
        .lock()
        .expect("lock")
        .push((addr, session.token().to_string()));
    tokio::spawn(async move {
        let _ = topcoat::serve(listener, router(session)).await;
    });
    Harness { addr, fake, dir }
}

#[tokio::test]
async fn a_wiki_page_opens_in_an_editor_and_saving_it_patches_the_page_new_pages_are_posted() {
    let api = "https://codeberg.org/api/v1/repos/o/r/wiki";
    let h = start_gitea().await;
    h.fake.push_json(
        &format!("{api}/pages"),
        r#"[{"title":"Home","sub_url":"Home"}]"#,
    );
    let list = get(h.addr, "/community?tab=wiki").await;
    assert!(
        list.body.contains("New page") && list.body.contains("wiki-new"),
        "{}",
        list.body
    );
    assert!(list.body.contains("Home"));

    // "# Hi <b>" in base64
    h.fake.push_json(
        &format!("{api}/page/Home"),
        r#"{"title":"Home","sub_url":"Home","content_base64":"IyBIaSA8Yj4="}"#,
    );
    let page = get(
        h.addr,
        "/community?repo=codeberg.org%2Fo%2Fr&tab=wiki&page=Home",
    )
    .await;
    assert_eq!(page.status, 200, "{}", page.body);
    assert!(page.body.contains("<textarea id=\"wiki\""), "{}", page.body);
    assert!(
        page.body.contains("# Hi &lt;b&gt;"),
        "escaped: {}",
        page.body
    );
    assert!(
        page.body.contains("Save page")
            && page
                .body
                .contains("&quot;action&quot;:&quot;wiki_save&quot;")
    );

    h.fake.push_json(&format!("{api}/page/Home"), "{}");
    let saved = request(
        h.addr,
        "POST",
        "/api/action",
        r#"{"action":"wiki_save","repo":"codeberg.org/o/r","slug":"Home","title":"Home","content":"changed"}"#,
    )
    .await;
    assert_eq!(saved.status, 200, "{}", saved.body);
    let last = h.fake.requests().pop().expect("request");
    assert_eq!(last.method, gitkraft_platforms::http::Method::Patch);

    h.fake.push_json(&format!("{api}/new"), "{}");
    let created = request(
        h.addr,
        "POST",
        "/api/action",
        r#"{"action":"wiki_save","repo":"codeberg.org/o/r","title":"Roadmap","content":"later"}"#,
    )
    .await;
    assert_eq!(created.status, 200, "{}", created.body);
    assert_eq!(
        h.fake.requests().pop().expect("request").method,
        gitkraft_platforms::http::Method::Post
    );
}

#[tokio::test]
async fn a_project_on_two_forges_gets_a_remote_switcher_and_rows_name_their_forge() {
    let root = tempfile::tempdir().expect("tmp");
    let mine = root.path().join("r");
    std::fs::create_dir_all(&mine).expect("mk");
    git(&mine, &["init", "-q"]);
    git(
        &mine,
        &["remote", "add", "origin", "git@github.com:o/r.git"],
    );
    git(
        &mine,
        &[
            "remote",
            "add",
            "codeberg",
            "ssh://git@codeberg.org/o/r.git",
        ],
    );
    let h = start_with(vec![mine.clone()]).await;
    h.fake
        .push_json("https://api.github.com/repos/o/r/pulls", PULLS);
    let all = get(h.addr, "/").await;
    assert_eq!(all.status, 200);
    for needle in [
        "class=\"scope\"",
        "all remotes",
        "href=\"/?s=0&amp;r=origin\"",
        "href=\"/?s=0&amp;r=codeberg\"",
        "o/r @github",
    ] {
        assert!(
            all.body.contains(needle),
            "missing {needle:?} in {}",
            all.body
        );
    }
    // only the other forge: the GitHub rows are filtered out
    h.fake
        .push_json("https://api.github.com/repos/o/r/pulls", PULLS);
    let cb = get(h.addr, "/?s=0&r=codeberg").await;
    assert!(!cb.body.contains("Second"), "{}", cb.body);
    assert!(cb.body.contains("Nothing here."), "{}", cb.body);
    assert!(
        cb.body.contains("class=\"on\""),
        "the shown remote is marked: {}",
        cb.body
    );
    // an unknown remote shows everything
    h.fake
        .push_json("https://api.github.com/repos/o/r/pulls", PULLS);
    let odd = get(h.addr, "/?r=nope").await;
    assert!(odd.body.contains("Second"));
}

#[tokio::test]
async fn a_single_forge_project_has_no_switcher() {
    let root = tempfile::tempdir().expect("tmp");
    let mine = root.path().join("r");
    std::fs::create_dir_all(&mine).expect("mk");
    git(&mine, &["init", "-q"]);
    git(
        &mine,
        &["remote", "add", "origin", "git@github.com:o/r.git"],
    );
    let h = start_with(vec![mine]).await;
    h.fake
        .push_json("https://api.github.com/repos/o/r/pulls", PULLS);
    let page = get(h.addr, "/").await;
    assert!(
        !page.body.contains("all remotes") && !page.body.contains("@github"),
        "{}",
        page.body
    );
}
