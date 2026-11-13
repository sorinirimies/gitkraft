//! GitKraft Settings
//!
//! Settings, layout, repository history and theme data shared by every
//! GitKraft front-end and background service.
//!
//! | Module | What lives here |
//! |--------|-----------------|
//! | [`persistence`] | `AppSettings`, `LayoutSettings`, repo history and their atomic load/save |
//! | [`notify`] | layered per-project notification config, event filter, bell status |
//! | [`theme`] | `AppTheme`, presets and system dark-mode detection (plain `Rgb` data) |
//!
//! This crate has NO git2, network or UI dependencies.

pub mod notify;
pub mod persistence;
pub mod theme;
