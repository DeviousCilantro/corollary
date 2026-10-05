#!/usr/bin/env bash
# Build the site into public/.
#
#   ./scripts/build.sh                                   # base URL from site.toml / $SITE_BASE_URL
#   ./scripts/build.sh --base-url https://example.org    # override it
#
# Without wasm-pack the page is still complete; it only loses the theme toggle
# and the copy buttons.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo run --quiet --release -p sitegen -- "$@"

if command -v wasm-pack >/dev/null 2>&1; then
  (cd wasm && wasm-pack --quiet build --release --target web --no-typescript --no-pack \
    --out-dir ../public/assets/wasm)
else
  printf 'wasm-pack not found; built static HTML without the theme toggle and copy buttons.\n' >&2
fi
