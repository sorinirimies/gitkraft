#![no_main]
//! PR diffs come from the forge: mapping lines to inline-comment anchors must not panic, and
//! gives exactly one entry per line (the web and GUI index it by line).
use libfuzzer_sys::fuzz_target;

fuzz_target!(|diff: &str| {
    let pos = gitkraft_platforms::diffpos::positions(diff);
    assert_eq!(pos.len(), diff.lines().count());
    for p in &pos {
        let _ = p.anchor();
    }
});
