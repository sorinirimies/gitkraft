//! GitKraft Platforms
//!
//! GitHub, GitLab and Gitea/Forgejo/Codeberg behind one normalized model.
//! Front-ends talk to [`Backend`] and never see a platform-specific type.
//!
//! | Module | What lives here |
//! |--------|-----------------|
//! | [`model`] | `PullRequest`, `Issue`, `Review`, `Notification`, `ActivityDay`… |
//! | [`platform`] | `PlatformKind`, `RepoId`, remote-URL parsing, host detection |
//! | [`account`], [`auth`] | accounts and token lookup (env, platform CLI) |
//! | [`http`] | the `HttpClient` seam, the `reqwest` client, the ETag cache |
//! | [`ssh`] | SSH keys: setup, `~/.ssh/config` block, registering keys on the platform |
//! | [`notify`] | poll diffing, alerts, inbox, desktop dispatch, poller status |
//! | [`backend`] | the per-account adapter used by every front-end |
//! | [`testing`] | a fake client for tests |
//!
//! The crate never creates an async runtime: the binary owns it.

pub mod account;
pub mod api;
pub mod auth;
pub mod backend;
mod community;
pub mod credentials;
pub mod diffpos;
pub mod error;
mod github_like;
mod gitlab;
pub mod http;
pub mod model;
pub mod notify;
pub mod oauth;
pub mod platform;
pub mod ssh;
pub mod testing;

pub use account::{Account, Secret, Woodpecker};
pub use backend::Backend;
pub use error::{PlatformError, Result};
pub use github_like::GitHubLike;
pub use gitlab::{GitLab, ItemKind};
pub use http::{CachedClient, DynClient, HttpClient, ReqwestClient};
pub use model::{MergeOptions, MergeOutcome, MergeTiming, UpdateStyle};
pub use platform::{HostMap, PlatformKind, RepoId, Transport, parse_remote_url, url_encode};
