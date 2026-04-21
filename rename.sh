#!/usr/bin/env bash
# Rename the Purrgotchi kitten.
# Usage: bash <path-to-this>/rename.sh "New Name"
# State file lives in the Claude config dir (same across every install).
set -euo pipefail

PURR_STATE_DIR="$HOME/.claude/purrgotchi"
STATE="$PURR_STATE_DIR/state.json"

if [ $# -ne 1 ] || [ -z "$1" ]; then
    echo "usage: $0 <new-name>" >&2
    exit 1
fi

NEW_NAME="$1"
NOW=$(date +%s)

mkdir -p "$PURR_STATE_DIR"

# If state.json exists, update the name field preserving everything else.
# If it doesn't exist, create a fresh state with the given name.
if [ -f "$STATE" ]; then
    TMP=$(mktemp "$PURR_STATE_DIR/.state.XXXXXX.tmp")
    jq --arg n "$NEW_NAME" '.name = $n' "$STATE" > "$TMP"
    mv -f "$TMP" "$STATE"
else
    cat > "$STATE" <<JSON
{
  "version": 1,
  "name": "$NEW_NAME",
  "birth_time": $NOW,
  "last_fed_at": $NOW,
  "total_tool_calls": 0,
  "coins": 0,
  "level": 1,
  "lifetime_cost_usd": 0.0
}
JSON
fi

echo "kitten renamed to: $NEW_NAME"
