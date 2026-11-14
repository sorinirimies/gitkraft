//! The pages. Each is a plain GET; the data comes from [`Session`], the markup is Topcoat.

use chrono::{Duration, Utc};
use gitkraft_ci::annotations::{Severity, extract};
use gitkraft_ci::log::parse_log;
use gitkraft_ci::model::Status;
use gitkraft_ci::timing::{format_seconds, gantt, job_seconds};
use gitkraft_hub::SectionData;
use gitkraft_platforms::model::{
    ChecksState, ItemState, NotificationReason, ReviewVerdict, SubjectKind,
};
use gitkraft_platforms::{RepoId, url_encode};
use gitkraft_settings::notify::{StatusInput, status};
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        Body,
        error::not_found,
        page, query_params,
        response::{IntoResponse, Response},
        route,
    },
    view::{Child, Unescaped, View, component, view},
};

use crate::render;
use crate::session::Session;
use crate::style::SCRIPT;

/// One entry of the top navigation.
#[derive(Debug, Clone)]
pub struct NavItem {
    /// Link target.
    pub href: String,
    /// Text.
    pub label: String,
    /// The current page.
    pub on: bool,
}

/// One row of a dashboard list, whatever the section kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Where the row links to.
    pub href: String,
    /// A one-character state mark.
    pub mark: &'static str,
    /// CSS class of the mark.
    pub class: &'static str,
    /// Main text.
    pub title: String,
    /// Secondary text.
    pub meta: String,
}

fn item_href(path: &str, repo: &RepoId, number: u64) -> String {
    format!("{path}?repo={}&n={number}", url_encode(&repo.key()))
}

fn ago(dt: Option<chrono::DateTime<Utc>>) -> String {
    dt.map(gitkraft_core_relative).unwrap_or_default()
}

fn gitkraft_core_relative(dt: chrono::DateTime<Utc>) -> String {
    let d = Utc::now() - dt;
    match () {
        () if d < Duration::minutes(1) => "just now".into(),
        () if d < Duration::hours(1) => format!("{}m ago", d.num_minutes()),
        () if d < Duration::days(1) => format!("{}h ago", d.num_hours()),
        () if d < Duration::days(60) => format!("{}d ago", d.num_days()),
        () => format!("{}mo ago", d.num_days() / 30),
    }
}

fn state_mark(s: ItemState) -> (&'static str, &'static str) {
    match s {
        ItemState::Open => ("●", "ok"),
        ItemState::Merged => ("●", "acc"),
        ItemState::Closed => ("●", "err"),
    }
}

fn run_mark(s: Status) -> (&'static str, &'static str) {
    let class = match s {
        Status::Success => "ok",
        Status::Failed => "err",
        Status::Running => "warn",
        Status::Cancelled | Status::Skipped => "mu",
        Status::Queued | Status::Waiting | Status::Manual => "acc",
    };
    (s.glyph(false), class)
}

/// The rows of a section.
pub fn rows_of(data: &SectionData) -> Vec<Row> {
    match data {
        SectionData::Prs(r) => r
            .items
            .iter()
            .map(|p| {
                let (mark, class) = match (p.state, p.checks) {
                    (ItemState::Open, Some(ChecksState::Failure)) => ("✘", "err"),
                    (ItemState::Open, Some(ChecksState::Pending)) => ("●", "warn"),
                    (ItemState::Open, _) if p.draft => ("◌", "mu"),
                    (s, _) => state_mark(s),
                };
                Row {
                    href: item_href("/pr", &p.repo, p.number),
                    mark,
                    class,
                    title: format!("#{} {}", p.number, p.title),
                    meta: format!(
                        "{} · {} · {}",
                        p.repo.full_name(),
                        p.author.login,
                        ago(p.updated_at)
                    ),
                }
            })
            .collect(),
        SectionData::Issues(r) => r
            .items
            .iter()
            .map(|i| {
                let (mark, class) = state_mark(i.state);
                Row {
                    href: item_href("/issue", &i.repo, i.number),
                    mark,
                    class,
                    title: format!("#{} {}", i.number, i.title),
                    meta: format!(
                        "{} · {} · {}",
                        i.repo.full_name(),
                        i.author.login,
                        ago(i.updated_at)
                    ),
                }
            })
            .collect(),
        SectionData::Runs(r) => r
            .items
            .iter()
            .map(|run| {
                let (mark, class) = run_mark(run.status);
                Row {
                    href: format!("/ci/run?repo={}&id={}", url_encode(&run.repo.key()), run.id),
                    mark,
                    class,
                    title: format!(
                        "{} #{} · {} · {}",
                        run.name, run.number, run.branch, run.title
                    ),
                    meta: format!("{} · {}", run.actor, ago(run.created_at)),
                }
            })
            .collect(),
        SectionData::Inbox(r) => r
            .items
            .iter()
            .map(|n| {
                let (mark, class) = if n.unread {
                    ("●", "acc")
                } else {
                    ("○", "mu")
                };
                let href = match (&n.repo, n.number, n.kind) {
                    (Some(repo), Some(num), SubjectKind::PullRequest) => {
                        item_href("/pr", repo, num)
                    }
                    (Some(repo), Some(num), SubjectKind::Issue) => item_href("/issue", repo, num),
                    _ => n.url.clone(),
                };
                let why = match n.reason {
                    NotificationReason::ReviewRequested => "review requested",
                    NotificationReason::Mention => "mention",
                    NotificationReason::Assigned => "assigned",
                    NotificationReason::Comment => "comment",
                    NotificationReason::CiActivity => "CI",
                    NotificationReason::Subscribed | NotificationReason::Other => "watching",
                };
                Row {
                    href,
                    mark,
                    class,
                    title: n.title.clone(),
                    meta: format!(
                        "{why} · {} · {}",
                        n.repo.as_ref().map(RepoId::full_name).unwrap_or_default(),
                        ago(n.updated_at)
                    ),
                }
            })
            .collect(),
    }
}

fn nav(session: &Session, active: &str) -> Vec<NavItem> {
    let mut items: Vec<NavItem> = session
        .dashboard()
        .visible()
        .map(|(i, s)| NavItem {
            href: format!("/?s={i}"),
            label: s.name.clone(),
            on: active == format!("s{i}"),
        })
        .collect();
    for (key, href, label) in [
        ("pulse", "/pulse", "Pulse"),
        ("inbox", "/inbox", "Inbox"),
        ("community", "/community", "Discussions & wiki"),
        ("keys", "/keys", "SSH keys"),
        ("notify", "/notify", "🔔 Notifications"),
    ] {
        items.push(NavItem {
            href: href.into(),
            label: label.into(),
            on: active == key,
        });
    }
    items
}

