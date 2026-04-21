# Purrgotchi — Phase 2 Feature Backlog

**Handoff doc for Ultraplan.** This describes features requested after v1.0 shipped.
Use this as the source of truth for what needs to be implemented in Phase 2.

## Current state (v1.0 — already working)

End-to-end bash statusline rendering in Claude Code on Windows + Git Bash, with:

- **6-row cyberpunk HUD** with kitten sprite on the LEFT, HUD box on the RIGHT
- **Single jq call** for session JSON extraction (performance-critical)
- **`printf -v`** throughout to avoid subshells (Windows process-spawn is expensive)
- **`IFS=$'\x01'`** (SOH control char) as field separator — bash collapses consecutive whitespace IFS chars, SOH avoids that trap
- **State in `~/.claude/purrgotchi/state.json`** (per-machine, unversioned)
- **Source in `~/Desktop/Work/purrgotchi/`** (this repo, versioned)
- **PostToolUse hook** (`feed_hook.py`) feeds Thalia 1 coin per tool call; atomic state.json writes
- **Six moods**: `calm`, `alert`, `panic`, `hungry`, `sleep`, `happy` — each with 1-2 sprite frames
- **Palette**: Cyberpunk-Neon — cyan `#0abdc6`, pink `#ea00d9`, magenta `#711c91`, yellow `#f3f31b`, green `#57eca6`, red `#ff5468`

### Fields currently displayed

| Row | Content |
|---|---|
| H1 | `╔═╡ NETRUNNER // Thalia ╞══════════╗` (title rail) |
| H2 | `MODEL <name> │ DIR <cwd> │ GIT <branch>` |
| H3 | `CTX ███░ N% │ 5h ░░ N% │ 7d ░░ N%` |
| H4 | `COST $N │ LINES +N/-N │ TIME Nm Ns` |
| H5 | `LV N │ COINS N │ FED N ago │ MOOD <mood>` |
| H6 | `╚══════════════════════════════════╝` (bottom rail) |

Current performance: ~500–700ms per render on Windows + Git Bash.

---

## Phase 2 features to implement

### 1. Windows path normalization + truncation 🐛

**Problem**: Claude Code sends paths in Windows format (`C:\Users\Stefan.Markoski\Desktop\Work\Fidelity`). Our `short_cwd` compares against `$HOME` = `/c/Users/Stefan.Markoski` (forward-slash format), so no match → falls through to verbatim, printing the full 41-char path and overflowing the box border.

**Fix**:
1. **Normalize** before `short_cwd`: convert `C:\...` → `/c/...` using bash regex:
   ```bash
   if [[ "$cwd" =~ ^([A-Za-z]):\\(.*)$ ]]; then
       _drive="${BASH_REMATCH[1],,}"
       _rest="${BASH_REMATCH[2]//\\//}"
       cwd="/${_drive}/${_rest}"
   fi
   ```
2. **Truncate** if still too long (max 38 chars) using left-ellipsis: `...Work/Fidelity`

Location: `statusline.sh`, just after `cwd="$cwd_alt"` fallback assignment.

---

### 2. Rate limit reset times 🆕

**Source payload** (confirmed in `last-input.json` captures):
- `.rate_limits.five_hour.resets_at` — Unix epoch
- `.rate_limits.seven_day.resets_at` — Unix epoch

**New `time_until_reset` helper** in `lib/render.sh`, pure bash:

```bash
time_until_reset() {
    local _out="$1" reset_at="${2:-0}" now_ts="${3:-0}"
    if [ "${reset_at:-0}" -le 0 ] || [ "$reset_at" -le "$now_ts" ]; then
        printf -v "$_out" "---"
    else
        local delta=$(( reset_at - now_ts ))
        if [ "$delta" -lt 3600 ]; then
            printf -v "$_out" "in %dm" "$(( delta / 60 ))"
        else
            printf -v "$_out" "in %dh %dm" "$(( delta / 3600 ))" "$(( (delta % 3600) / 60 ))"
        fi
    fi
}
```

**Display placement**: new row `H4` between the current H3 (rate bars) and H4 (COST), showing:
```
║ RESET: 5h in 42m │ 7d in 3h 14m              ║
```

Existing H4/H5/H6 shift down.

---

### 3. Fix TIME format — show hours 🐛

**Problem**: `human_duration` in `lib/render.sh` only formats as `Xm Ys` or `Xs`. After an hour, it shows `139m 4s` (ugly).

**Fix**: extend to show `Xh Ym Zs`:

