# Purrgotchi

A cyberpunk-themed Claude Code statusline with a persistent tamagotchi kitten companion.

Renders a 7-row HUD showing model, DIR, GIT, rate-limit bars + reset times, per-turn cost / session total, LINES / TIME, and stat row — alongside a pixel-art cat whose mood reflects context pressure. The kitten persists across all Claude Code sessions on a machine, earning coins per tool call, levelling up, and speaking mood-contextual dialogue that rotates every 30s.

## Install

**v3.0+ (Rust, recommended)** — build the binary first, then run the installer:

```bash
cd rust && cargo build --release && cd ..
bash install.sh
```

`install.sh` auto-detects `rust/target/release/purrgotchi.exe` and wires all three entry points (statusline + `PostToolUse` feed hook + `UserPromptSubmit` turn hook) to the binary. Restart Claude Code after install.

**Bash fallback (v2.x, no Rust toolchain)** — run `bash install.sh` without building. It falls back to `statusline.sh` + `feed_hook.py` + `turn_hook.sh`. Requires `bash` 5.2+, `jq`, and Python 3.

Manual wiring for reference:

```json
{
  "statusLine": {
    "type": "command",
    "command": "C:/absolute/path/to/purrgotchi/rust/target/release/purrgotchi.exe statusline",
    "padding": 1
  },
  "refreshInterval": 1,
  "hooks": {
    "PostToolUse": [{
      "matcher": "*",
      "hooks": [{ "type": "command", "command": "C:/absolute/path/to/purrgotchi/rust/target/release/purrgotchi.exe feed" }]
    }],
    "UserPromptSubmit": [{
      "hooks": [{ "type": "command", "command": "C:/absolute/path/to/purrgotchi/rust/target/release/purrgotchi.exe turn" }]
    }]
  }
}
```

## Source vs state

- **Source** (this repo): Rust crate in `rust/`, bash fallback scripts at root, sprites in `sprites/` — versioned
- **State** (`~/.claude/purrgotchi/`): `state.json` (level, coins, feed time) and `turn-start-cost.txt` (per-turn cost anchor) — per-machine, never versioned

## Subcommands (v3.0+)

| Command | Purpose |
|---|---|
| `purrgotchi statusline` | Renders the HUD (reads CC payload on stdin) |
| `purrgotchi feed` | `PostToolUse` hook — bumps tool-call count, coins, level; updates `last_fed_at` |
| `purrgotchi turn` | `UserPromptSubmit` hook — clears the per-turn cost anchor |
| `purrgotchi rename <name>` | Renames the kitten in `state.json` |
| `purrgotchi --version` | Prints the crate version |

## Customize

- **Rename the kitten**: `rust/target/release/purrgotchi.exe rename "NewName"` (or `bash rename.sh "NewName"` on the bash fallback)
- **Add sprites**: drop new `<mood>_<frame>.txt` files into `sprites/`, add branches in `rust/src/sprites.rs`, rebuild
- **Add phrases**: edit the per-mood arrays in `rust/src/speech.rs`, rebuild
- **Tweak colors**: edit the constants at the top of `rust/src/colors.rs`
- **Debug payloads**: set `PURR_DEBUG=1` to write each render's input JSON to `$PURR_STATE_DIR/last-input.json`

## Cost display

- **Cost**: spend since your last prompt submit (anchor cleared by the `turn` hook)
- **Total**: session cumulative from `.cost.total_cost_usd`

A 500ms anchor grace window (500,000µs — microsecond-precision comparison via `$EPOCHREALTIME` in bash, `SystemTime::now()` in Rust) absorbs Claude Code's async token-finalization of the prior turn so its trailing cents don't leak into the new turn.

## Platform

Windows 11 only right now. Rust port is `x86_64-pc-windows-msvc`; bash fallback needs Git Bash 5.2+, `jq`, and Python 3. Linux/macOS builds are `cargo build --release` away but not yet validated.
