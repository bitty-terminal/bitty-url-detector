# bitty-url-detector

Pure-algorithm plaintext URL detector for Bitty (scanner side of [bitty#1760](https://github.com/bitty-terminal/bitty/issues/1760)). Read [AGENTS](AGENTS.md). Task management lives in CarryCtx.

## Status

**v0.0.1, initial detector implementation.** This repository defines the detector API that Bitty Core will consume. It is **not** wired into Core: grid mapping, hover cues, and click-to-open stay Core-owned work under bitty#1760 and are not claimed here.

## API

Crate `bitty-url-detector`, zero dependencies, `#![forbid(unsafe_code)]`:

```rust
use bitty_url_detector::detect_urls;

let found = detect_urls("see https://example.com/a?b=c#d.");
assert_eq!(found, vec![(4, 31, "https://example.com/a?b=c#d".to_string())]);
```

`pub fn detect_urls(input: &str) -> Vec<(col_start: usize, col_end: usize, url: String)>`

- `col_start` inclusive, `col_end` exclusive; byte offsets into `input` that are always UTF-8 char boundaries, so `input[col_start..col_end] == url` holds for every match.
- Schemes (ASCII case-insensitive): `http://`, `https://`, `git://`, `mailto:`.
- A match needs at least one URL-alphabet character after the scheme; empty schemes yield no match.
- Trailing prose punctuation (`.`, `,`, `;`, `:`, `!`, `?`, quotes) is trimmed; closing brackets (`)`, `]`, `}`) trim only when unbalanced, so balanced parens inside URLs survive.
- Bodies longer than `MAX_URL_LEN` (4096) bytes truncate to the cap; scanning stays linear with no regex and no backtracking.
- No terminal-grid, GPU, or window types anywhere in the API.

## Gates

```text
just check                                  # metadata + fmt + clippy + tests + supply chain
cargo test -p bitty-url-detector            # unit tests (ASCII, boundaries, unicode, caps)
cargo bench -p bitty-url-detector --bench scan   # throughput smoke (local numbers only)
```

Rust version 0.0.1, edition 2024, MSRV 1.85, MIT license.