```bash
human_duration() {
    local _out="$1" ms="${2:-0}"
    local s=$(( ms / 1000 ))
    local h=$(( s / 3600 ))
    local m=$(( (s % 3600) / 60 ))
    local rs=$(( s % 60 ))
    if   [ "$h" -gt 0 ]; then printf -v "$_out" "%dh %dm %ds" "$h" "$m" "$rs"
    elif [ "$m" -gt 0 ]; then printf -v "$_out" "%dm %ds" "$m" "$rs"
    else                       printf -v "$_out" "%ds" "$rs"
    fi
}
```

---

### 4. Move Thalia to the RIGHT side of the HUD 🎨

**Current layout**: `[16-char sprite]   [62-char HUD]` — sprite on left, HUD on right.

**New layout**: `[62-char HUD]   [16-char sprite]` — HUD on left, sprite on right.

**Why**: opens clean real estate below the sprite for a speech bubble (feature 5).

**Implementation**: reverse the final render loop's printf order. Keep total width at 81 chars for backward compatibility.

---

### 5. Rich personality dialogue — Thalia speaks ✨

**Inspired by**: Anthropic's Buddy feature (deprecated April 2026). Buddy had a "skeleton + soul" architecture — system prompt + mini-LLM generating contextual dialogue. We're NOT building LLM-powered dialogue (too slow, too expensive for a statusline that runs every second); we're doing a **rich canned phrase pool** that *feels* alive.

**Requirements**:
- **Longer than one word** — short sentences, not just "meow". User was emphatic about this.
- **Mood-contextual** — phrases match kitten's current mood
- **Multiple variants per mood** — randomize across 4-8 phrases so it doesn't repeat immediately
- **Slow rotation** — change phrase every ~30 seconds (based on `(epoch / 30) % N` for pseudo-random but stable-between-frames)
- **Occasional context awareness** — bonus points if phrases can reference current session state (e.g., `"dang, $X.XX already!"` when cost is climbing)

**Phrase pool design** (suggestion, Ultraplan can expand):

```
calm:    "just vibing over here", "code code code...", "purring softly", "what a nice day",
         "mrow.", "zzz... oh hi", "watching you work", "keeping an eye on things"

happy:   "you got this, human!", "yesss a tool call!!", "work work work! <3",
         "mrow! that felt productive", "teamwork makes the dream work", "nice one!"

alert:   "hmm, getting busy...", "context is filling up", "watching the bars...",
         "keep it tight", "gentle reminder: context"

panic:   "WE'RE LOSING CONTEXT!!", "CONDENSE! CONDENSE!!", "aaa too much!!",
         "/clear might be your friend...", "the walls are closing in"

hungry:  "mroooow? feed me with tool calls", "so empty... run something?",
         "I'm starving for work", "where are the tools? :("

sleep:   "zzz... just 5 more min...", "*snoring softly*", "don't wake me",
         "dreaming of fish", "...mrowww..."
```

**Data structure**: bash associative array keyed by `mood_index`:

```bash
declare -A _SPEECH=(
    [calm_0]="..." [calm_1]="..." [calm_2]="..."
    [happy_0]="..." [happy_1]="..."
    ...
)

phrase_idx=$(( (now / 30) % N ))    # N = phrase count for this mood
speech="${_SPEECH[${mood}_${phrase_idx}]}"
```

**Display placement** (requires feature 4 done first): new row 7, below the bottom rail, in the sprite column:

```
╔═╡ NETRUNNER // Thalia ╞══════╗    /\___/\  ~
║ MODEL ...                    ║   (  ^_^  )  )
║ CTX ... │ 5h ...             ║    \_ U _/  ~
║ RESET: ...                   ║    /|     |\
║ COST ... │ LINES ... │ TIME  ║   ( |     | )
║ LV ... │ COINS ... │ MOOD    ║     U   U
╚══════════════════════════════╝   " you got this "
```

---

### 6. Remove debug-logging overhead 🚀

**Problem**: The rolling `debug.log` append block in `statusline.sh` spawns subprocesses for `date`, `env | grep`, `wc -l`, `tail`, `mv` on every render. Adds ~700ms latency.

**Justification for removal**: The pipeline is now proven to work (`cat` stdin read is reliable, CC sends real JSON). We don't need continuous diagnostic capture anymore.

**What to keep**: The `last-input.json` write (useful for future schema changes), but gate it behind env var:

```bash
[ -n "${PURR_DEBUG:-}" ] && printf '%s\n' "$INPUT" > "$PURR_STATE_DIR/last-input.json" 2>/dev/null || true
```

**Expected latency recovery**: ~700ms → back to ~500ms. Getting closer to the sub-100ms target the Rust port will unlock.

---

## Constraints (don't break these)

