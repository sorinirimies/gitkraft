//! A local repository in the browser: history with a graph, the working tree, branches,
//! stashes and remotes. The same core calls the GUI and the TUI make; this file only turns
//! them into plain data and HTML-safe pieces, so the pages stay thin.

use std::path::Path;

use gitkraft_core::CommitAction;
use gitkraft_core::features::{branches, commits, diff, graph, log, remotes, repo, staging, stash};
use serde::Deserialize;

use crate::render::esc;

/// Commits shown at most; a search or a branch switch narrows it.
pub const COMMITS: usize = 300;
/// Diff lines shown at most.
pub const DIFF_LINES: usize = 3000;

/// A change to a local repository. Sent by the page as `{"action":"git","r":0,"op":"stage","path":"a.rs"}`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum GitOp {
    /// Stage a file.
    Stage {
        /// Path relative to the work tree.
        path: String,
    },
    /// Unstage a file.
    Unstage {
        /// Path relative to the work tree.
        path: String,
    },
    /// Stage everything.
    StageAll,
    /// Unstage everything.
    UnstageAll,
    /// Throw away the changes of a file.
    Discard {
        /// Path relative to the work tree.
        path: String,
    },
    /// Commit what is staged.
    Commit {
        /// The message.
        message: String,
    },
    /// Switch to a branch.
    Checkout {
        /// Branch name.
        name: String,
    },
    /// Create a branch at HEAD.
    NewBranch {
        /// Branch name.
        name: String,
    },
    /// Delete a local branch.
    DeleteBranch {
        /// Branch name.
        name: String,
    },
    /// Merge a branch into the current one.
    Merge {
        /// Branch name.
        name: String,
    },
    /// Stash the working tree.
    StashSave {
        /// Optional message.
        #[serde(default)]
        message: Option<String>,
    },
    /// Apply a stash and drop it.
    StashPop {
        /// Stash index.
        index: usize,
    },
    /// Apply a stash and keep it.
    StashApply {
        /// Stash index.
        index: usize,
    },
    /// Drop a stash.
    StashDrop {
        /// Stash index.
        index: usize,
    },
    /// Fetch a remote.
    Fetch {
        /// Remote name.
        remote: String,
    },
    /// Fast-forward the current branch from a remote.
    Pull {
        /// Remote name.
        remote: String,
        /// Branch name.
        branch: String,
    },
    /// Push a branch.
    Push {
        /// Remote name.
        remote: String,
        /// Branch name.
        branch: String,
    },
    /// Push a branch with `--force-with-lease`.
    ForcePush {
        /// Remote name.
        remote: String,
        /// Branch name.
        branch: String,
    },
    /// Rename a local branch.
    RenameBranch {
        /// Current name.
        name: String,
        /// New name.
        to: String,
    },
    /// Rebase the current branch onto another.
    Rebase {
        /// Branch to rebase onto.
        name: String,
    },
    /// Check out a remote branch as a local tracking branch.
    CheckoutRemote {
        /// `remote/branch`.
        name: String,
    },
    /// Delete a branch on its remote.
    DeleteRemoteBranch {
        /// `remote/branch`.
        name: String,
    },
    /// Apply a commit on top of the current branch.
    CherryPick {
        /// The commit.
        oid: String,
    },
    /// Make a commit that undoes another.
    Revert {
        /// The commit.
        oid: String,
    },
    /// Move the current branch to a commit.
    Reset {
        /// The commit.
        oid: String,
        /// `soft`, `mixed` or `hard`.
        mode: String,
    },
    /// Tag a commit; annotated when a message is given.
    Tag {
        /// The commit.
        oid: String,
        /// Tag name.
        name: String,
        /// Message of an annotated tag.
        #[serde(default)]
        message: Option<String>,
    },
    /// Create a branch at a commit.
    BranchAt {
        /// The commit.
        oid: String,
        /// Branch name.
        name: String,
    },
    /// Check out a commit without a branch.
    CheckoutDetached {
        /// The commit.
        oid: String,
    },
}

