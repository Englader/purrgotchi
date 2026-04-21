# Changelog

All notable changes to Purrgotchi are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Planned (Phase 3)

- Port to Rust for 25–50× speedup (~5–20ms per render)
- Eliminate jq, Python, bash/subshell dependencies
- Nerd Font icon mode (`PURR_NERD=1` gate)

## [2.0.0] - 2026-04-21

Phase 2 shipped plus live UX iteration. **Breaking** for anyone with a pinned v1.x settings.json — the HUD grew from 6 rows to 7 rows + 1 speech row, layout flipped (HUD left / sprite right), box widened 60→90 cells, and a new `UserPromptSubmit` hook is required for per-turn cost tracking. Re-run `install.sh` to auto-migrate.

### Added

- **Speech bubble** (`lib/speech.sh`) — 29 curated phrases across 6 moods, rotating every 30s via `(epoch/30) % N` dispatch. Lives in the sprite column next to the bottom rail
- **Rate-limit reset row** (H4) — `RESET: 5h in Xh Ym │ 7d in …`, pulls `resets_at` fields from the session payload via a new `time_until_reset` helper
- **Per-turn cost tracking** — `Cost $X.XX` (delta since your last prompt submit) and `Total $X.XX` (session cumulative) both display in col 3. Anchor persists in `~/.claude/purrgotchi/turn-start-cost.txt`, cleared by the new `UserPromptSubmit` hook
- **`turn_hook.sh`** — new bash hook script that deletes the cost anchor on every user submit
- **500ms anchor grace window** — pure-bash microsecond timing (`$EPOCHREALTIME`) absorbs Claude Code's async cost finalization of the prior turn, preventing a few-cent leak into the new turn
- **Hour-aware TIME format** — `human_duration` now renders `Xh Ym Zs` / `Xm Ys` / `Xs` depending on magnitude
- **Three-column fixed-width grid** (`W1=27 / W2=29 / W3=26`) — every HUD row uses the same grid so the two `│` separators align vertically across H2/H3/H4/H5/H6
- **`pad_col` / `center_col` / `split_col` / `anchor_col` layout helpers** in `statusline.sh` for composing column content
- **H6 split layout** — `LV │ COINS` anchored in col1 (COINS hugs middle `│`), empty col2, `MOOD │ FED` anchored in col3 (FED hugs middle `│`). MOOD comes first so the ticking FED timer doesn't shift leading content
- **`install.sh`** now also wires `hooks.UserPromptSubmit` to `turn_hook.sh`

### Changed

- **Layout flipped**: HUD on the left, sprite on the right (was sprite-left / HUD-right in v1.x)
- **Box widened**: `BOX_WIDTH` 60 → 90 content cells (total line width 81 → 111 chars) to fit long model names, full DIRs, and the new `FED │ MOOD` combo in col3
- **H3 column order**: `5h │ 7d │ CTX` (was `CTX │ 5h │ 7d`) — 7d usage now sits directly above the 7d reset in H4
- **H5 column order**: `LINES │ TIME │ Total` — cost moved from col1 to col3 next to per-turn Cost in H4
- **Windows path normalization** — `C:\Users\…` now converts to `/c/Users/…` before `short_cwd`, so `$HOME` matching actually works on Git Bash
- **DIR truncation limit** tightened 38 → 25 chars (left-ellipsis: `...tail/of/path`) to fit col2 budget
- **Mood trigger** — panic no longer fires on `cost_usd > 5`; only context drives panic (`>80%`) / alert (`>40%`). Thalia stops stress-reacting to spend
- **Rendered output width**: 93 → 97 → 111 chars across iterations (final: 111)

### Removed

- **Unconditional debug.log append block** — 23 lines of per-render shell work (`date`, `wc -l`, `tail`, `mv`) gone. Recovers ~700ms on Windows + Git Bash, bringing render under 500ms
- **Unconditional `last-input.json` write** — now gated behind `PURR_DEBUG=1` env var, and the output path moved from the repo to `$PURR_STATE_DIR`

### Fixed

- **Windows `C:\` path overflow** — long backslash paths used to bust the right rail; now normalized + truncated
- **MODEL row overflow** with long `display_name` (e.g. `"Opus 4.7 (1M context)"`) — fixed by the wider box plus fixed-width columns
- **`│` separator misalignment** between rows — every row now uses the same 3-column grid

## [1.0.1] - 2026-04-21

Packaging polish.

### Added

- `.gitattributes` enforcing LF line endings for shell/Python/markdown files (prevents Windows `autocrlf=true` from breaking cross-platform use)
- `install.sh` — idempotent installer that checks prerequisites, backs up `settings.json`, and merges Purrgotchi config via jq
- `CHANGELOG.md` (this file)

### Notes

- No behavior changes; source files untouched
- Installer preserves existing `PostToolUse` hooks (won't clobber other users' hooks)

## [1.0.0] - 2026-04-21

Initial release. End-to-end working bash statusline with persistent tamagotchi kitten on Windows + Git Bash.

### Added

- `statusline.sh` — hot-path render (single jq call, `printf -v` throughout, bash regex for state)
- `feed_hook.py` — PostToolUse hook with atomic `state.json` writes (tempfile + `os.replace`)
- `lib/render.sh` — ANSI color palette (Cyberpunk-Neon 24-bit RGB) + formatting helpers (progress_bar, human_duration, human_elapsed, short_cwd)
- `lib/mood.sh` — priority-ordered mood state machine (panic → alert → sleep → hungry → happy → calm)
- `lib/sprites.sh` — sprite file loader with fallback chain (mood_frame → mood_0 → calm_0)
- 11 hand-drawn 5-line ASCII sprites across 6 moods (happy, calm, alert, panic, hungry, sleep; 1–2 frames each)
- `rename.sh` — kitten rename utility (preserves existing state)
- `README.md` — project description and install instructions
- `.gitignore` — excludes runtime state and debug artifacts from version control

### Architecture

- **Source/state separation**: scripts in repo (versioned), `state.json` in `~/.claude/purrgotchi/` (per-machine)
- **Portable path resolution**: `PURR_DIR` computed from `$BASH_SOURCE`, so the repo can live anywhere on disk
- **Performance**: single jq subprocess, bash regex for state parsing, `printf -v` to avoid subshells (Windows process-spawn is expensive)
- **Alignment invariant**: every rendered row has equal visible-char count; verified by ANSI-strip + `awk length`

### Fixed during development

- Bash `IFS=$'\t'` collapses consecutive whitespace tabs (POSIX rule) — switched to `\x01` (SOH) for accurate empty-field preservation
- `$(</dev/stdin)` read was unreliable in CC's Windows spawn context — switched to `$(cat)` for reliability
- Decorative Unicode glyphs (⧖, ◆) have ambiguous east-asian width — removed from content, reserved only for box borders where we control every char
- CRLF injection from jq output on Windows — added non-digit stripping on numeric fields
- `DIR: 0` render bug from empty jq field parsing with wrong IFS — fixed via SOH separator

### Known issues (tracked in `PHASE_2_BACKLOG.md`)

- Windows-format paths (`C:\...`) overflow the box border — no normalization yet
- `TIME` field shows `Xm Ys` even past 60 minutes (no hour unit)
- Debug-log append adds ~700ms latency per render (should be removable now that pipeline is proven)
