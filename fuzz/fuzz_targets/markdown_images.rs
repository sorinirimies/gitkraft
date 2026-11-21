#![no_main]
//! PR and issue bodies are anyone's markdown.
use libfuzzer_sys::fuzz_target;

fuzz_target!(|md: &str| {
    for url in gitkraft_hub::markdown::image_urls(md) {
        assert!(!url.is_empty());
    }
});
