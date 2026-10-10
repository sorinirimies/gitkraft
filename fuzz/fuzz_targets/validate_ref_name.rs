#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|name: &str| {
    let _ = gitkraft_core::validate_ref_name(name);
});
