#!/usr/bin/env bash
# One-off probe, kept for engine bumps: does an unawaited `$` call started in a
# hook still run after that hook has returned, in a real interactive session?
#
#   bash mod/scripts/probe-after-return.sh [seconds]
#
# A throwaway mod (in a temp dir, never ./mod) starts `$.process.run(sleep 1.5;
# stamp)` without awaiting it, and returns at once, from three contexts:
#   cmd    a `command.run` hook for /purr-probe, typed into the pty (decisive:
#          an ordinary short-lived hook, like tool.call or UserPromptSubmit)
#   start  the `session.start` hook itself
#   timer  a `$.clock.after(1000, ...)` callback (the feed safety clock's case)
# A plugin's own `$.command.run` is not answered by its own hook, so the
# command is typed.
#
# Runs under `env -i` with a temp HOME (onboarding done, the dir trusted) and
# project/local settings only: no model call, no credentials, no purrgotchi
# binary, nothing under ~/.claude read or written, no link to a parent session.
# Prints AFTER-RETURN: OK or AFTER-RETURN: KILLED (exit 0 / 1) for `cmd`;
# exit 2 = inconclusive (the command hook never ran).
set -euo pipefail

secs="${1:-12}"
dir="$(mktemp -d)"
mod="$dir/mod"
mkdir -p "$mod/.claude-plugin" "$mod/hooks" "$dir/home"

cat > "$mod/.claude-plugin/plugin.json" <<'JSON'
{
  "name": "purrprobe",
  "version": "0.1.0",
  "description": "After-return probe for unawaited $ calls",
  "author": { "name": "Englader" }
}
JSON
echo '{ "modules": ["./register.ts"] }' > "$mod/hooks/hooks.json"

# `$` is escaped for the shell; $dir is expanded into the stamp paths.
cat > "$mod/hooks/register.ts" <<TS
import type { Register } from 'claude-code'

export const register: Register = on => {
  on('session.start', async (\$, e, next) => {
    const r = await next(e)
    await \$.command.register({ name: 'purr-probe', description: 'after-return probe' })
    void \$.process.run(['sh', '-c', 'sleep 1.5; date +%s%3N > $dir/start.ok'], { timeoutMs: 10000 })
    \$.clock.after(1000, () => {
      void \$.process.run(['sh', '-c', 'sleep 1.5; date +%s%3N > $dir/timer.ok'], { timeoutMs: 10000 })
    })
    return r
  })

  on('command.run', { command: 'purr-probe' }, async \$ => {
    void \$.process.run(['sh', '-c', 'sleep 1.5; date +%s%3N > $dir/cmd.ok'], { timeoutMs: 10000 })
    \$.ui.log('probe hook returning at ' + Date.now(), { to: 'debug' })
    return { text: 'returned' }
  })
}
TS

# The temp HOME's own config: onboarding done and the probe dir trusted
# (hooks modules load only after workspace trust).
cat > "$dir/home/.claude.json" <<JSON
{ "hasCompletedOnboarding": true, "theme": "dark", "projects": { "$dir": { "hasTrustDialogAccepted": true } } }
JSON

echo "probe: dir=$dir, session ${secs}s" >&2
(
  cd "$dir"
  { sleep 5; printf '/purr-probe'; sleep 0.5; printf '\r'; sleep "$secs"; } |
    env -i HOME="$dir/home" PATH="$PATH" TERM=xterm-256color LANG="${LANG:-C.UTF-8}" USER="${USER:-probe}" \
      timeout "$secs" script -qfc \
      "claude --plugin-dir '$mod' --setting-sources project,local --debug-file '$dir/dbg.log'" \
      "$dir/tty.log" >/dev/null 2>&1 || true
)

returned="$(grep -o 'probe hook returning at [0-9]*' "$dir/dbg.log" 2>/dev/null | grep -o '[0-9]*$' | head -1 || true)"
stamp() { cat "$dir/$1.ok" 2>/dev/null || echo missing; }
echo "probe: start.ok $(stamp start), timer.ok $(stamp timer), cmd hook returned at ${returned:-never}, cmd.ok $(stamp cmd)" >&2

if [ -z "$returned" ]; then
  echo "AFTER-RETURN: INCONCLUSIVE (the command hook never ran; see $dir/tty.log and $dir/dbg.log)"
  exit 2
fi
if [ -f "$dir/cmd.ok" ]; then
  echo "probe: the cmd call finished $(($(stamp cmd) - returned)) ms after its hook returned" >&2
  echo "AFTER-RETURN: OK"
  exit 0
fi
echo "AFTER-RETURN: KILLED"
exit 1