/// The page frame.
#[component]
pub(crate) async fn shell(
    title: String,
    nav: Vec<NavItem>,
    vars: String,
    token: String,
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <meta name="gk-token" content=(token)>
                <title>(title)</title>
                <style>(vars)</style>
                <link rel="stylesheet" href="/style.css">
                (Unescaped::new_unchecked(format!("<script>{SCRIPT}</script>")))
            </head>
            <body>
                <header class="top">
                    <span class="brand">"GitKraft"</span>
                    <nav>
                        for n in nav {
                            <a href=(n.href) class=(if n.on { "on" } else { "" })>(n.label)</a>
                        }
                    </nav>
                </header>
                <main>(child)</main>
            </body>
        </html>
    })
}

#[derive(Debug)]
#[topcoat::router::query_params(error = redirect("/"))]
pub struct SectionParams {
    /// Index into the dashboard.
    pub s: Option<String>,
}

#[page("/")]
async fn dashboard(cx: &Cx) -> Result<impl View> {
    let params = query_params::<SectionParams>(cx)?;
    let session: &Session = app_context(cx);
    let dash = session.dashboard();
    let first = dash.visible().next().map(|(i, _)| i);
    let index = params
        .s
        .as_deref()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|i| dash.sections.get(*i).is_some_and(|s| !s.hidden))
        .or(first);
    let (name, rows, errors) = match index {
        Some(i) => {
            let data = session.section_data(i).await;
            (
                dash.sections[i].name.clone(),
                data.as_ref().map(rows_of).unwrap_or_default(),
                data.map(|d| d.errors().to_vec()).unwrap_or_default(),
            )
        }
        None => (
            "Dashboard".into(),
            Vec::new(),
            vec!["No sections: add some to dashboard.toml".to_string()],
        ),
    };
    let empty = rows.is_empty() && errors.is_empty();
    let active = index.map(|i| format!("s{i}")).unwrap_or_default();
    Ok(view! {
        shell(title: format!("{name} · GitKraft"), nav: nav(session, &active), vars: render::css_vars(&session.theme()), token: session.token().to_string(),
            <h1>(name.clone())</h1>
            for e in errors { <div class="note err">(e)</div> }
            if empty { <p class="dim">"Nothing here."</p> }
            <table>
                for r in rows {
                    <tr>
                        <td class="mark"><span class=(r.class)>(r.mark)</span></td>
                        <td><a href=(r.href)>(r.title)</a><div class="meta small">(r.meta)</div></td>
                    </tr>
                }
            </table>
        )
    })
}

#[derive(Debug)]
#[topcoat::router::query_params(error = redirect("/"))]
pub struct ItemParams {
    /// `host/owner/name`.
    pub repo: Option<String>,
    /// Number.
    pub n: Option<String>,
    /// PR tab.
    pub tab: Option<String>,
    /// Run id.
    pub id: Option<String>,
    /// Job id.
    pub job: Option<String>,
    /// Days.
    pub days: Option<String>,
    /// Wiki page slug.
    pub page: Option<String>,
}

fn parse_item(p: &ItemParams) -> Option<(RepoId, u64)> {
    Some((
        RepoId::from_key(p.repo.as_deref()?)?,
        p.n.as_deref()?.parse().ok()?,
    ))
}

/// A line of a diff with its CSS class.
pub fn diff_class(l: &str) -> &'static str {
    if l.starts_with("+++") || l.starts_with("---") || l.starts_with("diff ") {
        "ctx"
    } else if l.starts_with('+') {
        "add"
    } else if l.starts_with('-') {
        "del"
    } else if l.starts_with("@@") {
        "hunk"
    } else {
        "ctx"
    }
}

/// Lines of the diff shown at most (the rest is cut with a note).
const DIFF_LINES: usize = 4000;

fn act_json(v: serde_json::Value) -> String {
    v.to_string()
}

