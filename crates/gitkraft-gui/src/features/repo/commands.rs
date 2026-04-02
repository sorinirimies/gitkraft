//! Async command helpers for repository operations.
//!
//! Each function returns an `iced::Task<Message>` that performs blocking git
//! work on a background thread via `std::thread::spawn` + a `futures` oneshot
//! channel, then maps the result into a [`Message`] variant the update loop
//! can handle.

use std::path::PathBuf;

use iced::Task;

use crate::message::{Message, RepoPayload};

/// Open a folder-picker dialog and return the selected path.
pub fn pick_folder_open() -> Task<Message> {
    Task::perform(
        async {
            let handle = rfd::AsyncFileDialog::new()
                .set_title("Open Git Repository")
                .pick_folder()
                .await;
            handle.map(|h| h.path().to_path_buf())
        },
        Message::RepoSelected,
    )
}

/// Open a folder-picker dialog for initialising a new repository.
pub fn pick_folder_init() -> Task<Message> {
    Task::perform(
        async {
            let handle = rfd::AsyncFileDialog::new()
                .set_title("Choose folder for new repository")
                .pick_folder()
                .await;
            handle.map(|h| h.path().to_path_buf())
        },
        Message::RepoInitSelected,
    )
}

/// Load (open) a repository at `path` and gather all initial state.
pub fn load_repo(path: PathBuf) -> Task<Message> {
    Task::perform(
        async move {
            let (tx, rx) = futures::channel::oneshot::channel();
            std::thread::spawn(move || {
                let _ = tx.send(load_repo_blocking(&path));
            });
            rx.await.map_err(|_| "Task cancelled".to_string())?
        },
        Message::RepoOpened,
    )
}

/// Initialise a new repository at `path` and then load it.
pub fn init_repo(path: PathBuf) -> Task<Message> {
    Task::perform(
        async move {
            let (tx, rx) = futures::channel::oneshot::channel();
            std::thread::spawn(move || {
                let result = (|| {
                    gitkraft_core::features::repo::init_repo(&path).map_err(|e| e.to_string())?;
                    load_repo_blocking(&path)
                })();
                let _ = tx.send(result);
            });
            rx.await.map_err(|_| "Task cancelled".to_string())?
        },
        Message::RepoOpened,
    )
}

/// Refresh all data for an already-open repository.
pub fn refresh_repo(path: PathBuf) -> Task<Message> {
    Task::perform(
        async move {
            let (tx, rx) = futures::channel::oneshot::channel();
            std::thread::spawn(move || {
                let _ = tx.send(load_repo_blocking(&path));
            });
            rx.await.map_err(|_| "Task cancelled".to_string())?
        },
        Message::RepoRefreshed,
    )
}

/// Blocking helper shared by `load_repo` and `refresh_repo`.
///
/// Opens the repository and collects every piece of state the UI needs into a
/// single [`RepoPayload`].
fn load_repo_blocking(path: &std::path::Path) -> Result<RepoPayload, String> {
    // Open the repository once and reuse the handle for every operation.
    // `list_stashes` needs `&mut`, so we declare the binding as `mut`.
    let mut repo = gitkraft_core::features::repo::open_repo(path).map_err(|e| e.to_string())?;

    let info = gitkraft_core::features::repo::get_repo_info(&repo).map_err(|e| e.to_string())?;
    let branches =
        gitkraft_core::features::branches::list_branches(&repo).map_err(|e| e.to_string())?;
    let commits =
        gitkraft_core::features::commits::list_commits(&repo, 500).map_err(|e| e.to_string())?;
    let graph_rows = gitkraft_core::features::graph::build_graph(&commits);
    let unstaged =
        gitkraft_core::features::diff::get_working_dir_diff(&repo).map_err(|e| e.to_string())?;
    let staged =
        gitkraft_core::features::diff::get_staged_diff(&repo).map_err(|e| e.to_string())?;
    let remotes =
        gitkraft_core::features::remotes::list_remotes(&repo).map_err(|e| e.to_string())?;
    let stashes =
        gitkraft_core::features::stash::list_stashes(&mut repo).map_err(|e| e.to_string())?;

    Ok(RepoPayload {
        info,
        branches,
        commits,
        graph_rows,
        unstaged,
        staged,
        stashes,
        remotes,
    })
}
