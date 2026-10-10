#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|msg: &str| {
    let _ = gitkraft_core::check_commit_message(msg);
});
