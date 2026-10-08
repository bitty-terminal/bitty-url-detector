# Detector fuzzing (dev tooling only)

`cargo-fuzz` harness for `bitty-url-detector`. This crate is not a workspace
member and ships no runtime code: it feeds arbitrary `&str` input into
`detect_urls` and asserts the documented contract on every match.

## Layout

- `fuzz_targets/detect.rs`: single target asserting `s <= e`, char
  boundaries, `input.get(s..e) == url`, and ordered non-overlapping matches.
- `corpus/detect/`: seed inputs (empty, plain text, simple URL, unicode with
  URL, over-cap truncation case).

## Run

All commands run from the repository root with the nightly toolchain
installed:

```text
cargo +nightly fuzz run detect -- -max_total_time=120
```

Build the target without running it:

```text
cargo +nightly fuzz build
```

## Latest run

- Target: `detect`
- Duration: 121 s (`-max_total_time=120`)
- Execs: 13,696,193
- Corpus: 5 seeds kept (535 fuzzer-generated inputs observed mid-run, pruned)
- Crashes: none found (`artifacts/` empty, exit 0)
