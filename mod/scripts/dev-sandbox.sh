#!/usr/bin/env bash
# Sandboxed `purrgotchi` for live dev sessions: a wrapper that pins
# PURR_STATE_DIR to <dir>/state, so a live run never touches ~/.claude/purrgotchi.
#
#   eval "$(bash mod/scripts/dev-sandbox.sh [dir])"
#
# Prints the `export PATH=...` line to eval (and PURR_SANDBOX_STATE for checks).
# Diagnostics go to stderr so the eval only sees the exports.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
dir="${1:-$(mktemp -d)}"
mkdir -p "$dir"
dir="$(cd "$dir" && pwd)"

case "$dir" in
  "$HOME/.claude" | "$HOME/.claude/"*)
    echo "dev-sandbox: refusing to use $dir (under ~/.claude)" >&2
    exit 1
    ;;
esac

bin="$repo/rust/target/debug/purrgotchi"
if [ ! -x "$bin" ]; then
  echo "dev-sandbox: building debug binary..." >&2
  cargo build --manifest-path "$repo/rust/Cargo.toml" >&2
fi

mkdir -p "$dir/bin" "$dir/state"
cat > "$dir/bin/purrgotchi" <<EOF
#!/usr/bin/env bash
export PURR_STATE_DIR='$dir/state'
exec '$bin' "\$@"
EOF
chmod +x "$dir/bin/purrgotchi"

echo "export PATH='$dir/bin':\"\$PATH\""
echo "export PURR_SANDBOX_STATE='$dir/state'"
echo "dev-sandbox: PURR_STATE_DIR=$dir/state" >&2
