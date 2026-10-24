#!/bin/sh
# Local/CI distribution preparation. Does not publish or launch QQ.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)"
if [ "$(uname -s)" != Darwin ] || [ "$(uname -m)" != arm64 ]; then
    echo "macOS packaging requires native Apple Silicon." >&2
    exit 1
fi
output_directory=${1:-artifacts/macos}
cargo build --release --offline --locked --target aarch64-apple-darwin -j1
package_version=$(cargo metadata --offline --locked --no-deps --format-version 1 |
    python3 -c 'import json,sys; p=[p for p in json.load(sys.stdin)["packages"] if p["name"]=="qq-cli"]; assert len(p)==1; print(p[0]["version"])')
executable=$(cargo metadata --offline --locked --no-deps --format-version 1 |
    python3 -c 'import json,sys; from pathlib import Path; print(Path(json.load(sys.stdin)["target_directory"])/"aarch64-apple-darwin/release/qq-cli")')
if [ "$("$executable" --version)" != "qq-cli $package_version" ]; then
    echo "Executable version differs from Cargo metadata." >&2
    exit 1
fi
python3 scripts/macos_release.py --executable "$executable" --version "$package_version" --output "$output_directory"
