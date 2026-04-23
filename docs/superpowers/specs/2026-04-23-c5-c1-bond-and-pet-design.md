# Purrgotchi v3.4.5 — Bond stat + `pet` command (C5 + C1)

_Brainstormed 2026-04-23. Phase 2 Group C, items C5 and C1 bundled as one point release._

## Goal

Introduce **bond** — the first dynamic (non-identity) stat on the kitten — and the **`purrgotchi pet`** command that feeds it. Bond grows via deliberate interaction and decays over wall-clock time when neglected, giving the kitten the first real "living thing" feedback loop. Ships as v3.4.5.

This is the foundation for the rest of Group C (`play`, `treat`) and the D-group events that will also touch bond in future releases. Everything C2/C3 will need (cooldown gate, mood override, bond read/write) gets proven out here first.

## Guiding decisions (locked during brainstorming)

- **Hybrid model**: bond is `0..=100`, decays over time, gated by a per-action cooldown so it can't be spammed.
- **Daily rhythm**: cooldown 4h, pet bump +15, decay 5/day. Keeping bond maxed takes 2–3 deliberate pets per day; a week of silence drops to ~65%, a month to ~20%.
- **Lazy decay**: bond is stored raw; decay is computed on read from `now - last_pet_at`. No tick/cron. Saves exactly on `pet`.
- **Additive schema**: `bond` and `last_pet_at` ride `#[serde(default)]` onto the existing v2 state. No version bump, no re-hatch.
- **No AFFECTION coupling (yet)**: identity stats do not modulate bond mechanics in v1. Clean mental model; future tweak.
- **No `CelebrationKind` reuse**: post-pet "happy" mood is a time-window check inside `compute_mood`, not a render counter.

## Tuning constants

All bond math lives behind named constants in one place (`state.rs` or a new `bond.rs` module — TBD during planning):

```rust
const BOND_START: u8 = 50;                    // hatch default & migration default
const BOND_PET_BUMP: u8 = 15;                 // points added per successful pet
const BOND_PET_COOLDOWN_SEC: u64 = 4 * 3600;  // 4h gate between effective pets
const BOND_DECAY_PER_DAY: u32 = 5;            // points lost per full day of neglect
const BOND_HAPPY_MOOD_WINDOW_SEC: i64 = 120;  // how long "happy" mood lingers post-pet
```

Tuning is a single edit. If the daily-feel turns out too fast or too slow, we change numbers, not structure.

## State schema changes

Two additive fields on `State` (rust/src/state.rs):

```rust
#[serde(default = "default_bond")]
pub bond: u8,

#[serde(default)]
pub last_pet_at: u64,
```

- `default_bond()` returns 50. Existing v3.4.4 state files that lack the field load at 50 — upgraders are not punished with a half-starved bond.
- `last_pet_at: 0` means "never petted." Decay formula treats 0 as never-stamped and returns `bond` unchanged.
- `State::base(now)` sets `bond = BOND_START`, `last_pet_at = 0`.

No migration. No version bump. Old state.json files remain loadable by the old parser's `version == 2` check.

## Bond mechanics

### Effective bond (lazy decay)

```rust
pub fn effective_bond(bond: u8, last_pet_at: u64, now: u64) -> u8 {
    if last_pet_at == 0 { return bond; }
    let secs = now.saturating_sub(last_pet_at);
    let decay_points = (secs as u128 * BOND_DECAY_PER_DAY as u128 / 86_400) as u32;
    bond.saturating_sub(decay_points.min(u8::MAX as u32) as u8)
}
```

Pure function. Testable in isolation. Never mutates state. Used by the HUD, the card, and the `pet` command's pre/post display.

### Applying a pet

```rust
pub enum PetResult {
    Applied { old: u8, new: u8, capped: bool },
    Cooldown { wait_secs: u64 },
}

pub fn try_pet(state: &mut State, now: u64) -> PetResult {
    let elapsed = now.saturating_sub(state.last_pet_at);
    if state.last_pet_at != 0 && elapsed < BOND_PET_COOLDOWN_SEC {
        return PetResult::Cooldown { wait_secs: BOND_PET_COOLDOWN_SEC - elapsed };
    }
    let effective = effective_bond(state.bond, state.last_pet_at, now);
    let new = effective.saturating_add(BOND_PET_BUMP).min(100);
    let capped = effective == 100;
    state.bond = new;
    state.last_pet_at = now;
    PetResult::Applied { old: effective, new, capped }
}
```