#[page("/pr")]
async fn pull_request(cx: &Cx) -> Result<impl View> {
    let params = query_params::<ItemParams>(cx)?;
    let session: &Session = app_context(cx);
    let (repo, number) = parse_item(params).ok_or_else(not_found)?;
    let detail = session
        .hub()
        .pr_detail(&repo, number)
        .await
        .map_err(|_| not_found())?;
    let tab = params.tab.clone().unwrap_or_else(|| "overview".into());
    let pr = detail.pr.clone();
    let key = repo.key();
    let base = format!("/pr?repo={}&n={number}", url_encode(&key));
    let state_word = match (pr.state, pr.draft) {
        (ItemState::Open, true) => "draft",
        (ItemState::Open, false) => "open",
        (ItemState::Merged, _) => "merged",
        (ItemState::Closed, _) => "closed",
    };
    let checks = match pr.checks {
        Some(ChecksState::Success) => "✔ checks passed",
        Some(ChecksState::Failure) => "✘ checks failed",
        Some(ChecksState::Pending) => "● checks running",
        None => "",
    };
    let reviews: Vec<String> = detail
        .reviews
        .iter()
        .map(|r| {
            let what = match r.verdict {
                Some(ReviewVerdict::Approve) => "approved",
                Some(ReviewVerdict::RequestChanges) => "requested changes",
                _ => "commented",
            };
            format!("{} {what}", r.author.login)
        })
        .collect();
    let mut diff: Vec<(&'static str, String)> = detail
        .diff
        .lines()
        .take(DIFF_LINES)
        .map(|l| (diff_class(l), l.to_string()))
        .collect();
    if detail.diff.lines().count() > DIFF_LINES {
        diff.push(("hunk", format!("… diff cut after {DIFF_LINES} lines")));
    }
    let images = gitkraft_hub::markdown::image_urls(&format!(
        "{}\n{}",
        pr.body,
        detail
            .comments
            .iter()
            .map(|c| c.body.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    ));
    let open = pr.state == ItemState::Open;
    let target = serde_json::json!({ "repo": key, "number": number });
    let with = |extra: serde_json::Value| {
        let mut v = target.clone();
        if let (Some(o), Some(e)) = (v.as_object_mut(), extra.as_object()) {
            o.extend(e.clone());
        }
        act_json(v)
    };
    let approve = with(serde_json::json!({"action": "review", "verdict": "approve"}));
    let changes = with(serde_json::json!({"action": "review", "verdict": "request_changes"}));
    let comment = with(serde_json::json!({"action": "comment", "kind": "pr"}));
    let merge = with(serde_json::json!({"action": "merge", "method": "merge"}));
    let squash = with(serde_json::json!({"action": "merge", "method": "squash"}));
    let with_method = |when: &str, force: bool| {
        with(serde_json::json!({"action": "merge", "when": when, "force": force}))
    };
    let when_green = with_method("when_green", false);
    let queue = with_method("queue", false);
    let force = with_method("now", true);
    let update = with(serde_json::json!({"action": "update_branch"}));
    let cancel_auto = with(serde_json::json!({"action": "cancel_auto_merge"}));
    let worktree = with(serde_json::json!({"action": "worktree"}));
    let toggle = with(serde_json::json!({"action": "set_open", "kind": "pr", "open": !open}));
    let toggle_label = if open { "Close" } else { "Reopen" };
    let labels: Vec<String> = pr.labels.iter().map(|l| l.name.clone()).collect();
    Ok(view! {
        shell(title: format!("{} · #{number}", pr.title), nav: nav(session, ""), vars: render::css_vars(&session.theme()), token: session.token().to_string(),
            <h1>(pr.title.clone())</h1>
            <div class="meta">
                <span class=(format!("badge {state_word}"))>(state_word)</span>
                " "(format!("{}#{number} by {} · {} → {}", repo.full_name(), pr.author.login, pr.source_branch, pr.target_branch))
                " "<span>(checks)</span>
                if pr.mergeable == Some(false) { " "<span class="err">"⚠ conflicts"</span> }
                else if pr.behind == Some(true) { " "<span class="warn">"↓ behind target · update branch"</span> }
                if let (Some(a), Some(d)) = (pr.additions, pr.deletions) { " "<span class="mu">(format!("+{a} −{d}"))</span> }
            </div>
            if !labels.is_empty() { <div class="meta small">"labels: "(labels.join(", "))</div> }
            for e in detail.errors.clone() { <div class="note warn">(e)</div> }
            <div class="bar">
                <button class="primary" data-act=(approve) data-from="note" onclick="actBtn(this)">"Approve"</button>
                <button data-act=(changes) data-from="note" onclick="actBtn(this)">"Request changes"</button>
                if open {
                    <button data-act=(merge) data-confirm="Merge this pull request?" onclick="actBtn(this)">"Merge"</button>
                    <button data-act=(squash) data-confirm="Squash and merge?" onclick="actBtn(this)">"Squash"</button>
                    <select id="mm" title="Method for the buttons that follow">
                        <option value="merge">"merge commit"</option>
                        <option value="squash">"squash"</option>
                        <option value="rebase">"rebase"</option>
                    </select>
                    <button data-act=(when_green) data-method="mm" data-confirm="Merge when the checks pass?" onclick="actBtn(this)">"When green"</button>
                    <button data-act=(queue) data-method="mm" data-confirm="Add to the merge queue?" onclick="actBtn(this)">"Queue"</button>
                    <button class="danger" data-act=(force) data-method="mm" data-confirm="FORCE merge past branch rules? This needs admin rights and skips required checks and reviews." onclick="actBtn(this)">"Force merge"</button>
                    <button data-act=(update) data-confirm="Update this branch with its target branch?" onclick="actBtn(this)">"Update branch"</button>
                    <button data-act=(cancel_auto) data-confirm="Cancel the scheduled merge or leave the merge queue?" onclick="actBtn(this)">"Cancel auto-merge"</button>
                }
                <button data-act=(worktree) title="Fetch this pull request into a worktree next to your local checkout" onclick="actBtn(this)">"Check out"</button>
                <button class="danger" data-act=(toggle) data-confirm=(format!("{toggle_label} this pull request?")) onclick="actBtn(this)">(toggle_label)</button>
            </div>
            <nav class="tabs">
                <a href=(format!("{base}&tab=overview")) class=(if tab == "overview" { "on" } else { "" })>"Overview"</a>
                <a href=(format!("{base}&tab=diff")) class=(if tab == "diff" { "on" } else { "" })>"Diff"</a>
                <a href=(format!("{base}&tab=comments")) class=(if tab == "comments" { "on" } else { "" })>(format!("Comments {}", detail.comments.len()))</a>
            </nav>
            if tab == "diff" {
                <div class="diff">
                    for (class, text) in diff { <div class=(class)>(text)</div> }
                </div>
            } else if tab == "comments" {
                for c in detail.comments.clone() {
                    <div class="comment"><b>(c.author.login.clone())</b>" "<span class="meta small">(ago(c.created_at))</span><pre>(c.body.clone())</pre></div>
                }
                <h2>"Write a comment"</h2>
                <textarea id="note" placeholder="Comment, or the note for a review"></textarea>
                <div class="bar"><button class="primary" data-act=(comment) data-from="note" onclick="actBtn(this)">"Comment"</button></div>
            } else {
                for r in reviews { <div>(r)</div> }
                <div class="panel"><pre>(if pr.body.trim().is_empty() { "No description.".to_string() } else { pr.body.clone() })</pre></div>
                for url in images { <img class="shot" loading="lazy" referrerpolicy="no-referrer" src=(url)> }
                <h2>"Note for a review"</h2>
                <textarea id="note" placeholder="Optional for approving, required for requesting changes"></textarea>
            }
        )
    })
}

#[page("/issue")]
async fn issue(cx: &Cx) -> Result<impl View> {
    let params = query_params::<ItemParams>(cx)?;
    let session: &Session = app_context(cx);
    let (repo, number) = parse_item(params).ok_or_else(not_found)?;
    let backend = session.hub().backend(&repo.host).ok_or_else(not_found)?;
    let issue = backend
        .issue(&repo, number)
        .await
        .map_err(|_| not_found())?;
    let comments = backend
        .comments(&repo, gitkraft_platforms::ItemKind::Issue, number)
        .await
        .unwrap_or_default();
    let open = issue.state == ItemState::Open;
    let key = repo.key();
    let comment = act_json(
        serde_json::json!({"action": "comment", "repo": key, "number": number, "kind": "issue"}),
    );
    let toggle = act_json(
        serde_json::json!({"action": "set_open", "repo": key, "number": number, "kind": "issue", "open": !open}),
    );
    let images = gitkraft_hub::markdown::image_urls(&format!(
        "{}\n{}",
        issue.body,
        comments
            .iter()
            .map(|c| c.body.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    ));
    let labels: Vec<String> = issue.labels.iter().map(|l| l.name.clone()).collect();
    Ok(view! {
        shell(title: format!("{} · #{number}", issue.title), nav: nav(session, ""), vars: render::css_vars(&session.theme()), token: session.token().to_string(),
            <h1>(issue.title.clone())</h1>
            <div class="meta">
                <span class=(if open { "badge open" } else { "badge closed" })>(if open { "open" } else { "closed" })</span>
                " "(format!("{}#{number} by {}", repo.full_name(), issue.author.login))
            </div>
            if !labels.is_empty() { <div class="meta small">"labels: "(labels.join(", "))</div> }
            <div class="panel"><pre>(if issue.body.trim().is_empty() { "No description.".to_string() } else { issue.body.clone() })</pre></div>
            for url in images { <img class="shot" loading="lazy" referrerpolicy="no-referrer" src=(url)> }
            for c in comments { <div class="comment"><b>(c.author.login.clone())</b>" "<span class="meta small">(ago(c.created_at))</span><pre>(c.body.clone())</pre></div> }
            <textarea id="note" placeholder="Write a comment"></textarea>
            <div class="bar">
                <button class="primary" data-act=(comment) data-from="note" onclick="actBtn(this)">"Comment"</button>
                <button class="danger" data-act=(toggle) data-confirm="Change the state of this issue?" onclick="actBtn(this)">(if open { "Close" } else { "Reopen" })</button>
            </div>
        )
    })
}

fn parse_run(p: &ItemParams) -> Option<(RepoId, u64)> {
    Some((
        RepoId::from_key(p.repo.as_deref()?)?,
        p.id.as_deref()?.parse().ok()?,
    ))
}

#[page("/ci/run")]
async fn ci_run(cx: &Cx) -> Result<impl View> {
    let params = query_params::<ItemParams>(cx)?;
    let session: &Session = app_context(cx);
    let (repo, id) = parse_run(params).ok_or_else(not_found)?;
    let ci = session.hub().ci(&repo.host).ok_or_else(not_found)?;
    let detail = ci.run_detail(&repo, id).await.map_err(|_| not_found())?;
    let run = detail.run.clone();
    let key = repo.key();
    let (mark, class) = run_mark(run.status);
    let timeline = Unescaped::new_unchecked(render::gantt_svg(&gantt(&detail)));
    let jobs: Vec<(String, &'static str, &'static str, String, String)> = detail
        .jobs
        .iter()
        .map(|j| {
            let (m, c) = run_mark(j.status);
            (
                format!("/ci/log?repo={}&job={}&id={id}", url_encode(&key), j.id),
                m,
                c,
                format!(
                    "{}{}",
                    j.stage
                        .as_deref()
                        .map(|s| format!("{s} · "))
                        .unwrap_or_default(),
                    j.name
                ),
                job_seconds(j).map(format_seconds).unwrap_or_default(),
            )
        })
        .collect();
    let rerun_failed = act_json(
        serde_json::json!({"action": "rerun", "repo": key, "run_id": id, "branch": run.branch, "scope": "failed"}),
    );
    let rerun_all = act_json(
        serde_json::json!({"action": "rerun", "repo": key, "run_id": id, "branch": run.branch, "scope": "all"}),
    );
    let cancel = act_json(serde_json::json!({"action": "cancel", "repo": key, "run_id": id}));
    let active = run.status.is_active();
    let failed = detail.jobs.iter().any(|j| j.status == Status::Failed);
    let explain = act_json(serde_json::json!({"action": "explain", "repo": key, "run_id": id}));
    let dispatch_href = format!("/ci/dispatch?repo={}", url_encode(&key));
    // Platforms without artifacts (Woodpecker) answer with an error: the section is just absent.
    let artifacts: Vec<(String, String, String)> = ci
        .artifacts(&repo, id)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|a| !a.expired)
        .map(|a| {
            (
                format!(
                    "/ci/artifact?repo={}&id={}&job={}",
                    url_encode(&key),
                    id,
                    a.id
                ),
                a.name,
                format_bytes(a.size),
            )
        })
        .collect();
    Ok(view! {
        shell(title: format!("{} #{}", run.name, run.number), nav: nav(session, ""), vars: render::css_vars(&session.theme()), token: session.token().to_string(),
            <h1><span class=(class)>(mark)</span>" "(format!("{} #{}", run.name, run.number))</h1>
            <div class="meta">(format!("{} · {} · {} · {}", repo.full_name(), run.branch, run.event, run.actor))</div>
            <div class="meta small">(run.title.clone())</div>
            <div class="bar">
                <button data-act=(rerun_failed) data-confirm="Re-run the failed jobs?" onclick="actBtn(this)">"Re-run failed"</button>
                <button data-act=(rerun_all) data-confirm="Re-run every job?" onclick="actBtn(this)">"Re-run all"</button>
                if active { <button class="danger" data-act=(cancel) data-confirm="Cancel this run?" onclick="actBtn(this)">"Cancel"</button> }
                if failed { <button data-act=(explain) data-show="ai" title="Ask the AI why it failed (a draft)" onclick="actBtn(this)">"Explain failure"</button> }
                <a class="btn" href=(dispatch_href)>"Run a workflow…"</a>
                if active { <meta http-equiv="refresh" content="10"> }
            </div>
            <pre id="ai" class="panel" hidden="hidden"></pre>
            <h2>"Timeline"</h2>
            <div class="panel">(timeline)</div>
            if !artifacts.is_empty() {
                <h2>"Artifacts"</h2>
                <table>
                    for (href, name, size) in artifacts {
                        <tr><td><a href=(href)>(name)</a></td><td class="meta">(size)</td></tr>
                    }
                </table>
            }
            <h2>"Jobs"</h2>
            <table>
                for (href, m, c, name, secs) in jobs {
                    <tr><td class="mark"><span class=(c)>(m)</span></td><td><a href=(href)>(name)</a></td><td class="meta">(secs)</td></tr>
                }
            </table>
        )
    })
}

/// `12.3 MB`.
fn format_bytes(n: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut v = n as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{n} B")
    } else {
        format!("{v:.1} {}", UNITS[u])
    }
}

/// The form that starts a workflow by hand.
#[page("/ci/dispatch")]
async fn ci_dispatch(cx: &Cx) -> Result<impl View> {
    let params = query_params::<ItemParams>(cx)?;
    let session: &Session = app_context(cx);
    let repo =
        RepoId::from_key(params.repo.as_deref().unwrap_or_default()).ok_or_else(not_found)?;
    let ci = session.hub().ci(&repo.host).ok_or_else(not_found)?;
    let workflows: Vec<(String, String)> = ci
        .list_workflows(&repo)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|w| w.active)
        .map(|w| (w.file, w.name))
        .collect();
    let key = repo.key();
    let back = format!("/ci/dispatch?repo={}", url_encode(&key));
    Ok(view! {
        shell(title: format!("Run a workflow · {}", repo.full_name()), nav: nav(session, ""), vars: render::css_vars(&session.theme()), token: session.token().to_string(),
            <h1>"Run a workflow"</h1>
            <div class="meta">(repo.full_name())" · "<a href=(back)>"reload"</a></div>
            <div class="panel">
                <input type="hidden" id="d-repo" value=(key)>
                if workflows.is_empty() {
                    <div class="meta">"No workflow files: this platform starts a pipeline on a branch."</div>
                } else {
                    <label>"Workflow "
                        <select id="d-workflow">
                            for (file, name) in workflows { <option value=(file.clone())>(format!("{name} ({file})"))</option> }
                        </select>
                    </label>
                }
                <label>"Branch or tag "<input id="d-ref" value="main"></label>
                <label>"Inputs (key=value, one per line)"<textarea id="d-inputs" rows="4"></textarea></label>
                <button class="primary" onclick="gkDispatch()">"Start"</button>
            </div>
        )
    })
}

