#!/usr/bin/env bash
# Lay the engine's generated mod types into mod/.claude-plugin/types/.
#
# The engine writes them only when an INTERACTIVE session loads the mod from a
# hot-reloaded folder (--plugin-dir). So this idles a session in a pty for a few
# seconds and kills it; no model call is made. The idle session runs the mod's
# session.start, so it runs inside the dev sandbox (a sandboxed `purrgotchi`
# first on PATH) and never touches ~/.claude/purrgotchi. User settings are not
# loaded (--setting-sources project,local).
#
# Re-run on every Claude Code version bump. Never commit mod/.claude-plugin/types/.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo"

mkdir -p /tmp/claude-1000
tmp="$(mktemp -d /tmp/claude-1000/purr-lay.XXXXXX)"
eval "$(bash mod/scripts/dev-sandbox.sh "$tmp/sandbox")"

dts="mod/.claude-plugin/types/claude-code/index.d.ts"
log="$tmp/purr-debug.log"

timeout -k 3 25 script -qfc \
  "claude --plugin-dir ./mod --setting-sources project,local --debug-file '$log'" \
  "$tmp/purr-tty.log" </dev/null >/dev/null || true

# The engine logs "type root of <plugin> ... wrote <files>" (or "wrote nothing"
# when the laid types are already current); either means this engine laid them.
if [ ! -f "$dts" ] || ! grep -q "type root of purrgotchi" "$log" 2>/dev/null; then
  echo "lay-types: the engine did not lay $dts in this run." >&2
  echo "lay-types: debug log: $log   tty log: $tmp/purr-tty.log" >&2
  echo "lay-types: (a trust or login prompt in the pty can block the load; check the tty log)" >&2
  exit 1
fi

if [ ! -f mod/tsconfig.json ]; then
  printf '{ "extends": "./.claude-plugin/types/tsconfig.json" }\n' > mod/tsconfig.json
fi

grep -m1 -o "type root of purrgotchi.*" "$log" >&2
echo "lay-types: $dts $(wc -c < "$dts") bytes"
echo "lay-types: $(claude --version)"
