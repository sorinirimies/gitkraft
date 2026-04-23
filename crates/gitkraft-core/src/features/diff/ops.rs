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

/// Return just the list of changed files for a commit — no hunk / line parsing.
///
/// This is much faster than [`get_commit_diff`] because it only reads the
/// tree-level delta metadata.  The GUI uses this to instantly populate the
/// file sidebar when a commit is selected.
pub fn get_commit_file_list(
    repo: &Repository,
    oid_str: &str,
) -> Result<Vec<super::types::DiffFileEntry>> {
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

    let diff = repo
        .diff_tree_to_tree(parent_tree.as_ref(), Some(&commit_tree), None)
        .context("failed to diff commit against parent")?;

    Ok(diff
        .deltas()
        .map(|delta| super::types::DiffFileEntry {
            old_file: delta
                .old_file()
                .path()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            new_file: delta
                .new_file()
                .path()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            status: FileStatus::from_delta(delta.status()),
        })
        .collect())
}

/// Return the diff for a **single file** within a commit.
///
/// Uses `pathspec` filtering so that git2 only walks the hunks / lines for the
/// requested file — much faster than parsing the entire commit diff.
pub fn get_single_file_diff(repo: &Repository, oid_str: &str, file_path: &str) -> Result<DiffInfo> {
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
    opts.pathspec(file_path);

    let diff = repo
        .diff_tree_to_tree(parent_tree.as_ref(), Some(&commit_tree), Some(&mut opts))
        .context("failed to diff commit against parent for single file")?;

    let infos = parse_diff(&diff)?;
    infos
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("file '{}' not found in commit diff", file_path))
}

/// Return the diff of a file between a specific commit and the current working directory.
///
/// This lets the user compare an old revision of a file with their current changes.
/// If the file no longer exists in the working tree, shows the entire file as
/// deleted (all lines removed). If the file is identical, returns an empty diff.
pub fn diff_file_commit_vs_workdir(
    repo: &Repository,
    oid_str: &str,
    file_path: &str,
) -> Result<DiffInfo> {
    let oid =
        git2::Oid::from_str(oid_str).with_context(|| format!("invalid OID string: {oid_str}"))?;
    let commit = repo
        .find_commit(oid)
        .with_context(|| format!("commit {oid_str} not found"))?;
    let commit_tree = commit.tree().context("commit has no tree")?;

    let mut opts = DiffOptions::new();
    opts.pathspec(file_path);

    // Diff: commit tree → working directory (including the index)
    let diff = repo
        .diff_tree_to_workdir_with_index(Some(&commit_tree), Some(&mut opts))
        .context("failed to diff commit tree against working directory")?;

    let infos = parse_diff(&diff)?;

    if let Some(info) = infos.into_iter().next() {
        return Ok(info);
    }

    // Empty diff — check WHY it's empty.
    let in_commit = commit_tree
        .get_path(std::path::Path::new(file_path))
        .is_ok();

    // Check if file exists in the working tree
    let workdir = repo.workdir().context("bare repository")?;
    let in_workdir = workdir.join(file_path).exists();

    match (in_commit, in_workdir) {
        (true, true) => {
            // File exists in both — no changes (identical)
            Ok(DiffInfo {
                old_file: file_path.to_string(),
                new_file: file_path.to_string(),
                status: FileStatus::Modified,
                hunks: vec![DiffHunk {
                    header: "@@ No changes — file is identical @@".to_string(),
                    lines: vec![DiffLine::HunkHeader(
                        "@@ No changes — file is identical to working tree @@".to_string(),
                    )],
                }],
            })
        }
        (true, false) => {
            // File exists in commit but not in working tree — show as all-deleted
            let blob_entry = commit_tree.get_path(std::path::Path::new(file_path))?;
            let mut hunks = Vec::new();
            if let Ok(blob) = repo.find_blob(blob_entry.id()) {
                let content = String::from_utf8_lossy(blob.content());
                let lines: Vec<DiffLine> = std::iter::once(DiffLine::HunkHeader(format!(
                    "@@ File deleted since commit {} @@",
                    &oid_str[..7.min(oid_str.len())]
                )))
                .chain(content.lines().map(|l| DiffLine::Deletion(l.to_string())))
                .collect();

                hunks.push(DiffHunk {
                    header: lines
                        .first()
                        .map(|l| match l {
                            DiffLine::HunkHeader(h) => h.clone(),
                            _ => String::new(),
                        })
                        .unwrap_or_default(),
                    lines,
                });
            }

            Ok(DiffInfo {
                old_file: file_path.to_string(),
                new_file: String::new(),
                status: FileStatus::Deleted,
                hunks,
            })
        }
        (false, true) => {
            // File exists in working tree but not in commit — new file since commit
            Err(anyhow::anyhow!(
                "file '{}' did not exist at commit {} — it was added later",
                file_path,
                &oid_str[..7.min(oid_str.len())]
            ))
        }
        (false, false) => Err(anyhow::anyhow!(
            "file '{}' not found in commit {} or working tree — it may have been renamed",
            file_path,
            &oid_str[..7.min(oid_str.len())]
        )),
    }
}

