//! Internal macros shared across all GUI feature modules.

/// Build an `iced::Task<Message>` that runs blocking git work on a background
/// thread via a `futures` oneshot channel.
///
/// # Arguments
///
/// * `$msg` – the `Message` constructor passed to `Task::perform` as the
///   mapping function (e.g. `Message::BranchCheckedOut`).
/// * `$body` – an expression evaluated inside the spawned thread that must
///   return `Result<T, String>`.  Use an IIFE `(|| { … })()` when the
///   `?`-operator is needed, or a plain function call when a single fallible
///   call suffices.
///
/// # Example
///
/// ```ignore
/// git_task!(Message::BranchCheckedOut, (|| {
///     let repo = open_repo(&path)?;
///     checkout_branch(&repo, &name).map_err(|e| e.to_string())
/// })())
/// ```
macro_rules! git_task {
    ($msg:expr, $body:expr) => {
        Task::perform(
            async move {
                let (tx, rx) = futures::channel::oneshot::channel();
                std::thread::spawn(move || {
                    let _ = tx.send($body);
                });
                rx.await.map_err(|_| "Task cancelled".to_string())?
            },
            $msg,
        )
    };
}

/// Guard an update-handler arm behind the active tab's `repo_path`.
///
/// Reads `$state.active_tab().repo_path`, returns `Task::none()` when it is
/// `None`, and evaluates `$cmd` with `repo_path: PathBuf` in scope when it is
/// `Some`.
///
/// Two forms:
///
/// * **Plain** – sets only `status_message`, then evaluates `$cmd`:
///   ```ignore
///   with_repo!(state, "Staging…".into(), |repo_path|
///       commands::stage_file(repo_path, f))
///   ```
///
/// * **Loading** – additionally sets `is_loading = true` before `$cmd`:
///   ```ignore
///   with_repo!(state, loading, "Checking out…".into(), |repo_path|
///       commands::checkout_branch(repo_path, name))
///   ```
///
/// The `|$ident|` token before `$cmd` names the `PathBuf` variable that is
/// bound from `repo_path` and made available inside `$cmd`.  Passing the name
/// as a macro parameter avoids Rust's `macro_rules!` hygiene restriction that
/// would otherwise hide a macro-internal binding from call-site expressions.
///
/// Extra tab mutations can be performed inside a block `$cmd`; the
/// `is_loading` / `status_message` mutable borrow has already been dropped.
macro_rules! with_repo {
    // ── plain variant (status only) ────────────────────────────────────────
    ($state:expr, $status:expr, |$rp:ident| $cmd:expr) => {{
        let $rp = $state.active_tab().repo_path.clone();
        if let Some($rp) = $rp {
            $state.active_tab_mut().status_message = Some($status);
            $cmd
        } else {
            Task::none()
        }
    }};

    // ── loading variant (is_loading + status) ──────────────────────────────
    ($state:expr, loading, $status:expr, |$rp:ident| $cmd:expr) => {{
        let $rp = $state.active_tab().repo_path.clone();
        if let Some($rp) = $rp {
            {
                let tab = $state.active_tab_mut();
                tab.is_loading = true;
                tab.status_message = Some($status);
            }
            $cmd
        } else {
            Task::none()
        }
    }};
}

/// Create a Bootstrap icon text widget.
///
/// Two forms: 3-arg `icon!(CHAR, SIZE, COLOR)` or 2-arg `icon!(CHAR, SIZE)`.
macro_rules! icon {
    ($char:expr, $size:expr, $color:expr) => {
        text($char)
            .font(iced_fonts::BOOTSTRAP_FONT)
            .size($size)
            .color($color)
    };
    ($char:expr, $size:expr) => {
        text($char).font(iced_fonts::BOOTSTRAP_FONT).size($size)
    };
}

/// Open a git repository at the given path, mapping the error to `String`.
///
/// Shorthand for `gitkraft_core::features::repo::open_repo($path).map_err(|e| e.to_string())?`.
/// Used inside `git_task!` closures where the `?` operator is available.
macro_rules! open_repo {
    ($path:expr) => {
        gitkraft_core::features::repo::open_repo($path).map_err(|e| e.to_string())?
    };
}
