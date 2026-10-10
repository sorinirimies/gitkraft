#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: (&str, u8)| {
    let (s, n) = data;
    let _ = gitkraft_core::truncate_str(s, n as usize);
    let _ = gitkraft_core::path_basename(s);
    let _ = gitkraft_core::short_oid_str(s);
    let _ = gitkraft_core::theme_index_by_name(s);
});
