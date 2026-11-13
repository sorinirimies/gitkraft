//! GitKraft CI
//!
//! Runs, jobs, steps and logs of GitHub Actions, Gitea/Forgejo Actions and
//! GitLab CI behind one model, plus the UI-free logic around them.

pub mod annotations;
pub mod backend;
pub mod events;
mod github_like;
mod gitlab;
pub mod log;
pub mod model;
pub mod stats;
pub mod timing;
pub mod watch;

pub use model::{Job, RerunScope, Run, RunDetail, RunQuery, Status, Step};

pub use backend::CiBackend;
pub use github_like::ActionsBackend;
pub use gitlab::GitLabCi;
