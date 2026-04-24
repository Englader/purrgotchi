# Purrgotchi v3.4.6 — C2 + C3 + C4 commands (design spec)

_Started 2026-04-24. Master plan reference: `docs/superpowers/plans/kitten-mechanics-master-plan.md`._

Bundles the remaining three items from the C-group of Phase 2:

- **C2** — `purrgotchi play` (coin-costing bond interaction)
- **C3** — `purrgotchi treat` (coin-costing hunger reset + minor bond)
- **C4** — `purrgotchi stats` (curated human-readable state view)

Follows directly from v3.4.5's C5+C1 bundle (bond stat + pet command). Reuses the same result-enum + cooldown + mood-window pattern established there.

---

## 1. Goals

- Ship the full C-group command surface in one release.
- Introduce the first coin-spending sinks (coins have accumulated since Phase 1 with no use).
- Extend the happy-mood window to three interaction signals (pet / play / treat), all converging on the existing `happy` sprite.
- Zero sprite authoring. Zero state-schema version bump. Only additive `serde(default)` fields.

## 2. Non-goals (explicit out-of-scope)

- **No new mood.** Master plan said C2 triggers "excited"; we reuse `happy` because dedicated excited sprites would balloon scope (~44 files). Noted as a possible future point release.
- **No D-group events** (D1–D5 stay for a future bundle).
- **No tool-call → coin rate rebalance** (stays +1 per call).
- **No `Cost:` line in stats card** — `state.lifetime_cost_usd` is dead data. Cost accumulator is deferred to v3.4.7 as its own focused patch (see master plan).
- **No HUD layout change** — BOND bar from v3.4.5 stays where it is.

## 3. Locked tuning constants

```rust
// in rust/src/bond.rs alongside the existing BOND_* constants
pub const PLAY_COST: u64 = 50;
pub const PLAY_BOND_BUMP: u8 = 20;
pub const PLAY_COOLDOWN_SEC: u64 = 6 * 3600;
pub const PLAY_HAPPY_MOOD_WINDOW_SEC: i64 = 180;

pub const TREAT_COST: u64 = 25;
pub const TREAT_BOND_BUMP: u8 = 5;
pub const TREAT_COOLDOWN_SEC: u64 = 4 * 3600;
pub const TREAT_HAPPY_MOOD_WINDOW_SEC: i64 = 120;
pub const TREAT_HUNGER_THRESHOLD_SEC: u64 = 3600;  // last_fed_at within this = "not hungry"
```

All values treated as tweakable — rebalancing is a one-PR constant change post-launch.

## 4. State schema (additive only)

```rust
// in rust/src/state.rs inside State struct
#[serde(default)] pub last_played_at: u64,
#[serde(default)] pub last_treated_at: u64,
```

- No version bump (stays at v2).
- Existing state.json files (v3.4.5-shaped) load with default `0` for both fields, interpreted as "never played / never treated" — same convention as `last_pet_at`.
- `State::base(now)` initializes both to `0`.

## 5. Module architecture

All interaction logic stays in `rust/src/bond.rs`. No new module. Rationale: `bond.rs` already owns the cooldown + decay + bump primitives and the result-enum pattern; splitting would churn shipped code for no real-world benefit.

New top-level items in `bond.rs`:

- Tuning constants (see §3)
- `PlayResult`, `TreatResult` enums (§6)
- `try_play(state: &mut State, now: u64) -> PlayResult`
- `try_treat(state: &mut State, now: u64) -> TreatResult`
- Unit tests covering all result paths
- `last_interaction_at(&State) -> u64` helper (see §6.1)

Existing `try_pet`, `BOND_*` constants unchanged. `effective_bond` **signature stays the same** but callers now pass the max-of-three timestamp instead of just `last_pet_at` (see §6.1).

### 5.1 Bond decay semantics — corrected

v3.4.5 anchors bond decay to `last_pet_at` only. That was fine when `pet` was the only interaction. With `play` and `treat` added, decay anchored to `last_pet_at` would be wrong: a user who plays daily but never pets would still appear "neglected" for decay purposes.

Fix: introduce a helper and use it everywhere bond is read for display or bumping:

