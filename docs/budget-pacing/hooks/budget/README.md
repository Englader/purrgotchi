# budget

- `budget.py` - core. Subcommands: `cache`, `line`, `hook-prompt`, `hook-posttool`, `hook-sessionstart`, `handover-path`.
- `statusline-wrap.sh` - wraps the purrgotchi statusline, passes stdin/stdout/exit code through, caches rate limits and context data to `cache/<session_id>.json`.
- `test_budget.py` - unit tests (`python3 test_budget.py`). Env: `BUDGET_HOME` (default `~/.claude`), `BUDGET_WRAPPED_CMD`, `BUDGET_NO_NET=1`.
- `cache/`, `state/` - runtime data, created lazily. Handovers live in `~/.claude/handovers/<slug>.md`, archived to `handovers/archive/`.
- Skill: `~/.claude/skills/budget/SKILL.md`.

## settings.json

```json
{
  "statusLine": {"type": "command", "command": "/home/englader/.claude/hooks/budget/statusline-wrap.sh"},
  "hooks": {
    "UserPromptSubmit": [{"hooks": [{"type": "command", "command": "python3 /home/englader/.claude/hooks/budget/budget.py hook-prompt", "timeout": 5}]}],
    "PostToolUse": [{"matcher": "*", "hooks": [{"type": "command", "command": "python3 /home/englader/.claude/hooks/budget/budget.py hook-posttool", "timeout": 5}]}],
    "SessionStart": [{"hooks": [{"type": "command", "command": "python3 /home/englader/.claude/hooks/budget/budget.py hook-sessionstart", "timeout": 5}]}]
  }
}
```
(Merge into existing `hooks` arrays; keep any existing statusLine fields such as padding.)
