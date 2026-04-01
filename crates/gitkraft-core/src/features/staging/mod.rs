//! Staging-area (index) operations — stage, unstage, and discard changes.

mod ops;

pub use ops::{discard_file_changes, stage_all, stage_file, unstage_all, unstage_file};