```rust
pub fn last_interaction_at(s: &State) -> u64 {
    s.last_pet_at.max(s.last_played_at).max(s.last_treated_at)
}
```

Call sites updated to use `effective_bond(state.bond, last_interaction_at(state), now)`:
- `try_pet` (change: from `state.last_pet_at` to the helper)
- `try_play`, `try_treat` (new — use helper)
- `format_stats_card` (new)
- Any HUD code reading bond for display

The `effective_bond` "never" short-circuit (`if last_interaction_at == 0 { return bond; }`) still holds — means "never interacted at all", decay doesn't start until first interaction of any kind.

Tests must cover: play-only user never pets → decay still anchored correctly; treat-only user ditto; mixed interactions → decay anchored to the most recent.

## 6. Result enums

Shape mirrors existing `PetResult` for consistency.

```rust
pub enum PlayResult {
    Applied { old: u8, new: u8, capped: bool, cost: u64 },
    Cooldown { wait_secs: u64 },
    InsufficientCoins { need: u64, have: u64 },
}

pub enum TreatResult {
    Applied {
        old: u8,
        new: u8,
        capped: bool,
        cost: u64,
        was_hungry: bool,  // false = coins + cooldown consumed, no bond gain
    },
    Cooldown { wait_secs: u64 },
    InsufficientCoins { need: u64, have: u64 },
}
```

### Gate order (both commands)

1. **Cooldown check** — if within cooldown, return `Cooldown { wait_secs }`. No state change.
2. **Coin check** — if `state.coins < COST`, return `InsufficientCoins { need, have }`. No state change.
3. **Apply** — compute `effective` bond (decay applied), bump, cap at 100, debit coins, update last_*_at. For treat: also update `last_fed_at` (hunger reset) and set `was_hungry` based on pre-check of the old `last_fed_at`.

### Treat hunger-gate behavior (3a ruling)

```text
was_hungry = (now - state.last_fed_at) >= TREAT_HUNGER_THRESHOLD_SEC
```

