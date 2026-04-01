//! Diff operations — working-directory, staged, and per-commit diffs.

use anyhow::{Context, Result};
use git2::{Diff, DiffFormat, DiffOptions, Repository};

use super::types::{DiffHunk, DiffInfo, DiffLine, FileStatus};

// ── Public API ────────────────────────────────────────────────────────────────

/// Return the diff of unstaged (working-directory) changes against the index.
///
/// Includes untracked files.
pub fn get_working_dir_diff(repo: &Repository) -> Result<Vec<DiffInfo>> {
    let mut opts = DiffOptions::new();
    opts.include_untracked(true);
    opts.recurse_untracked_dirs(true);

    let diff = repo
        .diff_index_to_workdir(None, Some(&mut opts))
        .context("failed to diff working directory against index")?;
    parse_diff(&diff)
}

/// Return the diff of staged (index) changes against HEAD.
///
/// For an initial commit (no HEAD yet), diffs the full index as all-new files.
pub fn get_staged_diff(repo: &Repository) -> Result<Vec<DiffInfo>> {
    let head_tree = match repo.head() {
        Ok(reference) => {
            let commit = reference
                .peel_to_commit()
                .context("HEAD does not point to a commit")?;
            Some(commit.tree().context("commit has no tree")?)
        }
        // No HEAD yet (empty repo) — diff the full index as "new"
        Err(_) => None,
    };

    let diff = repo
        .diff_tree_to_index(head_tree.as_ref(), None, None)
        .context("failed to diff index against HEAD tree")?;
    parse_diff(&diff)
}

