# crates.io publish runbook: bitty-url-detector

Priority: P2 | Area: area:packaging | Labels: feat,P2,area:packaging
| Milestone: v0.1.0 | Issue: bitty-url-detector#14
| Upstream: bitty-terminal/bitty#1512

Scope is this repository only. This note records the first publish of
`bitty-url-detector` at version `0.0.1`, so the owner can execute it after
the registry token rotation lands (see the Blockers section).

## Readiness (verified)

- The crate name returns "does not exist" on the crates.io API: free.
- `cargo publish --dry-run -p bitty-url-detector` is green. The only
  diagnostic is a harmless warning: the dev-only `scan` benchmark lives
  outside the packaged file set and is excluded from the package, which is
  intended (benches never ship to the registry).
- The crate is dependency-free (std only), so Core pulls no new transitive
  tree and no manifest change is needed.

## Prerequisites

- The registry token rotation recorded upstream is complete. Never publish
  with a token that may have been exposed; each publish is permanent
  (yank is the only undo).
- The working tree is clean and on `main`; the crate is unchanged since the
  dry-run verification above.

## Publish command

Run from the repository root:

```sh
cargo publish -p bitty-url-detector
```

## Verification

- `cargo search bitty-url-detector --limit 1` shows the version once the
  index picks it up.
- The crate page on crates.io shows "Builds on docs.rs" once the docs build
  finishes.