/// Get the list of files that differ between a commit and the current working directory.
pub fn file_list_commit_vs_workdir(
    repo: &Repository,
    oid_str: &str,
) -> Result<Vec<super::types::DiffFileEntry>> {
    let oid =
        git2::Oid::from_str(oid_str).with_context(|| format!("invalid OID string: {oid_str}"))?;
    let commit = repo
        .find_commit(oid)
        .with_context(|| format!("commit {oid_str} not found"))?;
    let commit_tree = commit.tree().context("commit has no tree")?;

    let diff = repo
        .diff_tree_to_workdir_with_index(Some(&commit_tree), None)
        .context("failed to diff commit tree against working directory")?;

    Ok(diff
        .deltas()
        .map(|delta| super::types::DiffFileEntry {
            old_file: delta
                .old_file()
                .path()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            new_file: delta
                .new_file()
                .path()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            status: FileStatus::from_delta(delta.status()),
        })
        .collect())
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
        let found_idx = infos[current_delta_idx..]
            .iter()
            .position(|info| info.new_file == delta_new && info.old_file == delta_old)
            .map(|pos| pos + current_delta_idx)
            .or_else(|| {
                // Also search from the beginning in case deltas are reordered
                infos[..current_delta_idx]
                    .iter()
                    .position(|info| info.new_file == delta_new && info.old_file == delta_old)
            });

        let found = found_idx.is_some();
        if let Some(idx) = found_idx {
            current_delta_idx = idx;
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

    #[test]
    fn commit_file_list_returns_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commit(tmp.path());
        let head_oid = repo.head().unwrap().target().unwrap().to_string();
        let files = get_commit_file_list(&repo, &head_oid).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].new_file, "hello.txt");
        assert_eq!(files[0].status, FileStatus::New);
        assert_eq!(files[0].display_path(), "hello.txt");
    }

    #[test]
    fn single_file_diff_returns_correct_file() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commit(tmp.path());
        let head_oid = repo.head().unwrap().target().unwrap().to_string();
        let diff = get_single_file_diff(&repo, &head_oid, "hello.txt").unwrap();
        assert_eq!(diff.new_file, "hello.txt");
        assert_eq!(diff.status, FileStatus::New);
        assert!(!diff.hunks.is_empty());
    }

    #[test]
    fn diff_file_commit_vs_workdir_shows_changes() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commit(tmp.path());
        let head_oid = repo.head().unwrap().target().unwrap().to_string();

        // Modify the file in the working directory
        std::fs::write(tmp.path().join("hello.txt"), "Modified content!\n").unwrap();

        let diff = diff_file_commit_vs_workdir(&repo, &head_oid, "hello.txt").unwrap();
        assert_eq!(diff.new_file, "hello.txt");
        assert!(!diff.hunks.is_empty());
    }

    #[test]
    fn file_list_commit_vs_workdir_detects_modified() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commit(tmp.path());
        let head_oid = repo.head().unwrap().target().unwrap().to_string();

        // Modify the file
        std::fs::write(tmp.path().join("hello.txt"), "Changed!\n").unwrap();

        let files = file_list_commit_vs_workdir(&repo, &head_oid).unwrap();
        assert!(!files.is_empty());
        assert_eq!(files[0].display_path(), "hello.txt");
        assert_eq!(files[0].status, FileStatus::Modified);
    }

    #[test]
    fn file_list_commit_vs_workdir_detects_new_file() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commit(tmp.path());
        let head_oid = repo.head().unwrap().target().unwrap().to_string();

        // Add a new file and stage it
        std::fs::write(tmp.path().join("new_file.txt"), "new\n").unwrap();
        let mut index = repo.index().unwrap();
        index
            .add_path(std::path::Path::new("new_file.txt"))
            .unwrap();
        index.write().unwrap();

        let files = file_list_commit_vs_workdir(&repo, &head_oid).unwrap();
        let new = files.iter().find(|f| f.display_path() == "new_file.txt");
        assert!(new.is_some(), "new_file.txt should appear in the diff list");
    }

    #[test]
    fn file_list_commit_vs_workdir_detects_deletion() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commit(tmp.path());
        let head_oid = repo.head().unwrap().target().unwrap().to_string();

        // Delete the committed file
        std::fs::remove_file(tmp.path().join("hello.txt")).unwrap();

        let files = file_list_commit_vs_workdir(&repo, &head_oid).unwrap();
        let deleted = files.iter().find(|f| f.display_path() == "hello.txt");
        assert!(deleted.is_some());
        assert_eq!(deleted.unwrap().status, FileStatus::Deleted);
    }

    #[test]
    fn file_list_commit_vs_workdir_empty_when_unchanged() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commit(tmp.path());
        let head_oid = repo.head().unwrap().target().unwrap().to_string();

        // No changes
        let files = file_list_commit_vs_workdir(&repo, &head_oid).unwrap();
        assert!(
            files.is_empty(),
            "should be empty when working tree matches commit"
        );
    }

    #[test]
    fn single_file_diff_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commit(tmp.path());
        let head_oid = repo.head().unwrap().target().unwrap().to_string();
        let result = get_single_file_diff(&repo, &head_oid, "nonexistent.txt");
        assert!(result.is_err());
    }
}
