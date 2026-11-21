#![no_main]
//! Raw CI logs (ANSI colours, group markers, annotations) come from the forge.
use libfuzzer_sys::fuzz_target;

fuzz_target!(|raw: &str| {
    let _ = gitkraft_ci::log::parse_log(raw);
});