- **Source/state separation**: scripts live in this repo; state stays at `~/.claude/purrgotchi/state.json`. Use `PURR_DIR` for source (computed from `$BASH_SOURCE`), `PURR_STATE_DIR` for runtime.
- **Width budget**: total line width = 16 sprite + 3 gap + 62 box = **81 chars**. Must not exceed.
- **No Unicode ambiguous-width chars in content** (⧖, ◆, emoji) — breaks padding math. Use ASCII or confirmed-single-col chars.
- **No new dependencies**: bash + jq + python (stdlib only). No additional pip/npm installs.
- **Performance**: render must stay under ~500ms. Prefer pure-bash operations over subprocesses.
- **Alignment invariant**: every rendered row must have the same visible-char count after ANSI stripping. Verify with `sed -r 's/\x1b\[[0-9;]*m//g' | awk '{print length}'` — all rows equal.

---

## Architecture reference

### Files

| File | Purpose |
|---|---|
| `statusline.sh` | Hot-path render (reads stdin JSON, outputs 7-line HUD + sprite) |
| `feed_hook.py` | PostToolUse hook — atomic state.json updates |
| `rename.sh` | Kitten rename utility (touches state only) |
| `lib/render.sh` | ANSI color constants + formatting helpers (progress_bar, human_duration, human_elapsed, short_cwd) |
| `lib/mood.sh` | Mood state machine (priority-ordered, first-match-wins) |
| `lib/sprites.sh` | Sprite file loader with fallback chain |
| `sprites/*.txt` | 11 hand-drawn 5-line ASCII sprites (6 moods × 1-2 frames) |

### Key variables

- `PURR_DIR` — this repo (source of libs, sprites). Computed from `$BASH_SOURCE`.
- `PURR_STATE_DIR` — `~/.claude/purrgotchi/` (runtime state). Hardcoded.
- `PURR_STATE` — `$PURR_STATE_DIR/state.json`.

### Known JSON payload fields (from CC, captured in `last-input.json`)

```json
{
  "session_id": "...",
  "transcript_path": "...",
  "cwd": "C:\\Users\\...",
  "session_name": "...",
  "model": { "id": "...", "display_name": "..." },
  "workspace": { "current_dir": "C:\\Users\\..." },
  "context_window": { "used_percentage": 25, ... },
  "cost": { "total_cost_usd": 0.04, "total_duration_ms": 144000,
            "total_lines_added": 0, "total_lines_removed": 0 },
  "rate_limits": {
    "five_hour": { "used_percentage": 50, "resets_at": 1776765600 },
    "seven_day": { "used_percentage": 8, "resets_at": 1776852000 }
  }
}
```

---

## Nice-to-haves (not blocking Phase 2)

- [x] `.gitattributes` forcing LF for shell scripts — **shipped in v1.0.1**
- [x] `install.sh` auto-configuring settings.json on new machines — **shipped in v1.0.1**
- [x] `CHANGELOG.md` for version tracking — **shipped in v1.0.1**
- [ ] Nerd Font glyph upgrades (gated behind `PURR_NERD=1` env var) — deferred to post-Phase-2
  - Replace labels `MODEL`, `DIR`, `GIT`, `TIME`, `COST`, `LV`, `COINS`, `FED`, `MOOD` with single-width icons (``, ``, ``, `󰔟`, ``, ``, ``, ``, `:3`)
  - Buys back ~20-30 columns inside the 60-char content area — room for new fields without shrinking bars
  - Requires user to install a Nerd Font (https://www.nerdfonts.com/) and set it in the terminal

---

## Future: Rust port (Phase 3)

Once Phase 2 is locked and proven stable, port `statusline.sh` and `feed_hook.py` to Rust:

- **Target latency**: 5–20ms per render (25–50× speedup over current bash)
- **Eliminates**: jq dependency, bash CRLF issues, IFS whitespace quirks, Python install requirement, subshell cost
- **Stack**: Rust + `serde_json` + stdlib `std::io::stdin`, no extra crates needed
- **Keep**: sprite `.txt` files on disk (text-editable), `state.json` as source of truth
- **Output**: single `purrgotchi.exe` binary. `settings.json` points at it directly, no `bash` or `py.exe` wrappers

---

## Testing checklist (for whatever Phase 2 implementation lands)

- [ ] Render with real CC payload — MODEL, DIR, GIT all populate correctly
- [ ] DIR truncates cleanly at 38 chars with `...` prefix
- [ ] RESET row shows real times (or `---` if payload lacks `resets_at`)
- [ ] TIME shows `Xh Ym Zs` for >1h sessions
- [ ] Thalia appears on RIGHT side of HUD
- [ ] Speech row appears on row 7, changes with mood, rotates every 30s
- [ ] All 6 moods trigger correctly: calm / alert / panic / hungry / sleep / happy
- [ ] Borders align perfectly — all rows same visible-char count
- [ ] Timing ≤ 500ms per render (without debug overhead)
- [ ] `state.json` not modified by render path — only by `feed_hook.py`
- [ ] Restart CC, verify live statusline renders correctly
