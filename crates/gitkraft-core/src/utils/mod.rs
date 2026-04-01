//! Utility helpers shared across the crate — OID formatting, relative time, etc.

pub mod time;

pub use time::{fmt_oid, relative_time, short_oid};
