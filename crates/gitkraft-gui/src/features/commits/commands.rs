//! Async command helpers for commit operations.
//!
//! Each function spawns blocking work on a background thread via the
//! `git_task!` macro, performs the git operation, and maps the result into a
//! [`Message`].

use std::path::PathBuf;

use iced::Task;

use crate::message::Message;

/// Load just the file list (paths + statuses) for a commit — no line parsing.
pub fn load_commit_file_list(path: PathBuf, oid: String) -> Task<Message> {
    git_task!(
        Message::CommitFileListLoaded,
        (|| {
            let repo = open_repo!(&path);
            gitkraft_core::features::diff::get_commit_file_list(&repo, &oid)
                .map_err(|e| e.to_string())
        })()
    )
}

/// Load the full diff for a single file in a commit.
pub fn load_single_file_diff(path: PathBuf, oid: String, file_path: String) -> Task<Message> {
    git_task!(
        Message::SingleFileDiffLoaded,
        (|| {
            let repo = open_repo!(&path);
            gitkraft_core::features::diff::get_single_file_diff(&repo, &oid, &file_path)
                .map_err(|e| e.to_string())
        })()
    )
}

/// Create a new commit with the currently staged changes.
/// Search commits by query string (searches message, author, SHA).
pub fn search_commits(path: PathBuf, query: String) -> Task<Message> {
    git_task!(
        Message::SearchResultsLoaded,
        (|| {
            let repo = open_repo!(&path);
            gitkraft_core::features::log::search_commits(&repo, &query, 100)
                .map_err(|e| e.to_string())
        })()
    )
}

/// Load the file list of changes between a commit and the current working tree.
pub fn search_diff_file_list(path: PathBuf, oid: String) -> Task<Message> {
    git_task!(
        Message::SearchDiffFilesLoaded,
        (|| {
            let repo = open_repo!(&path);
            gitkraft_core::features::diff::file_list_commit_vs_workdir(&repo, &oid)
                .map_err(|e| e.to_string())
        })()
    )
}

/// Diff a single file from a specific commit against the current working tree (for search overlay).
pub fn search_diff_file(path: PathBuf, oid: String, file_path: String) -> Task<Message> {
    git_task!(
        Message::SearchFileDiffLoaded,
        (|| {
            let repo = open_repo!(&path);
            gitkraft_core::features::diff::diff_file_commit_vs_workdir(&repo, &oid, &file_path)
                .map_err(|e| e.to_string())
        })()
    )
}

/// Diff multiple files from a specific commit against the current working tree.
pub fn search_diff_multi_files(
    path: PathBuf,
    oid: String,
    file_paths: Vec<String>,
) -> Task<Message> {
    git_task!(
        Message::SearchMultiDiffLoaded,
        (|| {
            let repo = open_repo!(&path);
            let mut diffs = Vec::with_capacity(file_paths.len());
            for fp in &file_paths {
                match gitkraft_core::features::diff::diff_file_commit_vs_workdir(&repo, &oid, fp) {
                    Ok(diff) => diffs.push(diff),
                    Err(e) => {
                        return Err(format!("{fp}: {e}"));
                    }
                }
            }
            Ok(diffs)
        })()
    )
}

/// Diff a file from a specific commit against the current working tree.
pub fn diff_file_with_working_tree(path: PathBuf, oid: String, file_path: String) -> Task<Message> {
    git_task!(
        Message::DiffWithWorkingTreeLoaded,
        (|| {
            let repo = open_repo!(&path);
            gitkraft_core::features::diff::diff_file_commit_vs_workdir(&repo, &oid, &file_path)
                .map_err(|e| e.to_string())
        })()
    )
}

pub fn create_commit(path: PathBuf, message: String) -> Task<Message> {
    git_task!(
        Message::CommitCreated,
        (|| {
            let repo = open_repo!(&path);
            gitkraft_core::features::commits::create_commit(&repo, &message)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })()
    )
}