fn err<E: std::fmt::Display>(e: E) -> String {
    format!("{e:#}")
}

/// Everything about a repository the pages show, in one blocking call.
///
/// # Errors
/// The path is not a repository.
pub fn snapshot(path: &Path) -> Result<repo::RepoSnapshot, String> {
    repo::load_repo_snapshot_with_depth(path, COMMITS).map_err(err)
}

/// A commit and its diff.
///
/// # Errors
/// Unknown commit.
pub fn commit(
    path: &Path,
    oid: &str,
) -> Result<(commits::CommitInfo, Vec<diff::DiffInfo>), String> {
    let r = repo::open_repo(path).map_err(err)?;
    let info = commits::get_commit_details(&r, oid).map_err(err)?;
    let files = diff::get_commit_diff(&r, oid).map_err(err)?;
    Ok((info, files))
}

/// The diff of one file of the working tree (`staged` or not).
///
/// # Errors
/// Unknown file.
pub fn file_diff(path: &Path, file: &str, staged: bool) -> Result<diff::DiffInfo, String> {
    let r = repo::open_repo(path).map_err(err)?;
    let found = if staged {
        diff::get_staged_single_file_diff(&r, file)
    } else {
        diff::get_working_dir_single_file_diff(&r, file)
    };
    found.map_err(err)
}

/// Commits whose message or author contain `query`.
///
/// # Errors
/// The path is not a repository.
pub fn search(path: &Path, query: &str) -> Result<Vec<commits::CommitInfo>, String> {
    let r = repo::open_repo(path).map_err(err)?;
    log::search_commits(&r, query, COMMITS).map_err(err)
}

/// Runs a commit action (the same ones the GUI and TUI offer); "`done` <short id>" on success.
fn on_commit(path: &Path, oid: &str, action: CommitAction, done: &str) -> anyhow::Result<String> {
    action
        .execute(path, oid)
        .map(|()| format!("{done} {}", short(oid)))
}

/// The first 7 characters of a commit id.
fn short(oid: &str) -> &str {
    oid.get(..7).unwrap_or(oid)
}

fn need(name: &str, what: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        Err(format!("type a {what}"))
    } else {
        Ok(())
    }
}

