# Purrgotchi

A cyberpunk-themed Claude Code statusline with a persistent tamagotchi kitten companion.

Renders a multi-line HUD showing model, session context, rate limits, cost, and code diff metrics, alongside a pixel-art cat whose mood reflects session state. The kitten persists across all Claude Code sessions on a machine — earning coins per tool call, levelling up, and reacting to context pressure and cost spikes.

## Install

1. Clone this repo anywhere (e.g. `~/Desktop/Work/purrgotchi/`)
2. Point Claude Code at it in `~/.claude/settings.json`:
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
       }]
     }
   }
   ```
3. Restart Claude Code.

## Source vs state

- **Source** (this repo): scripts, libs, sprites — versioned
- **State** (`~/.claude/purrgotchi/state.json`): level, coins, feed time — per-machine, never versioned

## Customize

- **Rename the kitten**: `bash rename.sh "NewName"`
- **Add moods**: drop new `<mood>_<frame>.txt` files into `sprites/`, update `lib/mood.sh`
- **Tweak colors**: edit the `C_*` palette constants at the top of `lib/render.sh`

## Platform

Developed on Windows 11 + Git Bash. Needs `jq` and Python 3 (for the feeding hook).
