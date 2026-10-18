# Budget Pacing

Hooks inject a `[budget]` line (5h plan usage vs. % of window elapsed, 7d usage, context size). On-demand: `/budget` skill. Goal: keep working the full 5h window without hitting the limit, and hand over cleanly before context gets heavy. Don't wait for the user to say "we're at 80%".

## 5-hour plan limit
- **ON-PACE** (used ≤ elapsed + 5): work normally.
- **AHEAD**: conserve. The main Opus thread is the biggest cost, so delegate more to Sonnet/Haiku, keep own turns short, no parallel fan-outs, no workflows, no high-effort reviews or advisor calls unless critical.
- **WALL** (≥90%): finish the current atomic step, commit/save it, then:
  - Reset < 60 min away → wait (see Waiting).
  - Reset ≥ 60 min away → write handover (below) and tell the user.
  - User said they're AFK / asked for autonomous work → always wait, however long, then continue.
- **7d ≥ 85%** (7D CRITICAL): overrides everything above. Tell the user and only do critical work.

## Escalation
If a Sonnet/Haiku subagent fails the same task twice (returns failing or incomplete), hand it to Opus with both failure reports attached.

## Waiting
Use a background Bash `sleep` (re-invokes on exit), in chunks of ~50 min. The prompt cache TTL is 1h, so pinging more often wastes budget. Each wake: run `/budget`, then either sleep again or resume. Keep wake turns minimal.
Long wall wait while AFK: stay in the session (no handover/clear), keep pinging so the cache stays warm, and time the last sleep to end ~5–10 min before the reset. That wake re-warms the cache on the old window's budget, so work resumes cleanly in the new window.

## Context soft cap (400k)
- **CTX HANDOVER ZONE** (≥300k): don't start new large pieces. Finish the current step, run finishing tasks (tests, commit if appropriate), then write the handover.
- **CTX OVER CAP** (≥400k): stop and hand over now.
- Handover file: `python3 ~/.claude/hooks/budget/budget.py handover-path`. It is auto-injected (then archived) on the next `/clear` or new session in the same directory. Tell the user: "type `/clear`, then `go`".
- Handover contents: goal, done (with commit SHAs), in flight, exact next step, open decisions, test/verify commands, key file paths. Keep it under ~8k chars.

## While subagents run
The main session sees no budget data while idle. For a subagent or workflow expected to take >5 min, also start a background Bash: `python3 ~/.claude/hooks/budget/budget.py watch` (`--interval S`, default 180; `--max-minutes M`, default 50; `--threshold P` for an earlier wall).
It exits only on: pace worse than at start, 5h wall (≥90%), 7d ≥85%, window reset, or `heartbeat` (max-minutes). Context size is ignored. If started already at the wall it waits for the reset.
On exit read `[budget-watch] <reason>`:
- heartbeat → restart it if agents are still running.
- pace worse (ahead) → downshift, launch no more agents.
- wall / 7d → consider stopping running agents (TaskStop), follow the WALL rules.
- reset → resume.
When the agents finish, stop the watcher with TaskStop on its task id (a plain `pkill -f "budget.py watch"` also kills the calling shell).