/// Downloads an artifact zip of a run.
#[route(GET "/ci/artifact")]
async fn ci_artifact(cx: &Cx) -> Result<ZipDownload> {
    let params = query_params::<ItemParams>(cx)?;
    let session: &Session = app_context(cx);
    let repo =
        RepoId::from_key(params.repo.as_deref().unwrap_or_default()).ok_or_else(not_found)?;
    let run: u64 = params
        .id
        .as_deref()
        .and_then(|j| j.parse().ok())
        .ok_or_else(not_found)?;
    let artifact: u64 = params
        .job
        .as_deref()
        .and_then(|j| j.parse().ok())
        .ok_or_else(not_found)?;
    let ci = session.hub().ci(&repo.host).ok_or_else(not_found)?;
    let name = ci
        .artifacts(&repo, run)
        .await
        .map_err(|_| not_found())?
        .into_iter()
        .find(|a| a.id == artifact)
        .map(|a| a.name)
        .ok_or_else(not_found)?;
    let bytes = ci
        .download_artifact(&repo, artifact)
        .await
        .map_err(|_| not_found())?;
    Ok(ZipDownload {
        file: format!("{run}-{}.zip", gitkraft_hub::artifacts::safe_name(&name)),
        bytes,
    })
}

/// A zip sent as a download.
#[derive(Debug)]
struct ZipDownload {
    file: String,
    bytes: Vec<u8>,
}

