//! `bitty-url-detector`: pure-algorithm plaintext URL detector for Bitty.
//!
//! Scanner side of [bitty#1760](https://github.com/bitty-terminal/bitty):
//! a fast, non-blocking, hand-rolled scanner that finds plaintext URLs in a
//! terminal line. It is a pure function over `&str` and knows nothing about
//! terminal grids, cells, GPUs, or windows; mapping byte offsets to display
//! cells and wiring hover/click behavior belong to Bitty Core (separate task).
//!
//! # API
//!
//! ```rust
//! use bitty_url_detector::detect_urls;
//!
//! let found = detect_urls("see https://example.com/a?b=c#d for details");
//! assert_eq!(found.len(), 1);
//! assert_eq!(found[0].2, "https://example.com/a?b=c#d");
//! ```
//!
//! [`detect_urls`] returns one `(col_start, col_end, url)` tuple per match:
//!
//! - `col_start` is inclusive, `col_end` is exclusive. Both are **byte
//!   offsets** into the input (equal to terminal columns for the ASCII case).
//! - Both offsets are always UTF-8 char boundaries, so `input[col_start..col_end]`
//!   is always valid and equals `url`. Non-ASCII bytes terminate a match;
//!   the scanner never splits a multi-byte sequence.
//! - Schemes (case-insensitive): `http://`, `https://`, `git://`, `mailto:`.
//! - A match needs at least one URL-alphabet character after the scheme.
//! - Trailing prose punctuation (`.`, `,`, `;`, `:`, `!`, `?`, quotes) is
//!   trimmed; closing brackets (`)`, `]`, `}`) are trimmed only when
//!   unbalanced, so balanced parens inside URLs survive.
//! - Matches longer than [`MAX_URL_LEN`] body bytes are truncated to the cap
//!   (documented best-effort bound, not a validity claim).
//!
//! # Performance
//!
//! Time is linear in the input length, space is linear in the number of
//! matches. There is no regex engine and no backtracking: the scanner makes a
//! single left-to-right pass with constant-time scheme checks. See
//! `benches/scan.rs` (`cargo bench -p bitty-url-detector --bench scan`).
//!
//! # Guarantees
//!
//! - `#![forbid(unsafe_code)]`, `std` only, zero dependencies.
//! - No `unwrap`/`expect`/`panic!` in non-test code: degenerate input yields
//!   fewer matches, never a panic (enforced by review, not by the type system).

#![forbid(unsafe_code)]

/// Maximum number of URL-body bytes scanned after a scheme prefix.
///
/// Bound for the non-blocking requirement: a pathological line (megabyte of
/// digits) still scans in linear time with a bounded per-match tail. Matches
/// longer than the cap are truncated, never skipped. The value is a named
/// policy constant, not a validity limit: URL validity stays with Core's
/// safety validation pipeline.
pub const MAX_URL_LEN: usize = 4096;

