# Purrgotchi

A cyberpunk-themed Claude Code statusline with a persistent tamagotchi kitten companion.

Renders a 7-row HUD showing model, DIR, GIT, rate-limit bars + reset times, per-turn cost / session total, LINES / TIME, and stat row — alongside a pixel-art cat whose mood reflects context pressure. The kitten persists across all Claude Code sessions on a machine, earning coins per tool call, levelling up, and speaking mood-contextual dialogue that rotates every 30s.

## Install

Build the binary, then run the installer:

```bash
cd rust && cargo build --release && cd ..
bash install.sh
```

`install.sh` wires all three entry points (statusline + `PostToolUse` feed hook + `UserPromptSubmit` turn hook) to `rust/target/release/purrgotchi.exe`. Restart Claude Code after install.

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

- **Source** (this repo): Rust crate in `rust/`, sprites in `sprites/` (embedded via `include_str!`) — versioned
- **State** (`~/.claude/purrgotchi/`): `state.json` (level, coins, feed time) and `turn-start-cost.txt` (per-turn cost anchor) — per-machine, never versioned

## Your Kitten

On first run, `purrgotchi` automatically hatches a kitten deterministically from your machine identity (username + hostname + birth time). The same machine always produces the same kitten unless you explicitly re-hatch.

Run `purrgotchi card` to see the full identity card — breed, rarity, stats, and personality blurb.

### Rarity Tiers

| Rarity | Stars | Distribution |
|---|---|---|
| Common | ★☆☆☆☆ | 60% |
| Uncommon | ★★☆☆☆ | 25% |
| Rare | ★★★☆☆ | 10% |
| Epic | ★★★★☆ | 4% |
| Legendary | ★★★★★ | 1% |
| + Shiny | ✦ prefix | +1% any tier |

### Dev Backdoors

Force a specific identity at hatch:

```bash
purrgotchi hatch --rarity legendary --breed "Void Cat" --shiny
# or via env vars:
PURR_FORCE_RARITY=epic PURR_FORCE_BREED="Sphinx" purrgotchi hatch
```

## Subcommands (v3.0+)

| Command | Purpose |
|---|---|
| `purrgotchi statusline` | Renders the HUD (reads CC payload on stdin) |
| `purrgotchi feed` | `PostToolUse` hook — bumps tool-call count, coins, level; updates `last_fed_at` |
| `purrgotchi turn` | `UserPromptSubmit` hook — clears the per-turn cost anchor |
| `purrgotchi rename <name>` | Renames the kitten in `state.json` |
| `purrgotchi hatch` | Re-rolls the kitten's breed, rarity, and stats |
| `purrgotchi card` | Prints the full identity card with stats and personality |
| `purrgotchi --version` | Prints the crate version |

## Customize

- **Rename the kitten**: `rust/target/release/purrgotchi.exe rename "NewName"`
- **Add sprites**: drop new `<mood>_<frame>.txt` files into `sprites/`, add branches in `rust/src/sprites.rs`, rebuild
- **Add phrases**: edit the per-mood arrays in `rust/src/speech.rs`, rebuild
- **Tweak colors**: edit the constants at the top of `rust/src/colors.rs`
- **Debug**: pass `--debug` (or set `PURR_DEBUG=1`) to log every invocation and render-timing to `$PURR_STATE_DIR/debug.log`, and dump the raw stdin JSON to `$PURR_STATE_DIR/last-input.json`. Enable by appending `--debug` to the `statusLine.command` in `~/.claude/settings.json` and restarting Claude Code.

## Cost display

- **Cost**: spend since your last prompt submit (anchor cleared by the `turn` hook)
- **Total**: session cumulative from `.cost.total_cost_usd`

A 500ms anchor grace window (microsecond-precision via `SystemTime::now()`) absorbs Claude Code's async token-finalization of the prior turn so its trailing cents don't leak into the new turn.

## Platform

Built and validated on Windows 11 (`x86_64-pc-windows-msvc`). Linux/macOS builds are `cargo build --release` away but not yet validated. Requires a Rust toolchain to build and `jq` to run the installer.
