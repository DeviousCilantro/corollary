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

# The WebAssembly module is built first, into wasm/pkg/, so that the generator
# can copy it into public/ and fingerprint its URLs as it does the
# stylesheets'. Cleared first, so a build without wasm-pack never ships a
# module left over from an earlier one.
rm -rf wasm/pkg
if command -v wasm-pack >/dev/null 2>&1; then
  (cd wasm && wasm-pack --quiet build --release --target web --no-typescript --no-pack \
    --out-dir pkg)
else
  printf 'wasm-pack not found; building static HTML without the theme toggle and copy buttons.\n' >&2
fi

cargo run --quiet --release -p sitegen -- "$@"
