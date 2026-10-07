# URL detector repository guidance

Pure-algorithm plaintext URL detector for Bitty (bitty#1760, scanner side).
This repository defines the detector API that Bitty Core will consume; Core
wiring (grid mapping, hover, click-to-open) is a separate task. Never touch
the `bitty` checkout from this repository.

English only for all content (code comments, docs, commits, Issues, PRs).
Rust version 0.0.1, edition 2024, MSRV 1.85 (no let-chains; desugar to nested
`if let`). Channel pinned in `rust-toolchain.toml`; never bump pins in
unrelated tasks. Quality gates only via the justfile: `just check` (metadata
plus `rust-fmt`, `rust-clippy -D warnings`, `rust-test`, `supply-chain`).
Never invoke formatters/linters bare.

`#![forbid(unsafe_code)]` in every crate. No `unwrap`/`expect`/`panic` in
non-test code; the scanner returns empty matches on degenerate input instead
of panicking. Zero runtime dependencies (`std` only) so Core pulls no new
transitive tree; dev tooling only.

Never hardcode host/environment values (paths, usernames, URLs, ports).
Named constants for policy, bound, timeout, or default values; literals only
when intrinsic and obvious. Scanner bounds (scheme set, URL alphabet, maximum
match length) are named constants with documented rationale.

CTX-0001 -> CTX-0002 -> CTX-0003 -> CTX-0004 orders
bootstrap/detector-implementation/Core-contract-acceptance/independent
verification. Named sessions, narrow scopes, managed hooks, and task worktrees
after the first commit. Direct bootstrap authorized; no self-acceptance. No
commit/push/release without authority; redacted snapshots only. Preserve
unrelated work, no silent installs/destructive cleanup/unowned process kills.