/// Runs `op` in the repository at `path`; a short confirmation or the reason it failed.
///
/// # Errors
/// What git said, in plain words.
pub fn run(path: &Path, op: &GitOp) -> Result<String, String> {
    use GitOp::*;
    let open = || repo::open_repo(path).map_err(err);
    let result = match op {
        Stage { path: f } => staging::stage_file(&open()?, f).map(|()| format!("staged {f}")),
        Unstage { path: f } => staging::unstage_file(&open()?, f).map(|()| format!("unstaged {f}")),
        StageAll => staging::stage_all(&open()?).map(|()| "staged everything".into()),
        UnstageAll => staging::unstage_all(&open()?).map(|()| "unstaged everything".into()),
        Discard { path: f } => {
            staging::discard_file_changes(&open()?, f).map(|()| format!("discarded {f}"))
        }
        Commit { message } => {
            need(message, "commit message")?;
            commits::create_commit(&open()?, message.trim())
                .map(|c| format!("committed {}", c.short_oid()))
        }
        Checkout { name } => {
            branches::checkout_branch(&open()?, name).map(|()| format!("on {name}"))
        }
        NewBranch { name } => {
            need(name, "branch name")?;
            branches::validate_ref_name(name.trim()).map_err(str::to_string)?;
            branches::create_branch(&open()?, name.trim()).map(|_| format!("created {name}"))
        }
        DeleteBranch { name } => {
            branches::delete_branch(&open()?, name).map(|()| format!("deleted {name}"))
        }
        Merge { name } => branches::merge_branch(&open()?, name).map(|()| format!("merged {name}")),
        StashSave { message } => {
            let mut r = open()?;
            stash::stash_save(&mut r, message.as_deref().filter(|m| !m.trim().is_empty()))
                .map(|_| "stashed".into())
        }
        StashPop { index } => {
            stash::stash_pop(&mut open()?, *index).map(|()| "stash applied and dropped".into())
        }
        StashApply { index } => {
            stash::stash_apply(&mut open()?, *index).map(|()| "stash applied".into())
        }
        StashDrop { index } => {
            stash::stash_drop(&mut open()?, *index).map(|()| "stash dropped".into())
        }
        Fetch { remote } => remotes::transport::run_git(path, &["fetch", remote])
            .map(|()| format!("fetched {remote}")),
        Pull { remote, branch } => {
            remotes::transport::run_git(path, &["pull", "--ff-only", remote, branch])
                .map(|()| format!("pulled {remote}/{branch}"))
        }
        Push { remote, branch } => {
            branches::push_branch(path, branch, remote).map(|()| format!("pushed {branch}"))
        }
        ForcePush { remote, branch } => branches::force_push_branch(path, branch, remote)
            .map(|()| format!("force-pushed {branch}")),
        RenameBranch { name, to } => {
            need(to, "branch name")?;
            branches::validate_ref_name(to.trim()).map_err(str::to_string)?;
            branches::rename_branch(&open()?, name, to.trim())
                .map(|()| format!("renamed {name} to {}", to.trim()))
        }
        Rebase { name } => {
            branches::rebase_onto(path, name).map(|()| format!("rebased onto {name}"))
        }
        CheckoutRemote { name } => {
            branches::checkout_remote_branch(path, name).map(|()| format!("checked out {name}"))
        }
        DeleteRemoteBranch { name } => {
            branches::delete_remote_branch(path, name).map(|()| format!("deleted {name}"))
        }
        CherryPick { oid } => on_commit(path, oid, CommitAction::CherryPick, "cherry-picked"),
        Revert { oid } => on_commit(path, oid, CommitAction::Revert, "reverted"),
        Reset { oid, mode } => {
            let action = match mode.as_str() {
                "soft" => CommitAction::ResetSoft,
                "mixed" => CommitAction::ResetMixed,
                "hard" => CommitAction::ResetHard,
                other => return Err(format!("unknown reset mode {other:?}")),
            };
            on_commit(path, oid, action, &format!("reset ({mode}) to"))
        }
        Tag { oid, name, message } => {
            need(name, "tag name")?;
            let name = name.trim().to_string();
            branches::validate_ref_name(&name).map_err(str::to_string)?;
            let action = match message.as_deref().map(str::trim).filter(|m| !m.is_empty()) {
                Some(m) => CommitAction::CreateAnnotatedTag(name.clone(), m.to_string()),
                None => CommitAction::CreateTag(name.clone()),
            };
            on_commit(path, oid, action, "tagged").map(|m| format!("{m} as {name}"))
        }
        BranchAt { oid, name } => {
            need(name, "branch name")?;
            let name = name.trim().to_string();
            branches::validate_ref_name(&name).map_err(str::to_string)?;
            on_commit(
                path,
                oid,
                CommitAction::CreateBranchHere(name.clone()),
                "branched",
            )
            .map(|m| format!("{m} as {name}"))
        }
        CheckoutDetached { oid } => {
            on_commit(path, oid, CommitAction::CheckoutDetached, "detached at")
        }
    };
    result.map_err(err)
}

/// CSS class for a line of a diff.
fn class_of(l: &diff::DiffLine) -> (&'static str, String) {
    match l {
        diff::DiffLine::Context(t) => ("ctx", format!(" {t}")),
        diff::DiffLine::Addition(t) => ("add", format!("+{t}")),
        diff::DiffLine::Deletion(t) => ("del", format!("-{t}")),
        diff::DiffLine::HunkHeader(t) => ("hunk", t.clone()),
    }
}