- `was_hungry == true`: full effect. Bond +5. Hunger reset (`last_fed_at = now`). Cooldown starts. Coins debited.
- `was_hungry == false`: bond gain skipped (`new = old`). Hunger still reset (the kitten ate the treat, even if it wasn't hungry). Cooldown starts. Coins debited. Happy mood window still fires.

### Max-bond handling (3c ruling)

- `try_play` / `try_treat` at bond=100: apply anyway — coins debited, cooldown consumed, happy window fires. `new = 100`, `capped = true`. Mirrors `try_pet` behavior.

## 7. CLI wiring

### `main.rs`

Three new match arms below `"pet"`:

```rust
"play"  => { hooks::play();  process::exit(0); }
"treat" => { hooks::treat(); process::exit(0); }
"stats" => { hooks::stats(); process::exit(0); }
```

Update usage error string to include the new commands.

### `hooks.rs`

```rust
pub fn play() {
    let mut s = crate::hatch::load_or_hatch();
    let now = state::now_secs();
    let result = bond::try_play(&mut s, now);
    println!("{}", format_play_message(&s.name, &result));
    if matches!(result, PlayResult::Applied { .. }) {
        let _ = state::save_atomic(&s);
    }
}

pub fn treat() { /* symmetric, with TreatResult */ }

pub fn stats() {
    let s = crate::hatch::load_or_hatch();
    let now = state::now_secs();
    println!("{}", format_stats_card(&s, now));
    // no state mutation
}
```

Pure format helpers (`format_play_message`, `format_treat_message`, `format_stats_card`) live in `hooks.rs` so they get unit-tested without touching the filesystem.

## 8. Mood module change

`compute_mood` signature grows to accept three pet-like deltas:

```rust
pub fn compute_mood(
    ctx_pct: f64,
    last_fed_delta_sec: i64,
    last_pet_delta_sec: i64,
    last_played_delta_sec: i64,
    last_treated_delta_sec: i64,
) -> &'static str
```

The "happy" branch becomes a three-signal OR, each with its own window constant:

```rust
} else if (0..BOND_HAPPY_MOOD_WINDOW_SEC).contains(&last_pet_delta_sec)
       || (0..PLAY_HAPPY_MOOD_WINDOW_SEC).contains(&last_played_delta_sec)
       || (0..TREAT_HAPPY_MOOD_WINDOW_SEC).contains(&last_treated_delta_sec)
{
    "happy"
}
```

Priority order unchanged: `panic > alert > happy > sleep > hungry > feed-recent happy > calm`.

Negative delta = "never" sentinel convention preserved (existing tests cover this).

`statusline.rs` is updated to compute the two new deltas alongside `last_pet_delta` and pass them into `compute_mood`.

## 9. Message copy

Three-variant rotation per category, selected deterministically by `last_X_at % 3` so the same second always yields the same message (testable, non-flaky).

### Play applied (not capped, was_hungry irrelevant — play doesn't care about hunger)

1. `🎾 {name} pounces on the feather wand! Bond: {old} → {new} (+{delta}) · -{cost} coins`
2. `🎾 {name} chases the laser with frantic joy. Bond: {old} → {new} (+{delta}) · -{cost} coins`
3. `🎾 {name} bats the toy around triumphantly. Bond: {old} → {new} (+{delta}) · -{cost} coins`

### Play capped (bond already 100)

Same base rotation + trailing ` (max, was {old})` — mirrors pet's capped treatment.

### Treat applied, was_hungry=true

1. `🐟 {name} devours the treat. Bond: {old} → {new} (+{delta}) · -{cost} coins`
2. `🐟 {name} gobbles it up with a happy chirp. Bond: {old} → {new} (+{delta}) · -{cost} coins`
3. `🐟 {name} eats enthusiastically. Bond: {old} → {new} (+{delta}) · -{cost} coins`

### Treat applied, was_hungry=false (bond unchanged)

1. `🐟 {name} nibbles the treat but wasn't really hungry. Bond: {new} (no change) · -{cost} coins`
2. `🐟 {name} sniffs it, eats it anyway. Bond: {new} (no change) · -{cost} coins`
3. `🐟 {name} accepts politely but isn't hungry. Bond: {new} (no change) · -{cost} coins`

### Cooldown

- Play: `{name} is still winded from the last play. (next play in Xh YYm)`
- Treat: `{name} isn't ready for another treat yet. (next treat in Xh YYm)`

Time format reuses the h/m formatter already in `hooks::format_pet_message`.

### Insufficient coins

- Play: `not enough coins for play (need 50, have {have})`
- Treat: `not enough coins for a treat (need 25, have {have})`

### ANSI colors

Match existing pet-message palette: `C_PINK` heart/emoji, `C_YELLOW` numbers and delta, `C_BOLD` on the new bond value, `C_RESET` at end. Cost amount in `C_YELLOW`.

## 10. `purrgotchi stats` — curated view

```text
┌─ Purrgotchi ──────────────────────────────────┐
│ Identity                                       │
│   Name:       Thalia                           │
│   Breed:      Void Cat (legendary ★★★★★)       │
│   Born:       2026-04-21 (3 days ago)          │
│   Life stage: kitten (level 4)                 │
│                                                │
│ Stats                                          │
│   CURIOSITY  ███████░░ 80  ← peak              │
│   MISCHIEF   █████░░░░ 55                      │
│   LAZINESS   ██░░░░░░░ 15  ← dump              │
│   AFFECTION  ████░░░░░ 40                      │
│   WISDOM     ██████░░░ 65                      │
│                                                │
│ Relationship                                   │
│   Bond:       ███████░░ 72                     │
│   Last pet:   2h ago                           │
│   Last play:  5h ago                           │
│   Last treat: never                            │
│                                                │
│ Progress                                       │
│   Tool calls: 217                              │
│   Coins:      1340                             │
└────────────────────────────────────────────────┘
```

### Rules

- `Bond:` uses `effective_bond(state.bond, state.last_pet_at, now)` — shows the current decayed value, not the raw stored value.
- `Last pet/play/treat` fields show `human_elapsed()` format already used in statusline (reuses `render.rs`). `0` sentinel renders as `never`.
- `peak` / `dump` annotations derived from the existing peak/dump algorithm in `identity.rs` (read-only — no recomputation, use whatever indices Phase 1 already exposes).
- Stat bars: 9-cell `████░░░░░` proportional to value/100 (nearest whole cell), same visual grammar as BOND bar in H6.
- Rarity stars row in `Identity` reuses `★` / `✦` helpers from `colors.rs` / `statusline.rs`.
- Pure function: `format_stats_card(state: &State, now: u64) -> String` — unit-testable with no I/O.
- `Cost:` line explicitly omitted (see §2 non-goals).

### Distinct from `purrgotchi card`

`card` is the flavor-text identity card (personality blurb, sprite). `stats` is the numeric data view. Both live — they serve different purposes.

## 11. Testing strategy

TDD, mirrors v3.4.5. All tests pure (no filesystem, no network).

### `bond.rs` — ~20 new tests

Per new command (`try_play` / `try_treat`):
- First-time call (last_*_at == 0): applies from raw bond, no "never" bugs
- Within cooldown: returns `Cooldown`, state unchanged
- Exactly at cooldown boundary: allowed
- Insufficient coins: returns `InsufficientCoins`, state unchanged
- Cooldown gate beats coin gate (both-blocked state returns `Cooldown`)
- Decay applied before bump (one-day elapsed, bumps from effective not raw)
- Capping: bumping to 100 → `capped: false`; starting at 100 → `capped: true`
- Coin debit arithmetic (saturating, no underflow)

Treat-specific:
- `was_hungry=true` path (last_fed_at > 1h old): bond gain applied, hunger reset
- `was_hungry=false` path (last_fed_at < 1h old): bond unchanged, hunger still reset, coins/cooldown consumed
- Hunger-threshold boundary (exactly 3600s old): `was_hungry=true`

### `state.rs`

Add back-compat test: v3.4.5-shaped JSON (no `last_played_at` / `last_treated_at`) loads with both defaulting to `0`.

### `mood.rs`

Extend existing tests for the new signature. New cases:
- Recent play triggers happy
- Recent treat triggers happy
- Play window uses `PLAY_HAPPY_MOOD_WINDOW_SEC` (180s boundary tested)
- All three signals expired + calm feed state → calm
- Panic/alert still beats three-signal happy

### `hooks.rs`

Format tests per result variant for both commands: applied, cooldown, insufficient-coins, capped, was_hungry=false. Assert name, numbers, deltas, cost amount, and stable 3-variant rotation (same `last_X_at` value → same message).

Stats format tests:
- Identity section renders all fields correctly
- Stats section marks peak/dump correctly
- Bond uses effective (decayed) value, not raw
- `never` sentinel for unset timestamps
- Stat bars render correct cell counts

### Manual smoke

- Build release binary
- `purrgotchi play` — success, cooldown, insufficient-coins paths
- `purrgotchi treat` — full rotation including was_hungry=false path (wait 1h+ vs. immediate retry)
- `purrgotchi stats` — visual inspection of the card
- Live statusline: confirm happy mood overlay fires after `play` / `treat`

## 12. Release workflow

1. Branch `c2-c3-c4-commands` off `main`
2. TDD implementation following the plan (written by `/superpowers:writing-plans` next)
3. `/superpowers:verification-before-completion`
4. `/superpowers:requesting-code-review`
5. Bump `Cargo.toml` to `3.4.6`, update CHANGELOG, rebuild release binary
6. Merge to `main`, tag `v3.4.6`, push
7. Update master plan progress log (check the C2+C3+C4 box)
8. Delete feature branch
9. Queue v3.4.7 (cost accumulator) per the master plan note

---

## Open questions

None at spec time. All design decisions resolved in the v3.4.6 brainstorming session:

- Mood: reuse `happy` (no new sprites) — locked
- Economy: play 50c/+20/6h/180s, treat 25c/+5/4h/120s — locked
- Edge cases: treat-not-hungry charges coins + resets hunger but skips bond; insufficient-coins refuses; max-bond applies anyway — locked
- Play doesn't reset hunger — locked
- Gate order: cooldown → coins → apply — locked
- Stats format: curated sections, no raw JSON dump, no Cost line — locked
- Cost accumulator: deferred to v3.4.7 as its own patch — locked
