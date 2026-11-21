//! The Repository page: a local repository's history, changes, branches, stashes and remotes.

use gitkraft_core::features::diff::FileStatus;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{page, query_params},
    view::{Unescaped, View, view},
};

use crate::local;
use crate::pages::{nav, shell};
use crate::session::Session;

#[derive(Debug)]
#[topcoat::router::query_params(error = redirect("/"))]
pub struct GitParams {
    /// Index of the repository.
    pub r: Option<String>,
    /// `history`, `changes`, `branches`, `stash` or `remotes`.
    pub tab: Option<String>,
    /// A commit to open.
    pub c: Option<String>,
    /// A file of the working tree to diff.
    pub f: Option<String>,
    /// `1` when that file is the staged one.
    pub staged: Option<String>,
    /// History search.
    pub q: Option<String>,
    /// A file to blame.
    pub blame: Option<String>,
}

/// A line of the blame view.
#[derive(Debug, Default)]
struct BlameRow {
    no: String,
    short: String,
    who: String,
    when: String,
    href: String,
    text: String,
    /// The first line of a run from the same commit (the others leave the commit columns empty).
    first: bool,
}

fn act(r: usize, op: serde_json::Value) -> String {
    let mut v = serde_json::json!({"action": "git", "r": r});
    if let (Some(o), Some(e)) = (v.as_object_mut(), op.as_object()) {
        o.extend(e.clone());
    }
    v.to_string()
}

fn status_glyph(s: &FileStatus) -> &'static str {
    match s {
        FileStatus::New | FileStatus::Untracked => "A",
        FileStatus::Modified => "M",
        FileStatus::Deleted => "D",
        FileStatus::Renamed => "R",
        FileStatus::Copied => "C",
        FileStatus::Typechange => "T",
    }
}

struct CommitRow {
    graph: String,
    refs: String,
    summary: String,
    who: String,
    when: String,
    short: String,
    href: String,
    open: bool,
}

struct FileRow {
    glyph: &'static str,
    path: String,
    href: String,
    primary: String,
    primary_label: &'static str,
    discard: String,
}

struct BranchRow {
    name: String,
    head: bool,
    sync: String,
    checkout: String,
    merge: String,
    rebase: String,
    rename: String,
    delete: String,
}

/// A button that runs one git operation: what it says, the action, and an optional question
/// (`ask`, its answer goes in `field`) or confirmation; `danger` paints it red.
struct ActBtn {
    label: &'static str,
    act: String,
    ask: &'static str,
    field: &'static str,
    confirm: &'static str,
    danger: bool,
}

impl ActBtn {
    fn new(label: &'static str, act: String) -> Self {
        Self {
            label,
            act,
            ask: "",
            field: "",
            confirm: "",
            danger: false,
        }
    }
    fn ask(mut self, question: &'static str, field: &'static str) -> Self {
        self.ask = question;
        self.field = field;
        self
    }
    fn confirm(mut self, question: &'static str) -> Self {
        self.confirm = question;
        self
    }
    fn danger(mut self) -> Self {
        self.danger = true;
        self
    }
}

/// The actions on a selected commit.
fn commit_actions(r: usize, oid: &str) -> Vec<ActBtn> {
    let a = |v: serde_json::Value| act(r, v);
    vec![
        ActBtn::new("Cherry-pick", a(serde_json::json!({"op": "cherry_pick", "oid": oid})))
            .confirm("Apply this commit on top of the current branch?"),
        ActBtn::new("Revert", a(serde_json::json!({"op": "revert", "oid": oid})))
            .confirm("Make a new commit that undoes this one?"),
        ActBtn::new("Branch here", a(serde_json::json!({"op": "branch_at", "oid": oid})))
            .ask("Name of the new branch", "name"),
        ActBtn::new("Tag", a(serde_json::json!({"op": "tag", "oid": oid})))
            .ask("Tag name", "name"),
        ActBtn::new("Rebase onto", a(serde_json::json!({"op": "rebase", "name": oid})))
            .confirm("Rebase the current branch onto this commit?"),
        ActBtn::new("Check out", a(serde_json::json!({"op": "checkout_detached", "oid": oid})))
            .confirm("Check out this commit without a branch (detached HEAD)?"),
        ActBtn::new("Reset soft", a(serde_json::json!({"op": "reset", "oid": oid, "mode": "soft"})))
            .confirm("Move the current branch here and keep every change staged?"),
        ActBtn::new("Reset mixed", a(serde_json::json!({"op": "reset", "oid": oid, "mode": "mixed"})))
            .confirm("Move the current branch here and keep the changes unstaged?"),
        ActBtn::new("Reset hard", a(serde_json::json!({"op": "reset", "oid": oid, "mode": "hard"})))
            .confirm("Move the current branch here and THROW AWAY every uncommitted change? This cannot be undone.")
            .danger(),
    ]
}

