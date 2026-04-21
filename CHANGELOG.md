# Changelog

All notable changes to Purrgotchi are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Planned (Phase 2 — see `PHASE_2_BACKLOG.md`)

- Windows path normalization (fix DIR overflow) and truncation
- Rate limit reset times (5h and 7d)
- Layout flip: HUD left, Thalia right
- Rich mood-contextual dialogue (speech bubble, Buddy-inspired)
- Extended TIME format (`Xh Ym Zs` for long sessions)
- Remove debug-log overhead (recover ~700ms render latency)

### Planned (Phase 3)

- Port to Rust for 25–50× speedup (~5–20ms per render)
- Eliminate jq, Python, bash/subshell dependencies

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