/// Detect plaintext URLs in `input`.
///
/// Returns `(col_start, col_end, url)` tuples in left-to-right order, where
/// `col_start` (inclusive) and `col_end` (exclusive) are byte offsets into
/// `input` that are always char boundaries, and `url` is the matched text.
///
/// Supported schemes (ASCII case-insensitive): `http://`, `https://`,
/// `git://`, `mailto:`. See the crate-level documentation for trimming,
/// truncation, and complexity rules.
pub fn detect_urls(input: &str) -> Vec<(usize, usize, String)> {
    let bytes = input.as_bytes();
    let total = bytes.len();
    let mut out: Vec<(usize, usize, String)> = Vec::new();
    let mut i = 0;
    while i < total {
        let prefix_len = match match_scheme_at(bytes, i) {
            Some(len) => len,
            None => {
                i += 1;
                continue;
            }
        };
        let body_start = i.saturating_add(prefix_len);
        let cap = body_start.saturating_add(MAX_URL_LEN).min(total);
        let mut j = body_start;
        while j < cap {
            match bytes.get(j) {
                Some(b) => {
                    if is_url_byte(*b) {
                        j += 1;
                    } else {
                        break;
                    }
                }
                None => break,
            }
        }
        let end = trim_trailing(bytes, body_start, j);
        if end > body_start && input.is_char_boundary(i) && input.is_char_boundary(end) {
            if let Some(text) = input.get(i..end) {
                out.push((i, end, text.to_string()));
                i = end;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Length of the scheme prefix starting at byte offset `i`, or `None`.
///
/// Matching is ASCII case-insensitive (`HTTP://` works). Only ASCII bytes are
/// compared, so this can never match inside a multi-byte UTF-8 sequence.
fn match_scheme_at(bytes: &[u8], i: usize) -> Option<usize> {
    let first = bytes.get(i)?.to_ascii_lowercase();
    match first {
        b'h' => {
            if matches_prefix(bytes, i, b"https://") {
                Some(8)
            } else if matches_prefix(bytes, i, b"http://") {
                Some(7)
            } else {
                None
            }
        }
        b'g' => {
            if matches_prefix(bytes, i, b"git://") {
                Some(6)
            } else {
                None
            }
        }
        b'm' => {
            if matches_prefix(bytes, i, b"mailto:") {
                Some(7)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Case-insensitive ASCII prefix comparison. Panic-free: out-of-range bytes
/// compare as mismatch via [`slice::get`].
fn matches_prefix(bytes: &[u8], i: usize, prefix: &[u8]) -> bool {
    let mut k = 0;
    while k < prefix.len() {
        match bytes.get(i.saturating_add(k)) {
            Some(b) => {
                if b.to_ascii_lowercase() != prefix[k] {
                    return false;
                }
            }
            None => return false,
        }
        k += 1;
    }
    true
}

/// URL-body alphabet: ASCII alphanumerics plus the RFC 3986 sub-delims and
/// structural characters the detector accepts. Everything else (whitespace,
/// quotes except `'`, angle brackets, backslash, caret, backtick, braces for
/// the body scan, controls, DEL, non-ASCII) terminates the match.
fn is_url_byte(b: u8) -> bool {
    matches!(
        b,
        b'0'..=b'9'
            | b'A'..=b'Z'
            | b'a'..=b'z'
            | b'-'
            | b'.'
            | b'_'
            | b'~'
            | b':'
            | b'/'
            | b'?'
            | b'#'
            | b'['
            | b']'
            | b'@'
            | b'!'
            | b'$'
            | b'&'
            | b'\''
            | b'('
            | b')'
            | b'*'
            | b'+'
            | b','
            | b';'
            | b'='
            | b'%'
    )
}

/// Trim trailing prose punctuation from a candidate `[start, end)` match.
///
/// Unconditional trim set: `.` `,` `;` `:` `!` `?` `'` `"`. Closing brackets
/// (`)` `]` `}`) are trimmed only while unbalanced against their opener, so
/// `(.../Test_(a))` keeps its balanced paren while `(see .../foo)` drops the
/// prose paren. Panic-free; runs in linear time over the candidate.
///
/// Note: `{`, `}`, and `"` never appear in the candidate because the body
/// scan stops at them (see [`is_url_byte`]), so the `}` and `"` arms below
/// are unreachable today. They stay as defense-in-depth for future alphabet
/// edits; `brace_and_quote_bytes_terminate_scan` pins the termination.
fn trim_trailing(bytes: &[u8], start: usize, end: usize) -> usize {
    let mut open_paren: usize = 0;
    let mut close_paren: usize = 0;
    let mut open_bracket: usize = 0;
    let mut close_bracket: usize = 0;
    let mut open_brace: usize = 0;
    let mut close_brace: usize = 0;
    let mut k = start;
    while k < end {
        match bytes.get(k) {
            Some(b'(') => open_paren += 1,
            Some(b')') => close_paren += 1,
            Some(b'[') => open_bracket += 1,
            Some(b']') => close_bracket += 1,
            Some(b'{') => open_brace += 1,
            Some(b'}') => close_brace += 1,
            _ => {}
        }
        k += 1;
    }
    let mut e = end;
    loop {
        if e <= start {
            break;
        }
        let b = match bytes.get(e - 1) {
            Some(b) => *b,
            None => break,
        };
        if matches!(b, b'.' | b',' | b';' | b':' | b'!' | b'?' | b'\'' | b'"') {
            e -= 1;
            continue;
        }
        if b == b')' && close_paren > open_paren {
            close_paren -= 1;
            e -= 1;
            continue;
        }
        if b == b']' && close_bracket > open_bracket {
            close_bracket -= 1;
            e -= 1;
            continue;
        }
        if b == b'}' && close_brace > open_brace {
            close_brace -= 1;
            e -= 1;
            continue;
        }
        break;
    }
    e
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(input: &str) -> (usize, usize, String) {
        let found = detect_urls(input);
        assert_eq!(found.len(), 1, "expected one URL in {input:?}");
        found.into_iter().next().unwrap()
    }

    #[test]
    fn empty_input_finds_nothing() {
        assert!(detect_urls("").is_empty());
    }

    #[test]
    fn plain_text_finds_nothing() {
        assert!(detect_urls("just some words 12345").is_empty());
        assert!(detect_urls("mailtoinfo httpfoo gitbar").is_empty());
    }

    #[test]
    fn http_basic_offsets() {
        let input = "see http://example.com here";
        let (s, e, url) = one(input);
        assert_eq!(url, "http://example.com");
        assert_eq!(s, 4);
        assert_eq!(e, 4 + url.len());
        assert_eq!(input.get(s..e), Some(url.as_str()));
    }

    #[test]
    fn https_with_path_query_fragment() {
        let (s, e, url) = one("go https://example.com/a/b?x=1&y=2#frag end");
        assert_eq!(url, "https://example.com/a/b?x=1&y=2#frag");
        assert_eq!(s, 3);
        assert_eq!(e, 3 + url.len());
    }

    #[test]
    fn git_scheme() {
        let (_, _, url) = one("clone git://git.example.com/repo.git now");
        assert_eq!(url, "git://git.example.com/repo.git");
    }

    #[test]
    fn mailto_basic() {
        let (_, _, url) = one("contact mailto:ops@example.com today");
        assert_eq!(url, "mailto:ops@example.com");
    }

    #[test]
    fn mailto_with_query() {
        let (_, _, url) = one("write mailto:ops@example.com?subject=hi&body=x!");
        assert_eq!(url, "mailto:ops@example.com?subject=hi&body=x");
    }

    #[test]
    fn multiple_urls_in_order() {
        let input = "a http://one.example b https://two.example/c d";
        let found = detect_urls(input);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].2, "http://one.example");
        assert_eq!(found[1].2, "https://two.example/c");
        assert!(found[0].0 < found[1].0);
        for (s, e, url) in &found {
            assert_eq!(input.get(*s..*e), Some(url.as_str()));
        }
    }

    #[test]
    fn url_at_start_and_end() {
        let (s, e, url) = one("https://example.com/a");
        assert_eq!((s, e), (0, url.len()));
        let input = "go https://example.com/a";
        let (s2, e2, url2) = one(input);
        assert_eq!(e2, input.len());
        assert_eq!(input.get(s2..e2), Some(url2.as_str()));
        assert_eq!(url2, "https://example.com/a");
    }

    #[test]
    fn trailing_prose_punctuation_trimmed() {
        for (input, want) in [
            ("x https://example.com/a.", "https://example.com/a"),
            ("x https://example.com/a,", "https://example.com/a"),
            ("x https://example.com/a;", "https://example.com/a"),
            ("x https://example.com/a:", "https://example.com/a"),
            ("x https://example.com/a!", "https://example.com/a"),
            ("x https://example.com/a?", "https://example.com/a"),
            ("x https://example.com/a...", "https://example.com/a"),
            ("x mailto:ops@example.com.", "mailto:ops@example.com"),
        ] {
            let (_, _, url) = one(input);
            assert_eq!(url, want, "input {input:?}");
        }
    }

    #[test]
    fn balanced_paren_survives() {
        let (_, _, url) = one("(https://en.wikipedia.org/wiki/Test_(a))");
        assert_eq!(url, "https://en.wikipedia.org/wiki/Test_(a)");
    }

    #[test]
    fn prose_paren_trimmed() {
        let (_, _, url) = one("(see https://example.com/foo)");
        assert_eq!(url, "https://example.com/foo");
        let (_, _, url) = one("link (https://example.com/foo).");
        assert_eq!(url, "https://example.com/foo");
    }

    #[test]
    fn prose_brackets_trimmed() {
        let (_, _, url) = one("[https://example.com/x]");
        assert_eq!(url, "https://example.com/x");
    }

    #[test]
    fn quotes_and_angles_excluded() {
        let (_, _, url) = one("\"https://example.com/a\"");
        assert_eq!(url, "https://example.com/a");
        let (_, _, url) = one("<https://example.com/a>");
        assert_eq!(url, "https://example.com/a");
    }

    #[test]
    fn unicode_prefix_keeps_char_boundary() {
        let input = "café https://example.com/x";
        let (s, e, url) = one(input);
        assert_eq!(s, "café ".len());
        assert_eq!(url, "https://example.com/x");
        assert!(input.is_char_boundary(s));
        assert!(input.is_char_boundary(e));
        assert_eq!(input.get(s..e), Some(url.as_str()));
    }

    #[test]
    fn emoji_and_cjk_boundaries() {
        let input = "🎉 go https://example.com/a 🎉";
        let (s, e, url) = one(input);
        assert_eq!(url, "https://example.com/a");
        assert!(input.is_char_boundary(s));
        assert!(input.is_char_boundary(e));
        let cjk = "日本語 https://example.co.jp/ 日本語";
        let (s2, e2, url2) = one(cjk);
        assert!(cjk.is_char_boundary(s2));
        assert!(cjk.is_char_boundary(e2));
        assert_eq!(cjk.get(s2..e2), Some(url2.as_str()));
        assert_eq!(url2, "https://example.co.jp/");
    }

    #[test]
    fn scheme_case_insensitive() {
        let (_, _, a) = one("Visit HTTP://EXAMPLE.COM/A");
        assert_eq!(a, "HTTP://EXAMPLE.COM/A");
        let (_, _, b) = one("Write Mailto:Foo@Bar.com");
        assert_eq!(b, "Mailto:Foo@Bar.com");
        let (_, _, c) = one("clone GIT://git.example.com/r");
        assert_eq!(c, "GIT://git.example.com/r");
    }

    #[test]
    fn empty_body_finds_nothing() {
        assert!(detect_urls("http://").is_empty());
        assert!(detect_urls("visit http:// now").is_empty());
        assert!(detect_urls("mailto:").is_empty());
        assert!(detect_urls("https://").is_empty());
    }

    #[test]
    fn port_userinfo_percent_encoding() {
        let (_, _, url) = one("x https://user@host:8080/p%20q?x=1#f y");
        assert_eq!(url, "https://user@host:8080/p%20q?x=1#f");
    }

    #[test]
    fn multiline_blob_finds_each_line() {
        let input = "l1 https://a.example\nl2 https://b.example/x";
        let found = detect_urls(input);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].2, "https://a.example");
        assert_eq!(found[1].2, "https://b.example/x");
    }

    #[test]
    fn all_offsets_are_char_boundaries() {
        let input =
            "é 🎉 https://a.example/1, then mailto:bé@example.com. end https://c.example/(x)";
        let found = detect_urls(input);
        assert!(!found.is_empty());
        for (s, e, url) in &found {
            assert!(input.is_char_boundary(*s), "start {s}");
            assert!(input.is_char_boundary(*e), "end {e}");
            assert_eq!(input.get(*s..*e), Some(url.as_str()));
        }
    }

    #[test]
    fn repeated_prefixes_terminate() {
        let input = "http://".repeat(500);
        let found = detect_urls(&input);
        for (s, e, url) in &found {
            assert!(input.is_char_boundary(*s));
            assert!(input.is_char_boundary(*e));
            assert_eq!(input.get(*s..*e), Some(url.as_str()));
        }
    }

    #[test]
    fn overlong_body_truncates_at_cap() {
        let input = format!("http://{}", "a".repeat(MAX_URL_LEN + 500));
        let found = detect_urls(&input);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].2.len(), "http://".len() + MAX_URL_LEN);
        assert_eq!(found[0].1 - found[0].0, found[0].2.len());
    }

    #[test]
    fn readme_example_offsets() {
        let input = "see https://example.com/a?b=c#d.";
        let found = detect_urls(input);
        assert_eq!(
            found,
            vec![(4, 31, "https://example.com/a?b=c#d".to_string())]
        );
    }

    #[test]
    fn truncation_cap_boundary() {
        let exact = format!("http://{}", "a".repeat(MAX_URL_LEN));
        let found = detect_urls(&exact);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].2.len(), "http://".len() + MAX_URL_LEN);
        let over = format!("http://{}", "a".repeat(MAX_URL_LEN + 1));
        let found = detect_urls(&over);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].2.len(), "http://".len() + MAX_URL_LEN);
        assert!(found[0].1 < over.len());
        assert_eq!(over.get(found[0].0..found[0].1), Some(found[0].2.as_str()));
        assert_eq!(found[0].1 - found[0].0, found[0].2.len());
    }

    #[test]
    fn url_adjacent_to_unicode_without_space() {
        let input = "éhttp://example.com/x";
        let (s, e, url) = one(input);
        assert_eq!(url, "http://example.com/x");
        assert_eq!(s, "é".len());
        assert!(input.is_char_boundary(s));
        assert!(input.is_char_boundary(e));
        assert_eq!(input.get(s..e), Some(url.as_str()));
    }

    #[test]
    fn scheme_followed_only_by_trimmable_punctuation_finds_nothing() {
        assert!(detect_urls("https://.").is_empty());
        assert!(detect_urls("see http://, now").is_empty());
        assert!(detect_urls("git://").is_empty());
    }

    #[test]
    fn large_line_scans_without_hang() {
        let mut input = String::new();
        for n in 0..2000 {
            input.push_str("log line with https://example.com/item-");
            input.push_str(&n.to_string());
            input.push_str(" done; ");
        }
        let found = detect_urls(&input);
        assert_eq!(found.len(), 2000);
    }

    #[test]
    fn scheme_prefix_overlap_is_single_maximal_match() {
        // The body alphabet contains scheme characters, so a second scheme
        // prefix inside the body extends the same match (maximal munch).
        let input = "http://http://example.com";
        let found = detect_urls(input);
        assert_eq!(found, vec![(0, input.len(), input.to_string())]);
        let nested = "mailto:mailto:ops@example.com";
        let (s, e, url) = one(nested);
        assert_eq!((s, e), (0, nested.len()));
        assert_eq!(url, nested);
    }

    #[test]
    fn brace_and_quote_bytes_terminate_scan() {
        // `{`, `}`, and `"` sit outside the URL alphabet, so they end the
        // body scan before trimming; the `}`/`"` trim arms stay defensive.
        let (_, _, url) = one("see http://example.com/a}b");
        assert_eq!(url, "http://example.com/a");
        let (_, _, url) = one("{http://example.com/a}");
        assert_eq!(url, "http://example.com/a");
        let (_, _, url) = one("see http://example.com/a{b");
        assert_eq!(url, "http://example.com/a");
        let (_, _, url) = one("see http://example.com/a\"b");
        assert_eq!(url, "http://example.com/a");
    }

    #[test]
    fn mailto_comma_or_punctuation_only_body_finds_nothing() {
        assert!(detect_urls("mailto:,").is_empty());
        assert!(detect_urls("mailto:;").is_empty());
        assert!(detect_urls("mailto:.").is_empty());
        assert!(detect_urls("contact mailto:, today").is_empty());
    }

    #[test]
    fn whitespace_only_and_scheme_only_inputs_find_nothing() {
        for input in [
            "",
            "   ",
            "\t\n  \n",
            "http://",
            "https://",
            "git://",
            "mailto:",
            "visit http://",
            "http:// ",
            "git:// ",
            "mailto: ",
        ] {
            assert!(detect_urls(input).is_empty(), "input {input:?}");
        }
    }

    #[test]
    fn punctuation_only_body_finds_nothing() {
        for input in [
            "http://...",
            "http://!!!",
            "https://???",
            "https://''",
            "git://...",
        ] {
            assert!(detect_urls(input).is_empty(), "input {input:?}");
        }
    }

    #[test]
    fn non_ascii_byte_terminates_body_scan() {
        let input = "see http://abécd more";
        let (s, e, url) = one(input);
        assert_eq!(url, "http://ab");
        assert_eq!(input.get(s..e), Some(url.as_str()));
        assert!(detect_urls("http://éx").is_empty());
        assert!(detect_urls("mailto:é@example.com").is_empty());
    }

    #[test]
    fn truncation_cap_before_multibyte_stays_on_boundary() {
        // The cap lands on ASCII (the body scan only consumes ASCII bytes),
        // so `end` is a char boundary even with a multibyte char at the cap.
        let input = format!("http://{}é https://example.com/y", "a".repeat(MAX_URL_LEN));
        let found = detect_urls(&input);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].2.len(), "http://".len() + MAX_URL_LEN);
        assert_eq!(found[0].1 - found[0].0, found[0].2.len());
        assert_eq!(found[1].2, "https://example.com/y");
        for (s, e, url) in &found {
            assert!(input.is_char_boundary(*s), "start {s}");
            assert!(input.is_char_boundary(*e), "end {e}");
            assert_eq!(input.get(*s..*e), Some(url.as_str()));
        }
    }

    #[test]
    fn oversized_input_without_scheme_terminates_empty() {
        let input = "a".repeat(200_000);
        assert!(detect_urls(&input).is_empty());
        let digits = "0123456789".repeat(20_000);
        assert!(detect_urls(&digits).is_empty());
    }

    #[test]
    fn empty_match_between_schemes_is_skipped() {
        let input = "http:// http://example.com/x";
        let (s, e, url) = one(input);
        assert_eq!(url, "http://example.com/x");
        assert_eq!(s, "http:// ".len());
        assert_eq!(input.get(s..e), Some(url.as_str()));
    }

    #[test]
    fn large_mixed_input_scales_linearly() {
        // Linear-behavior floor without wall-clock assertions: a large mixed
        // blob (unicode, parens, trailing punctuation) completes and yields
        // the exact expected count with valid offsets.
        let mut input = String::new();
        for n in 0..2000 {
            input.push_str("café log https://example.com/item-");
            input.push_str(&n.to_string());
            input.push_str(" (see https://other.example/x). ");
        }
        let found = detect_urls(&input);
        assert_eq!(found.len(), 4000);
        for (s, e, url) in &found {
            assert!(input.is_char_boundary(*s), "start {s}");
            assert!(input.is_char_boundary(*e), "end {e}");
            assert_eq!(input.get(*s..*e), Some(url.as_str()));
        }
    }
}
