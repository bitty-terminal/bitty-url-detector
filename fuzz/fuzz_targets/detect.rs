#![forbid(unsafe_code)]
#![no_main]

//! Fuzz target for `bitty-url-detector` (dev tooling only, never shipped).
//!
//! Feeds arbitrary `&str` input into [`detect_urls`] and asserts the
//! documented contract on every match: `start <= end`, both offsets are
//! UTF-8 char boundaries, `input.get(start..end)` equals the reported text,
//! and matches come back ordered and non-overlapping.

use bitty_url_detector::detect_urls;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    let found = detect_urls(data);
    let mut prev_end: usize = 0;
    for (start, end, url) in found {
        assert!(start <= end);
        assert!(start >= prev_end);
        assert!(data.is_char_boundary(start));
        assert!(data.is_char_boundary(end));
        assert_eq!(data.get(start..end), Some(url.as_str()));
        prev_end = end;
    }
});
