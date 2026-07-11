#!/usr/bin/env bash
# Purrgotchi installer — wires Claude Code to the purrgotchi.exe binary.
#
# What this does:
#   1. Checks prerequisites (jq, compiled Rust binary)
#   2. Backs up ~/.claude/settings.json (with timestamped suffix)
#   3. Creates ~/.claude/purrgotchi/ state directory
#   4. Merges statusLine + refreshInterval + PostToolUse + UserPromptSubmit
#      hooks into settings.json, pointing each at purrgotchi.exe
#   5. Prints verification instructions
#
# Safe to run multiple times — it's idempotent.

set -euo pipefail

REPO_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
SETTINGS="$HOME/.claude/settings.json"
STATE_DIR="$HOME/.claude/purrgotchi"
RUST_BIN="$REPO_DIR/rust/target/release/purrgotchi.exe"

echo "=============================================="
echo "  Purrgotchi installer"
echo "  Source: $REPO_DIR"
echo "=============================================="
echo ""

# --- 1. Prerequisites -------------------------------------------------------
echo "[1/5] Checking prerequisites..."

missing=()
command -v jq >/dev/null 2>&1 || missing+=("jq")

if [ ! -x "$RUST_BIN" ]; then
    missing+=("purrgotchi.exe (build with: cd rust && cargo build --release)")
fi

if [ ${#missing[@]} -gt 0 ]; then
    echo "  MISSING:"
    for m in "${missing[@]}"; do echo "    - $m"; done
    echo ""
    echo "  Install instructions:"
    echo "    jq:    https://jqlang.github.io/jq/download/"
    echo "    Rust:  https://rustup.rs  (then: cd rust && cargo build --release)"
    exit 1
fi
echo "  OK: jq found"
echo "  OK: Rust binary found ($RUST_BIN)"
echo ""

# --- 2. State directory -----------------------------------------------------
echo "[2/5] Creating state directory..."
mkdir -p "$STATE_DIR"
echo "  $STATE_DIR"
# The binary reads LLM keys only from $STATE_DIR/.env (never from a project
# cwd). Seed it from the repo's .env on first install.
if [ -f "$REPO_DIR/.env" ] && [ ! -f "$STATE_DIR/.env" ]; then
    cp "$REPO_DIR/.env" "$STATE_DIR/.env"
    echo "  Copied .env -> $STATE_DIR/.env (binary reads keys only from there)"
fi
echo ""

# --- 3. Backup settings.json ------------------------------------------------
echo "[3/5] Backing up settings.json..."
mkdir -p "$HOME/.claude"
if [ ! -f "$SETTINGS" ]; then
    echo '{}' > "$SETTINGS"
    echo "  Created fresh settings.json (didn't exist)"
else
    BACKUP="${SETTINGS}.bak.$(date +%Y%m%d-%H%M%S)"
    cp "$SETTINGS" "$BACKUP"
    echo "  Backup: $BACKUP"
fi
echo ""

# --- 4. Merge settings.json -------------------------------------------------
echo "[4/5] Updating settings.json with Purrgotchi config..."

# Refuse to merge into a broken file — with `set -e` jq would otherwise die
# mid-pipeline with a raw parse error and leave a stray ${SETTINGS}.new behind.
if ! jq empty "$SETTINGS" 2>/dev/null; then
    echo "  ERROR: $SETTINGS is not valid JSON. Fix it (or restore a .bak) and re-run."
    exit 1
fi

# Command strings quote the binary path — an unquoted path with spaces
# (C:/Users/John Doe/...) word-splits when Claude Code runs it via the shell.
STATUSLINE_CMD="\"$RUST_BIN\" statusline"
FEED_CMD="\"$RUST_BIN\" feed"
TURN_CMD="\"$RUST_BIN\" turn"

# Merge our keys while preserving any existing config. Replace our managed
# hook entries (any command containing "purrgotchi" — matches both legacy
# bash/python entries and current Rust binary paths) while leaving unrelated
# hooks untouched.
#
# refreshInterval lives INSIDE the statusLine object (a top-level key is
# silently ignored by Claude Code — that was the pre-v3.7.0 bug that kept
# sprites from animating). 1s refresh + wall-clock frame = live animation.
jq --arg sl "$STATUSLINE_CMD" --arg feed "$FEED_CMD" --arg turn "$TURN_CMD" '
    .statusLine = {"type":"command", "command":$sl, "padding":1, "refreshInterval":1}
    | del(.refreshInterval)
    | .hooks //= {}
    | .hooks.PostToolUse = (
        (.hooks.PostToolUse // []) | map(select(.matcher != "*" or ((.hooks[0].command // "") | test("purrgotchi") | not)))
        + [{"matcher":"*","hooks":[{"type":"command","command":$feed}]}]
      )
    | .hooks.UserPromptSubmit = (
        (.hooks.UserPromptSubmit // []) | map(select(((.hooks[0].command // "") | test("purrgotchi") | not)))
        + [{"hooks":[{"type":"command","command":$turn}]}]
      )
' "$SETTINGS" > "${SETTINGS}.new"

if jq empty "${SETTINGS}.new" 2>/dev/null; then
    mv "${SETTINGS}.new" "$SETTINGS"
    echo "  Updated: $SETTINGS"
else
    rm -f "${SETTINGS}.new"
    echo "  ERROR: jq produced invalid JSON. Settings unchanged."
    exit 1
fi
echo ""

# --- 5. Summary -------------------------------------------------------------
echo "[5/5] Install complete!"
echo ""
echo "Verify by running:"
echo "  echo '{}' | \"$RUST_BIN\" statusline"
echo ""
echo "Then restart Claude Code — you should see the HUD render on your next turn."
echo ""
echo "To rename the kitten:"
echo "  \"$RUST_BIN\" rename \"YourCatName\""
echo ""
echo "To rollback:"
echo "  cp <latest .bak file> $SETTINGS"
echo "  (then restart Claude Code)"
