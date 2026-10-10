#![no_main]
use libfuzzer_sys::fuzz_target;

// Settings files are user-editable JSON: deserialization must never panic.
fuzz_target!(|data: &[u8]| {
    let _ = serde_json::from_slice::<gitkraft_core::AppSettings>(data);
});
