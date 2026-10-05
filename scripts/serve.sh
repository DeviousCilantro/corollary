#!/usr/bin/env bash
# Build for local preview and serve it at http://localhost:8000.
#
#   ./scripts/serve.sh            # port 8000
#   PORT=4000 ./scripts/serve.sh  # another port
#
# The preview is built for localhost whatever base_url says, so it works even
# when the real site lives under a subpath. Rerun after editing; it is quick.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PORT="${PORT:-8000}"

"$ROOT/scripts/build.sh" --base-url "http://localhost:$PORT"
printf 'Serving %s at http://localhost:%s (Ctrl-C to stop)\n' "$ROOT/public" "$PORT"
exec python3 -m http.server --bind 127.0.0.1 --directory "$ROOT/public" "$PORT"