/// The lines of some file diffs, each with its CSS class, cut after [`DIFF_LINES`].
#[must_use]
pub fn diff_lines(files: &[diff::DiffInfo]) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    for f in files {
        out.push(("hunk", format!("{} {}", f.status, f.display_path())));
        for h in &f.hunks {
            for l in &h.lines {
                if out.len() >= DIFF_LINES {
                    out.push(("hunk", format!("… cut after {DIFF_LINES} lines")));
                    return out;
                }
                out.push(class_of(l));
            }
        }
    }
    out
}

const LANE: f32 = 14.0;
const ROW: f32 = 26.0;

/// The graph cell of one commit row as an inline SVG (lines, then the node on top).
#[must_use]
pub fn graph_svg(row: &graph::GraphRow, width_cols: usize) -> String {
    let w = (width_cols.max(1) as f32) * LANE;
    let x = |c: usize| c as f32 * LANE + LANE / 2.0;
    let mid = ROW / 2.0;
    let mut s = format!(
        "<svg class=\"g\" width=\"{w}\" height=\"{ROW}\" viewBox=\"0 0 {w} {ROW}\" aria-hidden=\"true\">"
    );
    for e in &row.edges {
        let (a, b) = (x(e.from_column), x(e.to_column));
        let c = e.color_index % 8;
        if e.from_column == e.to_column {
            s.push_str(&format!(
                "<line class=\"e{c}\" x1=\"{a}\" y1=\"0\" x2=\"{a}\" y2=\"{ROW}\"/>"
            ));
        } else {
            s.push_str(&format!(
                "<path class=\"e{c}\" d=\"M{a} {mid} C{a} {} {b} {} {b} {ROW}\"/>",
                mid + 6.0,
                mid + 8.0
            ));
        }
    }
    s.push_str(&format!(
        "<circle class=\"n{}\" cx=\"{}\" cy=\"{mid}\" r=\"4.5\"/></svg>",
        row.node_color % 8,
        x(row.node_column)
    ));
    s
}

/// The widest graph row, in lanes.
#[must_use]
pub fn graph_width(rows: &[graph::GraphRow]) -> usize {
    rows.iter().map(|r| r.width).max().unwrap_or(1).min(12)
}

