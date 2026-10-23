#!/bin/sh
# Native Apple Silicon Rust checks; Windows packaging/PowerShell gates remain separate.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)"
if [ "$(uname -s)" != Darwin ] || [ "$(uname -m)" != arm64 ]; then
    echo "This check requires native Apple Silicon macOS." >&2
    exit 1
fi
# macOS /tmp and /var are aliases; private-state tests deliberately reject symlink ancestors.
TMPDIR=$(CDPATH= cd -- "${TMPDIR:-/tmp}" && pwd -P)
export TMPDIR
node --test tests/macos_adapter.cjs tests/macos_entry.cjs
cargo fmt --all -- --check
cargo clippy --all-targets --offline --locked -j1 -- -D warnings
cargo test --offline --locked -j1 -- --test-threads=1
cargo build --release --offline --locked --target aarch64-apple-darwin -j1