Key subtlety: the pet is applied to **effective** bond, not the stored raw. Otherwise a user who pets after a long absence would get credit for decayed points they never "had." Post-pet, the raw `bond` field is stamped to the new value and `last_pet_at = now` — decay resets cleanly.

### Happy-mood override

In `mood.rs`, `compute_mood` gains one new argument and one new top-priority rule:

```rust
pub fn compute_mood(ctx_pct: f64, last_fed_delta_sec: i64, last_pet_delta_sec: i64) -> &'static str {
    if last_pet_delta_sec >= 0 && last_pet_delta_sec < BOND_HAPPY_MOOD_WINDOW_SEC {
        return "happy";
    }
    // ... existing panic/alert/sleep/hungry/calm logic unchanged
}
```

Callers (`statusline.rs`) compute `last_pet_delta_sec` the same way they compute `last_fed_delta`, then pass both in. The window (120s) is short enough that across typical render intervals it appears as a one-shot flash, and long enough that a pet reliably triggers at least one statusline render showing the happy sprite/speech even if the user pauses briefly after running `pet`.

This replaces the "reuse CelebrationKind render counter" idea — no write on render, no new enum variant, and happy mood composes cleanly with everything except the still-higher-priority panic/alert (which is correct: a context emergency beats a pet).

## `purrgotchi pet` command

### Wiring

- New arm in `main.rs` match: `"pet" => { hooks::pet(); process::exit(0); }`
- New function `hooks::pet()` (or a new `pet.rs` module if it grows; for v1 `hooks.rs` is fine):
  - `let mut s = hatch::load_or_hatch();`
  - `let now = state::now_secs();`
  - `match bond::try_pet(&mut s, now) { ... }` → print result, then `state::save_atomic(&s)` if Applied.
- Usage line in the `_` arm updates to: `usage: purrgotchi <statusline|feed|turn|rename|hatch|card|pet>`

### Output

**Applied, not capped:**
```
♥ Thalia purrs contentedly. Bond: 65 → 80 (+15)
```

**Applied, at cap (was already 100):**
```
♥ Thalia purrs contentedly. Bond: 100 (max)
```

**On cooldown:**
```
Thalia is already purring at you. (next pet in 2h 47m)
```

**No state yet** — `load_or_hatch` handles this; first pet on a fresh machine hatches the kitten then pets.

Heart glyph uses `C_PINK`. Number deltas use `C_YELLOW`. Wait-time uses `human_duration` / `human_elapsed` formatting from `render.rs` for consistency with the HUD.

## HUD changes (statusline)

Three layout shifts, all inside the already-existing 7-row 94-wide box. No sprite-column changes.

### H1 rail — rarity stars join the title, PEAK shifts right

```
╔═╡ NETRUNNER // Thalia ╞═ ★★★★★ ═══╡ OFF PEAK 9PM-3PM ╞═╗
```

- Stars render via existing `identity::rarity_stars(&rarity, shiny)`. Colored by existing `colors::rarity_color(&rarity, shiny, now)`. No label ("RARITY:").
- Shiny gets the `✦` prefix already built into `rarity_stars`.
- PEAK label's centering math in `peak_label`/H1 assembly changes target from `w2_center` (column 2 midpoint) to `w3_center` (column 3 midpoint: `1 + W1 + 3 + W2 + 3 + W3/2`).
- The rail between `Thalia ╞` and `╡ OFF PEAK` gets a single-space pad on each side of the stars so they sit as `═ ★★★★★ ═` inside the rail glyphs.
- H1 character-count tests update to cover both cases (peak + off-peak × with/without shiny).

### H6 col 3 — BOND replaces RARITY

```
 LV: 15   COINS: 748 │ MOOD: calm   FED: 53s │ BOND: ♥ ███████░░░  65
```

