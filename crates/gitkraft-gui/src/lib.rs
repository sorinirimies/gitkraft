//! GitKraft GUI — Iced 0.13.1 desktop frontend for the GitKraft Git IDE.
//!
//! This crate provides the graphical user interface built on top of
//! [`gitkraft_core`] using the [Iced](https://iced.rs) toolkit.

#[macro_use]
mod macros;

pub mod features;
pub mod icons;
pub mod message;
pub mod state;
pub mod theme;
pub mod update;
pub mod view;
pub mod view_utils;
pub mod widgets;

pub use message::Message;
pub use state::{GitKraft, RepoTab};
