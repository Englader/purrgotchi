# Purrgotchi

A cyberpunk-themed Claude Code mod and statusline with a persistent tamagotchi kitten companion.

As a **Claude Code mod** (v4) the kitten lives in a live pane or band inside Claude Code, with pixel-art sprites, toasts, `/purr-*` slash commands and an optional Netrunner theme. The original **classic statusline** still works unchanged.

Renders a 7-row HUD showing model, DIR, GIT, rate-limit bars + reset times, per-turn cost / session total, LINES / TIME, and a stat row — alongside a pixel-art cat whose mood reflects context pressure. The kitten persists across all Claude Code sessions on a machine, earns coins per tool call, levels up, evolves through life stages, builds bond through interaction, celebrates milestones, and (optionally) speaks LLM-generated mood-contextual dialogue that reacts to what you just did.

---

## Requirements

- **Rust toolchain** (any recent stable; tested on 1.78+) — `rustup install stable`.
- **Claude Code** ([anthropic.com/claude-code](https://www.anthropic.com/claude-code)).
  - Mod path: **≥ 2.1.302**. The mods API is early access; the version range is pinned, and the engine types are regenerated on each Claude Code bump with `bash mod/scripts/lay-types.sh`.
  - Classic path: runs as the statusline + hooks on any recent Claude Code (playground scenes need ≥ 2.1.153).
- **`jq`** and **Bash** (classic path only, for `install.sh`) — `winget install jqlang.jq` on Windows, `brew install jq` on macOS, your package manager on Linux. On Windows, use Git Bash, MSYS2, or WSL. The mod needs neither.
- **(Optional) An LLM provider API key** — Anthropic, Gemini, or OpenAI — to enable AI-generated personality blurbs and in-character react quips. Without one, Purrgotchi falls back to a curated static phrase pool with no errors.

Built and validated on Windows 11 (`x86_64-pc-windows-msvc`). Linux and macOS builds compile cleanly via `cargo build --release` but are not yet exercised in production.

---

## Install as a mod (recommended)

1. Build the binary and put it on your `PATH`:

   ```bash
   git clone https://github.com/Englader/purrgotchi.git
   cd purrgotchi
   cargo install --path rust
   ```

   This installs `purrgotchi` into Cargo's bin directory. On Windows, or for a custom location, leave it where it is and set **Binary path** in `/config` (Purrgotchi's options).

2. In Claude Code, add the marketplace and install the plugin:

   ```
   /plugin marketplace add Englader/purrgotchi
   /plugin install purrgotchi@purrgotchi
   ```

   The CLI equivalents are `claude plugin marketplace add Englader/purrgotchi` and `claude plugin install purrgotchi@purrgotchi`. On the desktop app: `+` → Plugins → Add marketplace → Add plugin. (Claude Code ≥ 2.1.275 also accepts the one-step form `/plugin install purrgotchi --marketplace Englader/purrgotchi`.)

3. The hooks are live immediately: no restart and no `jq`. The first tool call hatches your kitten with a toast. With the default `dock` layout the pane opens by itself on a wide terminal (≥ 144 columns); `/purr` opens it at any width.

4. **Migrating from classic:** if you installed the statusline hooks earlier, run `bash install.sh --uninstall-hooks`. It removes the feed/turn hooks that `install.sh` wrote (the mod does the feeding now) and keeps the statusline, in case you want both. The mod warns with a toast while old hooks are still present, because they would feed the kitten twice. Your kitten and its state carry over: same `~/.claude/purrgotchi/`.

### Commands

| Command | Purpose |
|---|---|
| `/purr` | Open the kitten pane |
| `/purr-pet` | Pet the kitten (+bond) |
| `/purr-play` | Play (costs coins, starts a playground scene) |
| `/purr-treat` | Give a treat (costs coins, resets hunger) |
| `/purr-card` | Show the trading card |
| `/purr-stats` | Show identity, mechanics, progress and lifetime cost |
| `/purr-rename <name>` | Rename the kitten |
| `/purr-layout dock\|bottom\|classic\|off` | Switch where the kitten lives |
| `/purr-hatch` | Developer command: hatch a fresh kitten (only with **Developer commands** on) |

Plugin commands are not namespaced, and `/stats` and `/rename` are Claude Code built-ins, so every Purrgotchi command carries the `purr` prefix. The pane also has pet / treat / play buttons.

### Options

Set these in `/config` (they are the plugin's `userConfig`):

| Option | Default | What it does |
|---|---|---|
| Binary path | empty | Explicit path to the `purrgotchi` binary; empty = find it on `PATH` |
| Layout | `dock` | `dock`, `bottom`, `classic` or `off` (see below) |
| Netrunner restyle | off | Neon restyle of tool rows, user turns and the spinner |
| Open pane on start | on | Open the kitten pane at session start when the terminal is wide enough |
| Streak toasts | off | Toast on every 5-call streak (noisy) |
| Quips | `own-key` | Kitten one-liners: `off`, `session-model` or `own-key` |
| Developer commands | off | Expose `/purr-hatch` |

### Layouts

- **`dock`** — a live pane beside the conversation, with the pixel-art kitten, HUD, XP and lifetime cost. Streams frames from `purrgotchi serve`.
- **`bottom`** — a band above the prompt instead of a pane: four columns (portrait, kitten, session, limits) when there is room, a single compact row otherwise.
- **`classic`** — no pane or band; the kitten lives in the status entry and the classic statusline (from `install.sh`) if you have it. The mod still feeds the kitten and fires toasts, and never auto-opens the pane.
- **`off`** — no game at all: the mod never runs the binary. Combine it with the restyle for **skin-only** mode: just the Netrunner look, no kitten, no binary needed.

Switch any time with `/purr-layout`; `/purr-layout` stays available even when the layout is `off`.

### Netrunner theme

Pick **Netrunner** in `/theme` (a neon cyan/magenta palette on a dark base). Turn on **Netrunner restyle** in `/config` to also restyle tool rows, your prompts and the spinner. The theme is never forced on you. A mod cannot change the terminal background, font or prompt text colour, so for the full look set your terminal colour scheme to match (background `#0d0a14`).

### Quips

- **`own-key`** (default) — the classic provider in `~/.claude/purrgotchi/.env` (see below).
- **`session-model`** — the kitten's reactions come from your Claude Code session's own model (a Haiku call about once per 5 turns). It spends your Claude usage, and needs no API key.
- **`off`** — static phrases only.

### Troubleshooting

- **"Purrgotchi binary not found" toast.** Run `cargo install --path rust` and make sure Cargo's bin directory is on `PATH`, or set **Binary path** in `/config`. In `layout: off` (skin-only) the binary is never needed.
- **"binary is too old for this mod" toast.** Rebuild it with `cargo install --path rust`.
- **"double feeding" toast.** Old classic hooks are still in `~/.claude/settings.json`. Run `bash install.sh --uninstall-hooks` (or remove them by hand).
- **The mod misbehaves or won't load.** `claude --safe-mode` starts with mods disabled, and `claude --debug-file /tmp/claude.log` records what the mod did.

---

## Install as a classic statusline

The original statusline + hooks, no plugin involved:

```bash
git clone https://github.com/Englader/purrgotchi.git
cd purrgotchi
cd rust && cargo build --release && cd ..
bash install.sh
```

`install.sh` is idempotent — safe to re-run after upgrades. It:

1. Verifies prerequisites (`jq`, the compiled binary).
2. Backs up `~/.claude/settings.json` with a timestamped suffix.
3. Creates the state directory at `~/.claude/purrgotchi/`.
4. Merges the statusline + `PostToolUse` (feed) + `UserPromptSubmit` (turn) hooks into `settings.json`, all pointing at `rust/target/release/purrgotchi.exe`.

**Restart Claude Code after install** so the new settings take effect. To drop the hooks again later (for example to move to the mod) while keeping the statusline, run `bash install.sh --uninstall-hooks`.

### Manual wiring (reference)

If you'd rather edit `~/.claude/settings.json` by hand:

```json
{
  "statusLine": {
    "type": "command",
    "command": "C:/absolute/path/to/purrgotchi/rust/target/release/purrgotchi.exe statusline",
    "padding": 1,
    "refreshInterval": 1
  },
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

Note that `refreshInterval` sits **inside** the `statusLine` object — Claude Code ignores it anywhere else. It re-runs the statusline every second even while the session idles, which is what makes the kitten actually animate.

### Optional: a CLI wrapper

For quick interactive commands (`purrgotchi pet`, `purrgotchi card`, etc.), add a small wrapper to your `PATH`. On Windows / Git Bash:

```bash
mkdir -p ~/.local/bin
cat > ~/.local/bin/purr <<'EOF'
#!/usr/bin/env bash
exec "$HOME/path/to/purrgotchi/rust/target/release/purrgotchi.exe" "$@"
EOF
chmod +x ~/.local/bin/purr
```

Now `purr card`, `purr stats`, `purr pet`, etc. work from any directory.

---

## Optional: enable AI personality and react quips

Without an API key, Purrgotchi works fine with a curated static phrase pool. To unlock:

- A unique LLM-generated personality blurb at hatch (matched to your kitten's breed, rarity, and stat profile).
- A 1-in-5 chance per turn that your kitten reacts in-character to whatever you just did in Claude Code (~1s latency, ~10 output tokens).

Copy the example env file and uncomment **one** provider block:

```bash
cp .env.example .env
```

```bash
# .env — uncomment one block:

# --- Anthropic ---
# PURR_LLM_PROVIDER=anthropic
# ANTHROPIC_API_KEY=sk-ant-api03-...
# PURR_LLM_MODEL=claude-haiku-4-5

# --- Gemini ---
# PURR_LLM_PROVIDER=gemini
# GEMINI_API_KEY=...
# PURR_LLM_MODEL=gemini-2.5-flash

# --- OpenAI ---
# PURR_LLM_PROVIDER=openai
# OPENAI_API_KEY=sk-...
# PURR_LLM_MODEL=gpt-4o-mini
```

Purrgotchi reads `.env` from **`~/.claude/purrgotchi/.env`** only (`install.sh` copies the repo's `.env` there on first install). It deliberately does not load `.env` from the current directory — hooks run with the cwd of whatever project you have open, and a stranger repo's `.env` should never be able to inject keys or config. Process-level environment variables (e.g. shell-exported) win over `.env` values. `PURR_LLM_PROVIDER` is **required**; `PURR_LLM_MODEL` is optional and defaults to the cheapest fast model for each provider. Daily LLM cost runs roughly **$0.01–$0.06** at typical Claude Code usage rates.

After hatching with no provider configured, you'll see a one-line stderr hint reminding you these env vars exist. Configure later via `purrgotchi rename`-style on-disk edits — no restart of the kitten needed.

---

## Your kitten

On first run, Purrgotchi automatically hatches a kitten deterministically from your machine identity (`username:hostname:birth_time` seed). Kittens are permanent — the same machine always produces the same kitten.

```bash
purrgotchi card     # full identity card — breed, rarity stars, stats, personality, bond
purrgotchi stats    # human-readable view of identity, mechanics, progress, lifetime cost
```

### Rarity tiers

| Rarity | Stars | Distribution |
|---|---|---|
| Common | ★☆☆☆☆ | 60% |
| Uncommon | ★★☆☆☆ | 25% |
| Rare | ★★★☆☆ | 10% |
| Epic | ★★★★☆ | 4% |
| Legendary | ★★★★★ | 1% |
| Shiny | ✦ prefix | +1% on top of any tier |

Sprite color follows rarity: cyan → green → purple → gold → cycling rainbow (legendary) → animated shimmer (shiny).

### Life stages

Kittens evolve as they level up: **kitten → young cat → adult**. Each stage has its own sprite set and its own celebration animation when crossing the threshold.

### Stats

Five feline traits — **Curiosity, Mischief, Laziness, Affection, Wisdom** — generated with peak/dump distribution. Higher rarity raises the floor on the middle three.

### Bond

A 0–100 dynamic counter starting at 50. Grows when you interact (pet, play, treat); decays slowly when neglected (5/day, anchored to the most recent interaction). Visible as a bar on the card and the HUD.

---

## Subcommands

| Command | Purpose |
|---|---|
| `purrgotchi statusline` | Renders the HUD (reads Claude Code's payload on stdin) |
| `purrgotchi feed` | `PostToolUse` hook — bumps tool-call count, coins, level; updates `last_fed_at` |
| `purrgotchi turn` | `UserPromptSubmit` hook — clears per-turn cost anchor; rolls 1-in-5 dice for `react` |
| `purrgotchi react <transcript>` | Detached LLM worker (spawned by `turn`) — generates an in-character quip |
| `purrgotchi card` | Full identity card with stats, personality, bond |
| `purrgotchi stats` | Human-readable view of identity, mechanics, lifetime cost |
| `purrgotchi pet` | +15 bond, 4h cooldown, 120s happy mood window |
| `purrgotchi play` | 50 coins, +20 bond, 6h cooldown, 180s happy mood window |
| `purrgotchi treat` | 25 coins, +5 bond, 4h cooldown, 120s happy; resets hunger |
| `purrgotchi rename <name>` | Renames the kitten in `state.json` (personality re-templates at render time) |
| `purrgotchi frame [--json] [--width N] [--advance] [--pixels]` | Read-only snapshot of the kitten for the mod (`--pixels` adds the 32x32 sprite grid) |
| `purrgotchi serve [--interval-ms N]` | Streams `frame` as NDJSON, one line per tick (what the dock pane reads) |
| `purrgotchi cost --session <id> --total <usd>` | Folds a session's cost into the lifetime total |
| `purrgotchi quip-prompt` / `quip-store <text>` | Build the prompt for a session-model quip / store the sanitized reply |
| `purrgotchi --version` | Prints the crate version |

Every action command (`feed`, `turn`, `pet`, `play`, `treat`, `rename`, `stats`, `card`) also takes `--json` for a versioned machine-readable result (what the mod uses), and `feed --count N` batches N tool calls into one call. Set `PURR_STATE_DIR` to an absolute path to run against a throwaway state directory instead of `~/.claude/purrgotchi/`.

### Events and celebrations

- **Tool-call streaks** — a "5 in a row!" one-shot speech overlay.
- **Milestones** — 100 / 500 / 1000 lifetime tool calls each get a unique celebration.
- **Birthday** — on the anniversary of your kitten's hatch, a special sprite + speech for the day.
- **Daily greeting** — first prompt of each day triggers a "welcome back".
- **Random drops** — low-probability "+10 coins" surprises.

Priority order for HUD overlays: **celebration > birthday > scene > LLM quip > mood**.

---

## Playground scenes

Thalia doesn't just sit beside her card — every few minutes she plays in the
open space to the right of it: chasing a bouncing ball, stalking a butterfly,
doing zoomies, rolling across the screen, or pushing a ball of yarn. Scenes
last 20–40 seconds, pick themselves at random (never the same one twice in a
row), and `purr play` starts one immediately.

- Needs a reasonably wide terminal (~125+ columns) and Claude Code v2.1.153 or
  newer (which reports the terminal width to the status line). On narrower
  terminals the playground simply stays off — nothing breaks.
- To turn scenes off entirely, add `PURR_PLAYGROUND=0` to
  `~/.claude/purrgotchi/.env`.

---

## Source vs state

- **Source** (this repo): Rust crate in `rust/`, sprites in `sprites/` (embedded via `include_str!`), specs/plans in `docs/superpowers/` — versioned in git.
- **State** (`~/.claude/purrgotchi/`): `state.json` (level, coins, bond, feed time, ...), `cost-sessions.json` (per-session cost sidecar with 30-day GC), `llm-quip.json` (transient quip cache, 6-render TTL), `turn-start-cost.txt` — per-machine, **never** versioned.

---

## Customize

- **Rename the kitten:** `purrgotchi rename "NewName"`.
- **Add sprites:** drop new `<mood>_<frame>.txt` files into `sprites/`, add branches in `rust/src/sprites.rs`, rebuild.
- **Add static phrases:** edit the per-mood arrays in `rust/src/speech.rs`, rebuild.
- **Tweak colors:** edit the constants at the top of `rust/src/colors.rs`.
- **Debug logging:** set `PURR_DEBUG=1` (or pass `--debug`) to write timestamped lines to `~/.claude/purrgotchi/debug.log` — covers LLM call latency, token counts, react worker outcomes, watchdog events, and statusline render timing. Enable system-wide by appending `--debug` to the `statusLine.command` in `~/.claude/settings.json` and restarting Claude Code.

---

## Cost display

- **Cost** — spend since your last prompt submit (anchor cleared by the `turn` hook).
- **Total** — session cumulative from `.cost.total_cost_usd`.
- **Lifetime** — accumulated across every session ever (visible in `purrgotchi stats`).

A 500ms anchor grace window (microsecond-precision via `SystemTime::now()`) absorbs Claude Code's async token-finalization of the prior turn so trailing cents don't leak into the new turn.

Cost tracking currently captures Anthropic-only API spend; Gemini and OpenAI calls don't update the lifetime accumulator.

---

## Architecture & docs

- **Per-release CHANGELOG:** [`CHANGELOG.md`](./CHANGELOG.md).
- **Design specs:** `docs/superpowers/specs/` — one per major feature or hotfix, written before implementation.
- **Implementation plans:** `docs/superpowers/plans/` — bite-sized TDD task lists derived from each spec.
- **Master plan:** `docs/superpowers/plans/kitten-mechanics-master-plan.md` — phase roadmap and progress log.

All four phases of the master plan have shipped (Phase 1 identity → Phase 2 character depth → Phase 3 LLM speech → Phase 4 provider-agnostic LLM). Current release: **v4.0.0** (Claude Code mod).
