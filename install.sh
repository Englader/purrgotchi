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
#
# bash install.sh --uninstall-hooks removes the feed/turn hooks and keeps statusLine.

set -euo pipefail

REPO_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
SETTINGS="$HOME/.claude/settings.json"
STATE_DIR="$HOME/.claude/purrgotchi"
# Cargo only appends .exe on Windows targets (Git Bash/MSYS2/Cygwin); WSL,
# Linux and macOS produce a bare `purrgotchi`.
case "${OSTYPE:-}" in
    msys*|cygwin*|win32*) BIN_EXT=".exe" ;;
    *)                    BIN_EXT="" ;;
esac
RUST_BIN="$REPO_DIR/rust/target/release/purrgotchi${BIN_EXT}"

usage() {
    echo "usage: bash install.sh                    # wire statusLine + feed/turn hooks"
    echo "       bash install.sh --uninstall-hooks  # remove purrgotchi feed/turn hooks, keep statusLine"
}

MODE="install"
case "${1:-}" in
    "")                MODE="install" ;;
    --uninstall-hooks) MODE="uninstall-hooks" ;;
    -h|--help)         usage; exit 0 ;;
    *)                 echo "unknown option: $1"; usage; exit 2 ;;
esac

# --- Uninstall hooks ---------------------------------------------------------
# Removes the feed/turn hooks so nothing feeds the kitten. statusLine is kept
# (it never counts anything). Needs only jq — never the compiled binary.
if [ "$MODE" = "uninstall-hooks" ]; then
    echo "Purrgotchi: removing feed/turn hooks from $SETTINGS (statusLine is kept)"
    if ! command -v jq >/dev/null 2>&1; then
        echo "  MISSING: jq (https://jqlang.github.io/jq/download/)"
        exit 1
    fi
    if [ ! -f "$SETTINGS" ]; then
        echo "  No settings.json — nothing to do."
        exit 0
    fi
    if ! jq empty "$SETTINGS" 2>/dev/null; then
        echo "  ERROR: $SETTINGS is not valid JSON. Fix it (or restore a .bak) and re-run."
        exit 1
    fi
    # Match ONLY what install.sh writes: PostToolUse entries with matcher "*"
    # running `"<bin>" feed`, and UserPromptSubmit running `"<bin>" turn`
    # (<bin> = .../purrgotchi[.exe], optionally quoted; or the `purr` wrapper).
    # A user's own hook that merely mentions "purrgotchi" is never touched.
    BIN_RE='^\s*("([^"]*[/\\])?purr(gotchi)?(\.(exe|cmd|sh|bat))?"|([^\s"]*[/\\])?purr(gotchi)?(\.(exe|cmd|sh|bat))?)\s+'
    HOOK_DEFS='
      def is_purr($verb): ((.command // "") | test($re + $verb + "(\\s|$)"));
      def entry_ok($needstar): ((.matcher == "*") or ($needstar | not));
    '
    COUNT_FILTER="$HOOK_DEFS"'
      [ (.hooks | objects | .PostToolUse | arrays | .[] | objects | select(entry_ok(true))
          | .hooks | arrays | .[] | objects | select(is_purr("feed"))),
        (.hooks | objects | .UserPromptSubmit | arrays | .[] | objects | select(entry_ok(false))
          | .hooks | arrays | .[] | objects | select(is_purr("turn"))) ] | length'
    # Only entries/arrays/objects this run emptied are pruned; pre-existing
    # empty ones are left alone.
    UNINSTALL_FILTER="$HOOK_DEFS"'
      def strip($verb; $needstar):
        [ .[] | if (type == "object") and ((.hooks | type) == "array") and entry_ok($needstar) then
                  (.hooks) as $h
                  | ($h | map(select(((type == "object") and is_purr($verb)) | not))) as $kept
                  | if ($kept | length) == 0 and ($h | length) > 0 then empty else .hooks = $kept end
                else . end ];
      def fix($k; $verb; $needstar):
        if (.[$k] | type) == "array" then
          (.[$k]) as $old | (.[$k] | strip($verb; $needstar)) as $new
          | if ($new | length) == 0 and ($old | length) > 0 then del(.[$k]) else .[$k] = $new end
        else . end;
      if (.hooks | type) != "object" then . else
        .hooks |= (fix("PostToolUse"; "feed"; true) | fix("UserPromptSubmit"; "turn"; false))
        | if .hooks == {} then del(.hooks) else . end
      end'
    FOUND="$(jq --arg re "$BIN_RE" "$COUNT_FILTER" "$SETTINGS" 2>/dev/null || echo ERR)"
    if [ "$FOUND" = "ERR" ]; then
        echo "  ERROR: could not scan settings.json with jq. Settings unchanged."
        exit 1
    fi
    if [ "$FOUND" = "0" ]; then
        echo "  No purrgotchi hooks found — settings.json unchanged."
        exit 0
    fi
    if ! jq --arg re "$BIN_RE" "$UNINSTALL_FILTER" "$SETTINGS" > "${SETTINGS}.new" 2>/dev/null \
       || ! jq empty "${SETTINGS}.new" 2>/dev/null; then
        rm -f "${SETTINGS}.new"
        echo "  ERROR: could not rewrite settings.json with jq. Settings unchanged."
        exit 1
    fi
    BACKUP="${SETTINGS}.bak.$(date +%Y%m%d-%H%M%S)"
    cp "$SETTINGS" "$BACKUP"
    echo "  Backup: $BACKUP"
    mv "${SETTINGS}.new" "$SETTINGS"
    echo "  Removed $FOUND purrgotchi feed/turn hook(s). statusLine left in place."
    echo "  To rollback: cp \"$BACKUP\" \"$SETTINGS\""
    exit 0
fi

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
    missing+=("purrgotchi${BIN_EXT} (build with: cd rust && cargo build --release)")
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
# Under Git Bash $RUST_BIN is MSYS-style (/c/Users/...), which Claude Code
# (running outside MSYS) can't resolve. Write the mixed form (C:/Users/...).
CMD_BIN="$RUST_BIN"
if [ -n "$BIN_EXT" ] && command -v cygpath >/dev/null 2>&1; then
    CMD_BIN="$(cygpath -m "$RUST_BIN")"
fi
STATUSLINE_CMD="\"$CMD_BIN\" statusline"
FEED_CMD="\"$CMD_BIN\" feed"
TURN_CMD="\"$CMD_BIN\" turn"

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
