# Purrgotchi

A cyberpunk-themed Claude Code statusline with a persistent tamagotchi kitten companion.

Renders a 7-row HUD showing model, DIR, GIT, rate-limit bars + reset times, per-turn cost / session total, LINES / TIME, and stat row — alongside a pixel-art cat whose mood reflects context pressure. The kitten persists across all Claude Code sessions on a machine, earning coins per tool call, levelling up, and speaking mood-contextual dialogue that rotates every 30s.

## Install

Run the installer — it configures `settings.json` idempotently:

```bash
bash install.sh
```

Or wire it manually in `~/.claude/settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "bash /absolute/path/to/purrgotchi/statusline.sh",
    "padding": 1
  },
  "refreshInterval": 1,
  "hooks": {
    "PostToolUse": [{
      "matcher": "*",
      "hooks": [{ "type": "command", "command": "/c/Windows/py.exe /absolute/path/to/purrgotchi/feed_hook.py" }]
    }],
    "UserPromptSubmit": [{
      "hooks": [{ "type": "command", "command": "bash /absolute/path/to/purrgotchi/turn_hook.sh" }]
    }]
  }
}
```

Restart Claude Code after install.

## Source vs state

- **Source** (this repo): scripts, libs, sprites — versioned
- **State** (`~/.claude/purrgotchi/`): `state.json` (level, coins, feed time) and `turn-start-cost.txt` (per-turn cost anchor) — per-machine, never versioned

## Hooks

| Event | Script | Purpose |
|---|---|---|
| `PostToolUse` | `feed_hook.py` | Feeds Thalia 1 coin per tool call; atomic `state.json` writes |
| `UserPromptSubmit` | `turn_hook.sh` | Clears the per-turn cost anchor so `Cost` resets at each new message |

## Customize

- **Rename the kitten**: `bash rename.sh "NewName"`
- **Add moods**: drop new `<mood>_<frame>.txt` files into `sprites/`, update `lib/mood.sh`
- **Add phrases**: edit `lib/speech.sh` (`_SPEECH` assoc array keyed `mood_N`, `_SPEECH_COUNTS` map)
- **Tweak colors**: edit the `C_*` palette constants at the top of `lib/render.sh`
- **Debug payloads**: set `PURR_DEBUG=1` to write each render's input JSON to `$PURR_STATE_DIR/last-input.json`

## Cost display

- **Cost**: spend since your last prompt submit (cleared by `turn_hook.sh`)
- **Total**: session cumulative from `.cost.total_cost_usd`

A 500ms anchor grace absorbs Claude Code's async token-finalization of the prior turn so its trailing cents don't leak into the new turn.

## Platform

Developed on Windows 11 + Git Bash (bash 5.2+ for `$EPOCHREALTIME`). Needs `jq` and Python 3 (for the feeding hook).
