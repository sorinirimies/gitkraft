#![no_main]
//! What the user types in the clone field (GUI, `gitkraft-tui clone`). Whatever it is, the clone
//! goes into a new folder directly inside the one picked, never above or beside it.
use libfuzzer_sys::fuzz_target;
use std::path::{Component, Path};

fuzz_target!(|spec: &str| {
    let parent = Path::new("/picked");
    let accounts = gitkraft_hub::accounts::AccountsFile::default();
    if let Ok(plan) = gitkraft_hub::clone::plan(spec, None, None, &accounts, parent) {
        assert!(!plan.url.is_empty());
        let rest = plan.dest.strip_prefix(parent).expect("dest inside the picked folder");
        let parts: Vec<_> = rest.components().collect();
        assert!(
            matches!(parts.as_slice(), [Component::Normal(_)]),
            "{spec:?} -> {}",
            plan.dest.display()
        );
    }
});
