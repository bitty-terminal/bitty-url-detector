# Metadata and detector-crate gates. Metadata gates are not product evidence;
# Rust gates prove the detector crate (fmt, clippy, tests, docs). Run quality
# gates only via this justfile, never bare.
prettier_version := "3.9.6"
markdownlint_version := "0.23.1"
actionlint_version := "1.7.12"

fmt-check:
    bunx --bun prettier@{{prettier_version}} --check . --ignore-unknown

markdownlint:
    bunx --bun markdownlint-cli2@{{markdownlint_version}}

metadata:
    test -s README.md && test -s AGENTS.md && test -s repo.toml
    test ! -e TODO.md
    test -s .carryctx/config.toml
    python3 -c 'import tomllib; from pathlib import Path; [tomllib.loads(p.read_text()) for p in [Path("repo.toml"), Path(".carryctx/config.toml")]]'

hygiene:
    #!/usr/bin/env bash
    set -euo pipefail
    bad=0
    while IFS= read -r -d '' f; do
        case "$f" in
            *.sqlite|*.db|*.zst|*.tar|*.tar.gz|*.tgz|node_modules/*|target/*|dist/*|.worktrees/*)
                echo "unexpected artifact tracked: $f" >&2; bad=1 ;;
        esac
    done < <(git ls-files -z --cached --others --exclude-standard)
    test "$bad" -eq 0

paths:
    #!/usr/bin/env bash
    set -euo pipefail
    pattern='(/hom''e/|/Use''rs/|/mn''t/[A-Za-z]|[A-Za-z]:[\\/]Use''rs[\\/])'
    found=0
    while IFS= read -r -d '' f; do
        if grep -nEI "$pattern" "$f"; then found=1; fi
    done < <(git ls-files -z --cached --others --exclude-standard)
    if [ "$found" -ne 0 ]; then
        echo 'hardcoded host path detected (portable-path gate)' >&2
        exit 1
    fi
    echo 'portable-path gate passed'

actionlint:
    @installed="$(actionlint --version | head -n 1)"; test "$installed" = "{{actionlint_version}}" || { echo "actionlint {{actionlint_version}} required; found $installed" >&2; exit 1; }
    actionlint -color -shellcheck=

check:
    just fmt-check
    just markdownlint
    just metadata
    just hygiene
    just paths
    just rust-fmt
    just rust-clippy
    just rust-test
    just supply-chain

workflows:
    actionlint
    act -n

# Rust detector-crate gates. These are product evidence for the shipped crate,
# unlike the metadata gates above. Run via the justfile, never bare.
rust-fmt:
    cargo fmt --all -- --check

rust-clippy:
    cargo clippy --workspace --all-targets --locked -- -D warnings

rust-test:
    cargo test --workspace --locked --all-targets
    cargo test --workspace --doc --locked

rust-typecheck:
    cargo check --workspace --all-targets --locked

# Local supply-chain gate mirroring the CI `Supply chain` job:
# `cargo deny check` (advisories, bans, licenses, sources per deny.toml,
# auto-discovered at the repo root).
supply-chain:
    cargo deny check

# Publish a redacted CarryCtx snapshot to refs/heads/carryctx-snapshots.
workflow-publish:
    #!/usr/bin/env bash
    set -euo pipefail
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT
    carryctx export --pack-format dir -o "$tmp" --publication
    git push origin refs/heads/carryctx-snapshots

# Fetch and import the published CarryCtx snapshot (fresh-clone recovery).
workflow-import:
    git fetch origin refs/heads/carryctx-snapshots:refs/remotes/origin/carryctx-snapshots
    carryctx import --from-git refs/remotes/origin/carryctx-snapshots