impl IntoResponse for ZipDownload {
    fn into_response(self, _cx: &Cx) -> Result<Response> {
        Ok(Response::builder()
            .header("Content-Type", "application/zip")
            .header(
                "Content-Disposition",
                format!("attachment; filename=\"{}\"", self.file),
            )
            .body(Body::from(self.bytes))?)
    }
}

#[page("/ci/log")]
async fn ci_log(cx: &Cx) -> Result<impl View> {
    let params = query_params::<ItemParams>(cx)?;
    let session: &Session = app_context(cx);
    let repo =
        RepoId::from_key(params.repo.as_deref().unwrap_or_default()).ok_or_else(not_found)?;
    let job: u64 = params
        .job
        .as_deref()
        .and_then(|j| j.parse().ok())
        .ok_or_else(not_found)?;
    let ci = session.hub().ci(&repo.host).ok_or_else(not_found)?;
    let raw = ci.job_log(&repo, job).await.map_err(|_| not_found())?;
    let lines = parse_log(&raw);
    let found: Vec<(String, String, &'static str)> = extract(&lines)
        .into_iter()
        .map(|a| {
            let at = match (&a.file, a.line) {
                (Some(f), Some(l)) => format!("{f}:{l}"),
                (Some(f), None) => f.clone(),
                _ => String::new(),
            };
            (
                format!("#L{}", lines[a.log_index].number),
                format!("{at} {}", a.message),
                if a.severity == Severity::Error {
                    "err"
                } else {
                    "warn"
                },
            )
        })
        .collect();
    let explain = act_json(serde_json::json!({
        "action": "explain",
        "repo": repo.key(),
        "run_id": params.id.as_deref().and_then(|i| i.parse::<u64>().ok()).unwrap_or(0),
        "job_id": job,
    }));
    let back = params
        .id
        .as_deref()
        .map(|id| format!("/ci/run?repo={}&id={id}", url_encode(&repo.key())))
        .unwrap_or_else(|| "/".into());
    let body = Unescaped::new_unchecked(render::log_html(&lines));
    Ok(view! {
        shell(title: format!("Log · job {job}"), nav: nav(session, ""), vars: render::css_vars(&session.theme()), token: session.token().to_string(),
            <h1>(format!("Log of job {job}"))</h1>
            <div class="meta"><a href=(back)>"← back to the run"</a>" · "(format!("{} lines", lines.len()))</div>
            <div class="bar"><button data-act=(explain) data-show="ai" title="Ask the AI about this job (a draft)" onclick="actBtn(this)">"Explain failure"</button></div>
            <pre id="ai" class="panel" hidden="hidden"></pre>
            if !found.is_empty() {
                <h2>(format!("{} problems found", found.len()))</h2>
                <div class="panel">
                    for (href, text, class) in found { <div><a href=(href) class=(class)>(text)</a></div> }
                </div>
            }
            (body)
        )
    })
}

#[page("/pulse")]
async fn pulse(cx: &Cx) -> Result<impl View> {
    let params = query_params::<ItemParams>(cx)?;
    let session: &Session = app_context(cx);
    let days: u32 = params
        .days
        .as_deref()
        .and_then(|d| d.parse().ok())
        .filter(|d| (1..=365).contains(d))
        .unwrap_or(30);
    let dash = session.dashboard();
    let mut repos: Vec<String> = dash
        .sections
        .iter()
        .flat_map(|s| s.repos.iter().cloned())
        .collect();
    repos.sort();
    repos.dedup();
    let chosen = params
        .repo
        .clone()
        .filter(|r| RepoId::from_key(r).is_some())
        .or_else(|| repos.first().cloned());
    let contributions = session.hub().contributions().await;
    let today = Utc::now().date_naive();
    let heat = Unescaped::new_unchecked(render::heatmap_svg(&contributions.items, today));
    let total = render::year_total(&contributions.items, today);
    let data = match chosen.as_deref().and_then(RepoId::from_key) {
        Some(repo) => Some(session.hub().repo_pulse(&repo, days).await),
        None => None,
    };
    let bars = data
        .as_ref()
        .map(|p| Unescaped::new_unchecked(render::bars_svg(&p.prs_per_day, "PRs opened")));
    let stats: Vec<(String, usize)> = data
        .as_ref()
        .map(|p| {
            vec![
                ("PRs opened".to_string(), p.prs_opened),
                ("PRs merged".into(), p.prs_merged),
                ("PRs closed".into(), p.prs_closed),
                ("PRs open now".into(), p.prs_open_now),
                ("Issues opened".into(), p.issues_opened),
                ("Issues closed".into(), p.issues_closed),
                ("CI green".into(), p.ci_green),
                ("CI red".into(), p.ci_red),
            ]
        })
        .unwrap_or_default();
    let authors: Vec<String> = data
        .as_ref()
        .map(|p| {
            p.top_authors
                .iter()
                .map(|(a, n)| format!("{a} · {n}"))
                .collect()
        })
        .unwrap_or_default();
    let errors = contributions.errors.clone();
    let range_links: Vec<(String, String, bool)> = [7u32, 30, 90, 365]
        .iter()
        .map(|d| {
            (
                format!(
                    "/pulse?repo={}&days={d}",
                    url_encode(chosen.as_deref().unwrap_or_default())
                ),
                format!("{d} days"),
                *d == days,
            )
        })
        .collect();
    let repo_links: Vec<(String, String, bool)> = repos
        .iter()
        .map(|r| {
            (
                format!("/pulse?repo={}&days={days}", url_encode(r)),
                r.clone(),
                Some(r) == chosen.as_ref(),
            )
        })
        .collect();
    Ok(view! {
        shell(title: "Pulse · GitKraft".to_string(), nav: nav(session, "pulse"), vars: render::css_vars(&session.theme()), token: session.token().to_string(),
            <h1>"Contributions"</h1>
            <div class="meta">(format!("{total} contributions in the last year, all accounts"))</div>
            for e in errors { <div class="note warn">(e)</div> }
            <div class="panel">(heat)</div>
            <div class="meta small dim">"Click a day (or move with the arrow keys and press Enter) to see what you did."</div>
            <div id="day" class="panel" hidden="hidden"></div>
            <h1>"Pulse"</h1>
            <nav class="tabs">for (href, label, on) in repo_links { <a href=(href) class=(if on { "on" } else { "" })>(label)</a> }</nav>
            <nav class="tabs">for (href, label, on) in range_links { <a href=(href) class=(if on { "on" } else { "" })>(label)</a> }</nav>
            if data.is_none() { <p class="dim">"Add repos to your dashboard sections to see their pulse."</p> }
            <div class="grid">for (label, n) in stats { <div class="stat"><b>(n.to_string())</b>(label)</div> }</div>
            if let Some(b) = bars { <h2>"PRs opened per day"</h2><div class="panel">(b)</div> }
            if !authors.is_empty() { <h2>"Most active authors"</h2><div class="panel">for a in authors { <div>(a)</div> }</div> }
        )
    })
}

/// Discussions and wikis of one repository.
#[page("/community")]
async fn community(cx: &Cx) -> Result<impl View> {
    let params = query_params::<ItemParams>(cx)?;
    let session: &Session = app_context(cx);
    let dash = session.dashboard();
    let mut repos: Vec<String> = dash
        .sections
        .iter()
        .flat_map(|s| s.repos.iter().cloned())
        .collect();
    repos.sort();
    repos.dedup();
    let chosen = params
        .repo
        .clone()
        .filter(|r| RepoId::from_key(r).is_some())
        .or_else(|| repos.first().cloned());
    use gitkraft_hub::community::{self, Item, Tab};
    let repo = chosen.as_deref().and_then(RepoId::from_key);
    let tab = match params.tab.as_deref() {
        Some("wiki") => Tab::Wiki,
        Some("boards") => Tab::Board,
        _ => Tab::Discussions,
    };
    let key = chosen.clone().unwrap_or_default();
    let repo_links: Vec<(String, String, bool)> = repos
        .iter()
        .map(|r| {
            (
                format!("/community?repo={}&tab={}", url_encode(r), tab.label()),
                r.clone(),
                Some(r) == chosen.as_ref(),
            )
        })
        .collect();
    let tab_links: Vec<(String, String, bool)> = [
        (Tab::Discussions, "Discussions"),
        (Tab::Wiki, "Wiki"),
        (Tab::Board, "Boards"),
    ]
    .iter()
    .map(|(t, label)| {
        (
            format!("/community?repo={}&tab={}", url_encode(&key), t.label()),
            label.to_string(),
            *t == tab,
        )
    })
    .collect();
    // what the platform says is shown as a note: GitHub has no wiki API, the others have no discussions
    let mut note: Option<String> = None;
    let mut list: Vec<(String, String, String)> = Vec::new();
    let mut heading = String::new();
    let mut text: Option<String> = None;
    let mut comments: Vec<(String, String, String)> = Vec::new();
    let mut act = String::new();
    if let Some(repo) = &repo {
        // wiki pages are named by `page`, discussions and cards by `n`
        let open = match tab {
            Tab::Wiki => params.page.clone(),
            _ => params.n.clone(),
        };
        match open {
            Some(k) => {
                let item = Item {
                    key: k.clone(),
                    title: k.clone(),
                    meta: String::new(),
                };
                match community::page(session.hub(), repo, tab, &item).await {
                    Ok(p) => {
                        heading = p.title.clone();
                        text = Some(p.text);
                        comments = p
                            .comments
                            .into_iter()
                            .map(|(who, body)| (who, String::new(), body))
                            .collect();
                        if tab == Tab::Discussions {
                            act = act_json(
                                serde_json::json!({"action": "discussion_comment", "repo": key, "number": k.parse::<u64>().unwrap_or(0)}),
                            );
                        }
                    }
                    Err(e) => note = Some(e),
                }
            }
            None => match community::list(session.hub(), repo, tab).await {
                Ok(items) => {
                    list = items
                        .into_iter()
                        .map(|i| {
                            let href = if i.key.is_empty() {
                                String::new()
                            } else {
                                format!(
                                    "/community?repo={}&tab={}&{}={}",
                                    url_encode(&key),
                                    tab.label(),
                                    if tab == Tab::Wiki { "page" } else { "n" },
                                    url_encode(&i.key)
                                )
                            };
                            (href, i.title, i.meta)
                        })
                        .collect()
                }
                Err(e) => note = Some(e),
            },
        }
    }
    let has_text = text.is_some();
    let text = text.unwrap_or_default();
    let can_comment = !act.is_empty();
    Ok(view! {
        shell(title: "Discussions & wiki · GitKraft".to_string(), nav: nav(session, "community"), vars: render::css_vars(&session.theme()), token: session.token().to_string(),
            <h1>"Discussions & wiki"</h1>
            <nav class="tabs">for (href, label, on) in repo_links { <a href=(href) class=(if on { "on" } else { "" })>(label)</a> }</nav>
            <nav class="tabs">for (href, label, on) in tab_links { <a href=(href) class=(if on { "on" } else { "" })>(label)</a> }</nav>
            if repo.is_none() { <p class="dim">"Add repos to your dashboard sections to browse their discussions and wiki."</p> }
            if let Some(n) = note { <div class="note warn">(n)</div> }
            for (href, title, meta) in list { <div class="row">if href.is_empty() { (title) } else { <a href=(href)>(title)</a> }" "<span class="meta small">(meta)</span></div> }
            if !heading.is_empty() { <h2>(heading)</h2> }
            if has_text { <div class="panel"><pre>(text)</pre></div> }
            for (who, when, body) in comments { <div class="comment"><b>(who)</b>" "<span class="meta small">(when)</span><pre>(body)</pre></div> }
            if can_comment {
                <textarea id="note" placeholder="Write a comment"></textarea>
                <div class="bar"><button class="primary" data-act=(act) data-from="note" onclick="actBtn(this)">"Comment"</button></div>
            }
        )
    })
}

#[page("/inbox")]
async fn inbox(cx: &Cx) -> Result<impl View> {
    let session: &Session = app_context(cx);
    let engine = session.engine();
    let saved: Vec<(String, String, String)> = engine
        .inbox
        .entries()
        .iter()
        .map(|e| {
            (
                format!("{} · {}", e.alert.headline(), e.alert.title),
                if e.read {
                    "mu".to_string()
                } else {
                    "acc".to_string()
                },
                ago(Some(e.at)),
            )
        })
        .collect();
    let inbox_section = gitkraft_settings::dashboard::Section {
        name: "Inbox".into(),
        kind: gitkraft_settings::dashboard::SectionKind::Inbox,
        repos: vec![],
        involvement: None,
        state: Default::default(),
        label: None,
        branch: None,
        hidden: false,
    };
    let live = session.hub().load_section(&inbox_section).await;
    let rows = rows_of(&live);
    let errors = live.errors().to_vec();
    Ok(view! {
        shell(title: "Inbox · GitKraft".to_string(), nav: nav(session, "inbox"), vars: render::css_vars(&session.theme()), token: session.token().to_string(),
            <h1>"Inbox"</h1>
            for e in errors { <div class="note err">(e)</div> }
            <table>
                for r in rows {
                    <tr><td class="mark"><span class=(r.class)>(r.mark)</span></td><td><a href=(r.href)>(r.title)</a><div class="meta small">(r.meta)</div></td></tr>
                }
            </table>
            if !saved.is_empty() {
                <h2>"Raised by the poller"</h2>
                <table>
                    for (text, class, when) in saved { <tr><td class=(class)>(text)</td><td class="meta">(when)</td></tr> }
                </table>
            }
        )
    })
}

/// SSH keys of every account, with set-up, test and removal.
#[page("/keys")]
async fn keys_page(cx: &Cx) -> Result<impl View> {
    let session: &Session = app_context(cx);
    let mine = match session.ssh_dir() {
        Some(dir) => {
            let dir = dir.to_path_buf();
            tokio::task::spawn_blocking(move || {
                gitkraft_hub::keys::local_fingerprints(&dir, &gitkraft_platforms::ssh::SystemSsh)
            })
            .await
            .unwrap_or_default()
        }
        None => Vec::new(),
    };
    let data = gitkraft_hub::keys::load(session.hub(), &mine).await;
    let can_change = session.ssh_dir().is_some();
    let hosts: Vec<(String, String, String)> = data
        .hosts
        .iter()
        .map(|h| {
            (
                h.clone(),
                act_json(serde_json::json!({"action": "ssh_setup", "host": h})),
                act_json(serde_json::json!({"action": "ssh_test", "host": h})),
            )
        })
        .collect();
    let rows: Vec<(String, String, String, String, String, bool)> = data
        .rows
        .iter()
        .map(|r| {
            (
                r.host.clone(),
                r.key.title.clone(),
                r.key.fingerprint.clone().unwrap_or_default(),
                ago(r.key.created_at),
                act_json(
                    serde_json::json!({"action": "ssh_revoke", "host": r.host, "id": r.key.id}),
                ),
                r.mine,
            )
        })
        .collect();
    let errors = data.errors.clone();
    Ok(view! {
        shell(title: "SSH keys · GitKraft".to_string(), nav: nav(session, "keys"), vars: render::css_vars(&session.theme()), token: session.token().to_string(),
            <h1>"SSH keys"</h1>
            <div class="meta">"● = a key of this machine. Set-up creates or reuses this machine's key, registers it, and adds a marked block to ~/.ssh/config."</div>
            for e in errors { <div class="err">(e)</div> }
            <table>
                for (host, setup, test) in hosts {
                    <tr>
                        <td>(host)</td>
                        <td>
                            if can_change {
                                <button data-act=(setup) onclick="actBtn(this)">"Set up ssh"</button>
                                <button data-act=(test) onclick="actBtn(this)">"Test"</button>
                            }
                        </td>
                    </tr>
                }
            </table>
            <h2>"Registered keys"</h2>
            <table>
                for (host, title, fp, when, revoke, mine) in rows {
                    <tr>
                        <td class="mark">(if mine { "●" } else { "" })</td>
                        <td>(host)</td>
                        <td>(title)<div class="meta small"><code>(fp)</code></div></td>
                        <td class="meta">(when)</td>
                        <td><button data-act=(revoke) data-confirm="Remove this key from the account?" onclick="actBtn(this)">"Remove"</button></td>
                    </tr>
                }
            </table>
        )
    })
}

#[page("/notify")]
async fn notify(cx: &Cx) -> Result<impl View> {
    let session: &Session = app_context(cx);
    let config = session.notify_config();
    let engine = session.engine();
    let dash = session.dashboard();
    let mut keys: Vec<String> = config.projects.keys().cloned().collect();
    keys.extend(dash.sections.iter().flat_map(|s| s.repos.iter().cloned()));
    keys.sort();
    keys.dedup();
    let now = Utc::now();
    let rows: Vec<(String, String, String, String, bool)> = keys
        .into_iter()
        .map(|key| {
            let eff = config.resolve(&key, None);
            let st = engine.status.repos.get(&key).cloned().unwrap_or_default();
            let s = status(
                &StatusInput {
                    enabled: eff.enabled,
                    muted_until: engine.state.muted_until(&key),
                    last_poll: st.last_poll,
                    last_error: st.last_error,
                    poller_heartbeat: engine.status.heartbeat,
                    interval: eff.poll_interval,
                },
                now,
            );
            let on = eff.enabled;
            (
                key.clone(),
                s.glyph(true).to_string(),
                s.describe(now),
                act_json(serde_json::json!({"action": "notify_toggle", "repo": key, "on": !on})),
                on,
            )
        })
        .collect();
    let global_on = config.defaults.enabled.unwrap_or(false);
    let global =
        act_json(serde_json::json!({"action": "notify_toggle", "repo": "", "on": !global_on}));
    let mode = if engine
        .status
        .heartbeat
        .is_some_and(|h| now - h < Duration::minutes(5))
    {
        engine.status.mode.clone()
    } else {
        "not running".to_string()
    };
    Ok(view! {
        shell(title: "Notifications · GitKraft".to_string(), nav: nav(session, "notify"), vars: render::css_vars(&session.theme()), token: session.token().to_string(),
            <h1>"Notifications"</h1>
            <div class="meta">(format!("poller: {mode} · start it with `gitkraft-tui notify --daemon`"))</div>
            <div class="bar"><button data-act=(global) onclick="actBtn(this)">(if global_on { "Turn all off" } else { "Turn all on" })</button></div>
            <table>
                for (key, glyph, text, toggle, on) in rows {
                    <tr>
                        <td class="mark"><code>(glyph)</code></td>
                        <td>(key)<div class="meta small">(text)</div></td>
                        <td><button data-act=(toggle) onclick="actBtn(this)">(if on { "Turn off" } else { "Turn on" })</button></td>
                    </tr>
                }
            </table>
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitkraft_hub::SectionResult;
    use gitkraft_platforms::model::{Notification, PullRequest, User};

    fn repo() -> RepoId {
        RepoId::new("github.com", "o", "r")
    }

    #[test]
    fn pr_rows_link_to_the_pr_page_and_show_checks() {
        let pr = |n, checks, draft| PullRequest {
            repo: repo(),
            number: n,
            title: format!("t{n}"),
            author: User {
                login: "a".into(),
                avatar_url: None,
            },
            checks,
            draft,
            ..PullRequest::default()
        };
        let rows = rows_of(&SectionData::Prs(SectionResult {
            items: vec![
                pr(1, Some(ChecksState::Failure), false),
                pr(2, None, true),
                pr(3, None, false),
            ],
            errors: vec![],
        }));
        assert_eq!(rows[0].href, "/pr?repo=github.com%2Fo%2Fr&n=1");
        assert_eq!((rows[0].mark, rows[0].class), ("✘", "err"));
        assert_eq!((rows[1].mark, rows[1].class), ("◌", "mu"));
        assert_eq!((rows[2].mark, rows[2].class), ("●", "ok"));
        assert_eq!(rows[0].title, "#1 t1");
        assert!(rows[0].meta.starts_with("o/r · a"));
    }

    #[test]
    fn inbox_rows_link_into_the_app_when_they_can() {
        let n = |kind, number| Notification {
            id: "1".into(),
            repo: Some(repo()),
            kind,
            number,
            title: "t".into(),
            reason: NotificationReason::ReviewRequested,
            unread: true,
            updated_at: None,
            url: "https://github.com/o/r/pull/1".into(),
        };
        let rows = rows_of(&SectionData::Inbox(SectionResult {
            items: vec![
                n(SubjectKind::PullRequest, Some(1)),
                n(SubjectKind::Issue, Some(2)),
                n(SubjectKind::Release, None),
            ],
            errors: vec![],
        }));
        assert_eq!(rows[0].href, "/pr?repo=github.com%2Fo%2Fr&n=1");
        assert_eq!(rows[1].href, "/issue?repo=github.com%2Fo%2Fr&n=2");
        assert_eq!(rows[2].href, "https://github.com/o/r/pull/1");
        assert!(rows[0].meta.starts_with("review requested"));
    }

    #[test]
    fn relative_times_step_through_units() {
        let ago = |d: Duration| gitkraft_core_relative(Utc::now() - d);
        assert_eq!(ago(Duration::seconds(5)), "just now");
        assert_eq!(ago(Duration::minutes(5)), "5m ago");
        assert_eq!(ago(Duration::hours(5)), "5h ago");
        assert_eq!(ago(Duration::days(5)), "5d ago");
        assert_eq!(ago(Duration::days(100)), "3mo ago");
    }

    #[test]
    fn diff_lines_get_their_colour_class() {
        for (line, class) in [
            ("+x", "add"),
            ("-x", "del"),
            ("@@ -1 +1 @@", "hunk"),
            (" ctx", "ctx"),
            ("+++ b/f", "ctx"),
            ("--- a/f", "ctx"),
            ("diff --git a b", "ctx"),
        ] {
            assert_eq!(diff_class(line), class, "{line}");
        }
    }
}
