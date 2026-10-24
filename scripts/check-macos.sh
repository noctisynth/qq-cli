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
python3 -m unittest discover -s scripts/tests -v
node --test tests/macos_adapter.cjs tests/macos_entry.cjs
cargo fmt --all -- --check
cargo clippy --all-targets --offline --locked -j1 -- -D warnings
cargo test --offline --locked -j1 -- --test-threads=1
# Register the real private worker dispatchers in an independent test host.
# Only its original synthetic app is copied/signed; QQ is never invoked here.
rustfmt --edition 2024 --check tests/probes/macos_assembly.rs
cargo build --lib --offline --locked -j1
assembly_probe_dir=$(mktemp -d "$TMPDIR/qq-cli-assembly.XXXXXX")
trap 'rm -rf "$assembly_probe_dir"' EXIT
rustc --edition=2024 tests/probes/macos_assembly.rs --extern qq_cli=target/debug/libqq_cli.rlib -L dependency=target/debug/deps -o "$assembly_probe_dir/probe"
"$assembly_probe_dir/probe" synthetic "$assembly_probe_dir/state"
cargo build --release --offline --locked --target aarch64-apple-darwin -j1
