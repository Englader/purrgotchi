---
name: budget
description: Check current 5-hour/7-day plan usage, pace, and context size on demand. Use before launching expensive work (parallel agents, workflows, big reviews) or when the injected budget line may be stale.
---

# Budget check

Run:

```
python3 ~/.claude/hooks/budget/budget.py line
```

It prints one line: 5h usage vs elapsed time (ON-PACE / AHEAD / WALL), reset time, 7d usage, and context size vs the 400k soft cap. If AHEAD or near the wall, avoid parallel agents and big reviews; if context is in the handover zone, write a handover.

Handover doc path for the current directory (write the handover there; it is auto-loaded on the next `startup`/`clear` session):

```
python3 ~/.claude/hooks/budget/budget.py handover-path
```

While subagents run, start `python3 ~/.claude/hooks/budget/budget.py watch` as a background Bash; it exits with `[budget-watch] <reason>` when pace worsens, the wall/7d is hit, the window resets, or on heartbeat (see rules/budget.md).
