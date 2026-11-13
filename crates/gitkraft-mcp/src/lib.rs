//! GitKraft as an MCP (Model Context Protocol) server.
//!
//! The same accounts, platforms and CI backends the TUI, the GUI and the web front-end use, offered
//! to an AI assistant as tools. Read tools are always there; tools that change something exist only
//! when the server is started with `--write`, and a force merge needs `--allow-force` on top.
//!
//! Stdout carries the protocol, so nothing but MCP is ever printed there.

pub mod config;
pub mod server;

mod read;
mod write;

pub use config::{Cli, Config, HELP, Policy};
pub use server::GitkraftServer;

#[cfg(test)]
mod tests;