/// Return the diff introduced by a specific commit (compared to its first parent).
///
/// For a root commit (no parents), diffs against an empty tree.
pub fn get_commit_diff(repo: &Repository, oid_str: &str) -> Result<Vec<DiffInfo>> {
    let oid =
        git2::Oid::from_str(oid_str).with_context(|| format!("invalid OID string: {oid_str}"))?;
    let commit = repo
        .find_commit(oid)
        .with_context(|| format!("commit {oid_str} not found"))?;
    let commit_tree = commit.tree().context("commit has no tree")?;

    let parent_tree = if commit.parent_count() > 0 {
        let parent = commit.parent(0).context("failed to read parent commit")?;
        Some(parent.tree().context("parent commit has no tree")?)
    } else {
        None
    };

    let mut opts = DiffOptions::new();
    let diff = repo
        .diff_tree_to_tree(parent_tree.as_ref(), Some(&commit_tree), Some(&mut opts))
        .context("failed to diff commit against parent")?;
    parse_diff(&diff)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Walk every delta / hunk / line in a `git2::Diff` and produce our domain
/// `Vec<DiffInfo>`.
fn parse_diff(diff: &Diff<'_>) -> Result<Vec<DiffInfo>> {
    let num_deltas = diff.deltas().len();
    let mut infos: Vec<DiffInfo> = Vec::with_capacity(num_deltas);

    // Pre-populate DiffInfo shells for each delta so the print callback can
    // index into them.
    for delta in diff.deltas() {
        let old_file = delta
            .old_file()
            .path()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let new_file = delta
            .new_file()
            .path()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let status = FileStatus::from_delta(delta.status());
        infos.push(DiffInfo {
            old_file,
            new_file,
            status,
            hunks: Vec::new(),
        });
    }

    // Walk through the diff with the print callback which gives us
    // file / hunk / line events in order.
    let mut current_delta_idx: usize = 0;

    diff.print(DiffFormat::Patch, |delta, maybe_hunk, line| {
        // Identify which delta we are currently processing by matching paths.
        let delta_new = delta
            .new_file()
            .path()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let delta_old = delta
            .old_file()
            .path()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();

        // Find the matching DiffInfo — usually at current_delta_idx or later.
        let mut found = false;
        for i in current_delta_idx..infos.len() {
            if infos[i].new_file == delta_new && infos[i].old_file == delta_old {
                current_delta_idx = i;
                found = true;
                break;
            }
        }
        if !found {
            // Also search from the beginning in case deltas are reordered
            for i in 0..current_delta_idx {
                if infos[i].new_file == delta_new && infos[i].old_file == delta_old {
                    current_delta_idx = i;
                    found = true;
                    break;
                }
            }
        }
        if !found {
            return true; // skip unknown delta
        }

        let info = &mut infos[current_delta_idx];

        // If we have a hunk header, potentially create a new hunk.
        if let Some(hunk) = maybe_hunk {
            let header = String::from_utf8_lossy(hunk.header())
                .trim_end()
                .to_string();

            // Only create a new hunk if the header differs from the current one.
            let needs_new = match info.hunks.last() {
                Some(h) => h.header != header,
                None => true,
            };
            if needs_new {
                info.hunks.push(DiffHunk {
                    header: header.clone(),
                    lines: vec![DiffLine::HunkHeader(header)],
                });
            }
        }

        // Map the line origin to our DiffLine type and append to the current hunk.
        if let Some(hunk) = info.hunks.last_mut() {
            let content = String::from_utf8_lossy(line.content())
                .trim_end_matches('\n')
                .trim_end_matches('\r')
                .to_string();

            let diff_line = match line.origin() {
                '+' | '>' => DiffLine::Addition(content),
                '-' | '<' => DiffLine::Deletion(content),
                ' ' => DiffLine::Context(content),
                // File-level headers ('F'), binary notices ('B'), hunk header origin ('H')
                // — we skip these as they are handled above or are informational.
                _ => return true,
            };
            hunk.lines.push(diff_line);
        }

        true
    })
    .context("failed to walk diff")?;

    Ok(infos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn init_repo_with_commit(dir: &std::path::Path) -> git2::Repository {
        let repo = git2::Repository::init(dir).unwrap();
        {
            let file_path = dir.join("hello.txt");
            fs::write(&file_path, "Hello, world!\n").unwrap();

            let mut index = repo.index().unwrap();
            index.add_path(std::path::Path::new("hello.txt")).unwrap();
            index.write().unwrap();

            let tree_oid = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_oid).unwrap();
            let sig = git2::Signature::now("Test", "test@test.com").unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
                .unwrap();
        }
        repo
    }

    #[test]
    fn working_dir_diff_shows_changes() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commit(tmp.path());

        // Modify the file
        fs::write(tmp.path().join("hello.txt"), "Hello, modified!\n").unwrap();

        let diffs = get_working_dir_diff(&repo).unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].new_file, "hello.txt");
        assert_eq!(diffs[0].status, FileStatus::Modified);
        assert!(!diffs[0].hunks.is_empty());
    }

    #[test]
    fn staged_diff_shows_staged_changes() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commit(tmp.path());

        // Modify and stage the file
        fs::write(tmp.path().join("hello.txt"), "Hello, staged!\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("hello.txt")).unwrap();
        index.write().unwrap();

        let diffs = get_staged_diff(&repo).unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].new_file, "hello.txt");
        assert_eq!(diffs[0].status, FileStatus::Modified);
    }

    #[test]
    fn commit_diff_shows_initial_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commit(tmp.path());

        let head_oid = repo.head().unwrap().target().unwrap().to_string();
        let diffs = get_commit_diff(&repo, &head_oid).unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].new_file, "hello.txt");
        assert_eq!(diffs[0].status, FileStatus::New);
    }

    #[test]
    fn working_dir_diff_untracked_file() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commit(tmp.path());

        // Create a new untracked file
        fs::write(tmp.path().join("new_file.txt"), "I am new!\n").unwrap();

        let diffs = get_working_dir_diff(&repo).unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].new_file, "new_file.txt");
        assert_eq!(diffs[0].status, FileStatus::Untracked);
    }
}