struct StashRow {
    label: String,
    pop: String,
    apply: String,
    drop: String,
}

struct RemoteRow {
    name: String,
    url: String,
    fetch: String,
    pull: String,
    push: String,
    force: String,
}

/// A branch on a remote and what can be done with it.
struct RemoteBranchRow {
    name: String,
    checkout: String,
    delete: String,
}

/// Everything the page shows; `problem` set means only that is shown.
#[derive(Default)]
struct Page {
    problem: String,
    title: String,
    head: String,
    state: String,
    demo: bool,
    r: usize,
    q: String,
    base: String,
    picker: Vec<(String, String, bool)>,
    tabs: Vec<(String, String, bool)>,
    is_history: bool,
    is_changes: bool,
    is_branches: bool,
    is_stash: bool,
    is_remotes: bool,
    searching: bool,
    has_detail: bool,
    d_head: String,
    d_message: String,
    d_diff: Vec<(&'static str, String)>,
    d_actions: Vec<ActBtn>,
    history: Vec<CommitRow>,
    stage_all: String,
    unstage_all: String,
    commit: String,
    nothing_changed: bool,
    unstaged: Vec<FileRow>,
    staged: Vec<FileRow>,
    f_diff: Vec<(&'static str, String)>,
    new_branch: String,
    local_branches: Vec<BranchRow>,
    remote_branches: Vec<RemoteBranchRow>,
    stash_save: String,
    stashes: Vec<StashRow>,
    remotes: Vec<RemoteRow>,
    d_files: Vec<(String, String)>,
    f_blame: String,
    f_open: String,
    blame_path: String,
    blame_close: String,
    blame_open: String,
    blame_error: String,
    blame: Vec<BlameRow>,
}

fn problem(msg: impl Into<String>) -> Page {
    Page {
        problem: msg.into(),
        ..Page::default()
    }
}

async fn load(session: &Session, params: &GitParams) -> Page {
    let repos = session.repos();
    let r: usize = params
        .r
        .as_deref()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let tab = params.tab.clone().unwrap_or_else(|| "history".into());
    let q = params.q.clone().unwrap_or_default();
    let demo = session.is_demo();

    let Some(path) = repos.get(r).cloned() else {
        return problem(if repos.is_empty() {
            "No repository here. Start gitkraft-web inside one, or pass --checkout DIR."
        } else {
            "That repository is not known to this server."
        });
    };
    let picker: Vec<(String, String, bool)> = repos
        .iter()
        .enumerate()
        .map(|(i, p)| {
            (
                format!("/git?r={i}"),
                p.file_name().map_or_else(
                    || p.display().to_string(),
                    |n| n.to_string_lossy().into_owned(),
                ),
                i == r,
            )
        })
        .collect();

    let p = path.clone();
    let snap = match tokio::task::spawn_blocking(move || local::snapshot(&p)).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return problem(e),
        Err(e) => return problem(e.to_string()),
    };
    let head = snap
        .info
        .head_branch
        .clone()
        .unwrap_or_else(|| "(detached)".into());
    let state = snap.info.state.to_string();
    let base = format!("/git?r={r}");

    // History
    let searching = !q.is_empty();
    let found = if searching {
        let p = path.clone();
        let q2 = q.clone();
        tokio::task::spawn_blocking(move || local::search(&p, &q2))
            .await
            .ok()
            .and_then(Result::ok)
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let width = local::graph_width(&snap.graph_rows);
    let selected = params.c.clone().unwrap_or_default();
    let history: Vec<CommitRow> = if searching {
        found.iter().map(|c| (c, String::new())).collect::<Vec<_>>()
    } else {
        snap.commits
            .iter()
            .zip(&snap.graph_rows)
            .map(|(c, g)| (c, local::graph_svg(g, width)))
            .collect()
    }
    .into_iter()
    .map(|(c, graph)| CommitRow {
        graph,
        refs: local::ref_badges(c),
        summary: c.summary.clone(),
        who: c.author_name.clone(),
        when: c.relative_time(),
        short: c.short_oid().to_string(),
        href: format!("{base}&tab=history&c={}", c.oid),
        open: c.oid == selected,
    })
    .collect();
    let detail = if tab == "history" && !selected.is_empty() {
        let p = path.clone();
        let s2 = selected.clone();
        tokio::task::spawn_blocking(move || local::commit(&p, &s2))
            .await
            .ok()
            .and_then(Result::ok)
    } else {
        None
    };
    let (d_head, d_message, d_diff) = match &detail {
        Some((info, files)) => (
            format!(
                "{} · {} <{}> · {}",
                info.short_oid(),
                info.author_name,
                info.author_email,
                info.relative_time()
            ),
            info.message.clone(),
            local::diff_lines(files),
        ),
        None => (String::new(), String::new(), Vec::new()),
    };

    // Changes
    let file_rows = |files: &[gitkraft_core::features::diff::DiffFileEntry],
                     staged: bool|
     -> Vec<FileRow> {
        files
            .iter()
            .map(|f| {
                let p = f.display_path().to_string();
                FileRow {
                    glyph: status_glyph(&f.status),
                    href: format!("{base}&tab=changes&f={}&staged={}", gitkraft_platforms::url_encode(&p), u8::from(staged)),
                    primary: act(r, serde_json::json!({"op": if staged { "unstage" } else { "stage" }, "path": p})),
                    primary_label: if staged { "Unstage" } else { "Stage" },
                    discard: act(r, serde_json::json!({"op": "discard", "path": p})),
                    path: p,
                }
            })
            .collect()
    };
    let unstaged = file_rows(&snap.unstaged, false);
    let staged = file_rows(&snap.staged, true);
    let file = params.f.clone().unwrap_or_default();
    let file_is_staged = params.staged.as_deref() == Some("1");
    let f_diff = if tab == "changes" && !file.is_empty() {
        let p = path.clone();
        let f2 = file.clone();
        tokio::task::spawn_blocking(move || local::file_diff(&p, &f2, file_is_staged))
            .await
            .ok()
            .and_then(Result::ok)
            .map(|d| local::diff_lines(&[d]))
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let blame_href = |file: &str| {
        format!(
            "{base}&tab={tab}&blame={}",
            gitkraft_platforms::url_encode(file)
        )
    };
    let open_act = |file: &str| act(r, serde_json::json!({"op": "open_in_editor", "path": file}));
    let (f_blame, f_open) = if file.is_empty() {
        (String::new(), String::new())
    } else {
        (blame_href(&file), open_act(&file))
    };
    let blame_path = params.blame.clone().unwrap_or_default();
    let (blame, blame_error) = if blame_path.is_empty() {
        (Vec::new(), String::new())
    } else {
        let p = path.clone();
        let b2 = blame_path.clone();
        match tokio::task::spawn_blocking(move || local::blame(&p, &b2)).await {
            Ok(Ok(lines)) => {
                let mut prev = String::new();
                let rows = lines
                    .iter()
                    .map(|l| {
                        let first = l.oid != prev;
                        prev.clone_from(&l.oid);
                        BlameRow {
                            no: l.line_number.to_string(),
                            short: if first {
                                l.short_oid().to_string()
                            } else {
                                String::new()
                            },
                            who: if first {
                                l.author_name.clone()
                            } else {
                                String::new()
                            },
                            when: if first {
                                l.relative_time()
                            } else {
                                String::new()
                            },
                            href: format!("{base}&tab=history&c={}", l.oid),
                            text: l.content.clone(),
                            first,
                        }
                    })
                    .collect();
                (rows, String::new())
            }
            Ok(Err(e)) => (Vec::new(), e),
            Err(e) => (Vec::new(), e.to_string()),
        }
    };
    let blame_close = format!("{base}&tab={tab}");
    let blame_open = if blame_path.is_empty() {
        String::new()
    } else {
        open_act(&blame_path)
    };
    let d_files: Vec<(String, String)> = detail
        .as_ref()
        .map(|(_, files)| {
            files
                .iter()
                .map(|f| (f.display_path().to_string(), blame_href(f.display_path())))
                .collect()
        })
        .unwrap_or_default();
    let stage_all = act(r, serde_json::json!({"op": "stage_all"}));
    let unstage_all = act(r, serde_json::json!({"op": "unstage_all"}));
    let commit = act(r, serde_json::json!({"op": "commit"}));

    // Branches
    let mut local_branches = Vec::new();
    let mut remote_branches = Vec::new();
    for b in &snap.branches {
        if b.is_remote() {
            remote_branches.push(RemoteBranchRow {
                name: b.name.clone(),
                checkout: act(
                    r,
                    serde_json::json!({"op": "checkout_remote", "name": b.name}),
                ),
                delete: act(
                    r,
                    serde_json::json!({"op": "delete_remote_branch", "name": b.name}),
                ),
            });
        } else {
            local_branches.push(BranchRow {
                name: b.name.clone(),
                head: b.is_head,
                sync: b.upstream_status().unwrap_or_default(),
                checkout: act(r, serde_json::json!({"op": "checkout", "name": b.name})),
                merge: act(r, serde_json::json!({"op": "merge", "name": b.name})),
                rebase: act(r, serde_json::json!({"op": "rebase", "name": b.name})),
                rename: act(
                    r,
                    serde_json::json!({"op": "rename_branch", "name": b.name}),
                ),
                delete: act(
                    r,
                    serde_json::json!({"op": "delete_branch", "name": b.name}),
                ),
            });
        }
    }
    let new_branch = act(r, serde_json::json!({"op": "new_branch"}));

    // Stashes
    let stashes: Vec<StashRow> = snap
        .stashes
        .iter()
        .map(|s| StashRow {
            label: format!("{} · {}", s.ref_name(), s.message),
            pop: act(r, serde_json::json!({"op": "stash_pop", "index": s.index})),
            apply: act(
                r,
                serde_json::json!({"op": "stash_apply", "index": s.index}),
            ),
            drop: act(r, serde_json::json!({"op": "stash_drop", "index": s.index})),
        })
        .collect();
    let stash_save = act(r, serde_json::json!({"op": "stash_save"}));

    // Remotes
    let remotes: Vec<RemoteRow> = snap
        .remotes
        .iter()
        .map(|m| RemoteRow {
            name: m.name.clone(),
            url: m.display_url().to_string(),
            fetch: act(r, serde_json::json!({"op": "fetch", "remote": m.name})),
            pull: act(
                r,
                serde_json::json!({"op": "pull", "remote": m.name, "branch": head}),
            ),
            push: act(
                r,
                serde_json::json!({"op": "push", "remote": m.name, "branch": head}),
            ),
            force: act(
                r,
                serde_json::json!({"op": "force_push", "remote": m.name, "branch": head}),
            ),
        })
        .collect();

    let tabs: Vec<(String, String, bool)> = [
        ("history", format!("History {}", snap.commits.len())),
        (
            "changes",
            format!("Changes {}", snap.unstaged.len() + snap.staged.len()),
        ),
        ("branches", format!("Branches {}", local_branches.len())),
        ("stash", format!("Stashes {}", snap.stashes.len())),
        ("remotes", format!("Remotes {}", snap.remotes.len())),
    ]
    .into_iter()
    .map(|(k, label)| (format!("{base}&tab={k}"), label, tab == k))
    .collect();
    let title = path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let is_history = tab == "history";
    let is_changes = tab == "changes";
    let is_branches = tab == "branches";
    let is_stash = tab == "stash";
    let is_remotes = tab == "remotes";
    let has_detail = detail.is_some();
    let d_actions = match &detail {
        Some((info, _)) => commit_actions(r, &info.oid),
        None => Vec::new(),
    };
    let nothing_changed = unstaged.is_empty() && staged.is_empty();
    Page {
        problem: String::new(),
        title,
        head,
        state,
        demo,
        r,
        q,
        base,
        picker,
        tabs,
        is_history,
        is_changes,
        is_branches,
        is_stash,
        is_remotes,
        searching,
        has_detail,
        d_head,
        d_message,
        d_diff,
        d_actions,
        history,
        stage_all,
        unstage_all,
        commit,
        nothing_changed,
        unstaged,
        staged,
        f_diff,
        new_branch,
        local_branches,
        remote_branches,
        stash_save,
        stashes,
        remotes,
        d_files,
        f_blame,
        f_open,
        blame_path,
        blame_close,
        blame_open,
        blame_error,
        blame,
    }
}

/// A local repository: history with a graph, the working tree, branches, stashes and remotes.
#[page("/git")]
async fn repository(cx: &Cx) -> Result<impl View> {
    let params = query_params::<GitParams>(cx)?;
    let session: &Session = app_context(cx);
    let Page {
        problem,
        title,
        head,
        state,
        demo,
        r,
        q,
        base,
        picker,
        tabs,
        is_history,
        is_changes,
        is_branches,
        is_stash,
        is_remotes,
        searching,
        has_detail,
        d_head,
        d_message,
        d_diff,
        d_actions,
        history,
        stage_all,
        unstage_all,
        commit,
        nothing_changed,
        unstaged,
        staged,
        f_diff,
        new_branch,
        local_branches,
        remote_branches,
        stash_save,
        stashes,
        remotes,
        d_files,
        f_blame,
        f_open,
        blame_path,
        blame_close,
        blame_open,
        blame_error,
        blame,
    } = load(session, params).await;
    let failed = !problem.is_empty();
    let has_blame = !blame_path.is_empty();
    let f_name = params.f.clone().unwrap_or_default();
    let clone_act = serde_json::json!({"action": "clone"}).to_string();
    let clone_parent = session.clone_parent().display().to_string();

    Ok(view! {
        shell(title: format!("{title} · GitKraft"), nav: nav(session, "git"), vars: session.css_vars(), token: session.token().to_string(),
            <h1>(if failed { "Repository".to_string() } else { title.clone() })</h1>
            if failed { <div class="note err">(problem)</div> }
            if !demo {
                <details class="panel" open=(failed)>
                    <summary>"Clone a repository"</summary>
                    <div class="bar">
                        <input id="clone-spec" placeholder="owner/name, host/owner/name or a URL" aria-label="What to clone">
                        <input id="clone-into" value=(clone_parent) aria-label="Folder to clone into">
                        <button class="primary" data-act=(clone_act) data-from="clone-spec" data-field="spec" data-dest="clone-into" onclick="actBtn(this)">"Clone"</button>
                    </div>
                    <div class="meta small">"Over ssh or https as accounts.toml says for the host; the clone opens here when it is done."</div>
                </details>
            }
            if !failed {
            <div class="meta">"on "<b>(head.clone())</b>if state != "Clean" { " · "(state.clone()) }if demo { " · demo: read-only" }</div>
            if picker.len() > 1 {
                <div class="scope">
                    for (href, name, on) in picker { <a href=(href) class=(if on { "on" } else { "" })>(name)</a> }
                </div>
            }
            <nav class="tabs">
                for (href, label, on) in tabs { <a href=(href) class=(if on { "on" } else { "" })>(label)</a> }
            </nav>
            if has_blame {
                <div class="panel">
                    <div class="bar">
                        <b>"Blame · "(blame_path.clone())</b>
                        if !demo { <button data-act=(blame_open) onclick="actBtn(this)">"Open in editor"</button> }
                        <a class="btn" href=(blame_close)>"Close"</a>
                    </div>
                    if !blame_error.is_empty() { <div class="note err">(blame_error)</div> }
                    <table class="blame">
                        for b in blame {
                            <tr class=(if b.first { "run" } else { "" })>
                                <td><a href=(b.href)><code>(b.short)</code></a></td>
                                <td class="meta">(b.who)</td>
                                <td class="meta">(b.when)</td>
                                <td class="meta">(b.no)</td>
                                <td><pre>(b.text)</pre></td>
                            </tr>
                        }
                    </table>
                </div>
            }
            if is_history {
                <form class="bar" method="get" action="/git">
                    <input type="hidden" name="r" value=(r.to_string())>
                    <input type="search" name="q" value=(q.clone()) placeholder="Search messages and authors" aria-label="Search commits">
                    <button>"Search"</button>
                    if searching { <a class="btn" href=(base.clone())>"Clear"</a> }
                </form>
                <form class="bar" method="get" action="/git">
                    <input type="hidden" name="r" value=(r.to_string())>
                    <input type="hidden" name="tab" value="history">
                    <input name="blame" placeholder="path/of/a/file" aria-label="File to blame">
                    <button>"Blame"</button>
                </form>
                if has_detail {
                    <div class="panel">
                        <div class="meta small">(d_head)</div>
                        <pre>(d_message)</pre>
                        <div class="meta small">
                            for (file, href) in d_files { <a href=(href) title="Blame (as it is now)">(file)</a>" " }
                        </div>
                        if !demo {
                            <div class="bar">
                                for b in d_actions {
                                    <button class=(if b.danger { "danger" } else { "" }) data-act=(b.act) data-ask=(b.ask) data-field=(b.field) data-confirm=(b.confirm) onclick="actBtn(this)">(b.label)</button>
                                }
                            </div>
                        }
                    </div>
                    <div class="diff">
                        for (class, text) in d_diff { <div class=(class)>(text)</div> }
                    </div>
                }
                <table class="commits">
                    for c in history {
                        <tr class=(if c.open { "sel" } else { "" })>
                            <td class="gcell">(Unescaped::new_unchecked(c.graph))</td>
                            <td><a href=(c.href)>(c.summary)</a>" "(Unescaped::new_unchecked(c.refs))</td>
                            <td class="meta">(c.who)</td>
                            <td class="meta">(c.when)</td>
                            <td><code>(c.short)</code></td>
                        </tr>
                    }
                </table>
            } else if is_changes {
                <div class="bar">
                    <button data-act=(stage_all) onclick="actBtn(this)">"Stage all"</button>
                    <button data-act=(unstage_all) onclick="actBtn(this)">"Unstage all"</button>
                </div>
                if nothing_changed { <div class="panel">"Working tree clean."</div> }
                <h2>"Unstaged"</h2>
                <table>
                    for f in unstaged {
                        <tr>
                            <td class="mark">(f.glyph)</td>
                            <td><a href=(f.href)>(f.path)</a></td>
                            <td>
                                <button data-act=(f.primary) onclick="actBtn(this)">(f.primary_label)</button>
                                <button class="danger" data-act=(f.discard) data-confirm="Throw away the changes of this file? This cannot be undone." onclick="actBtn(this)">"Discard"</button>
                            </td>
                        </tr>
                    }
                </table>
                <h2>"Staged"</h2>
                <table>
                    for f in staged {
                        <tr>
                            <td class="mark">(f.glyph)</td>
                            <td><a href=(f.href)>(f.path)</a></td>
                            <td><button data-act=(f.primary) onclick="actBtn(this)">(f.primary_label)</button></td>
                        </tr>
                    }
                </table>
                <h2>"Commit"</h2>
                <textarea id="msg" placeholder="Commit message"></textarea>
                <div class="bar"><button class="primary" data-act=(commit) data-from="msg" data-field="message" onclick="actBtn(this)">"Commit staged changes"</button></div>
                if !f_blame.is_empty() {
                    <div class="bar">
                        <b>(f_name)</b>
                        <a class="btn" href=(f_blame)>"Blame"</a>
                        if !demo { <button data-act=(f_open) onclick="actBtn(this)">"Open in editor"</button> }
                    </div>
                }
                <div class="diff">
                    for (class, text) in f_diff { <div class=(class)>(text)</div> }
                </div>
            } else if is_branches {
                <div class="bar"><button class="primary" data-act=(new_branch) data-ask="Name of the new branch" data-field="name" onclick="actBtn(this)">"New branch"</button></div>
                <table>
                    for b in local_branches {
                        <tr>
                            <td class="mark">(if b.head { "●" } else { "" })</td>
                            <td>(b.name)</td>
                            <td class="meta">(b.sync)</td>
                            <td>
                                if !b.head {
                                    <button data-act=(b.checkout) onclick="actBtn(this)">"Check out"</button>
                                    <button data-act=(b.merge) data-confirm="Merge this branch into the current one?" onclick="actBtn(this)">"Merge"</button>
                                    <button data-act=(b.rebase) data-confirm="Rebase the current branch onto this one?" onclick="actBtn(this)">"Rebase onto"</button>
                                    <button class="danger" data-act=(b.delete) data-confirm="Delete this branch?" onclick="actBtn(this)">"Delete"</button>
                                }
                                <button data-act=(b.rename) data-ask="New name of the branch" data-field="to" onclick="actBtn(this)">"Rename"</button>
                            </td>
                        </tr>
                    }
                </table>
                <h2>"Remote branches"</h2>
                <table>
                    for b in remote_branches {
                        <tr>
                            <td class="meta">(b.name)</td>
                            <td>
                                <button data-act=(b.checkout) onclick="actBtn(this)">"Check out"</button>
                                <button class="danger" data-act=(b.delete) data-confirm="Delete this branch on the remote? Others lose it too." onclick="actBtn(this)">"Delete on remote"</button>
                            </td>
                        </tr>
                    }
                </table>
            } else if is_stash {
                <div class="bar"><button class="primary" data-act=(stash_save) onclick="actBtn(this)">"Stash changes"</button></div>
                if stashes.is_empty() { <div class="panel">"No stashes."</div> }
                <table>
                    for s in stashes {
                        <tr>
                            <td>(s.label)</td>
                            <td>
                                <button data-act=(s.pop) onclick="actBtn(this)">"Pop"</button>
                                <button data-act=(s.apply) onclick="actBtn(this)">"Apply"</button>
                                <button class="danger" data-act=(s.drop) data-confirm="Drop this stash?" onclick="actBtn(this)">"Drop"</button>
                            </td>
                        </tr>
                    }
                </table>
            } else if is_remotes {
                <table>
                    for m in remotes {
                        <tr>
                            <td>(m.name)</td>
                            <td class="meta">(m.url)</td>
                            <td>
                                <button data-act=(m.fetch) onclick="actBtn(this)">"Fetch"</button>
                                <button data-act=(m.pull) data-confirm="Fast-forward the current branch from this remote?" onclick="actBtn(this)">"Pull"</button>
                                <button data-act=(m.push) data-confirm="Push the current branch to this remote?" onclick="actBtn(this)">"Push"</button>
                                <button class="danger" data-act=(m.force) data-confirm="Force-push (with lease) the current branch? It overwrites the remote branch if nobody pushed since your last fetch." onclick="actBtn(this)">"Force push"</button>
                            </td>
                        </tr>
                    }
                </table>
            }
            }
        )
    })
}
