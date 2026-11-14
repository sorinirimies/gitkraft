//! One job from lock to inbox for the front-ends that stay open (TUI, GUI, web): the same steps
//! as `gitkraft-tui agents run`, except announcing is left to the caller, which owns the inbox
//! the notification poller keeps in memory (so nothing is overwritten).

use std::path::Path;
use std::sync::Arc;

use gitkraft_platforms::HttpClient;
use gitkraft_platforms::model::RepoInfo;

use super::{Deps, HarnessKind, Job, Outcome, Store, harness::SystemExec, run_and_file};
use crate::hub::Hub;

/// Runs `job` now and files its report; `known` is the repository list the picker already has.
///
/// # Errors
/// Another run holds the lock, the policy cannot be read, or the report cannot be written.
pub async fn run_in_app<C: HttpClient + Clone>(
    api: &Hub<C>,
    job: &Job,
    known: Option<Vec<RepoInfo>>,
    store: &Store,
    config_dir: &Path,
) -> Result<Outcome, String> {
    let _lock = store
        .try_lock()
        .ok_or("another run is in progress (the service?)")?;
    let all = match known {
        Some(r) => r,
        None => super::scan::universe(api).await.0,
    };
    let policy = crate::review_ai::Policy::load(config_dir)?;
    let llm = if job.harness == HarnessKind::Builtin {
        crate::wiki_git::blocking(crate::ai::remote_default)
            .await
            .ok()
            .and_then(Result::ok)
    } else {
        None
    };
    let deps = Deps {
        llm: llm.as_ref(),
        exec: Arc::new(SystemExec),
        policy: &policy,
        scratch: std::env::temp_dir(),
    };
    run_and_file(api, job, &all, &deps, store).await
}
