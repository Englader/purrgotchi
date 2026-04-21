#!/usr/bin/env bash
# Purrgotchi installer — configures Claude Code to use this statusline + feed hook.
#
# What this does:
#   1. Checks prerequisites (bash, jq, python)
#   2. Backs up ~/.claude/settings.json (with timestamped suffix)
#   3. Creates ~/.claude/purrgotchi/ state directory
#   4. Merges statusLine + refreshInterval + PostToolUse hook into settings.json
#   5. Prints verification instructions
#
# Safe to run multiple times — it's idempotent.

set -euo pipefail

# Resolve the repo path — this script lives at the repo root
REPO_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
SETTINGS="$HOME/.claude/settings.json"
STATE_DIR="$HOME/.claude/purrgotchi"

echo "=============================================="
echo "  Purrgotchi installer"
echo "  Source: $REPO_DIR"
echo "=============================================="
echo ""

# --- 1. Prerequisites -------------------------------------------------------
echo "[1/5] Checking prerequisites..."

missing=()
command -v jq   >/dev/null 2>&1 || missing+=("jq")

# Python: prefer py launcher on Windows, python3 elsewhere
PYTHON_CMD=""
if [ -x "/c/Windows/py.exe" ]; then
    PYTHON_CMD="/c/Windows/py.exe"
elif command -v python3 >/dev/null 2>&1; then
    PYTHON_CMD="python3"
elif command -v python  >/dev/null 2>&1; then
    # Verify it's real Python, not the Microsoft Store stub on Windows
    if python --version >/dev/null 2>&1; then
        PYTHON_CMD="python"
    fi
fi

if [ -z "$PYTHON_CMD" ]; then
    missing+=("python")
fi

if [ ${#missing[@]} -gt 0 ]; then
    echo "  MISSING: ${missing[*]}"
    echo ""
    echo "  Install instructions:"
    echo "    jq:     https://jqlang.github.io/jq/download/"
    echo "    python: https://www.python.org/downloads/ (check 'Add to PATH')"
    echo ""
    echo "  On Windows, also disable the MS Store python shim:"
    echo "    Settings -> Apps -> Advanced app settings -> App execution aliases"
    echo "    Toggle OFF python.exe and python3.exe"
    exit 1
fi
echo "  OK: jq found"
echo "  OK: $PYTHON_CMD found"
echo ""

# --- 2. State directory -----------------------------------------------------
echo "[2/5] Creating state directory..."
mkdir -p "$STATE_DIR"
echo "  $STATE_DIR"
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

STATUSLINE_CMD="bash $REPO_DIR/statusline.sh"
FEED_CMD="$PYTHON_CMD $REPO_DIR/feed_hook.py"
TURN_CMD="bash $REPO_DIR/turn_hook.sh"

# Use jq to merge in our keys while preserving any existing config.
# Replace our managed hook entries (matcher "*" pointing at feed_hook or
# turn_hook commands) while leaving unrelated hooks untouched.
jq --arg sl "$STATUSLINE_CMD" --arg feed "$FEED_CMD" --arg turn "$TURN_CMD" '
    .statusLine = {"type":"command", "command":$sl, "padding":1}
    | .refreshInterval = 1
    | .hooks //= {}
    | .hooks.PostToolUse = (
        (.hooks.PostToolUse // []) | map(select(.matcher != "*" or ((.hooks[0].command // "") | test("feed_hook") | not)))
        + [{"matcher":"*","hooks":[{"type":"command","command":$feed}]}]
      )
    | .hooks.UserPromptSubmit = (
        (.hooks.UserPromptSubmit // []) | map(select(((.hooks[0].command // "") | test("turn_hook") | not)))
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
echo "  echo '{}' | bash $REPO_DIR/statusline.sh"
echo ""
echo "Then restart Claude Code — you should see the HUD render on your next turn."
echo ""
echo "To rename the kitten:"
echo "  bash $REPO_DIR/rename.sh \"YourCatName\""
echo ""
echo "To rollback:"
echo "  cp <latest .bak file> $SETTINGS"
echo "  (then restart Claude Code)"
