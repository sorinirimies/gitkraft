//! Async command helpers for staging-area operations.
//!
//! Each function clones the `PathBuf`, spawns blocking work that opens the repo
//! inside, performs the staging operation, then re-reads both the working-dir
//! and staged diffs so the UI can update in one shot.

use std::path::PathBuf;

use iced::Task;

use crate::macros::StringErr;
use crate::message::{Message, StagingPayload};

/// Stage a single file, then return the refreshed staging state.
pub(crate) fn stage_file(path: PathBuf, file_path: String) -> Task<Message> {
    staging_op!(path, |repo| gitkraft_core::features::staging::stage_file(
        &repo, &file_path
    ))
}

/// Unstage a single file, then return the refreshed staging state.
pub(crate) fn unstage_file(path: PathBuf, file_path: String) -> Task<Message> {
    staging_op!(path, |repo| gitkraft_core::features::staging::unstage_file(
        &repo, &file_path
    ))
}

/// Stage all unstaged files, then return the refreshed staging state.
pub(crate) fn stage_all(path: PathBuf) -> Task<Message> {
    staging_op!(path, |repo| gitkraft_core::features::staging::stage_all(
        &repo
    ))
}

/// Unstage all staged files, then return the refreshed staging state.
pub(crate) fn unstage_all(path: PathBuf) -> Task<Message> {
    staging_op!(path, |repo| gitkraft_core::features::staging::unstage_all(
        &repo
    ))
}

/// Discard working-directory changes for a single file, then return the
/// refreshed staging state.
pub(crate) fn discard_file(path: PathBuf, file_path: String) -> Task<Message> {
    staging_op!(path, |repo| {
        gitkraft_core::features::staging::discard_file_changes(&repo, &file_path)
    })
}

/// Stage multiple files at once.
pub(crate) fn stage_files(path: PathBuf, file_paths: Vec<String>) -> Task<Message> {
    git_task!(
        Message::StagingUpdated,
        (|| {
            let repo = open_repo!(&path);
            for fp in &file_paths {
                gitkraft_core::features::staging::stage_file(&repo, fp).str_err()?;
            }
            refresh_staging_state(&path)
        })()
    )
}

/// Unstage multiple files at once.
pub(crate) fn unstage_files(path: PathBuf, file_paths: Vec<String>) -> Task<Message> {
    git_task!(
        Message::StagingUpdated,
        (|| {
            let repo = open_repo!(&path);
            for fp in &file_paths {
                gitkraft_core::features::staging::unstage_file(&repo, fp).str_err()?;
            }
            refresh_staging_state(&path)
        })()
    )
}

/// Discard changes for both unstaged and staged files.
/// Staged files are first unstaged, then discarded.
pub(crate) fn discard_all_selected(
    path: PathBuf,
    unstaged_paths: Vec<String>,
    staged_paths: Vec<String>,
) -> Task<Message> {
    git_task!(
        Message::StagingUpdated,
        (|| {
            let repo = open_repo!(&path);
            for fp in &unstaged_paths {
                gitkraft_core::features::staging::discard_file_changes(&repo, fp).str_err()?;
            }
            for fp in &staged_paths {
                gitkraft_core::features::staging::unstage_file(&repo, fp).str_err()?;
                gitkraft_core::features::staging::discard_file_changes(&repo, fp).str_err()?;
            }
            refresh_staging_state(&path)
        })()
    )
}

/// Discard a staged file by first unstaging, then discarding working dir changes.
pub(crate) fn discard_staged_file(path: PathBuf, file_path: String) -> Task<Message> {
    git_task!(
        Message::StagingUpdated,
        (|| {
            let repo = open_repo!(&path);
            gitkraft_core::features::staging::unstage_file(&repo, &file_path).str_err()?;
            gitkraft_core::features::staging::discard_file_changes(&repo, &file_path).str_err()?;
            refresh_staging_state(&path)
        })()
    )
}

// ── Helper ────────────────────────────────────────────────────────────────────

/// Re-read both the working-directory diff and the staged diff so the caller
/// can update the UI in one shot.
pub(crate) fn refresh_staging_state(path: &std::path::Path) -> Result<StagingPayload, String> {
    let repo = open_repo!(path);
    let unstaged = gitkraft_core::features::diff::get_working_dir_file_list(&repo).str_err()?;
    let staged = gitkraft_core::features::diff::get_staged_file_list(&repo).str_err()?;
    Ok(StagingPayload { unstaged, staged })
}

/// Load a single file's diff from the working directory or staging area.
pub(crate) fn load_staging_file_diff(
    path: PathBuf,
    file_path: String,
    staged: bool,
) -> Task<Message> {
    git_task!(
        Message::StagingFileDiffLoaded,
        (|| {
            let repo = open_repo!(&path);
            if staged {
                gitkraft_core::features::diff::get_staged_single_file_diff(&repo, &file_path)
                    .str_err()
            } else {
                gitkraft_core::features::diff::get_working_dir_single_file_diff(&repo, &file_path)
                    .str_err()
            }
        })()
    )
}