/// A commit's ref labels as small badges (escaped HTML).
#[must_use]
pub fn ref_badges(c: &commits::CommitInfo) -> String {
    use commits::RefKind;
    c.refs
        .iter()
        .map(|r| {
            let k = match r.kind {
                RefKind::Head => "head",
                RefKind::LocalBranch => "local",
                RefKind::RemoteBranch => "remote",
                RefKind::Tag => "tag",
            };
            format!("<span class=\"ref {k}\">{}</span>", esc(&r.name))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn git(dir: &Path, args: &[&str]) {
        let ok = Command::new("git")
            .current_dir(dir)
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(ok, "git {args:?}");
    }

    fn repo_with_commit() -> tempfile::TempDir {
        let d = tempfile::tempdir().expect("tmp");
        git(d.path(), &["init", "-q", "-b", "main"]);
        // The code under test commits and stashes through libgit2, which reads the repo's own
        // config; CI machines have no global identity.
        git(d.path(), &["config", "user.name", "t"]);
        git(d.path(), &["config", "user.email", "t@t"]);
        git(d.path(), &["config", "commit.gpgsign", "false"]);
        std::fs::write(d.path().join("a.txt"), "one\n").expect("write");
        git(d.path(), &["add", "."]);
        git(d.path(), &["commit", "-qm", "first"]);
        d
    }

    #[test]
    fn a_change_is_staged_committed_and_shows_in_history_with_its_diff() {
        let d = repo_with_commit();
        let p = d.path();
        std::fs::write(p.join("a.txt"), "one\ntwo\n").expect("write");
        let snap = snapshot(p).expect("snapshot");
        assert_eq!(snap.commits.len(), 1);
        assert_eq!(snap.unstaged.len(), 1);
        let lines = diff_lines(&[file_diff(p, "a.txt", false).expect("diff")]);
        assert!(
            lines
                .iter()
                .any(|(c, t)| *c == "add" && t == "+two\n" || t.starts_with("+two"))
        );
        run(
            p,
            &GitOp::Stage {
                path: "a.txt".into(),
            },
        )
        .expect("stage");
        assert_eq!(snapshot(p).expect("s").staged.len(), 1);
        assert!(
            run(
                p,
                &GitOp::Commit {
                    message: "  ".into()
                }
            )
            .is_err()
        );
        let ok = run(
            p,
            &GitOp::Commit {
                message: "second".into(),
            },
        )
        .expect("commit");
        assert!(ok.starts_with("committed "), "{ok}");
        let snap = snapshot(p).expect("s");
        assert_eq!(snap.commits.len(), 2);
        let (info, files) = commit(p, &snap.commits[0].oid).expect("commit page");
        assert_eq!(info.summary, "second");
        assert!(!diff_lines(&files).is_empty());
        assert_eq!(search(p, "first").expect("search").len(), 1);
    }

    /// HEAD's commit id.
    fn head(p: &Path) -> String {
        let out = std::process::Command::new("git")
            .current_dir(p)
            .args(["rev-parse", "HEAD"])
            .output()
            .expect("rev-parse");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    /// A repo with two commits on `main`; `(dir, first, second)`.
    fn repo_with_two_commits() -> (tempfile::TempDir, String, String) {
        let d = repo_with_commit();
        let first = head(d.path());
        std::fs::write(d.path().join("b.txt"), "two\n").expect("write");
        git(d.path(), &["add", "."]);
        git(d.path(), &["commit", "-qm", "second"]);
        let second = head(d.path());
        (d, first, second)
    }

    #[test]
    fn a_commit_is_tagged_branched_from_and_checked_out_detached() {
        let (d, first, _) = repo_with_two_commits();
        let p = d.path();
        let tag = |name: &str, message: Option<&str>| GitOp::Tag {
            oid: first.clone(),
            name: name.into(),
            message: message.map(str::to_string),
        };
        assert!(
            run(p, &tag("", None)).is_err(),
            "an empty tag name is refused"
        );
        assert!(run(p, &tag("bad name", None)).is_err());
        assert!(run(p, &tag("v0.1", None)).expect("light").contains("v0.1"));
        run(p, &tag("v0.2", Some("first release"))).expect("annotated");
        run(
            p,
            &GitOp::BranchAt {
                oid: first.clone(),
                name: "old".into(),
            },
        )
        .expect("branch at");
        let snap = snapshot(p).expect("snapshot");
        assert!(snap.branches.iter().any(|b| b.name == "old"));
        run(p, &GitOp::CheckoutDetached { oid: first.clone() }).expect("detach");
        assert_eq!(head(p), first);
    }

    #[test]
    fn cherry_pick_revert_and_the_three_resets_move_history() {
        let (d, first, second) = repo_with_two_commits();
        let p = d.path();
        let reset = |mode: &str| GitOp::Reset {
            oid: first.clone(),
            mode: mode.into(),
        };
        assert!(run(p, &reset("sideways")).is_err());
        run(p, &reset("soft")).expect("soft");
        assert_eq!(head(p), first);
        assert!(
            !snapshot(p).expect("s").staged.is_empty(),
            "soft keeps it staged"
        );
        run(p, &reset("mixed")).expect("mixed");
        let s = snapshot(p).expect("s");
        assert!(
            s.staged.is_empty() && !s.unstaged.is_empty(),
            "mixed unstages it"
        );
        // A hard reset throws away tracked changes; stage the file so it is one (untracked files
        // stay, as with git).
        run(
            p,
            &GitOp::Stage {
                path: "b.txt".into(),
            },
        )
        .expect("stage");
        run(p, &reset("hard")).expect("hard");
        assert!(!p.join("b.txt").exists(), "hard threw the staged file away");

        run(
            p,
            &GitOp::CherryPick {
                oid: second.clone(),
            },
        )
        .expect("cherry-pick");
        assert!(p.join("b.txt").exists());
        run(p, &GitOp::Revert { oid: head(p) }).expect("revert");
        assert!(!p.join("b.txt").exists(), "the revert undid the file");
    }

    #[test]
    fn a_branch_is_renamed_and_the_new_name_is_checked() {
        let d = repo_with_commit();
        let p = d.path();
        run(
            p,
            &GitOp::NewBranch {
                name: "feature".into(),
            },
        )
        .expect("new");
        let rename = |to: &str| GitOp::RenameBranch {
            name: "feature".into(),
            to: to.into(),
        };
        assert!(run(p, &rename(" ")).is_err());
        assert!(run(p, &rename("no spaces")).is_err());
        run(p, &rename("feature-2")).expect("rename");
        let names: Vec<_> = snapshot(p)
            .expect("s")
            .branches
            .into_iter()
            .map(|b| b.name)
            .collect();
        assert!(names.contains(&"feature-2".to_string()), "{names:?}");
        assert!(!names.contains(&"feature".to_string()));
        assert!(
            run(
                p,
                &GitOp::DeleteRemoteBranch {
                    name: "no-slash".into()
                }
            )
            .is_err(),
            "a remote branch needs remote/branch"
        );
    }

    #[test]
    fn branches_and_stashes_round_trip_and_bad_names_are_refused() {
        let d = repo_with_commit();
        let p = d.path();
        assert!(
            run(
                p,
                &GitOp::NewBranch {
                    name: "bad name".into()
                }
            )
            .is_err()
        );
        run(
            p,
            &GitOp::NewBranch {
                name: "topic".into(),
            },
        )
        .expect("new");
        run(
            p,
            &GitOp::Checkout {
                name: "topic".into(),
            },
        )
        .expect("checkout");
        assert_eq!(
            snapshot(p).expect("s").info.head_branch.as_deref(),
            Some("topic")
        );
        run(
            p,
            &GitOp::Checkout {
                name: "main".into(),
            },
        )
        .expect("back");
        run(
            p,
            &GitOp::DeleteBranch {
                name: "topic".into(),
            },
        )
        .expect("delete");
        std::fs::write(p.join("a.txt"), "changed\n").expect("write");
        run(
            p,
            &GitOp::StashSave {
                message: Some("wip".into()),
            },
        )
        .expect("stash");
        assert_eq!(snapshot(p).expect("s").stashes.len(), 1);
        assert!(snapshot(p).expect("s").unstaged.is_empty());
        run(p, &GitOp::StashPop { index: 0 }).expect("pop");
        assert!(snapshot(p).expect("s").stashes.is_empty());
        run(
            p,
            &GitOp::Discard {
                path: "a.txt".into(),
            },
        )
        .expect("discard");
        assert!(snapshot(p).expect("s").unstaged.is_empty());
    }

    #[test]
    fn the_graph_cell_is_an_svg_with_a_node_and_the_ref_badges_are_escaped() {
        let d = repo_with_commit();
        let snap = snapshot(d.path()).expect("s");
        let svg = graph_svg(&snap.graph_rows[0], graph_width(&snap.graph_rows));
        assert!(svg.starts_with("<svg") && svg.contains("<circle"), "{svg}");
        let mut c = snap.commits[0].clone();
        c.refs = vec![commits::RefLabel {
            name: "<b>".into(),
            kind: commits::RefKind::Tag,
        }];
        assert_eq!(ref_badges(&c), "<span class=\"ref tag\">&lt;b&gt;</span>");
    }

    #[test]
    fn a_long_diff_is_cut() {
        let big = diff::DiffInfo {
            old_file: "a".into(),
            new_file: "a".into(),
            status: diff::FileStatus::Modified,
            hunks: vec![diff::DiffHunk {
                header: "@@".into(),
                lines: (0..DIFF_LINES + 50)
                    .map(|i| diff::DiffLine::Addition(i.to_string()))
                    .collect(),
            }],
        };
        let l = diff_lines(&[big]);
        assert_eq!(l.len(), DIFF_LINES + 1);
        assert!(l.last().expect("last").1.contains("cut after"));
    }
}
