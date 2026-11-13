//! SSH for git: keys, `~/.ssh/config`, registering keys on the platform, and a guided setup.
//!
//! gitkraft keeps using the system `git` and `ssh` for transport. This module only helps get
//! a working key in place: it never implements the protocol and never sees a passphrase.

pub mod config;
pub mod key;
pub(crate) mod remote;
pub mod render;
pub mod setup;
pub mod tools;

pub use key::{LocalKey, PublicKey, local_keys};
pub use setup::{Options, Report, SetupError, Step};
pub use tools::{FakeSsh, Probe, SshTools, SystemSsh};
