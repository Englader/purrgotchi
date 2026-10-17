#!/usr/bin/env bash
# One-off probe, kept for engine bumps: does an unawaited `$` call started in a
# hook still run after that hook has returned, in a real interactive session?
#
#   bash mod/scripts/probe-after-return.sh [seconds]
#   bash mod/scripts/probe-after-return.sh --spawn [seconds]
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
# --spawn: does a `$.process.spawn` loop started from `session.start` keep its
# child after the hook returns (the `serve` stream's case)? Each child stamps a
# file at 6 s and at 12 s (past the 10 s hook budget), from two contexts:
#   start  the `session.start` `$`
#   timer  a `$.clock.after(0, ...)` timer's `$`, started from session.start
# Prints SPAWN-LIFETIME: <first context that stayed alive> (start, then timer;
# exit 0), or SPAWN-LIFETIME: NONE (exit 1). Also reports whether the children
# were gone once the session ended (no orphans).
#
# Runs under `env -i` with a temp HOME (onboarding done, the dir trusted) and
# project/local settings only: no model call, no credentials, no purrgotchi
# binary, nothing under ~/.claude read or written, no link to a parent session.
# Prints AFTER-RETURN: OK or AFTER-RETURN: KILLED (exit 0 / 1) for `cmd`;
# exit 2 = inconclusive (the command hook never ran).
set -euo pipefail

mode=run
if [ "${1:-}" = "--spawn" ]; then
  mode=spawn
  shift
fi
secs="${1:-$([ "$mode" = spawn ] && echo 16 || echo 12)}"
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
if [ "$mode" = spawn ]; then
  # Each child: alive at 6 s and 12 s, then sleeps on (killed with its loop).
  child() { printf "'sh', '-c', 'sleep 6; date +%%s%%3N > %s/%s.6s; sleep 6; date +%%s%%3N > %s/%s.12s; sleep 60; : purrprobe-%s'" "$dir" "$1" "$dir" "$1" "$1"; }
  cat > "$mod/hooks/register.ts" <<TS
import type { Register } from 'claude-code'

export const register: Register = on => {
  on('session.start', async (\$, e, next) => {
    const r = await next(e)
    void (async () => {
      for await (const c of \$.process.spawn({ argv: [$(child start)] })) void c
    })()
    \$.clock.after(0, () => {
      void (async () => {
        for await (const c of \$.process.spawn({ argv: [$(child timer)] })) void c
      })()
    })
    \$.ui.log('probe session.start returning at ' + Date.now(), { to: 'debug' })
    return r
  })
}
TS
else
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
fi

# The temp HOME's own config: onboarding done and the probe dir trusted
# (hooks modules load only after workspace trust).
cat > "$dir/home/.claude.json" <<JSON
{ "hasCompletedOnboarding": true, "theme": "dark", "projects": { "$dir": { "hasTrustDialogAccepted": true } } }
JSON

echo "probe: dir=$dir, mode $mode, session ${secs}s" >&2
if [ "$mode" = spawn ]; then
  (
    cd "$dir"
    sleep "$secs" |
      env -i HOME="$dir/home" PATH="$PATH" TERM=xterm-256color LANG="${LANG:-C.UTF-8}" USER="${USER:-probe}" \
        timeout "$secs" script -qfc \
        "stty cols 160 rows 50; claude --plugin-dir '$mod' --setting-sources project,local --debug-file '$dir/dbg.log'" \
        "$dir/tty.log" >/dev/null 2>&1 || true
  )
  sleep 1
  stamp() { cat "$dir/$1" 2>/dev/null || echo missing; }
  returned="$(grep -o 'probe session.start returning at [0-9]*' "$dir/dbg.log" 2>/dev/null | grep -o '[0-9]*$' | head -1 || true)"
  orphans="$(pgrep -f "purrprobe-(start|timer)" | wc -l || true)"
  pkill -f "purrprobe-(start|timer)" 2>/dev/null || true
  for c in start timer; do
    echo "probe: $c 6s $(stamp "$c.6s"), 12s $(stamp "$c.12s")" >&2
  done
  echo "probe: session.start returned at ${returned:-never}; children left after the session: $orphans" >&2
  if [ -z "$returned" ]; then
    echo "SPAWN-LIFETIME: INCONCLUSIVE (session.start never ran; see $dir/tty.log and $dir/dbg.log)"
    exit 2
  fi
  for c in start timer; do
    if [ -f "$dir/$c.12s" ]; then
      echo "SPAWN-LIFETIME: $c"
      exit 0
    fi
  done
  echo "SPAWN-LIFETIME: NONE"
  exit 1
fi
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