- Cell format: `BOND: ♥ {bar} {value}` where `{bar}` is a 10-char bar from `progress_bar(effective_bond as f64, 10)` (each block = 10 points).
- Heart in `C_PINK`; bar in `C_PINK`; number in `C_BOLD`.
- No change to H6 col 1 (LV/COINS) or col 2 (MOOD/FED).

### H2–H5 — no changes

### Statusline compute site

In `statusline::run`:
- Read `s.bond` and `s.last_pet_at`, compute `effective = bond::effective_bond(...)`.
- Compute `last_pet_delta` (analogous to `last_fed_delta`).
- Pass `last_pet_delta` into `compute_mood`.
- Use `effective` in the H6 col 3 render.

## Card changes

Insert a new divider + bond row between the identity stats block and the personality block:

```
...
║  WISDOM       ████████░░  80            ║
╠══════════════════════════════════════════╣
║  BOND         ♥ ██████░░░░   65          ║
╠══════════════════════════════════════════╣
║  "a personality blurb that wraps..."     ║
╚══════════════════════════════════════════╝
```

- Bond row uses the same `content = format!("  {:<11}{}  {:>3}", "BOND", bar, val)` shape as identity stats — one subtle difference: a heart glyph sits between the label pad and the bar, colored pink. Field width accounting in `card_line_widths_consistent` test needs an update to account for it.
- Bar uses the same `stat_bar(val)` function as identity stats (already exists in `card.rs`).
- Divider immediately before personality is removed — the new divider below BOND replaces it.

## Test plan

Unit tests (TDD):

1. `effective_bond` — never-petted (last_pet_at=0 → unchanged), fresh pet (delta=0 → unchanged), one-day decay (5 points), multi-day decay (floors at 0), saturating arithmetic (doesn't panic on extreme now).
2. `try_pet` — from mid-bond applies bump, from decayed state applies bump to effective value (not raw), saturates at 100 with `capped: true`, rejects within cooldown with correct `wait_secs`, allows after exactly `BOND_PET_COOLDOWN_SEC`.
3. `compute_mood` — returns "happy" when `last_pet_delta < 60`, falls through to existing logic at `>=60`, panic/alert still beat happy.
4. State serde — `v3.4.4`-shaped JSON (no bond/last_pet_at) loads with `bond=50`, `last_pet_at=0`. Round-trip of new fields preserves them.
5. HUD — H1 rail total width remains 94 chars for (peak × shiny) × (peak × non-shiny) × (off-peak × shiny) × (off-peak × non-shiny). H6 col 3 renders bond value at 0, 50, 100.
6. Card — contains "BOND", contains current bond value, `card_line_widths_consistent` still passes.

Manual verification (pre-merge):
- Run `PURR_FORCE_RARITY=legendary PURR_FORCE_SHINY=1 cargo run -- hatch --` then `cargo run -- card` — visually confirm stars in H1 rail, bond row in card.
- Run `cargo run -- pet` — confirm bond message, then `cargo run -- card` — confirm delta.
- Run `cargo run -- pet` twice within 4h — second call shows cooldown message.
- Set `last_pet_at` to 30 days ago in state.json by hand — confirm bond shows ~0 in HUD/card.

## What is explicitly *not* in this release

- **C2 `play`** (new "excited" mood + sprites, coin spending) — separate release.
- **C3 `treat`** (hunger reset, coin spending) — separate release.
- **C4 `stats`** (pretty-print dump) — separate release.
- **AFFECTION-modulated decay or bump** — defer; v1 keeps bond independent.
- **Bond-decay daily-ping speech** ("Thalia seems distant…") — tempting, but that's a D-item ("events") surface, not C.
- **Per-mood pet speech** — `pet`'s printed output is static for v1. Different phrases by stats/mood can layer on later without changing the command contract.

## Open questions

- ~~Bond decay rate~~: resolved — 5/day.
- ~~Cooldown length~~: resolved — 4h.
- ~~Display surface~~: resolved — card + statusline, with the layout shifts above.
- **Bond display color at low values**: should bond turn yellow/red below some threshold (e.g., <20 → red)? Likely yes, but deferring until v1 ships so we see how it feels at steady state. No spec decision now.
