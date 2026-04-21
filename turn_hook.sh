#!/usr/bin/env bash
# Purrgotchi UserPromptSubmit hook — clears the per-turn cost anchor so the
# next statusline render captures a fresh start-of-turn cost. With the anchor
# gone, statusline.sh recomputes delta as 0 and writes a new anchor for the
# duration of this turn.
#
# Register in ~/.claude/settings.json under hooks.UserPromptSubmit:
#   { "type": "command",
#     "command": "bash /c/Users/.../purrgotchi/turn_hook.sh" }
rm -f "$HOME/.claude/purrgotchi/turn-start-cost.txt" 2>/dev/null
exit 0
