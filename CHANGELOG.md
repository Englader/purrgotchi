# Changelog

All notable changes to Purrgotchi are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Planned

- Nerd Font icon mode (`PURR_NERD=1` gate)
- Linux/macOS builds (currently Windows-only)

## [3.3.0] - 2026-04-21

### Added

- **Token count in CTX column**. Context usage now renders as `345k (34%)` instead of bare `34%` — shows the absolute token count alongside the percentage. Uses `context_window.current_usage` (sum of input + output + cache_creation + cache_read tokens) from the CC payload. Formats as `N` / `1.2k` / `12k` / `1.2M`. 5h/7d bars stay percentage-only because CC doesn't send raw token counts for rate limits — asymmetric but honest. CTX bar narrowed from 12→11 chars to fit the token string with margin for the 100% edge case.

### Fixed

- **GIT column empty when cwd is a subdirectory**. `read_git_branch` previously only checked `cwd/.git/HEAD`, which returned nothing when CC ran with cwd inside a subfolder (e.g. `.../purrgotchi/rust`). Now walks up the parent chain until `.git/HEAD` is found or the filesystem root is reached — matches git's own discovery behavior.
- **Progress bar invisible at low percentages**. At <9% with a narrow bar, integer truncation (`pct * width / 100`) rounded to 0 filled blocks, so the bar looked completely empty. Now rounds to nearest and clamps to at least 1 block whenever `pct > 0`. Affects all three bars (5h, 7d, CTX); most visible on CTX where low context usage is common.

## [3.2.0] - 2026-04-21

### Added

- **Debug logging module** (`src/debug.rs`). Activated by either `PURR_DEBUG=1` env var OR a `--debug` flag anywhere in argv (so you can enable it via `settings.json` without a shell wrapper). Appends timestamped lines to `~/.claude/purrgotchi/debug.log` — one line per invocation with pid + subcommand, plus a second line after the statusline render with `mood`, `fed_delta`, `ctx`, and `render_us`. Silent on any I/O failure; never breaks the hot path.
- First use case: diagnosed that `refreshInterval: 1` is NOT firing as a 1-second metronome on Windows — observed render gaps of 0.2s–31.8s during idle. CC's idle tick scheduler isn't behaving as documented. Purrgotchi renders are 1–2ms with an occasional ~346ms Defender-cold outlier.

### Investigation notes

The chunky "FED: 7s ago → 23s ago" behavior is a Claude Code statusline-scheduler quirk, not a Purrgotchi bug. The binary computes `now - last_fed_at` fresh on every invocation (no caching), so the data is always correct — it just only refreshes when CC happens to invoke us, which on Windows is event-driven with long idle gaps.

## [3.1.1] - 2026-04-21

### Fixed

- **GIT column empty** on Windows. The v3.0 path normalizer produced MSYS-style `/c/Users/...` which Rust's `fs::read_to_string` can't resolve on Windows, so `.git/HEAD` reads silently failed. Normalizer now emits `C:/Users/...` form — Windows fs handles it natively, and the HOME-starts-with check in `short_cwd` still works.

## [3.1.0] - 2026-04-21

Cleanup release — retires the bash/python implementation now that v3.0 is stable. No behavior changes; the Rust binary has been running all three entry points on this machine since v3.0 merged.

### Removed

- `statusline.sh`, `rename.sh`, `turn_hook.sh`, `feed_hook.py` — all replaced by `purrgotchi.exe` subcommands since v3.0
- `lib/mood.sh`, `lib/render.sh`, `lib/speech.sh`, `lib/sprites.sh` — ported to Rust modules in v3.0
- `PHASE_2_BACKLOG.md` — historical planning doc, Phase 2 shipped in v2.0

### Changed

- **`install.sh`** no longer ships the bash/python fallback path. Requires `jq` and a built `rust/target/release/purrgotchi.exe`; clear error message if either is missing.
- **`README.md`** — all bash-fallback references removed; install instructions cut down to build + run.

## [3.0.0] - 2026-04-21

Phase 3 shipped — full Rust rewrite. **Breaking** for anyone with a pinned v1/v2 `settings.json`: the statusline command and both hooks now point at a single `purrgotchi.exe` binary. Re-run `install.sh` to auto-migrate. The installer auto-detects `rust/target/release/purrgotchi.exe` and wires the binary when present; falls back to the bash/python scripts otherwise.

### Added

- **`rust/`** — new Rust crate. One binary, four subcommands: `statusline`, `feed`, `turn`, `rename <name>`. Dependencies: `serde` + `serde_json` only. No `regex`, no `chrono`, no CLI crate — string ops, `std::time`, hand-rolled dispatch.
- **Embedded sprites** via `include_str!` — zero runtime disk reads for the 11 sprite files, they ship compiled into the binary.
- **Embedded speech phrases** — the `_SPEECH_COUNTS` + `_SPEECH` bash associative arrays are now `&[&str]` arrays per mood in `src/speech.rs`, selected by the same `(epoch/30) % N` rotation.
- **Release profile** tuned for size + speed: `opt-level=3`, `lto=true`, `codegen-units=1`, `strip=true`, `panic="abort"`. Final binary is ~120KB.

### Changed

- **`install.sh`** now picks Rust binary over bash/python when present. Python is no longer required when the Rust binary is available. jq is still required (installer still uses it to merge `settings.json`).
- **Hook commands** match-in-place rule loosened from `test("feed_hook")` / `test("turn_hook")` to `test("purrgotchi")`, so re-running the installer correctly replaces both legacy and new hook entries.
- **`.gitignore`** — `target/` instead of `/target/` (catches the `rust/target/` subdirectory), removed `Cargo.lock` ignore (commit the lockfile for a binary crate), added `.claude/` for per-machine CC settings.

### Performance

v3.0.0 render latency on Windows 11 (5 runs, PowerShell, no bash wrapper): **~180ms per render, median ~180ms** vs v2.0's median ~1430ms — **~8× speedup**. Per-invocation render work inside the binary is ~5ms; the remaining ~175ms is Windows process-spawn cost (plausibly Defender real-time scan). Hitting the 5–20ms plan target would need a code-signing cert, a Defender exclusion, or a long-running daemon — not worth it for a 1Hz statusline.

### Removed (from the runtime path)

- `bash` — statusline and `turn` hook no longer shell out
- `jq` — JSON parsing now via `serde_json` in-process (still needed by `install.sh` for settings merge)
- `python` — `feed` hook no longer spawns Python
- `lib/render.sh`, `lib/mood.sh`, `lib/speech.sh`, `lib/sprites.sh` — all ported to Rust modules
- `feed_hook.py`, `turn_hook.sh` — replaced by `purrgotchi.exe feed` / `purrgotchi.exe turn`

Bash/Python scripts remain in the repo as reference and for users without a Rust toolchain.

### Fixed

- **Windows HOME normalization** — MSYS path-translates `$HOME` to `C:\Users\...` form when passed to a native Windows process, which broke tilde-expansion in `short_cwd`. Rust version normalizes HOME with the same regex applied to cwd, so `~/Desktop/...` renders correctly.

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
